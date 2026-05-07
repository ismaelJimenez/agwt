use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;

use anyhow::{Context, Result, bail};

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
    let head = run_output(git(bare_dir).args(["symbolic-ref", "HEAD"]))?;
    let head = head.trim();
    head.strip_prefix("refs/heads/")
        .map(|s| s.to_string())
        .with_context(|| format!("unexpected HEAD format: {head}"))
}

pub fn list_worktrees(bare_dir: &Path, parent: &Path) -> Result<Vec<Worktree>> {
    let total_start = std::time::Instant::now();

    let t0 = std::time::Instant::now();
    let output = run_output(git(bare_dir).args(["worktree", "list", "--porcelain"]))?;
    let worktree_list_time = t0.elapsed();

    // Batch-load all agwt-base config values in a single git call
    let t0 = std::time::Instant::now();
    let base_map = get_all_branch_bases(bare_dir);
    let config_time = t0.elapsed();

    // Parse worktree list output
    struct RawEntry {
        path: PathBuf,
        name: String,
        branch: String,
        locked: bool,
    }

    let mut raw_entries = Vec::new();
    let mut worktree_path: Option<PathBuf> = None;
    let mut branch: Option<String> = None;
    let mut is_bare = false;
    let mut locked = false;

    for line in output.lines().chain(std::iter::once("")) {
        if line.is_empty() {
            if let Some(wt_path) = worktree_path.take() {
                if !is_bare {
                    let name = wt_path
                        .strip_prefix(parent)
                        .unwrap_or(&wt_path)
                        .to_string_lossy()
                        .into_owned();
                    let br = branch.take().unwrap_or_else(|| "(detached)".into());
                    raw_entries.push(RawEntry {
                        path: wt_path,
                        name,
                        branch: br,
                        locked,
                    });
                }
            }
            is_bare = false;
            branch = None;
            locked = false;
            continue;
        }
        if let Some(path) = line.strip_prefix("worktree ") {
            worktree_path = Some(PathBuf::from(path));
        } else if line == "bare" {
            is_bare = true;
        } else if let Some(ref_name) = line.strip_prefix("branch refs/heads/") {
            branch = Some(ref_name.to_string());
        } else if line.starts_with("detached") {
            branch = Some("(detached)".into());
        } else if line == "locked" || line.starts_with("locked ") {
            locked = true;
        }
    }

    // Collect status for each worktree in parallel
    let t0 = std::time::Instant::now();
    let handles: Vec<_> = raw_entries
        .into_iter()
        .map(|raw| {
            let base = base_map.get(&raw.branch).cloned();
            thread::spawn(move || {
                let (dirty, ahead, behind) = worktree_status(&raw.path, &raw.branch);
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
    let status_time = t0.elapsed();

    if std::env::var("AGWT_TIMING").is_ok() {
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
/// to iterate over worktrees (e.g. sync --all).
pub fn list_worktrees_basic(bare_dir: &Path, parent: &Path) -> Result<Vec<Worktree>> {
    let output = run_output(git(bare_dir).args(["worktree", "list", "--porcelain"]))?;

    let mut entries = Vec::new();
    let mut worktree_path: Option<PathBuf> = None;
    let mut branch: Option<String> = None;
    let mut is_bare = false;
    let mut locked = false;

    for line in output.lines().chain(std::iter::once("")) {
        if line.is_empty() {
            if let Some(wt_path) = worktree_path.take() {
                if !is_bare {
                    let name = wt_path
                        .strip_prefix(parent)
                        .unwrap_or(&wt_path)
                        .to_string_lossy()
                        .into_owned();
                    let br = branch.take().unwrap_or_else(|| "(detached)".into());
                    entries.push(Worktree {
                        path: wt_path,
                        name,
                        branch: br,
                        dirty: false,
                        ahead: 0,
                        behind: 0,
                        locked,
                        base: None,
                    });
                }
            }
            is_bare = false;
            branch = None;
            locked = false;
            continue;
        }
        if let Some(path) = line.strip_prefix("worktree ") {
            worktree_path = Some(PathBuf::from(path));
        } else if line == "bare" {
            is_bare = true;
        } else if let Some(ref_name) = line.strip_prefix("branch refs/heads/") {
            branch = Some(ref_name.to_string());
        } else if line.starts_with("detached") {
            branch = Some("(detached)".into());
        } else if line == "locked" || line.starts_with("locked ") {
            locked = true;
        }
    }

    Ok(entries)
}

/// Batch-fetch all branch.*.agwt-base config values in a single git call.
fn get_all_branch_bases(bare_dir: &Path) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let output = Command::new("git")
        .current_dir(bare_dir)
        .args(["config", "--get-regexp", r"^branch\..*\.agwt-base$"])
        .output()
        .ok();

    if let Some(o) = output {
        if o.status.success() {
            let text = String::from_utf8_lossy(&o.stdout);
            for line in text.lines() {
                // Format: "branch.BRANCH_NAME.agwt-base VALUE"
                if let Some((key, value)) = line.split_once(' ') {
                    if let Some(branch_name) = key
                        .strip_prefix("branch.")
                        .and_then(|s| s.strip_suffix(".agwt-base"))
                    {
                        map.insert(branch_name.to_string(), value.to_string());
                    }
                }
            }
        }
    }

    map
}

fn worktree_status(wt_path: &Path, _branch: &str) -> (bool, u32, u32) {
    // Use a single `git status --porcelain=v2 --branch` call to get both
    // dirty status and ahead/behind info, instead of two separate subprocess calls.
    let output = Command::new("git")
        .current_dir(wt_path)
        .args(["status", "--porcelain=v2", "--branch"])
        .output()
        .ok();

    let Some(o) = output else {
        return (false, 0, 0);
    };
    if !o.status.success() {
        return (false, 0, 0);
    }

    let stdout = String::from_utf8_lossy(&o.stdout);
    let mut dirty = false;
    let mut ahead: u32 = 0;
    let mut behind: u32 = 0;

    for line in stdout.lines() {
        if let Some(ab) = line.strip_prefix("# branch.ab ") {
            // Format: "+<ahead> -<behind>"
            for part in ab.split_whitespace() {
                if let Some(a) = part.strip_prefix('+') {
                    ahead = a.parse().unwrap_or(0);
                } else if let Some(b) = part.strip_prefix('-') {
                    behind = b.parse().unwrap_or(0);
                }
            }
        } else if !line.starts_with('#') {
            // Any non-header line means there are changes
            dirty = true;
        }
    }

    (dirty, ahead, behind)
}

pub fn set_branch_base(bare_dir: &Path, branch: &str, base: &str) {
    let _ = Command::new("git")
        .current_dir(bare_dir)
        .args(["config", &format!("branch.{branch}.agwt-base"), base])
        .status();
}
