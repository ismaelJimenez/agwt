use std::path::Path;

use anyhow::{Result, bail};

use crate::git::parent_of_bare;

pub fn cmd_cd(bare_dir: &Path, name: &str) -> Result<()> {
    let target_dir = parent_of_bare(bare_dir).join(name);
    if !target_dir.exists() {
        bail!(
            "worktree directory does not exist: {}",
            target_dir.display()
        );
    }
    println!("{}", target_dir.display());
    Ok(())
}
