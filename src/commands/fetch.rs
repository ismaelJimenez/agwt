use std::path::Path;

use anstream::eprintln;
use anyhow::Result;

use crate::GREEN;
use crate::git;

pub fn cmd_fetch(bare_dir: &Path) -> Result<()> {
    git::fetch_all_remotes(bare_dir)?;
    eprintln!("{GREEN}{:>12}{GREEN:#} all remotes", "Fetched");
    Ok(())
}
