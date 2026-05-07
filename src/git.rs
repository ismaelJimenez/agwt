use std::cell::Cell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::thread;

use anyhow::{Context, Result, bail};
use git2::{
    Cred, CredentialType, FetchOptions, FetchPrune, RemoteCallbacks, Repository, StatusOptions,
    WorktreeAddOptions,
};
use indicatif::{ProgressBar, ProgressStyle};

pub struct Worktree {
    pub path: PathBuf,
    pub name: String,
    pub branch: String,
    pub dirty: bool,
    pub ahead: u32,
    pub behind: u32,
    pub locked: bool,
    pub base: Option<String>,
}

pub fn resolve_bare_dir(explicit: Option<&Path>) -> Result<PathBuf> {
    if let Some(p) = explicit {
        let p = dunce::canonicalize(p)
            .with_context(|| format!("bare directory does not exist: {}", p.display()))?;
        return Ok(p);
    }

    let cwd = std::env::current_dir().context("cannot determine current directory")?;
    let mut dir = cwd.as_path();

    loop {
        let candidate = dir.join(".bare");
        if candidate.is_dir() && candidate.join("HEAD").exists() {
            return Ok(candidate);
        }

        match dir.parent() {
            Some(parent) => dir = parent,
            None => break,
        }
    }

    bail!(
        "Could not find a .bare repository directory.\n\
         Use --bare-dir to specify one, or run `agwt init <url>` to create one."
    )
}

pub fn parent_of_bare(bare_dir: &Path) -> PathBuf {
    bare_dir.parent().unwrap_or(bare_dir).to_path_buf()
}

pub fn git(bare_dir: &Path) -> Command {
    let mut cmd = Command::new("git");
    cmd.current_dir(bare_dir);
    cmd
}

pub fn run(cmd: &mut Command) -> Result<()> {
    let status = cmd
        .status()
        .with_context(|| format!("failed to execute: {:?}", cmd))?;
    if !status.success() {
        bail!("command failed with {}: {:?}", status, cmd);
    }
    Ok(())
}

pub fn run_output(cmd: &mut Command) -> Result<String> {
    let output = cmd
        .output()
        .with_context(|| format!("failed to execute: {:?}", cmd))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("command failed: {:?}\n{}", cmd, stderr);
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

pub fn get_default_branch(bare_dir: &Path) -> Result<String> {
    let repo = Repository::open_bare(bare_dir)
        .with_context(|| format!("failed to open bare repo: {}", bare_dir.display()))?;
    let head = repo.head().context("failed to read HEAD")?;
    let name = head
        .shorthand()
        .context("HEAD has no shorthand name")?
        .to_string();
    Ok(name)
}

pub fn list_worktrees(bare_dir: &Path, parent: &Path) -> Result<Vec<Worktree>> {
    let total_start = std::time::Instant::now();

    let repo = Repository::open_bare(bare_dir)
        .with_context(|| format!("failed to open bare repo: {}", bare_dir.display()))?;

    // Read config for all branch bases
    let t0 = std::time::Instant::now();
    let base_map = get_all_branch_bases_git2(&repo);
    let config_time = t0.elapsed();

    // Enumerate worktrees via libgit2
    let t0 = std::time::Instant::now();
    let raw_entries = enumerate_worktrees_git2(&repo, parent)?;
    let worktree_list_time = t0.elapsed();

    // Collect status for each worktree in parallel using libgit2
    let t0 = std::time::Instant::now();
    let handles: Vec<_> = raw_entries
        .into_iter()
        .map(|raw| {
            let base = base_map.get(&raw.branch).cloned();
            thread::spawn(move || {
                let (dirty, ahead, behind) = worktree_status_git2(&raw.path, &raw.branch);
                Worktree {
                    path: raw.path,
                    name: raw.name,
                    branch: raw.branch,
                    dirty,
                    ahead,
                    behind,
                    locked: raw.locked,
                    base,
                }
            })
        })
        .collect();

    let mut entries = Vec::with_capacity(handles.len());
    for handle in handles {
        entries.push(handle.join().expect("worktree status thread panicked"));
    }
    entries.sort_by(|a, b| a.name.cmp(&b.name));
    let status_time = t0.elapsed();

    if std::env::var_os("AGWT_TIMING").is_some() {
        eprintln!(
            "[timing] list_worktrees total={:?} (worktree_list={:?}, config_batch={:?}, status_per_wt={:?}, count={})",
            total_start.elapsed(),
            worktree_list_time,
            config_time,
            status_time,
            entries.len(),
        );
    }

    Ok(entries)
}

/// Lightweight worktree listing that only returns paths, names, and branches.
/// Skips expensive status checks and config lookups. Use when you only need
/// to iterate over worktrees (e.g. sync --all, doctor, remove --merged).
pub fn list_worktrees_basic(bare_dir: &Path, parent: &Path) -> Result<Vec<Worktree>> {
    let repo = Repository::open_bare(bare_dir)
        .with_context(|| format!("failed to open bare repo: {}", bare_dir.display()))?;
    let raw_entries = enumerate_worktrees_git2(&repo, parent)?;

    let mut entries: Vec<_> = raw_entries
        .into_iter()
        .map(|raw| Worktree {
            path: raw.path,
            name: raw.name,
            branch: raw.branch,
            dirty: false,
            ahead: 0,
            behind: 0,
            locked: raw.locked,
            base: None,
        })
        .collect();
    entries.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(entries)
}

struct RawEntry {
    path: PathBuf,
    name: String,
    branch: String,
    locked: bool,
}

/// Enumerate worktrees using libgit2 — no subprocess needed.
fn enumerate_worktrees_git2(repo: &Repository, parent: &Path) -> Result<Vec<RawEntry>> {
    let mut entries = Vec::new();

    let wt_names = repo
        .worktrees()
        .context("failed to list worktrees from repo")?;

    for wt_name in wt_names.iter() {
        let Some(name) = wt_name else { continue };
        let Ok(wt) = repo.find_worktree(name) else {
            continue;
        };
        let wt_path = wt.path().to_path_buf();
        if !wt_path.exists() {
            continue;
        }

        let locked = wt
            .is_locked()
            .map(|s| !matches!(s, git2::WorktreeLockStatus::Unlocked))
            .unwrap_or(false);

        // Open the worktree repo to get the branch
        let branch = match Repository::open(&wt_path) {
            Ok(wt_repo) => match wt_repo.head() {
                Ok(head) => {
                    if head.is_branch() {
                        head.shorthand().unwrap_or("(detached)").to_string()
                    } else {
                        "(detached)".to_string()
                    }
                }
                Err(_) => "(detached)".to_string(),
            },
            Err(_) => "(detached)".to_string(),
        };

        let dir_name = wt_path
            .strip_prefix(parent)
            .unwrap_or(&wt_path)
            .to_string_lossy()
            .into_owned();

        entries.push(RawEntry {
            path: wt_path,
            name: dir_name,
            branch,
            locked,
        });
    }

    Ok(entries)
}

/// Get branch config bases using libgit2 Config API — no subprocess.
fn get_all_branch_bases_git2(repo: &Repository) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let Ok(config) = repo.config() else {
        return map;
    };
    let Ok(mut entries) = config.entries(Some(r"^branch\..*\.agwt-base$")) else {
        return map;
    };
    while let Some(entry) = entries.next() {
        let Ok(entry) = entry else { continue };
        let Some(name) = entry.name() else { continue };
        let Some(value) = entry.value() else { continue };
        // name is "branch.BRANCH_NAME.agwt-base"
        if let Some(branch_name) = name
            .strip_prefix("branch.")
            .and_then(|s: &str| s.strip_suffix(".agwt-base"))
        {
            map.insert(branch_name.to_string(), value.to_string());
        }
    }
    map
}

/// Check worktree status using libgit2 — no subprocess.
fn worktree_status_git2(wt_path: &Path, branch: &str) -> (bool, u32, u32) {
    let Ok(repo) = Repository::open(wt_path) else {
        return (false, 0, 0);
    };

    // Check dirty status
    let mut opts = StatusOptions::new();
    opts.include_untracked(true)
        .recurse_untracked_dirs(false)
        .include_unmodified(false);
    let dirty = match repo.statuses(Some(&mut opts)) {
        Ok(statuses) => !statuses.is_empty(),
        Err(_) => false,
    };

    // Check ahead/behind
    let (ahead, behind) = get_ahead_behind_git2(&repo, branch);

    (dirty, ahead, behind)
}

fn get_ahead_behind_git2(repo: &Repository, branch: &str) -> (u32, u32) {
    let local_ref = format!("refs/heads/{branch}");
    let Ok(local_oid) = repo.refname_to_id(&local_ref) else {
        return (0, 0);
    };

    // Find upstream
    let Ok(local_branch) = repo.find_branch(branch, git2::BranchType::Local) else {
        return (0, 0);
    };
    let Ok(upstream) = local_branch.upstream() else {
        return (0, 0);
    };
    let Some(upstream_ref) = upstream.get().name() else {
        return (0, 0);
    };
    let Ok(upstream_oid) = repo.refname_to_id(upstream_ref) else {
        return (0, 0);
    };

    match repo.graph_ahead_behind(local_oid, upstream_oid) {
        Ok((ahead, behind)) => (ahead as u32, behind as u32),
        Err(_) => (0, 0),
    }
}

pub fn set_branch_base(bare_dir: &Path, branch: &str, base: &str) {
    if let Ok(repo) = Repository::open_bare(bare_dir) {
        if let Ok(mut config) = repo.config() {
            let _ = config.set_str(&format!("branch.{branch}.agwt-base"), base);
        }
    }
}

/// Get the current branch name for a worktree path using libgit2.
pub fn get_current_branch(wt_path: &Path) -> Result<String> {
    let repo = Repository::open(wt_path)
        .with_context(|| format!("not a git repo: {}", wt_path.display()))?;
    let head = repo.head().context("failed to read HEAD")?;
    head.shorthand()
        .map(|s| s.to_string())
        .with_context(|| "HEAD is detached".to_string())
}

/// Check if a path is inside a git repository.
pub fn is_git_dir(path: &Path) -> bool {
    Repository::open(path).is_ok()
}

/// Check if a local branch exists in the bare repo.
pub fn branch_exists(bare_dir: &Path, branch: &str) -> Result<bool> {
    let repo = Repository::open_bare(bare_dir)
        .with_context(|| format!("failed to open bare repo: {}", bare_dir.display()))?;
    Ok(repo.find_branch(branch, git2::BranchType::Local).is_ok())
}

/// Get all branches that are fully merged into the given target branch.
pub fn get_merged_branches(bare_dir: &Path, target: &str) -> Result<Vec<String>> {
    let repo = Repository::open_bare(bare_dir)
        .with_context(|| format!("failed to open bare repo: {}", bare_dir.display()))?;

    let target_ref = format!("refs/heads/{target}");
    let target_oid = repo
        .refname_to_id(&target_ref)
        .with_context(|| format!("branch '{target}' not found"))?;

    let mut merged = Vec::new();
    let branches = repo.branches(Some(git2::BranchType::Local))?;
    for branch_result in branches {
        let (branch, _) = branch_result?;
        let Some(name) = branch.name()? else { continue };
        if name == target {
            continue;
        }
        let Some(branch_oid) = branch.get().target() else {
            continue;
        };
        // A branch is "merged" if its tip is reachable from the target
        // (i.e., equal to or an ancestor of the target commit)
        if branch_oid == target_oid
            || repo
                .graph_descendant_of(target_oid, branch_oid)
                .unwrap_or(false)
        {
            merged.push(name.to_string());
        }
    }

    Ok(merged)
}

/// Get all branches whose upstream tracking branch is "gone" (deleted from remote).
pub fn get_gone_branches(bare_dir: &Path) -> Vec<String> {
    let Ok(repo) = Repository::open_bare(bare_dir) else {
        return Vec::new();
    };
    let Ok(branches) = repo.branches(Some(git2::BranchType::Local)) else {
        return Vec::new();
    };

    let mut gone = Vec::new();
    for branch_result in branches {
        let Ok((branch, _)) = branch_result else {
            continue;
        };
        let Ok(Some(name)) = branch.name() else {
            continue;
        };
        // Check if branch has an upstream configured but the upstream ref is gone
        let Ok(upstream_buf) = repo.branch_upstream_name(&format!("refs/heads/{name}")) else {
            // No upstream configured — not "gone"
            continue;
        };
        let Some(upstream_ref) = upstream_buf.as_str() else {
            continue;
        };
        // If the upstream ref doesn't resolve, the branch is "gone"
        if repo.refname_to_id(upstream_ref).is_err() {
            gone.push(name.to_string());
        }
    }
    gone
}

/// Delete a local branch by name. Returns true if successful.
pub fn delete_branch(bare_dir: &Path, branch: &str) -> bool {
    let Ok(repo) = Repository::open_bare(bare_dir) else {
        return false;
    };
    let Ok(mut br) = repo.find_branch(branch, git2::BranchType::Local) else {
        return false;
    };
    br.delete().is_ok()
}

/// Lock a worktree using libgit2.
pub fn lock_worktree(bare_dir: &Path, wt_path: &Path, reason: Option<&str>) -> Result<()> {
    let repo = Repository::open_bare(bare_dir)
        .with_context(|| format!("failed to open bare repo: {}", bare_dir.display()))?;
    let wt_name = find_worktree_name(&repo, wt_path)?;
    let wt = repo
        .find_worktree(&wt_name)
        .with_context(|| format!("worktree not found: {}", wt_path.display()))?;
    wt.lock(reason)
        .with_context(|| format!("failed to lock worktree: {}", wt_path.display()))?;
    Ok(())
}

/// Unlock a worktree using libgit2.
pub fn unlock_worktree(bare_dir: &Path, wt_path: &Path) -> Result<()> {
    let repo = Repository::open_bare(bare_dir)
        .with_context(|| format!("failed to open bare repo: {}", bare_dir.display()))?;
    let wt_name = find_worktree_name(&repo, wt_path)?;
    let wt = repo
        .find_worktree(&wt_name)
        .with_context(|| format!("worktree not found: {}", wt_path.display()))?;
    wt.unlock()
        .with_context(|| format!("failed to unlock worktree: {}", wt_path.display()))?;
    Ok(())
}

/// Find worktree name from its path.
fn find_worktree_name(repo: &Repository, wt_path: &Path) -> Result<String> {
    let wt_names = repo.worktrees().context("failed to list worktrees")?;
    for name in wt_names.iter() {
        let Some(name) = name else { continue };
        if let Ok(wt) = repo.find_worktree(name) {
            if wt.path() == wt_path {
                return Ok(name.to_string());
            }
        }
    }
    bail!("no worktree found at path: {}", wt_path.display())
}

/// Set upstream tracking for a branch.
pub fn set_branch_upstream(bare_dir: &Path, branch: &str, upstream: &str) -> Result<()> {
    let repo = Repository::open_bare(bare_dir)
        .with_context(|| format!("failed to open bare repo: {}", bare_dir.display()))?;
    let mut br = repo
        .find_branch(branch, git2::BranchType::Local)
        .with_context(|| format!("branch '{branch}' not found"))?;
    br.set_upstream(Some(upstream))
        .with_context(|| format!("failed to set upstream for '{branch}'"))?;
    Ok(())
}

// --- Network operations using libgit2 ---

/// Credential strategies the auth callback can try on each invocation.
#[derive(Debug, Clone, PartialEq, Eq)]
enum CredentialStrategy {
    /// Try SSH agent (first attempt only)
    SshAgent,
    /// Try file-based SSH keys
    SshKeyFiles,
    /// Try credential helper for userpass
    CredentialHelper,
    /// Try default credentials (Negotiate/NTLM)
    Default,
    /// No suitable credential found
    None,
    /// Too many attempts, give up
    TooManyAttempts,
}

/// Determine the ordered list of credential strategies for a given attempt number.
fn credential_strategies(attempt: u32, allowed_types: CredentialType) -> Vec<CredentialStrategy> {
    if attempt > 4 {
        return vec![CredentialStrategy::TooManyAttempts];
    }

    let mut strategies = Vec::new();

    if allowed_types.contains(CredentialType::SSH_KEY) {
        // Only try agent on the first attempt. If the callback fires again,
        // it means the previously returned credential was rejected (e.g. dead
        // SSH_AUTH_SOCK or agent keys not authorized on the server).
        if attempt == 1 {
            strategies.push(CredentialStrategy::SshAgent);
        }
        strategies.push(CredentialStrategy::SshKeyFiles);
    }

    if allowed_types.contains(CredentialType::USER_PASS_PLAINTEXT) {
        strategies.push(CredentialStrategy::CredentialHelper);
    }

    if allowed_types.contains(CredentialType::DEFAULT) {
        strategies.push(CredentialStrategy::Default);
    }

    if strategies.is_empty() {
        strategies.push(CredentialStrategy::None);
    }

    strategies
}

fn make_credentials_callback<'a>() -> RemoteCallbacks<'a> {
    let mut cb = RemoteCallbacks::new();
    // Track attempts to prevent infinite loops when the server rejects credentials.
    // libgit2 will repeatedly invoke this callback if auth fails.
    let attempts = Cell::new(0u32);
    cb.credentials(
        move |url: &str, username_from_url: Option<&str>, allowed_types: CredentialType| {
            let n = attempts.get() + 1;
            attempts.set(n);

            let strategies = credential_strategies(n, allowed_types);
            let user = username_from_url.unwrap_or("git");

            for strategy in strategies {
                match strategy {
                    CredentialStrategy::TooManyAttempts => {
                        return Err(git2::Error::from_str(
                            "authentication failed after multiple attempts",
                        ));
                    }
                    CredentialStrategy::SshAgent => {
                        if let Ok(cred) = Cred::ssh_key_from_agent(user) {
                            return Ok(cred);
                        }
                    }
                    CredentialStrategy::SshKeyFiles => {
                        let home = std::env::var("HOME").unwrap_or_default();
                        for key_name in &["id_ed25519", "id_rsa"] {
                            let key_path = PathBuf::from(&home).join(".ssh").join(key_name);
                            if key_path.exists() {
                                if let Ok(cred) = Cred::ssh_key(user, None, &key_path, None) {
                                    return Ok(cred);
                                }
                            }
                        }
                    }
                    CredentialStrategy::CredentialHelper => {
                        if let Ok(cfg) = git2::Config::open_default() {
                            if let Ok(cred) = Cred::credential_helper(&cfg, url, username_from_url)
                            {
                                return Ok(cred);
                            }
                        }
                    }
                    CredentialStrategy::Default => {
                        return Cred::default();
                    }
                    CredentialStrategy::None => {}
                }
            }

            Err(git2::Error::from_str("no suitable credential found"))
        },
    );
    cb
}

/// Create a progress bar for transfer operations.
fn make_transfer_progress_bar() -> Arc<ProgressBar> {
    let pb =
        ProgressBar::with_draw_target(Some(0), indicatif::ProgressDrawTarget::stderr_with_hz(20));
    pb.set_style(
        ProgressStyle::with_template(
            "       {bar:20.green/dim} {percent:>3}%  {pos}/{len} objects, {msg}",
        )
        .unwrap()
        .progress_chars("█░░"),
    );
    Arc::new(pb)
}

/// Finish the progress bar with a trailing blank line for spacing.
fn finish_progress(pb: &ProgressBar) {
    if pb.length() == Some(0) {
        pb.finish_and_clear();
    } else {
        pb.finish();
        eprintln!();
    }
}

/// Create FetchOptions with credential callbacks, transfer progress, and optional prune.
fn make_fetch_options(prune: bool, verbose: bool) -> (FetchOptions<'static>, Arc<ProgressBar>) {
    let mut cb = make_credentials_callback();
    let pb = make_transfer_progress_bar();
    let pb_clone = Arc::clone(&pb);
    cb.transfer_progress(move |stats| {
        let total = stats.total_objects() as u64;
        if pb_clone.length() != Some(total) {
            pb_clone.set_length(total);
        }
        pb_clone.set_position(stats.received_objects() as u64);
        let kib = stats.received_bytes() / 1024;
        pb_clone.set_message(format!("{kib} KiB"));
        true
    });
    if verbose {
        cb.sideband_progress(|data| {
            let msg = String::from_utf8_lossy(data);
            eprint!("remote: {msg}");
            true
        });
        cb.update_tips(|refname, old, new| {
            if old.is_zero() {
                eprintln!(" * [new ref]   {refname} -> {new}");
            } else {
                eprintln!("   {old:.7}..{new:.7} {refname}");
            }
            true
        });
    }
    let mut fo = FetchOptions::new();
    fo.remote_callbacks(cb);
    if prune {
        fo.prune(FetchPrune::On);
    }
    (fo, pb)
}

/// Fetch a specific remote (with prune) using libgit2.
pub fn fetch_remote(
    bare_dir: &Path,
    remote_name: &str,
    refspecs: &[&str],
    verbose: bool,
) -> Result<()> {
    let repo = Repository::open_bare(bare_dir)
        .with_context(|| format!("failed to open bare repo: {}", bare_dir.display()))?;
    let mut remote = repo
        .find_remote(remote_name)
        .with_context(|| format!("remote '{}' not found", remote_name))?;
    let (mut fo, pb) = make_fetch_options(true, verbose);
    remote
        .fetch(refspecs, Some(&mut fo), None)
        .with_context(|| format!("failed to fetch from '{}'", remote_name))?;
    finish_progress(&pb);
    Ok(())
}

/// Fetch all remotes (with prune) using libgit2.
pub fn fetch_all_remotes(bare_dir: &Path, verbose: bool) -> Result<()> {
    let repo = Repository::open_bare(bare_dir)
        .with_context(|| format!("failed to open bare repo: {}", bare_dir.display()))?;
    let remotes = repo.remotes().context("failed to list remotes")?;
    for remote_name in remotes.iter().flatten() {
        if verbose {
            eprintln!("Fetching {remote_name}...");
        }
        let mut remote = repo.find_remote(remote_name)?;
        let (mut fo, pb) = make_fetch_options(true, verbose);
        remote
            .fetch(&[] as &[&str], Some(&mut fo), None)
            .with_context(|| format!("failed to fetch from '{remote_name}'"))?;
        finish_progress(&pb);
    }
    Ok(())
}

/// Add a worktree for an existing local branch.
pub fn worktree_add(bare_dir: &Path, name: &str, path: &Path, branch: &str) -> Result<()> {
    let repo = Repository::open_bare(bare_dir)
        .with_context(|| format!("failed to open bare repo: {}", bare_dir.display()))?;
    let reference = repo
        .find_branch(branch, git2::BranchType::Local)
        .with_context(|| format!("branch '{}' not found", branch))?;
    let git_ref = reference.into_reference();
    let mut opts = WorktreeAddOptions::new();
    opts.reference(Some(&git_ref));
    repo.worktree(name, path, Some(&opts))
        .with_context(|| format!("failed to add worktree at {}", path.display()))?;
    Ok(())
}

/// Add a worktree with a new branch created from a start point (remote ref or commit).
pub fn worktree_add_new_branch(
    bare_dir: &Path,
    wt_name: &str,
    path: &Path,
    new_branch: &str,
    start_ref: &str,
) -> Result<()> {
    let repo = Repository::open_bare(bare_dir)
        .with_context(|| format!("failed to open bare repo: {}", bare_dir.display()))?;

    // Resolve the start point to a commit
    let start_oid = repo
        .refname_to_id(&format!("refs/remotes/{start_ref}"))
        .or_else(|_| repo.refname_to_id(&format!("refs/heads/{start_ref}")))
        .with_context(|| format!("cannot resolve start ref '{start_ref}'"))?;
    let commit = repo
        .find_commit(start_oid)
        .with_context(|| format!("cannot find commit for '{start_ref}'"))?;

    // Create the new branch pointing at the start commit
    let branch = repo
        .branch(new_branch, &commit, false)
        .with_context(|| format!("failed to create branch '{new_branch}'"))?;
    let git_ref = branch.into_reference();

    // Create the worktree with the new branch
    let mut opts = WorktreeAddOptions::new();
    opts.reference(Some(&git_ref));
    repo.worktree(wt_name, path, Some(&opts))
        .with_context(|| format!("failed to add worktree at {}", path.display()))?;
    Ok(())
}

/// Bare clone a repository using libgit2.
pub fn clone_bare(url: &str, dest: &Path, verbose: bool) -> Result<()> {
    let (fo, pb) = make_fetch_options(false, verbose);
    let mut builder = git2::build::RepoBuilder::new();
    builder.bare(true);
    builder.fetch_options(fo);
    builder
        .clone(url, dest)
        .with_context(|| format!("failed to clone '{}' into {}", url, dest.display()))?;
    finish_progress(&pb);
    Ok(())
}

/// Push a delete refspec to remove a remote branch.
pub fn push_delete_branch(
    bare_dir: &Path,
    remote_name: &str,
    branch: &str,
    verbose: bool,
) -> Result<()> {
    let repo = Repository::open_bare(bare_dir)
        .with_context(|| format!("failed to open bare repo: {}", bare_dir.display()))?;
    let mut remote = repo
        .find_remote(remote_name)
        .with_context(|| format!("remote '{}' not found", remote_name))?;
    let mut cb = make_credentials_callback();
    if verbose {
        cb.sideband_progress(|data| {
            let msg = String::from_utf8_lossy(data);
            eprint!("remote: {msg}");
            true
        });
        cb.push_update_reference(|refname, status| {
            if let Some(msg) = status {
                eprintln!(" ! [rejected] {refname} ({msg})");
            } else {
                eprintln!(" - [deleted] {refname}");
            }
            Ok(())
        });
    }
    let mut push_opts = git2::PushOptions::new();
    push_opts.remote_callbacks(cb);
    let refspec = format!(":refs/heads/{branch}");
    remote
        .push(&[&refspec], Some(&mut push_opts))
        .with_context(|| format!("failed to delete remote branch '{branch}'"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn credential_strategies_first_attempt_includes_agent() {
        let strategies = credential_strategies(1, CredentialType::SSH_KEY);
        assert_eq!(strategies[0], CredentialStrategy::SshAgent);
        assert_eq!(strategies[1], CredentialStrategy::SshKeyFiles);
    }

    #[test]
    fn credential_strategies_retry_skips_agent() {
        // On attempt 2+, agent should NOT be in the list — only key files
        for attempt in 2..=4 {
            let strategies = credential_strategies(attempt, CredentialType::SSH_KEY);
            assert!(
                !strategies.contains(&CredentialStrategy::SshAgent),
                "attempt {attempt} should not include SshAgent"
            );
            assert_eq!(strategies[0], CredentialStrategy::SshKeyFiles);
        }
    }

    #[test]
    fn credential_strategies_too_many_attempts() {
        let strategies = credential_strategies(5, CredentialType::SSH_KEY);
        assert_eq!(strategies, vec![CredentialStrategy::TooManyAttempts]);
    }

    #[test]
    fn credential_strategies_userpass() {
        let strategies = credential_strategies(1, CredentialType::USER_PASS_PLAINTEXT);
        assert_eq!(strategies, vec![CredentialStrategy::CredentialHelper]);
    }

    #[test]
    fn credential_strategies_combined_types() {
        let allowed = CredentialType::SSH_KEY | CredentialType::USER_PASS_PLAINTEXT;
        let strategies = credential_strategies(1, allowed);
        assert_eq!(
            strategies,
            vec![
                CredentialStrategy::SshAgent,
                CredentialStrategy::SshKeyFiles,
                CredentialStrategy::CredentialHelper,
            ]
        );

        // On retry, agent is skipped
        let strategies = credential_strategies(2, allowed);
        assert_eq!(
            strategies,
            vec![
                CredentialStrategy::SshKeyFiles,
                CredentialStrategy::CredentialHelper,
            ]
        );
    }

    #[test]
    fn credential_strategies_no_matching_type_returns_none() {
        // Empty credential type (nothing allowed)
        let strategies = credential_strategies(1, CredentialType::empty());
        assert_eq!(strategies, vec![CredentialStrategy::None]);
    }

    #[test]
    fn transfer_progress_bar_tracks_position() {
        let pb = make_transfer_progress_bar();
        pb.set_length(100);
        pb.set_position(50);
        pb.set_message("48 KiB".to_string());
        assert_eq!(pb.position(), 50);
        assert_eq!(pb.length(), Some(100));
        pb.finish();
        assert!(pb.is_finished());
    }
}
