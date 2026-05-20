use std::path::Path;

use anstream::eprintln;
use anyhow::{Result, bail};

use crate::git::{
    self, get_current_branch, get_default_branch, git, list_worktrees_basic, parent_of_bare, run,
};
use crate::{BOLD, GREEN, RED, YELLOW};

pub fn cmd_remove(
    bare_dir: &Path,
    name: &str,
    force: bool,
    delete_remote: bool,
    remote: &str,
    verbose: bool,
) -> Result<()> {
    let target_dir = parent_of_bare(bare_dir).join(name);

    if !target_dir.exists() {
        bail!(
            "worktree directory does not exist: {}",
            target_dir.display()
        );
    }

    // Get the branch name before removing
    let branch = get_current_branch(&target_dir).ok();

    // Confirm removal unless --force is passed
    if !force {
        let branch_info = branch
            .as_deref()
            .map(|b| format!(" (branch '{b}')"))
            .unwrap_or_default();
        eprint!("Remove worktree '{name}'{branch_info}? [y/N] ");
        let mut answer = String::new();
        std::io::stdin().read_line(&mut answer)?;
        if !answer.trim().eq_ignore_ascii_case("y") {
            eprintln!(
                "{YELLOW}{:>12}{YELLOW:#} removal of {BOLD}{name}{BOLD:#}",
                "Skipped"
            );
            return Ok(());
        }
    }

    let mut args = vec!["worktree", "remove"];
    if force {
        args.push("--force");
    }
    let target_str = target_dir.to_string_lossy().into_owned();
    args.push(&target_str);

    run(git(bare_dir).args(&args))?;
    eprintln!(
        "{GREEN}{:>12}{GREEN:#} worktree {BOLD}{name}{BOLD:#}",
        "Removed"
    );

    // Clean up the local branch (skip protected branches)
    if let Some(ref b) = branch {
        if b != "HEAD" && b != "master" && b != "main" {
            if git::delete_branch(bare_dir, b) {
                eprintln!("{GREEN}{:>12}{GREEN:#} branch {BOLD}{b}{BOLD:#}", "Deleted");
            }

            // Delete remote branch with confirmation
            if delete_remote {
                eprint!("Delete remote branch '{remote}/{b}'? [y/N] ");
                let mut answer = String::new();
                std::io::stdin().read_line(&mut answer)?;
                if answer.trim().eq_ignore_ascii_case("y") {
                    if git::push_delete_branch(bare_dir, remote, b, verbose).is_ok() {
                        eprintln!(
                            "{GREEN}{:>12}{GREEN:#} remote branch {BOLD}{remote}/{b}{BOLD:#}",
                            "Deleted"
                        );
                    } else {
                        eprintln!(
                            "{RED}{:>12}{RED:#} failed to delete remote branch {BOLD}{remote}/{b}{BOLD:#}",
                            "error"
                        );
                    }
                } else {
                    eprintln!("{YELLOW}{:>12}{YELLOW:#} remote branch deletion", "Skipped");
                }
            }
        }
    }

    // Auto-prune stale worktree references using libgit2
    if let Ok(repo) = git2::Repository::open_bare(bare_dir) {
        if let Ok(wt_names) = repo.worktrees() {
            for wt_name in wt_names.iter().flatten() {
                if let Ok(wt) = repo.find_worktree(wt_name) {
                    if wt.is_prunable(None).unwrap_or(false) {
                        let _ = wt.prune(None);
                    }
                }
            }
        }
    }

    Ok(())
}

pub fn cmd_remove_merged(
    bare_dir: &Path,
    force: bool,
    delete_remote: bool,
    remote: &str,
    verbose: bool,
) -> Result<()> {
    let default_branch = get_default_branch(bare_dir)?;
    let parent = parent_of_bare(bare_dir);
    let entries = list_worktrees_basic(bare_dir, &parent)?;

    // Get all merged branches using libgit2
    let merged_branches: std::collections::HashSet<String> =
        git::get_merged_branches(bare_dir, &default_branch)
            .unwrap_or_default()
            .into_iter()
            .collect();

    // Find worktrees whose branches are fully merged into the default branch
    let mut merged_names = Vec::new();
    for entry in &entries {
        if entry.branch == "(detached)" || entry.branch == default_branch {
            continue;
        }
        if merged_branches.contains(&entry.branch) {
            merged_names.push(entry.name.clone());
        }
    }

    if merged_names.is_empty() {
        eprintln!("No merged worktrees to remove");
        return Ok(());
    }

    eprintln!(
        "Found {} merged worktree(s): {}",
        merged_names.len(),
        merged_names.join(", ")
    );

    // Confirm the batch removal unless --force
    if !force {
        eprint!("Remove all merged worktrees? [y/N] ");
        let mut answer = String::new();
        std::io::stdin().read_line(&mut answer)?;
        if !answer.trim().eq_ignore_ascii_case("y") {
            eprintln!(
                "{YELLOW}{:>12}{YELLOW:#} merged worktree removal",
                "Skipped"
            );
            return Ok(());
        }
    }

    for name in &merged_names {
        // Skip per-item confirmation since user already confirmed the batch
        if let Err(e) = cmd_remove(bare_dir, name, true, delete_remote, remote, verbose) {
            eprintln!(
                "{RED}{:>12}{RED:#} removing {BOLD}{name}{BOLD:#}: {e:#}",
                "Failed"
            );
        }
    }

    Ok(())
}
