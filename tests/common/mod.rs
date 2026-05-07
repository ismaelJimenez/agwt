use assert_cmd::Command;
use tempfile::TempDir;

#[allow(dead_code)]
/// Helper: build an `agwt` command pointing at a specific bare dir.
pub fn gwt(bare_dir: &std::path::Path) -> Command {
    let mut cmd = Command::cargo_bin("agwt").unwrap();
    cmd.arg("--bare-dir").arg(bare_dir);
    cmd
}

/// Helper: create a local bare repo seeded with a commit on main.
/// Returns (TempDir, remote_path). The TempDir must be kept alive.
pub fn setup_local_remote() -> (TempDir, std::path::PathBuf) {
    let remote_tmp = TempDir::new().unwrap();
    let remote_path = remote_tmp.path().join("remote.git");
    std::process::Command::new("git")
        .args(["init", "--bare"])
        .arg(&remote_path)
        .output()
        .unwrap();

    // Seed it with a commit on main via a temporary working copy
    let seed_tmp = TempDir::new().unwrap();
    let seed_dir = seed_tmp.path().join("seed");
    std::process::Command::new("git")
        .args(["clone", remote_path.to_str().unwrap(), "seed"])
        .current_dir(seed_tmp.path())
        .output()
        .unwrap();
    std::fs::write(seed_dir.join("README.md"), "# test\n").unwrap();
    std::process::Command::new("git")
        .args(["add", "."])
        .current_dir(&seed_dir)
        .output()
        .unwrap();
    std::process::Command::new("git")
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
    std::process::Command::new("git")
        .args(["branch", "-M", "main"])
        .current_dir(&seed_dir)
        .output()
        .unwrap();
    std::process::Command::new("git")
        .args(["push", "-u", "origin", "main"])
        .current_dir(&seed_dir)
        .output()
        .unwrap();
    // Set HEAD on the bare remote to main
    std::process::Command::new("git")
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
    Command::cargo_bin("agwt")
        .unwrap()
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
    std::process::Command::new("git")
        .args(["init", "--bare"])
        .arg(&upstream_path)
        .output()
        .unwrap();
    // Seed it
    let seed_tmp = TempDir::new().unwrap();
    let seed_dir = seed_tmp.path().join("seed");
    std::process::Command::new("git")
        .args(["clone", upstream_path.to_str().unwrap(), "seed"])
        .current_dir(seed_tmp.path())
        .output()
        .unwrap();
    std::fs::write(seed_dir.join("README.md"), "# upstream\n").unwrap();
    std::process::Command::new("git")
        .args(["add", "."])
        .current_dir(&seed_dir)
        .output()
        .unwrap();
    std::process::Command::new("git")
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
    std::process::Command::new("git")
        .args(["branch", "-M", "main"])
        .current_dir(&seed_dir)
        .output()
        .unwrap();
    std::process::Command::new("git")
        .args(["push", "-u", "origin", "main"])
        .current_dir(&seed_dir)
        .output()
        .unwrap();
    std::process::Command::new("git")
        .args(["symbolic-ref", "HEAD", "refs/heads/main"])
        .current_dir(&upstream_path)
        .output()
        .unwrap();

    // Add it as a remote in the bare repo
    std::process::Command::new("git")
        .args(["remote", "add", "upstream", upstream_path.to_str().unwrap()])
        .current_dir(&bare_dir)
        .output()
        .unwrap();
    std::process::Command::new("git")
        .args(["fetch", "--quiet", "upstream"])
        .current_dir(&bare_dir)
        .output()
        .unwrap();

    (remote_tmp, upstream_tmp, project_tmp, bare_dir)
}
