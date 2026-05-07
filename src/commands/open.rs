use std::path::Path;
use std::process::Command;

use anstream::eprintln;
use anyhow::{Result, bail};

use crate::git::parent_of_bare;
use crate::{BOLD, GREEN};

pub fn cmd_open(bare_dir: &Path, name: &str, editor: Option<&str>) -> Result<()> {
    let target_dir = parent_of_bare(bare_dir).join(name);
    if !target_dir.exists() {
        bail!(
            "worktree directory does not exist: {}",
            target_dir.display()
        );
    }

    let editor = editor
        .map(|s| s.to_string())
        .or_else(|| std::env::var("VISUAL").ok())
        .or_else(|| std::env::var("EDITOR").ok())
        .unwrap_or_else(|| "code".to_string());

    let status = Command::new(&editor)
        .arg(target_dir.to_str().unwrap())
        .status()?;

    if !status.success() {
        bail!("editor '{editor}' exited with status {}", status);
    }

    eprintln!(
        "{GREEN}{:>12}{GREEN:#} worktree {BOLD}{name}{BOLD:#} in {BOLD}{editor}{BOLD:#}",
        "Opened"
    );
    Ok(())
}
