mod common;

use assert_cmd::Command;
use common::{init_fresh, setup_local_remote};
use predicates::prelude::*;
use tempfile::TempDir;

/// Init: clones bare repo and creates .bare directory
#[test]
fn init_creates_bare_dir() {
    let (_remote_tmp, remote_path) = setup_local_remote();
    let tmp = TempDir::new().unwrap();
    Command::cargo_bin("agwt")
        .unwrap()
        .args(["init", remote_path.to_str().unwrap(), "--name", "myproject"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stderr(predicate::str::contains("Cloning"));

    let bare_dir = tmp.path().join("myproject").join(".bare");
    assert!(bare_dir.exists(), ".bare directory should exist after init");
    assert!(
        bare_dir.join("HEAD").exists(),
        "HEAD should exist in bare repo"
    );
}

/// Init: --name override changes the folder name
#[test]
fn init_custom_name() {
    let (_remote_tmp, remote_path) = setup_local_remote();
    let tmp = TempDir::new().unwrap();
    Command::cargo_bin("agwt")
        .unwrap()
        .args([
            "init",
            remote_path.to_str().unwrap(),
            "--name",
            "custom-project",
        ])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stderr(predicate::str::contains("Cloning"));

    let bare_dir = tmp.path().join("custom-project").join(".bare");
    assert!(bare_dir.exists(), ".bare should exist under custom name");
}

/// Init: folder name derived from URL strips .git suffix
#[test]
fn init_derives_name_from_url() {
    // Name the remote "agwt.git" so derivation produces "agwt"
    let remote_tmp = TempDir::new().unwrap();
    let remote_path = remote_tmp.path().join("agwt.git");
    std::process::Command::new("git")
        .args(["init", "--bare"])
        .arg(&remote_path)
        .output()
        .unwrap();
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
    std::process::Command::new("git")
        .args(["symbolic-ref", "HEAD", "refs/heads/main"])
        .current_dir(&remote_path)
        .output()
        .unwrap();

    let tmp = TempDir::new().unwrap();
    Command::cargo_bin("agwt")
        .unwrap()
        .args(["init", remote_path.to_str().unwrap()])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Should be "agwt" (not "agwt.git")
    assert!(tmp.path().join("agwt").exists());
    assert!(!tmp.path().join("agwt.git").exists());
}

/// Init: fails if directory already exists
#[test]
fn init_fails_if_dir_exists() {
    let (_remote_tmp, remote_path) = setup_local_remote();
    let tmp = TempDir::new().unwrap();
    std::fs::create_dir(tmp.path().join("myproject")).unwrap();

    Command::cargo_bin("agwt")
        .unwrap()
        .args(["init", remote_path.to_str().unwrap(), "--name", "myproject"])
        .current_dir(tmp.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("already exists"));
}

/// Init: fails with invalid URL
#[test]
fn init_fails_invalid_url() {
    let tmp = TempDir::new().unwrap();
    Command::cargo_bin("agwt")
        .unwrap()
        .args(["init", "/tmp/no-such-path-xyz/repo.git"])
        .current_dir(tmp.path())
        .assert()
        .failure();
}

/// Init: configures fetch refspec and push.autoSetupRemote
#[test]
fn init_configures_git_settings() {
    let (_remote_tmp, _project_tmp, bare_dir) = init_fresh();

    // Check fetch refspec
    let output = std::process::Command::new("git")
        .args(["config", "--get", "remote.origin.fetch"])
        .current_dir(&bare_dir)
        .output()
        .unwrap();
    let fetch = String::from_utf8_lossy(&output.stdout);
    assert!(
        fetch.contains("+refs/heads/*:refs/remotes/origin/*"),
        "expected fetch refspec, got: {fetch}"
    );

    // Check push.autoSetupRemote
    let output = std::process::Command::new("git")
        .args(["config", "--get", "push.autoSetupRemote"])
        .current_dir(&bare_dir)
        .output()
        .unwrap();
    let auto_setup = String::from_utf8_lossy(&output.stdout);
    assert!(
        auto_setup.trim() == "true",
        "expected push.autoSetupRemote=true, got: {auto_setup}"
    );
}

/// Init: creates a worktree for the default branch
#[test]
fn init_creates_default_branch_worktree() {
    let (_remote_tmp, _project_tmp, bare_dir) = init_fresh();
    let project_dir = bare_dir.parent().unwrap();

    let main_wt = project_dir.join("main");
    assert!(main_wt.exists(), "main worktree directory should exist");
    assert!(
        main_wt.join("README.md").exists(),
        "main worktree should contain checked-out files"
    );
}

/// Init: default branch worktree tracks origin
#[test]
fn init_default_branch_tracks_upstream() {
    let (_remote_tmp, _project_tmp, bare_dir) = init_fresh();
    let project_dir = bare_dir.parent().unwrap();

    let main_wt = project_dir.join("main");
    let output = std::process::Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"])
        .current_dir(&main_wt)
        .output()
        .unwrap();
    let upstream = String::from_utf8_lossy(&output.stdout);
    assert!(
        upstream.trim() == "origin/main",
        "expected upstream origin/main, got: {upstream}"
    );
}

/// Init: output mentions the created worktree
#[test]
fn init_output_mentions_worktree() {
    let (_remote_tmp, remote_path) = setup_local_remote();
    let tmp = TempDir::new().unwrap();
    Command::cargo_bin("agwt")
        .unwrap()
        .args(["init", remote_path.to_str().unwrap(), "--name", "myproject"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stderr(
            predicate::str::contains("Cloning")
                .and(predicate::str::contains("Created"))
                .and(predicate::str::contains("main")),
        );
}

/// Init: prints a status message before fetching remote tracking refs
#[test]
fn init_prints_fetching_status() {
    let (_remote_tmp, remote_path) = setup_local_remote();
    let tmp = TempDir::new().unwrap();
    Command::cargo_bin("agwt")
        .unwrap()
        .args(["init", remote_path.to_str().unwrap(), "--name", "myproject"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stderr(predicate::str::contains("Fetching"));
}
