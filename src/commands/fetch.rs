use std::path::Path;

use anstream::eprintln;
use anyhow::Result;

use crate::GREEN;
use crate::git;

pub fn cmd_fetch(bare_dir: &Path, verbose: bool) -> Result<()> {
    git::fetch_active_remotes(bare_dir, verbose)?;
    eprintln!("{GREEN}{:>12}{GREEN:#} active branches", "Fetched");
    Ok(())
}
