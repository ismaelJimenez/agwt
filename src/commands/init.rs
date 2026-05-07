use std::process::Command;

use anstream::eprintln;
use anyhow::{Result, bail};

use crate::git::{get_default_branch, git, run};
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

    // Fetch to populate remote tracking refs
    run(Command::new("git")
        .current_dir(&bare_dir)
        .args(["fetch", "--quiet", "origin"]))?;

    // Enable auto upstream so `git push` just works for new branches
    run(Command::new("git").current_dir(&bare_dir).args([
        "config",
        "push.autoSetupRemote",
        "true",
    ]))?;

    // Create a worktree for the default branch
    let default_branch = get_default_branch(&bare_dir)?;
    let wt_name = default_branch.replace('/', "-");
    let wt_dir = project_dir.join(&wt_name);

    run(git(&bare_dir).args([
        "worktree",
        "add",
        "--quiet",
        wt_dir.to_str().unwrap(),
        &default_branch,
    ]))?;

    // Set upstream tracking for the default branch
    run(Command::new("git").current_dir(&wt_dir).args([
        "branch",
        "--set-upstream-to",
        &format!("origin/{default_branch}"),
    ]))?;

    eprintln!(
        "{GREEN}{:>12}{GREEN:#} worktree {BOLD}{wt_name}{BOLD:#} at {}",
        "Created",
        wt_dir.display(),
    );

    eprintln!(
        "{GREEN}{:>12}{GREEN:#} cd into {BOLD}{folder_name}/{wt_name}{BOLD:#} to start working",
        "Done"
    );
    Ok(())
}
