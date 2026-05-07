use std::path::Path;
use std::process::Command;
use std::thread;

use anstream::eprintln;
use anyhow::{Result, bail};

use crate::git::{get_current_branch, is_git_dir, list_worktrees_basic, parent_of_bare};
use crate::{BOLD, GREEN, RED, YELLOW};

pub fn cmd_sync(bare_dir: &Path, name: Option<&str>, all: bool, remote: &str) -> Result<()> {
    if all {
        return cmd_sync_all(bare_dir, remote);
    }

    let target_dir = match name {
        Some(n) => parent_of_bare(bare_dir).join(n),
        None => {
            // Try to use cwd if it's inside a worktree
            let cwd = std::env::current_dir()?;
            if !is_git_dir(&cwd) {
                bail!("current directory is not inside a git worktree; specify a name");
            }
            cwd
        }
    };

    sync_one(&target_dir, name, remote)
}

fn cmd_sync_all(bare_dir: &Path, remote: &str) -> Result<()> {
    let parent = parent_of_bare(bare_dir);
    let entries = list_worktrees_basic(bare_dir, &parent)?;

    if entries.is_empty() {
        eprintln!("No worktrees to sync");
        return Ok(());
    }

    // Sync all worktrees in parallel since each operates on a separate directory
    let remote = remote.to_string();
    let handles: Vec<_> = entries
        .into_iter()
        .map(|entry| {
            let remote = remote.clone();
            thread::spawn(move || {
                let result = sync_one_quiet(&entry.path, &entry.name, &remote);
                (entry.name, entry.path, result)
            })
        })
        .collect();

    let mut failures = Vec::new();
    for handle in handles {
        let (name, path, result) = handle.join().expect("sync thread panicked");
        match result {
            Ok(branch) => {
                eprintln!(
                    "{GREEN}{:>12}{GREEN:#} worktree {BOLD}{name}{BOLD:#} (branch {BOLD}{branch}{BOLD:#})",
                    "Synced"
                );
            }
            Err(SyncError::Conflict) => {
                eprintln!(
                    "{YELLOW}{:>12}{YELLOW:#} worktree {BOLD}{name}{BOLD:#} has rebase conflicts",
                    "Conflict"
                );
                eprintln!();
                eprintln!("  Resolve conflicts in the worktree, then:");
                eprintln!("    cd {}", path.display());
                eprintln!("    git rebase --continue");
                eprintln!();
                eprintln!("  Or abort the rebase:");
                eprintln!("    git rebase --abort");
                failures.push(name);
            }
            Err(SyncError::Failed(e)) => {
                eprintln!(
                    "{RED}{:>12}{RED:#} worktree {BOLD}{name}{BOLD:#}: {e}",
                    "Failed"
                );
                failures.push(name);
            }
        }
    }

    if !failures.is_empty() {
        bail!(
            "sync failed for {} worktree(s): {}",
            failures.len(),
            failures.join(", ")
        );
    }

    Ok(())
}

enum SyncError {
    Conflict,
    Failed(String),
}

/// Sync a single worktree, returning the branch name on success.
/// This is a thread-safe version that doesn't print directly.
fn sync_one_quiet(
    target_dir: &Path,
    name: &str,
    remote: &str,
) -> std::result::Result<String, SyncError> {
    let branch = get_current_branch(target_dir)
        .map_err(|_| SyncError::Failed(format!("cannot determine branch for '{name}'")))?;

    let pull_status = Command::new("git")
        .current_dir(target_dir)
        .args([
            "pull",
            "--rebase",
            "--autostash",
            "--quiet",
            remote,
            &branch,
        ])
        .status()
        .map_err(|e| SyncError::Failed(format!("failed to run git pull: {e}")))?;

    if !pull_status.success() {
        let rebase_dir = target_dir.join(".git/rebase-merge");
        let rebase_apply = target_dir.join(".git/rebase-apply");
        if rebase_dir.exists() || rebase_apply.exists() {
            return Err(SyncError::Conflict);
        }
        return Err(SyncError::Failed(format!(
            "pull --rebase failed for worktree '{name}'"
        )));
    }

    Ok(branch)
}

fn sync_one(target_dir: &Path, name: Option<&str>, remote: &str) -> Result<()> {
    let branch = get_current_branch(target_dir)?;

    let pull_status = Command::new("git")
        .current_dir(target_dir)
        .args([
            "pull",
            "--rebase",
            "--autostash",
            "--quiet",
            remote,
            &branch,
        ])
        .status()?;

    let display_name = name.unwrap_or_else(|| target_dir.file_name().unwrap().to_str().unwrap());

    if !pull_status.success() {
        // Check if we're mid-rebase
        let rebase_dir = target_dir.join(".git/rebase-merge");
        let rebase_apply = target_dir.join(".git/rebase-apply");
        if rebase_dir.exists() || rebase_apply.exists() {
            eprintln!(
                "{YELLOW}{:>12}{YELLOW:#} worktree {BOLD}{display_name}{BOLD:#} has rebase conflicts",
                "Conflict"
            );
            eprintln!();
            eprintln!("  Resolve conflicts in the worktree, then:");
            eprintln!("    cd {}", target_dir.display());
            eprintln!("    git rebase --continue");
            eprintln!();
            eprintln!("  Or abort the rebase:");
            eprintln!("    git rebase --abort");
            bail!("rebase conflict in worktree '{display_name}'");
        }
        bail!("pull --rebase failed for worktree '{display_name}'");
    }

    eprintln!(
        "{GREEN}{:>12}{GREEN:#} worktree {BOLD}{display_name}{BOLD:#} (branch {BOLD}{branch}{BOLD:#})",
        "Synced"
    );
    Ok(())
}
