use std::io::Write;

use anstream::eprintln;
use anyhow::{Result, bail};

use crate::git::{self, get_default_branch, set_branch_upstream};
use crate::{BOLD, GREEN};

pub fn cmd_init(url: &str, name: Option<&str>, verbose: bool) -> Result<()> {
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
    git::clone_bare(url, &bare_dir, verbose)?;

    // Configure fetch refspec and push settings using libgit2
    // Then create remote tracking refs locally from the cloned refs/heads/*,
    // avoiding a network round-trip.
    {
        let repo = git2::Repository::open_bare(&bare_dir)?;
        let mut config = repo.config()?;
        config.set_str("remote.origin.fetch", "+refs/heads/*:refs/remotes/origin/*")?;
        config.set_str("push.autoSetupRemote", "true")?;

        // Create refs/remotes/origin/* from refs/heads/*
        let refs: Vec<(String, git2::Oid)> = repo
            .references_glob("refs/heads/*")?
            .filter_map(|r| {
                let r = r.ok()?;
                let name = r.name().ok()?.strip_prefix("refs/heads/")?.to_string();
                let oid = r.target()?;
                Some((name, oid))
            })
            .collect();
        for (name, oid) in &refs {
            let refname = format!("refs/remotes/origin/{name}");
            repo.reference(&refname, *oid, true, "init: create remote tracking ref")?;
        }
    }

    // Create a worktree for the default branch
    let default_branch = get_default_branch(&bare_dir)?;
    let wt_name = default_branch.replace('/', "-");
    let wt_dir = project_dir.join(&wt_name);

    eprintln!(
        "{GREEN}{:>12}{GREEN:#} default worktree {BOLD}{wt_name}{BOLD:#}...",
        "Creating"
    );
    let _ = std::io::stderr().flush();
    git::worktree_add(&bare_dir, &wt_name, &wt_dir, &default_branch)?;

    // Set upstream tracking for the default branch
    let _ = set_branch_upstream(
        &bare_dir,
        &default_branch,
        &format!("origin/{default_branch}"),
    );

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
