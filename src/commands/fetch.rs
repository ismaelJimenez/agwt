use std::path::Path;

use anstream::eprintln;
use anyhow::Result;

use crate::GREEN;
use crate::git::{git, run};

pub fn cmd_fetch(bare_dir: &Path) -> Result<()> {
    run(git(bare_dir).args(["fetch", "--quiet", "--all", "--prune"]))?;
    eprintln!("{GREEN}{:>12}{GREEN:#} all remotes", "Fetched");
    Ok(())
}
