use std::path::Path;

use anstream::eprintln;
use anyhow::{Context, Result, bail};

use crate::git::{git, parent_of_bare, run};
use crate::{BOLD, GREEN};

pub fn cmd_checkout(bare_dir: &Path, name: &str, branch: &str, remote: &str) -> Result<()> {
    let target_dir = parent_of_bare(bare_dir).join(name);

    if target_dir.exists() {
        bail!("directory already exists: {}", target_dir.display());
    }

    let fetch_result = git(bare_dir)
        .args(["fetch", "--quiet", remote, branch])
        .stderr(std::process::Stdio::null())
        .status()
        .with_context(|| "failed to execute git fetch".to_string())?;

    if !fetch_result.success() {
        bail!(
            "branch '{branch}' not found on remote '{remote}'\n\
             \n\
             If you want to create a new branch, use:\n\
             \n\
             \x20   agwt create {branch}"
        );
    }

    let remote_ref = format!("{}/{}", remote, branch);
    let local_branch_exists = git(bare_dir)
        .args([
            "show-ref",
            "--verify",
            "--quiet",
            &format!("refs/heads/{}", branch),
        ])
        .status()
        .with_context(|| "failed to execute git show-ref")?
        .success();

    if local_branch_exists {
        run(git(bare_dir).args([
            "worktree",
            "add",
            "--quiet",
            target_dir.to_str().unwrap(),
            branch,
        ]))?;
    } else {
        run(git(bare_dir).args([
            "worktree",
            "add",
            "--quiet",
            "--track",
            "-b",
            branch,
            target_dir.to_str().unwrap(),
            &remote_ref,
        ]))?;
    }

    eprintln!(
        "{GREEN}{:>12}{GREEN:#} worktree {BOLD}{name}{BOLD:#} at {} (tracking {remote}/{branch})",
        "Created",
        target_dir.display(),
    );

    Ok(())
}
