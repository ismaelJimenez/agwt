use assert_cmd::Command;
use tempfile::TempDir;

/// Apply test-specific git configuration via environment variables.
/// This ensures tests work regardless of the user's global git config
/// (e.g. safe.bareRepository=explicit would otherwise block bare repo access in temp dirs).
/// Uses GIT_CONFIG_COUNT/KEY/VALUE which works on Windows, Mac, and Linux (Git 2.31+).
fn apply_test_git_env(cmd: &mut std::process::Command) {
    cmd.env("GIT_CONFIG_COUNT", "1")
        .env("GIT_CONFIG_KEY_0", "safe.bareRepository")
        .env("GIT_CONFIG_VALUE_0", "all");
}

#[allow(dead_code)]
/// Helper: build an `agwt` command pointing at a specific bare dir.
/// Injects test git config so bare repos in temp dirs are allowed.
pub fn gwt(bare_dir: &std::path::Path) -> Command {
    let mut cmd = Command::cargo_bin("agwt").unwrap();
    cmd.env("GIT_CONFIG_COUNT", "1")
        .env("GIT_CONFIG_KEY_0", "safe.bareRepository")
        .env("GIT_CONFIG_VALUE_0", "all");
    cmd.arg("--bare-dir").arg(bare_dir);
    cmd
}

#[allow(dead_code)]
/// Helper: build a `git` command with test-safe configuration applied.
pub fn git_cmd() -> std::process::Command {
    let mut cmd = std::process::Command::new("git");
    apply_test_git_env(&mut cmd);
    cmd
}

#[allow(dead_code)]
/// Helper: build an `agwt` command with test-safe git configuration.
/// Use this for calls that don't need `--bare-dir` (e.g. `init`).
pub fn agwt_cmd() -> Command {
    let mut cmd = Command::cargo_bin("agwt").unwrap();
    cmd.env("GIT_CONFIG_COUNT", "1")
        .env("GIT_CONFIG_KEY_0", "safe.bareRepository")
        .env("GIT_CONFIG_VALUE_0", "all");
    cmd
}

/// Helper: create a local bare repo seeded with a commit on main.
/// Returns (TempDir, remote_path). The TempDir must be kept alive.
pub fn setup_local_remote() -> (TempDir, std::path::PathBuf) {
    let remote_tmp = TempDir::new().unwrap();
    let remote_path = remote_tmp.path().join("remote.git");
    git_cmd()
        .args(["init", "--bare"])
        .arg(&remote_path)
        .output()
        .unwrap();

    // Seed it with a commit on main via a temporary working copy
    let seed_tmp = TempDir::new().unwrap();
    let seed_dir = seed_tmp.path().join("seed");
    git_cmd()
        .args(["clone", remote_path.to_str().unwrap(), "seed"])
        .current_dir(seed_tmp.path())
        .output()
        .unwrap();
    std::fs::write(seed_dir.join("README.md"), "# test\n").unwrap();
    git_cmd()
        .args(["add", "."])
        .current_dir(&seed_dir)
        .output()
        .unwrap();
    git_cmd()
        .args([
            "-c",
            "user.email=test@test.com",
            "-c",
            "user.name=Test",
            "commit",
            "-m",
            "init",
        ])
        .current_dir(&seed_dir)
        .output()
        .unwrap();
    git_cmd()
        .args(["branch", "-M", "main"])
        .current_dir(&seed_dir)
        .output()
        .unwrap();
    git_cmd()
        .args(["push", "-u", "origin", "main"])
        .current_dir(&seed_dir)
        .output()
        .unwrap();
    // Set HEAD on the bare remote to main
    git_cmd()
        .args(["symbolic-ref", "HEAD", "refs/heads/main"])
        .current_dir(&remote_path)
        .output()
        .unwrap();

    (remote_tmp, remote_path)
}

/// Helper: init a fresh project using a local remote.
/// Returns (remote_tmp, project_tmp, bare_dir PathBuf).
/// Both TempDirs must be kept alive for the duration of the test.
pub fn init_fresh() -> (TempDir, TempDir, std::path::PathBuf) {
    let (remote_tmp, remote_path) = setup_local_remote();

    let project_tmp = TempDir::new().unwrap();
    agwt_cmd()
        .args(["init", remote_path.to_str().unwrap(), "--name", "agwt"])
        .current_dir(project_tmp.path())
        .assert()
        .success();

    let bare_dir = project_tmp.path().join("agwt").join(".bare");
    (remote_tmp, project_tmp, bare_dir)
}

#[allow(dead_code)]
/// Helper: init a fresh project with a second remote named "upstream".
/// Returns (remote_tmp, upstream_tmp, project_tmp, bare_dir PathBuf).
pub fn init_fresh_with_second_remote() -> (TempDir, TempDir, TempDir, std::path::PathBuf) {
    let (remote_tmp, project_tmp, bare_dir) = init_fresh();

    // Create a second bare repo as "upstream"
    let upstream_tmp = TempDir::new().unwrap();
    let upstream_path = upstream_tmp.path().join("upstream.git");
    git_cmd()
        .args(["init", "--bare"])
        .arg(&upstream_path)
        .output()
        .unwrap();
    // Seed it
    let seed_tmp = TempDir::new().unwrap();
    let seed_dir = seed_tmp.path().join("seed");
    git_cmd()
        .args(["clone", upstream_path.to_str().unwrap(), "seed"])
        .current_dir(seed_tmp.path())
        .output()
        .unwrap();
    std::fs::write(seed_dir.join("README.md"), "# upstream\n").unwrap();
    git_cmd()
        .args(["add", "."])
        .current_dir(&seed_dir)
        .output()
        .unwrap();
    git_cmd()
        .args([
            "-c",
            "user.email=test@test.com",
            "-c",
            "user.name=Test",
            "commit",
            "-m",
            "init",
        ])
        .current_dir(&seed_dir)
        .output()
        .unwrap();
    git_cmd()
        .args(["branch", "-M", "main"])
        .current_dir(&seed_dir)
        .output()
        .unwrap();
    git_cmd()
        .args(["push", "-u", "origin", "main"])
        .current_dir(&seed_dir)
        .output()
        .unwrap();
    git_cmd()
        .args(["symbolic-ref", "HEAD", "refs/heads/main"])
        .current_dir(&upstream_path)
        .output()
        .unwrap();

    // Add it as a remote in the bare repo
    git_cmd()
        .args(["remote", "add", "upstream", upstream_path.to_str().unwrap()])
        .current_dir(&bare_dir)
        .output()
        .unwrap();
    git_cmd()
        .args(["fetch", "--quiet", "upstream"])
        .current_dir(&bare_dir)
        .output()
        .unwrap();

    (remote_tmp, upstream_tmp, project_tmp, bare_dir)
}
