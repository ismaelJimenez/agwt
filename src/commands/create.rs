use std::path::Path;

use anstream::eprintln;
use anyhow::{Result, bail};

use crate::git::{self, get_default_branch, parent_of_bare, set_branch_base};
use crate::{BOLD, GREEN};

pub fn cmd_create(
    bare_dir: &Path,
    name: &str,
    branch: &str,
    base: Option<&str>,
    remote: &str,
    verbose: bool,
) -> Result<()> {
    let target_dir = parent_of_bare(bare_dir).join(name);

    if target_dir.exists() {
        bail!("directory already exists: {}", target_dir.display());
    }

    let base_ref = match base {
        Some(b) => b.to_string(),
        None => get_default_branch(bare_dir)?,
    };

    // Fetch the base branch from the remote
    if git::fetch_remote(bare_dir, remote, &[&base_ref], verbose).is_err() {
        bail!("base branch '{base_ref}' not found on remote '{remote}'");
    }

    let remote_ref = format!("{}/{}", remote, base_ref);
    git::worktree_add_new_branch(bare_dir, name, &target_dir, branch, &remote_ref)?;

    set_branch_base(bare_dir, branch, &base_ref);

    // Set upstream tracking so `git push` works without arguments.
    // We set config directly because the remote tracking ref for the new
    // branch doesn't exist yet, so git2's set_upstream would fail.
    if let Ok(repo) = git2::Repository::open_bare(bare_dir) {
        if let Ok(mut config) = repo.config() {
            let _ = config.set_str(&format!("branch.{branch}.remote"), remote);
            let _ = config.set_str(
                &format!("branch.{branch}.merge"),
                &format!("refs/heads/{branch}"),
            );
        }
    }

    eprintln!(
        "{GREEN}{:>12}{GREEN:#} worktree {BOLD}{name}{BOLD:#} at {} (new branch {BOLD}{branch}{BOLD:#} from {BOLD}{base_ref}{BOLD:#})",
        "Created",
        target_dir.display(),
    );

    Ok(())
}
