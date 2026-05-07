use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};

pub struct Worktree {
    pub path: PathBuf,
    pub name: String,
    pub branch: String,
    pub dirty: bool,
    pub ahead: u32,
    pub behind: u32,
    pub locked: bool,
}

pub fn resolve_bare_dir(explicit: Option<&Path>) -> Result<PathBuf> {
    if let Some(p) = explicit {
        let p = dunce::canonicalize(p)
            .with_context(|| format!("bare directory does not exist: {}", p.display()))?;
        return Ok(p);
    }

    let cwd = std::env::current_dir().context("cannot determine current directory")?;
    let mut dir = cwd.as_path();

    loop {
        let candidate = dir.join(".bare");
        if candidate.is_dir() && candidate.join("HEAD").exists() {
            return Ok(candidate);
        }

        match dir.parent() {
            Some(parent) => dir = parent,
            None => break,
        }
    }

    bail!(
        "Could not find a .bare repository directory.\n\
         Use --bare-dir to specify one, or run `agwt init <url>` to create one."
    )
}

pub fn parent_of_bare(bare_dir: &Path) -> PathBuf {
    bare_dir.parent().unwrap_or(bare_dir).to_path_buf()
}

pub fn git(bare_dir: &Path) -> Command {
    let mut cmd = Command::new("git");
    cmd.current_dir(bare_dir);
    cmd
}

pub fn run(cmd: &mut Command) -> Result<()> {
    let status = cmd
        .status()
        .with_context(|| format!("failed to execute: {:?}", cmd))?;
    if !status.success() {
        bail!("command failed with {}: {:?}", status, cmd);
    }
    Ok(())
}

pub fn run_output(cmd: &mut Command) -> Result<String> {
    let output = cmd
        .output()
        .with_context(|| format!("failed to execute: {:?}", cmd))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("command failed: {:?}\n{}", cmd, stderr);
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

pub fn get_default_branch(bare_dir: &Path) -> Result<String> {
    let head = run_output(git(bare_dir).args(["symbolic-ref", "HEAD"]))?;
    let head = head.trim();
    head.strip_prefix("refs/heads/")
        .map(|s| s.to_string())
        .with_context(|| format!("unexpected HEAD format: {head}"))
}

pub fn list_worktrees(bare_dir: &Path, parent: &Path) -> Result<Vec<Worktree>> {
    let output = run_output(git(bare_dir).args(["worktree", "list", "--porcelain"]))?;
    let mut entries = Vec::new();

    let mut worktree_path: Option<PathBuf> = None;
    let mut branch: Option<String> = None;
    let mut is_bare = false;
    let mut locked = false;

    for line in output.lines().chain(std::iter::once("")) {
        if line.is_empty() {
            if let Some(wt_path) = worktree_path.take() {
                if !is_bare {
                    let name = wt_path
                        .strip_prefix(parent)
                        .unwrap_or(&wt_path)
                        .to_string_lossy()
                        .into_owned();
                    let br = branch.take().unwrap_or_else(|| "(detached)".into());
                    let (dirty, ahead, behind) = worktree_status(&wt_path, &br);
                    entries.push(Worktree {
                        path: wt_path,
                        name,
                        branch: br,
                        dirty,
                        ahead,
                        behind,
                        locked,
                    });
                }
            }
            is_bare = false;
            branch = None;
            locked = false;
            continue;
        }
        if let Some(path) = line.strip_prefix("worktree ") {
            worktree_path = Some(PathBuf::from(path));
        } else if line == "bare" {
            is_bare = true;
        } else if let Some(ref_name) = line.strip_prefix("branch refs/heads/") {
            branch = Some(ref_name.to_string());
        } else if line.starts_with("detached") {
            branch = Some("(detached)".into());
        } else if line == "locked" || line.starts_with("locked ") {
            locked = true;
        }
    }

    Ok(entries)
}

fn worktree_status(wt_path: &Path, branch: &str) -> (bool, u32, u32) {
    let dirty = Command::new("git")
        .current_dir(wt_path)
        .args(["status", "--porcelain"])
        .output()
        .ok()
        .map(|o| !o.stdout.is_empty())
        .unwrap_or(false);

    let (ahead, behind) = if branch != "(detached)" {
        Command::new("git")
            .current_dir(wt_path)
            .args(["rev-list", "--left-right", "--count", "HEAD...@{u}"])
            .output()
            .ok()
            .and_then(|o| {
                if !o.status.success() {
                    return None;
                }
                let s = String::from_utf8_lossy(&o.stdout);
                let mut parts = s.trim().split('\t');
                let a = parts.next()?.parse::<u32>().ok()?;
                let b = parts.next()?.parse::<u32>().ok()?;
                Some((a, b))
            })
            .unwrap_or((0, 0))
    } else {
        (0, 0)
    };

    (dirty, ahead, behind)
}
