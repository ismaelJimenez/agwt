use std::path::Path;
use std::process::Command;

use anstream::eprintln;
use anyhow::{Context, Result};

use crate::git::{git, list_worktrees, parent_of_bare, run, run_output};
use crate::{BOLD, GREEN, YELLOW};

pub fn cmd_doctor(bare_dir: &Path) -> Result<()> {
    let mut issues = 0u32;
    let parent = parent_of_bare(bare_dir);

    // 0. Fetch all remotes with prune to get fresh state
    eprintln!("{GREEN}{:>12}{GREEN:#} all remotes...", "Fetching");
    let _ = git(bare_dir)
        .args(["fetch", "--quiet", "--all", "--prune"])
        .status();

    // 1. Prune stale worktree references
    let prune_check = git(bare_dir)
        .args(["worktree", "prune", "--dry-run"])
        .output()
        .context("failed to run git worktree prune --dry-run")?;
    let prune_output = String::from_utf8_lossy(&prune_check.stderr);
    if !prune_output.trim().is_empty() {
        for line in prune_output.trim().lines() {
            eprintln!("{YELLOW}{:>12}{YELLOW:#} {line}", "Stale");
            issues += 1;
        }
        run(git(bare_dir).args(["worktree", "prune"]))?;
        eprintln!("{GREEN}{:>12}{GREEN:#} stale worktree references", "Pruned");
    }

    // 2. Check each worktree for issues
    let entries = list_worktrees(bare_dir, &parent)?;

    for wt in &entries {
        if !wt.path.exists() {
            continue;
        }

        let b = &wt.branch;
        if b == "(detached)" {
            continue;
        }

        // Check upstream status using for-each-ref (detects "gone" after fetch --prune)
        let upstream_status = run_output(git(bare_dir).args([
            "for-each-ref",
            "--format=%(upstream:track,nobracket)",
            &format!("refs/heads/{b}"),
        ]));

        if let Ok(ref status_str) = upstream_status {
            let status_str = status_str.trim();
            if status_str == "gone" {
                eprintln!(
                    "{YELLOW}{:>12}{YELLOW:#} {} — branch '{b}' no longer exists on remote",
                    "Gone", wt.name
                );
                issues += 1;
                continue;
            }
        }

        // Check if branch has upstream at all
        let upstream = Command::new("git")
            .current_dir(&wt.path)
            .args(["rev-parse", "--abbrev-ref", &format!("{b}@{{upstream}}")])
            .output();

        if let Ok(out) = upstream {
            if !out.status.success() {
                eprintln!(
                    "{YELLOW}{:>12}{YELLOW:#} {} — branch '{b}' has no upstream; push to publish or remove if stale",
                    "Warning", wt.name
                );
                issues += 1;
                continue;
            }
        }

        // Check ahead/behind and dirty state
        let status = Command::new("git")
            .current_dir(&wt.path)
            .args(["status", "--porcelain=v2", "--branch"])
            .output();

        if let Ok(out) = status {
            let stdout = String::from_utf8_lossy(&out.stdout);
            for sline in stdout.lines() {
                if let Some(ab) = sline.strip_prefix("# branch.ab ") {
                    let parts: Vec<&str> = ab.split_whitespace().collect();
                    if parts.len() == 2 {
                        let ahead: i32 = parts[0].parse().unwrap_or(0);
                        let behind: i32 = parts[1].parse().unwrap_or(0);
                        if behind < 0 {
                            eprintln!(
                                "{YELLOW}{:>12}{YELLOW:#} {} — {} commit(s) behind upstream",
                                "Behind", wt.name, -behind
                            );
                            issues += 1;
                        }
                        if ahead > 0 {
                            eprintln!(
                                "{YELLOW}{:>12}{YELLOW:#} {} — {} commit(s) ahead (unpushed)",
                                "Ahead", wt.name, ahead
                            );
                            issues += 1;
                        }
                    }
                }
            }

            let has_changes = stdout.lines().any(|l| !l.starts_with('#'));
            if has_changes {
                eprintln!(
                    "{YELLOW}{:>12}{YELLOW:#} {} — uncommitted changes",
                    "Dirty", wt.name
                );
                issues += 1;
            }
        }
    }

    if issues == 0 {
        eprintln!("{GREEN}{:>12}{GREEN:#} no issues found", "OK");
    } else {
        eprintln!();
        eprintln!("{BOLD}{issues} issue(s) found{BOLD:#}");
    }

    Ok(())
}
