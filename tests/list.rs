mod common;

use common::{gwt, init_fresh};
use predicates::prelude::*;
use tempfile::TempDir;

/// List: freshly-inited repo shows the default branch worktree
#[test]
fn list_after_init() {
    let (_remote_tmp, _project_tmp, bare_dir) = init_fresh();
    gwt(&bare_dir)
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("main"));
}

/// List: shows created worktree with correct name
#[test]
fn list_shows_worktree() {
    let (_remote_tmp, _project_tmp, bare_dir) = init_fresh();

    gwt(&bare_dir)
        .args(["create", "test/list-show"])
        .assert()
        .success();

    gwt(&bare_dir)
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("test-list-show"));

    // cleanup
    gwt(&bare_dir)
        .args(["remove", "test-list-show", "--force"])
        .assert()
        .success();
}

/// List: only default branch remains after removing a feature worktree
#[test]
fn list_after_remove() {
    let (_remote_tmp, _project_tmp, bare_dir) = init_fresh();

    gwt(&bare_dir)
        .args(["create", "test/list-rm"])
        .assert()
        .success();
    gwt(&bare_dir)
        .args(["remove", "test-list-rm", "--force"])
        .assert()
        .success();

    let output = gwt(&bare_dir).arg("list").assert().success();
    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    assert!(stdout.contains("main"), "should still list main");
    assert!(
        !stdout.contains("test-list-rm"),
        "removed worktree should be gone"
    );
}

/// List: shows dirty indicator when worktree has uncommitted changes
#[test]
fn list_shows_dirty_indicator() {
    let (_remote_tmp, _project_tmp, bare_dir) = init_fresh();
    let project_dir = bare_dir.parent().unwrap();

    gwt(&bare_dir)
        .args(["create", "test/list-dirty"])
        .assert()
        .success();

    // Make the worktree dirty
    std::fs::write(
        project_dir.join("test-list-dirty").join("dirty.txt"),
        "dirty",
    )
    .unwrap();

    gwt(&bare_dir)
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("*"));

    gwt(&bare_dir)
        .args(["remove", "test-list-dirty", "--force"])
        .assert()
        .success();
}

/// List: shows ahead indicator when worktree has unpushed commits
#[test]
fn list_shows_ahead_indicator() {
    let (_remote_tmp, _project_tmp, bare_dir) = init_fresh();
    let project_dir = bare_dir.parent().unwrap();

    gwt(&bare_dir)
        .args(["create", "test/list-ahead"])
        .assert()
        .success();

    let wt_dir = project_dir.join("test-list-ahead");

    // Push to set up upstream tracking
    let push = std::process::Command::new("git")
        .args(["push", "--quiet", "-u", "origin", "test/list-ahead"])
        .current_dir(&wt_dir)
        .output()
        .unwrap();
    assert!(
        push.status.success(),
        "push failed: {}",
        String::from_utf8_lossy(&push.stderr)
    );

    // Make a local commit (ahead of remote)
    std::fs::write(wt_dir.join("ahead.txt"), "ahead").unwrap();
    std::process::Command::new("git")
        .args(["add", "ahead.txt"])
        .current_dir(&wt_dir)
        .output()
        .unwrap();
    std::process::Command::new("git")
        .args([
            "-c",
            "user.email=test@test.com",
            "-c",
            "user.name=Test",
            "commit",
            "--quiet",
            "-m",
            "local only",
        ])
        .current_dir(&wt_dir)
        .output()
        .unwrap();

    gwt(&bare_dir)
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("↑1"));

    gwt(&bare_dir)
        .args(["remove", "test-list-ahead", "--force"])
        .assert()
        .success();
}

/// List: shows behind indicator when worktree is behind remote
#[test]
fn list_shows_behind_indicator() {
    let (_remote_tmp, _project_tmp, bare_dir) = init_fresh();
    let project_dir = bare_dir.parent().unwrap();

    gwt(&bare_dir)
        .args(["create", "test/list-behind"])
        .assert()
        .success();

    let wt_dir = project_dir.join("test-list-behind");

    // Push to set up upstream tracking
    let push = std::process::Command::new("git")
        .args(["push", "--quiet", "-u", "origin", "test/list-behind"])
        .current_dir(&wt_dir)
        .output()
        .unwrap();
    assert!(
        push.status.success(),
        "push failed: {}",
        String::from_utf8_lossy(&push.stderr)
    );

    // Advance the remote branch from a separate clone
    let remote_url = std::process::Command::new("git")
        .args(["remote", "get-url", "origin"])
        .current_dir(&bare_dir)
        .output()
        .unwrap();
    let remote_url = String::from_utf8_lossy(&remote_url.stdout)
        .trim()
        .to_string();

    let advance_tmp = TempDir::new().unwrap();
    std::process::Command::new("git")
        .args([
            "clone",
            "--quiet",
            "-b",
            "test/list-behind",
            &remote_url,
            "advance",
        ])
        .current_dir(advance_tmp.path())
        .output()
        .unwrap();
    let advance_dir = advance_tmp.path().join("advance");
    std::fs::write(advance_dir.join("remote.txt"), "remote").unwrap();
    std::process::Command::new("git")
        .args(["add", "."])
        .current_dir(&advance_dir)
        .output()
        .unwrap();
    std::process::Command::new("git")
        .args([
            "-c",
            "user.email=test@test.com",
            "-c",
            "user.name=Test",
            "commit",
            "--quiet",
            "-m",
            "remote advance",
        ])
        .current_dir(&advance_dir)
        .output()
        .unwrap();
    let push = std::process::Command::new("git")
        .args(["push", "--quiet"])
        .current_dir(&advance_dir)
        .output()
        .unwrap();
    assert!(
        push.status.success(),
        "push failed: {}",
        String::from_utf8_lossy(&push.stderr)
    );

    // Fetch so local knows about the remote advance
    std::process::Command::new("git")
        .args(["fetch", "--quiet", "origin"])
        .current_dir(&wt_dir)
        .output()
        .unwrap();

    gwt(&bare_dir)
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("↓1"));

    gwt(&bare_dir)
        .args(["remove", "test-list-behind", "--force"])
        .assert()
        .success();
}

/// List: shows locked indicator for locked worktrees
#[test]
fn list_shows_locked_indicator() {
    let (_remote_tmp, _project_tmp, bare_dir) = init_fresh();
    let project_dir = bare_dir.parent().unwrap();

    gwt(&bare_dir)
        .args(["create", "test/list-locked"])
        .assert()
        .success();

    // Lock the worktree
    let wt_dir = project_dir.join("test-list-locked");
    let lock = std::process::Command::new("git")
        .args(["worktree", "lock", wt_dir.to_str().unwrap()])
        .current_dir(&bare_dir)
        .output()
        .unwrap();
    assert!(
        lock.status.success(),
        "lock failed: {}",
        String::from_utf8_lossy(&lock.stderr)
    );

    gwt(&bare_dir)
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("locked"));

    // Unlock before removal
    std::process::Command::new("git")
        .args(["worktree", "unlock", wt_dir.to_str().unwrap()])
        .current_dir(&bare_dir)
        .output()
        .unwrap();

    gwt(&bare_dir)
        .args(["remove", "test-list-locked", "--force"])
        .assert()
        .success();
}
