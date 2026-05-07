use std::path::Path;

use anstream::eprintln;
use anyhow::{Result, bail};

use crate::git::{git, parent_of_bare, run};
use crate::{BOLD, GREEN};

pub fn cmd_move(bare_dir: &Path, name: &str, new_name: &str) -> Result<()> {
    let parent = parent_of_bare(bare_dir);
    let source_dir = parent.join(name);
    let dest_dir = parent.join(new_name);

    if !source_dir.exists() {
        bail!(
            "worktree directory does not exist: {}",
            source_dir.display()
        );
    }

    if dest_dir.exists() {
        bail!("destination already exists: {}", dest_dir.display());
    }

    run(git(bare_dir).args([
        "worktree",
        "move",
        source_dir.to_str().unwrap(),
        dest_dir.to_str().unwrap(),
    ]))?;

    eprintln!(
        "{GREEN}{:>12}{GREEN:#} worktree {BOLD}{name}{BOLD:#} → {BOLD}{new_name}{BOLD:#}",
        "Moved"
    );
    Ok(())
}
