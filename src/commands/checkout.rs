use std::io::Write;
use std::path::Path;

use anstream::eprintln;
use anyhow::{Result, bail};

use crate::git::{self, branch_exists, parent_of_bare, set_branch_base, set_branch_upstream};
use crate::{BOLD, GREEN};

pub fn cmd_checkout(
    bare_dir: &Path,
    name: &str,
    branch: &str,
    base: Option<&str>,
    remote: &str,
    verbose: bool,
) -> Result<()> {
    let target_dir = parent_of_bare(bare_dir).join(name);

    if target_dir.exists() {
        bail!("directory already exists: {}", target_dir.display());
    }

    // Prune stale worktree admin entry if the directory is gone but .bare/worktrees/<name> remains
    prune_stale_worktree(bare_dir, name);

    // Fetch the specific branch from the remote
    if git::fetch_remote(bare_dir, remote, &[branch], verbose).is_err() {
        bail!(
            "branch '{branch}' not found on remote '{remote}'\n\
             \n\
             If you want to create a new branch, use:\n\
             \n\
             \x20   agwt create {branch}"
        );
    }

    let remote_ref = format!("{}/{}", remote, branch);
    let local_branch_exists = branch_exists(bare_dir, branch)?;

    eprintln!(
        "{GREEN}{:>12}{GREEN:#} worktree {BOLD}{name}{BOLD:#}...",
        "Creating"
    );
    let _ = std::io::stderr().flush();

    if local_branch_exists {
        git::worktree_add(bare_dir, name, &target_dir, branch)?;
        let _ = set_branch_upstream(bare_dir, branch, &remote_ref);
    } else {
        git::worktree_add_new_branch(bare_dir, name, &target_dir, branch, &remote_ref)?;
        let _ = set_branch_upstream(bare_dir, branch, &remote_ref);
    }

    if let Some(base) = base {
        set_branch_base(bare_dir, branch, base);
    }

    eprintln!(
        "{GREEN}{:>12}{GREEN:#} worktree {BOLD}{name}{BOLD:#} at {} (tracking {remote}/{branch})",
        "Created",
        target_dir.display(),
    );

    Ok(())
}

/// Remove a stale worktree admin entry (.bare/worktrees/<name>) if the worktree
/// directory no longer exists. This allows re-creating the worktree without manual cleanup.
fn prune_stale_worktree(bare_dir: &Path, name: &str) {
    if let Ok(repo) = git2::Repository::open_bare(bare_dir) {
        if let Ok(wt) = repo.find_worktree(name) {
            if wt.is_prunable(None).unwrap_or(false) {
                let _ = wt.prune(None);
            }
        }
    }
}
