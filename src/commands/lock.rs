use std::path::Path;

use anstream::eprintln;
use anyhow::{Result, bail};

use crate::git::{git, parent_of_bare, run};
use crate::{BOLD, GREEN, YELLOW};

pub fn cmd_lock(bare_dir: &Path, name: &str, reason: Option<&str>) -> Result<()> {
    let target_dir = parent_of_bare(bare_dir).join(name);
    if !target_dir.exists() {
        bail!(
            "worktree directory does not exist: {}",
            target_dir.display()
        );
    }

    let mut cmd = git(bare_dir);
    cmd.args(["worktree", "lock", target_dir.to_str().unwrap()]);
    if let Some(r) = reason {
        cmd.args(["--reason", r]);
    }
    run(&mut cmd)?;

    eprintln!(
        "{GREEN}{:>12}{GREEN:#} worktree {BOLD}{name}{BOLD:#}",
        "Locked"
    );
    Ok(())
}

pub fn cmd_unlock(bare_dir: &Path, name: &str) -> Result<()> {
    let target_dir = parent_of_bare(bare_dir).join(name);
    if !target_dir.exists() {
        bail!(
            "worktree directory does not exist: {}",
            target_dir.display()
        );
    }

    run(git(bare_dir).args(["worktree", "unlock", target_dir.to_str().unwrap()]))?;

    eprintln!(
        "{YELLOW}{:>12}{YELLOW:#} worktree {BOLD}{name}{BOLD:#}",
        "Unlocked"
    );
    Ok(())
}
