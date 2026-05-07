use std::process::Command;

use anstream::eprintln;
use anyhow::{Result, bail};

use crate::git::run;
use crate::{BOLD, GREEN};

pub fn cmd_init(url: &str, name: Option<&str>) -> Result<()> {
    let cwd = std::env::current_dir()?;

    let folder_name = match name {
        Some(n) => n.to_string(),
        None => {
            let base = url.rsplit(['/', '\\']).next().unwrap_or(url);
            base.strip_suffix(".git").unwrap_or(base).to_string()
        }
    };

    let project_dir = cwd.join(&folder_name);
    if project_dir.exists() {
        bail!("directory already exists: {}", project_dir.display());
    }
    std::fs::create_dir_all(&project_dir)?;

    let bare_dir = project_dir.join(".bare");

    eprintln!(
        "{GREEN}{:>12}{GREEN:#} bare repository into {}/...",
        "Cloning", folder_name
    );
    run(Command::new("git").args([
        "clone",
        "--bare",
        "--quiet",
        url,
        bare_dir.to_str().unwrap(),
    ]))?;

    // Configure fetch refspec so `git fetch` works properly
    run(Command::new("git").current_dir(&bare_dir).args([
        "config",
        "remote.origin.fetch",
        "+refs/heads/*:refs/remotes/origin/*",
    ]))?;

    // Enable auto upstream so `git push` just works for new branches
    run(Command::new("git").current_dir(&bare_dir).args([
        "config",
        "push.autoSetupRemote",
        "true",
    ]))?;

    eprintln!(
        "{GREEN}{:>12}{GREEN:#} cd into {BOLD}{folder_name}{BOLD:#} and use {BOLD}agwt create <branch>{BOLD:#} or {BOLD}agwt checkout <branch>{BOLD:#}",
        "Done"
    );
    Ok(())
}
