use std::path::Path;

use anstream::eprintln;
use anyhow::{Context, Result, bail};

use crate::git::{get_default_branch, git, parent_of_bare, run};
use crate::{BOLD, GREEN};

pub fn cmd_create(
    bare_dir: &Path,
    name: &str,
    branch: &str,
    base: Option<&str>,
    remote: &str,
) -> Result<()> {
    let target_dir = parent_of_bare(bare_dir).join(name);

    if target_dir.exists() {
        bail!("directory already exists: {}", target_dir.display());
    }

    let base_ref = match base {
        Some(b) => b.to_string(),
        None => get_default_branch(bare_dir)?,
    };

    let fetch_result = git(bare_dir)
        .args(["fetch", "--quiet", remote, &base_ref])
        .stderr(std::process::Stdio::null())
        .status()
        .with_context(|| "failed to execute git fetch".to_string())?;

    if !fetch_result.success() {
        bail!("base branch '{base_ref}' not found on remote '{remote}'");
    }

    let remote_ref = format!("{}/{}", remote, base_ref);
    run(git(bare_dir).args([
        "worktree",
        "add",
        "--quiet",
        "-b",
        branch,
        target_dir.to_str().unwrap(),
        &remote_ref,
    ]))?;

    eprintln!(
        "{GREEN}{:>12}{GREEN:#} worktree {BOLD}{name}{BOLD:#} at {} (new branch {BOLD}{branch}{BOLD:#} from {BOLD}{base_ref}{BOLD:#})",
        "Created",
        target_dir.display(),
    );

    Ok(())
}
