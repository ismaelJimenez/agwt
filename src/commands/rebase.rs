use std::io::{self, IsTerminal, Write};
use std::path::Path;
use std::process::Command;

use anstream::eprintln;
use anyhow::{Result, bail};

use crate::git::{
    get_branch_base, get_current_branch, get_default_branch, is_git_dir, parent_of_bare,
};
use crate::{BOLD, GREEN, YELLOW};

pub fn cmd_rebase(bare_dir: &Path, name: Option<&str>, remote: &str, verbose: bool) -> Result<()> {
    let target_dir = match name {
        Some(n) => parent_of_bare(bare_dir).join(n),
        None => {
            let cwd = std::env::current_dir()?;
            if !is_git_dir(&cwd) {
                bail!("current directory is not inside a git worktree; specify a name");
            }
            cwd
        }
    };

    let branch = get_current_branch(&target_dir)?;
    let display_name = name.unwrap_or_else(|| target_dir.file_name().unwrap().to_str().unwrap());

    let base = match get_branch_base(bare_dir, &branch) {
        Some(b) => b,
        None => {
            let default = get_default_branch(bare_dir)?;
            if !io::stdin().is_terminal() {
                bail!(
                    "no base configured for branch '{branch}' and stdin is not a terminal; \
                     set one with: git config branch.{branch}.agwt-base <base>"
                );
            }
            eprint!(
                "No base configured for branch '{branch}'. Use default branch '{default}'? [y/N] "
            );
            io::stderr().flush()?;
            let mut answer = String::new();
            io::stdin().read_line(&mut answer)?;
            if !answer.trim().eq_ignore_ascii_case("y") {
                bail!("aborted; set a base with: git config branch.{branch}.agwt-base <base>");
            }
            default
        }
    };

    // Fetch the base branch from remote
    if crate::git::fetch_remote(bare_dir, remote, &[&base], verbose).is_err() {
        bail!("failed to fetch base branch '{base}' from remote '{remote}'");
    }

    // Rebase onto remote/base
    let remote_ref = format!("{remote}/{base}");
    let mut args = vec!["rebase", "--autostash"];
    if !verbose {
        args.push("--quiet");
    }
    args.push(&remote_ref);

    let status = Command::new("git")
        .current_dir(&target_dir)
        .args(&args)
        .status()?;

    if !status.success() {
        // In a worktree, .git is a file pointing to the real git dir.
        // Resolve the actual git dir to check for rebase state.
        let git_dir_output = Command::new("git")
            .current_dir(&target_dir)
            .args(["rev-parse", "--git-dir"])
            .output();
        let git_dir = git_dir_output
            .ok()
            .and_then(|o| {
                if o.status.success() {
                    Some(target_dir.join(String::from_utf8_lossy(&o.stdout).trim()))
                } else {
                    None
                }
            })
            .unwrap_or_else(|| target_dir.join(".git"));

        let rebase_dir = git_dir.join("rebase-merge");
        let rebase_apply = git_dir.join("rebase-apply");
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
        bail!("rebase onto '{remote_ref}' failed for worktree '{display_name}'");
    }

    eprintln!(
        "{GREEN}{:>12}{GREEN:#} worktree {BOLD}{display_name}{BOLD:#} (branch {BOLD}{branch}{BOLD:#} onto {BOLD}{base}{BOLD:#})",
        "Rebased"
    );
    Ok(())
}
