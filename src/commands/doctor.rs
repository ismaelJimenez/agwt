use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::thread;

use anstream::eprintln;
use anyhow::Result;

use crate::git::{self, list_worktrees_basic, parent_of_bare};
use crate::{BOLD, GREEN, YELLOW};

pub fn cmd_doctor(bare_dir: &Path, verbose: bool) -> Result<()> {
    let mut issues = 0u32;
    let parent = parent_of_bare(bare_dir);

    // 0. Fetch active branches with prune to get fresh state
    eprintln!("{GREEN}{:>12}{GREEN:#} active branches...", "Fetching");
    let _ = git::fetch_active_remotes(bare_dir, verbose);

    // 1. Detect stale worktree references (directory removed outside agwt)
    if let Ok(repo) = git2::Repository::open_bare(bare_dir) {
        if let Ok(wt_names) = repo.worktrees() {
            for wt_name in wt_names.iter().filter_map(|n| n.ok().flatten()) {
                if let Ok(wt) = repo.find_worktree(wt_name) {
                    if wt.is_prunable(None).unwrap_or(false) {
                        let branch = read_worktree_branch(bare_dir, wt_name);
                        let hint = match &branch {
                            Some(b) => format!(
                                " — branch '{b}' still exists\n\
                                 {:>12}   restore: `agwt checkout {b}`\n\
                                 {:>12}   clean up: `agwt remove {wt_name}`",
                                "", ""
                            ),
                            None => format!("\n{:>12}   clean up: `agwt remove {wt_name}`", ""),
                        };
                        eprintln!(
                            "{YELLOW}{:>12}{YELLOW:#} {wt_name} — worktree directory was removed outside agwt{hint}",
                            "Stale"
                        );
                        issues += 1;
                    }
                }
            }
        }
    }

    // 2. Get all "gone" branches using libgit2
    let gone_branches: HashSet<String> = git::get_gone_branches(bare_dir).into_iter().collect();

    // 3. Check each worktree for issues in parallel
    let entries = list_worktrees_basic(bare_dir, &parent)?;

    let handles: Vec<_> = entries
        .into_iter()
        .filter(|wt| wt.path.exists() && wt.branch != "(detached)")
        .map(|wt| {
            let is_gone = gone_branches.contains(&wt.branch);
            thread::spawn(move || diagnose_worktree(&wt.path, &wt.name, &wt.branch, is_gone))
        })
        .collect();

    for handle in handles {
        let diagnostics = handle.join().expect("doctor thread panicked");
        for diag in diagnostics {
            issues += 1;
            match diag {
                Diagnostic::Gone(name, branch) => {
                    eprintln!(
                        "{YELLOW}{:>12}{YELLOW:#} {name} — branch '{branch}' no longer exists on remote",
                        "Gone"
                    );
                }
                Diagnostic::NoUpstream(name, branch) => {
                    eprintln!(
                        "{YELLOW}{:>12}{YELLOW:#} {name} — branch '{branch}' has no upstream; push to publish or remove if stale",
                        "Warning"
                    );
                }
                Diagnostic::Behind(name, count) => {
                    eprintln!(
                        "{YELLOW}{:>12}{YELLOW:#} {name} — {count} commit(s) behind upstream",
                        "Behind"
                    );
                }
                Diagnostic::Ahead(name, count) => {
                    eprintln!(
                        "{YELLOW}{:>12}{YELLOW:#} {name} — {count} commit(s) ahead (unpushed)",
                        "Ahead"
                    );
                }
                Diagnostic::Dirty(name) => {
                    eprintln!(
                        "{YELLOW}{:>12}{YELLOW:#} {name} — uncommitted changes",
                        "Dirty"
                    );
                }
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

/// Read the branch name from a stale worktree's admin HEAD file.
/// Returns `None` if the file is missing or doesn't contain a branch ref.
fn read_worktree_branch(bare_dir: &Path, wt_name: &str) -> Option<String> {
    let head_path = bare_dir.join("worktrees").join(wt_name).join("HEAD");
    let content = fs::read_to_string(head_path).ok()?;
    let trimmed = content.trim();
    trimmed
        .strip_prefix("ref: refs/heads/")
        .map(|b| b.to_string())
}

enum Diagnostic {
    Gone(String, String),
    NoUpstream(String, String),
    Behind(String, u32),
    Ahead(String, u32),
    Dirty(String),
}

/// Diagnose a single worktree using libgit2 — no subprocess needed.
fn diagnose_worktree(wt_path: &Path, name: &str, branch: &str, is_gone: bool) -> Vec<Diagnostic> {
    let mut diags = Vec::new();

    if is_gone {
        diags.push(Diagnostic::Gone(name.to_string(), branch.to_string()));
        return diags;
    }

    let Ok(repo) = git2::Repository::open(wt_path) else {
        return diags;
    };

    // Check upstream
    let has_upstream = if let Ok(local_branch) = repo.find_branch(branch, git2::BranchType::Local) {
        if local_branch.upstream().is_ok() {
            true
        } else {
            diags.push(Diagnostic::NoUpstream(name.to_string(), branch.to_string()));
            return diags;
        }
    } else {
        false
    };

    // Check ahead/behind
    if has_upstream {
        let local_ref = format!("refs/heads/{branch}");
        if let Ok(local_oid) = repo.refname_to_id(&local_ref) {
            if let Ok(local_branch) = repo.find_branch(branch, git2::BranchType::Local) {
                if let Ok(upstream) = local_branch.upstream() {
                    if let Ok(upstream_ref) = upstream.get().name() {
                        if let Ok(upstream_oid) = repo.refname_to_id(upstream_ref) {
                            if let Ok((ahead, behind)) =
                                repo.graph_ahead_behind(local_oid, upstream_oid)
                            {
                                if behind > 0 {
                                    diags.push(Diagnostic::Behind(name.to_string(), behind as u32));
                                }
                                if ahead > 0 {
                                    diags.push(Diagnostic::Ahead(name.to_string(), ahead as u32));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Check dirty status
    let mut opts = git2::StatusOptions::new();
    opts.include_untracked(true)
        .recurse_untracked_dirs(false)
        .include_unmodified(false);
    if let Ok(statuses) = repo.statuses(Some(&mut opts)) {
        if !statuses.is_empty() {
            diags.push(Diagnostic::Dirty(name.to_string()));
        }
    }

    diags
}
