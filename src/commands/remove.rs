use std::path::Path;
use std::process::Command;

use anstream::eprintln;
use anyhow::{Result, bail};

use crate::git::{get_default_branch, git, list_worktrees, parent_of_bare, run, run_output};
use crate::{BOLD, GREEN, RED, YELLOW};

pub fn cmd_remove(
    bare_dir: &Path,
    name: &str,
    force: bool,
    delete_remote: bool,
    remote: &str,
) -> Result<()> {
    let target_dir = parent_of_bare(bare_dir).join(name);

    if !target_dir.exists() {
        bail!(
            "worktree directory does not exist: {}",
            target_dir.display()
        );
    }

    // Get the branch name before removing
    let branch = run_output(Command::new("git").current_dir(&target_dir).args([
        "rev-parse",
        "--abbrev-ref",
        "HEAD",
    ]))
    .ok()
    .map(|s| s.trim().to_string());

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
            if git(bare_dir).args(["branch", "-D", b]).status()?.success() {
                eprintln!("{GREEN}{:>12}{GREEN:#} branch {BOLD}{b}{BOLD:#}", "Deleted");
            }

            // Delete remote branch with confirmation
            if delete_remote {
                eprint!("Delete remote branch '{remote}/{b}'? [y/N] ");
                let mut answer = String::new();
                std::io::stdin().read_line(&mut answer)?;
                if answer.trim().eq_ignore_ascii_case("y") {
                    let status = git(bare_dir)
                        .args(["push", remote, "--delete", b])
                        .status()?;
                    if status.success() {
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

    // Auto-prune stale worktree references
    let _ = git(bare_dir).args(["worktree", "prune"]).status();

    Ok(())
}

pub fn cmd_remove_merged(
    bare_dir: &Path,
    force: bool,
    delete_remote: bool,
    remote: &str,
) -> Result<()> {
    let default_branch = get_default_branch(bare_dir)?;
    let parent = parent_of_bare(bare_dir);
    let entries = list_worktrees(bare_dir, &parent)?;

    // Find worktrees whose branches are fully merged into the default branch
    let mut merged_names = Vec::new();
    for entry in &entries {
        if entry.branch == "(detached)" {
            continue;
        }
        // Skip the default branch itself
        if entry.branch == default_branch {
            continue;
        }
        // Check if the branch is merged into the default branch
        let output = git(bare_dir)
            .args(["branch", "--merged", &default_branch])
            .output();
        if let Ok(output) = output {
            let branches = String::from_utf8_lossy(&output.stdout);
            let is_merged = branches.lines().any(|line| {
                let trimmed = line
                    .trim()
                    .trim_start_matches("* ")
                    .trim_start_matches("+ ");
                trimmed == entry.branch
            });
            if is_merged {
                merged_names.push(entry.name.clone());
            }
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

    for name in &merged_names {
        if let Err(e) = cmd_remove(bare_dir, name, force, delete_remote, remote) {
            eprintln!(
                "{RED}{:>12}{RED:#} removing {BOLD}{name}{BOLD:#}: {e:#}",
                "Failed"
            );
        }
    }

    Ok(())
}
