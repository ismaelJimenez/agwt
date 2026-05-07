use std::path::Path;

use anstream::eprintln;
use anyhow::{Result, bail};

use crate::git::{self, parent_of_bare};
use crate::{BOLD, GREEN, YELLOW};

pub fn cmd_lock(bare_dir: &Path, name: &str, reason: Option<&str>) -> Result<()> {
    let target_dir = parent_of_bare(bare_dir).join(name);
    if !target_dir.exists() {
        bail!(
            "worktree directory does not exist: {}",
            target_dir.display()
        );
    }

    git::lock_worktree(bare_dir, &target_dir, reason)?;

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

    git::unlock_worktree(bare_dir, &target_dir)?;

    eprintln!(
        "{YELLOW}{:>12}{YELLOW:#} worktree {BOLD}{name}{BOLD:#}",
        "Unlocked"
    );
    Ok(())
}
