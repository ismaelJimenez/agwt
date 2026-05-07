use std::path::Path;
use std::process::Command;

use anstream::eprintln;
use anyhow::{Result, bail};

use crate::git::{list_worktrees, parent_of_bare, run_output};
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
            // Verify it's a git directory
            let status = Command::new("git")
                .current_dir(&cwd)
                .args(["rev-parse", "--git-dir"])
                .output()?;
            if !status.status.success() {
                bail!("current directory is not inside a git worktree; specify a name");
            }
            cwd
        }
    };

    sync_one(&target_dir, name, remote)
}

fn cmd_sync_all(bare_dir: &Path, remote: &str) -> Result<()> {
    let parent = parent_of_bare(bare_dir);
    let entries = list_worktrees(bare_dir, &parent)?;

    if entries.is_empty() {
        eprintln!("No worktrees to sync");
        return Ok(());
    }

    let mut failures = Vec::new();

    for entry in &entries {
        if let Err(e) = sync_one(&entry.path, Some(&entry.name), remote) {
            eprintln!(
                "{RED}{:>12}{RED:#} worktree {BOLD}{}{BOLD:#}: {e:#}",
                "Failed", entry.name
            );
            failures.push(entry.name.clone());
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

fn sync_one(target_dir: &Path, name: Option<&str>, remote: &str) -> Result<()> {
    let branch = run_output(Command::new("git").current_dir(target_dir).args([
        "rev-parse",
        "--abbrev-ref",
        "HEAD",
    ]))?;
    let branch = branch.trim();

    let pull_status = Command::new("git")
        .current_dir(target_dir)
        .args(["pull", "--rebase", "--autostash", "--quiet", remote, branch])
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
