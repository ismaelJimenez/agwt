mod common;

use common::{git_cmd, gwt, init_fresh};
use predicates::prelude::*;
use tempfile::TempDir;

/// Doctor: healthy repo reports OK
#[test]
fn doctor_healthy() {
    let (_remote_tmp, _project_tmp, bare_dir) = init_fresh();
    gwt(&bare_dir)
        .arg("doctor")
        .assert()
        .success()
        .stderr(predicate::str::contains("OK"));
}

/// Doctor: detects branch with no upstream
#[test]
fn doctor_no_upstream() {
    let (_remote_tmp, _project_tmp, bare_dir) = init_fresh();
    let project_dir = bare_dir.parent().unwrap();

    gwt(&bare_dir)
        .args(["create", "test/doc-noup"])
        .assert()
        .success();

    // Unset the auto-configured upstream so doctor can detect it
    git_cmd()
        .args(["branch", "--unset-upstream"])
        .current_dir(project_dir.join("test-doc-noup"))
        .output()
        .unwrap();

    gwt(&bare_dir).arg("doctor").assert().success().stderr(
        predicate::str::contains("no upstream").and(predicate::str::contains("push to publish")),
    );

    gwt(&bare_dir)
        .args(["remove", "test-doc-noup", "--force"])
        .assert()
        .success();
}

/// Doctor: detects dirty worktree
#[test]
fn doctor_dirty() {
    let (_remote_tmp, _project_tmp, bare_dir) = init_fresh();
    let project_dir = bare_dir.parent().unwrap();

    gwt(&bare_dir)
        .args(["create", "test/doc-dirty"])
        .assert()
        .success();

    // Push so it has an upstream (otherwise "no upstream" fires first)
    let push = git_cmd()
        .args(["push", "--quiet", "-u", "origin", "test/doc-dirty"])
        .current_dir(project_dir.join("test-doc-dirty"))
        .output()
        .unwrap();
    assert!(
        push.status.success(),
        "push failed: {}",
        String::from_utf8_lossy(&push.stderr)
    );

    // Make dirty
    std::fs::write(project_dir.join("test-doc-dirty").join("dirty.txt"), "x").unwrap();

    gwt(&bare_dir)
        .arg("doctor")
        .assert()
        .success()
        .stderr(predicate::str::contains("Dirty"));

    gwt(&bare_dir)
        .args(["remove", "test-doc-dirty", "--force"])
        .assert()
        .success();
}

/// Doctor: detects ahead (unpushed commits)
#[test]
fn doctor_ahead() {
    let (_remote_tmp, _project_tmp, bare_dir) = init_fresh();
    let project_dir = bare_dir.parent().unwrap();

    gwt(&bare_dir)
        .args(["create", "test/doc-ahead"])
        .assert()
        .success();

    let wt_dir = project_dir.join("test-doc-ahead");

    // Push to set up upstream
    let push = git_cmd()
        .args(["push", "--quiet", "-u", "origin", "test/doc-ahead"])
        .current_dir(&wt_dir)
        .output()
        .unwrap();
    assert!(
        push.status.success(),
        "push failed: {}",
        String::from_utf8_lossy(&push.stderr)
    );

    // Make a local commit (ahead)
    std::fs::write(wt_dir.join("ahead.txt"), "ahead").unwrap();
    git_cmd()
        .args(["add", "ahead.txt"])
        .current_dir(&wt_dir)
        .output()
        .unwrap();
    git_cmd()
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
        .arg("doctor")
        .assert()
        .success()
        .stderr(predicate::str::contains("Ahead"));

    gwt(&bare_dir)
        .args(["remove", "test-doc-ahead", "--force"])
        .assert()
        .success();
}

/// Doctor: detects gone branches (remote branch deleted)
#[test]
fn doctor_gone_branch() {
    let (_remote_tmp, _project_tmp, bare_dir) = init_fresh();
    let project_dir = bare_dir.parent().unwrap();

    gwt(&bare_dir)
        .args(["create", "test/doc-gone"])
        .assert()
        .success();

    let wt_dir = project_dir.join("test-doc-gone");

    // Push to set upstream
    let push = git_cmd()
        .args(["push", "--quiet", "-u", "origin", "test/doc-gone"])
        .current_dir(&wt_dir)
        .output()
        .unwrap();
    assert!(
        push.status.success(),
        "push failed: {}",
        String::from_utf8_lossy(&push.stderr)
    );

    // Delete the remote branch (simulating someone else deleting it)
    git_cmd()
        .args(["push", "origin", "--delete", "test/doc-gone"])
        .current_dir(&bare_dir)
        .output()
        .unwrap();

    // Doctor should detect "gone" after its internal fetch --prune
    gwt(&bare_dir)
        .arg("doctor")
        .assert()
        .success()
        .stderr(predicate::str::contains("Gone"));

    gwt(&bare_dir)
        .args(["remove", "test-doc-gone", "--force"])
        .assert()
        .success();
}

/// Doctor: detects behind (local branch is behind upstream)
#[test]
fn doctor_behind() {
    let (_remote_tmp, _project_tmp, bare_dir) = init_fresh();
    let project_dir = bare_dir.parent().unwrap();

    gwt(&bare_dir)
        .args(["create", "test/doc-behind"])
        .assert()
        .success();

    let wt_dir = project_dir.join("test-doc-behind");

    // Push to set up upstream
    let push = git_cmd()
        .args(["push", "--quiet", "-u", "origin", "test/doc-behind"])
        .current_dir(&wt_dir)
        .output()
        .unwrap();
    assert!(
        push.status.success(),
        "push failed: {}",
        String::from_utf8_lossy(&push.stderr)
    );

    // Advance the remote branch by pushing a commit from a temporary clone
    let advance_tmp = TempDir::new().unwrap();
    let remote_url = git_cmd()
        .args(["remote", "get-url", "origin"])
        .current_dir(&bare_dir)
        .output()
        .unwrap();
    let remote_url = String::from_utf8_lossy(&remote_url.stdout)
        .trim()
        .to_string();

    git_cmd()
        .args([
            "clone",
            "--quiet",
            "-b",
            "test/doc-behind",
            &remote_url,
            "advance",
        ])
        .current_dir(advance_tmp.path())
        .output()
        .unwrap();
    let advance_dir = advance_tmp.path().join("advance");
    std::fs::write(advance_dir.join("new.txt"), "new").unwrap();
    git_cmd()
        .args(["add", "."])
        .current_dir(&advance_dir)
        .output()
        .unwrap();
    git_cmd()
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
    let push = git_cmd()
        .args(["push", "--quiet"])
        .current_dir(&advance_dir)
        .output()
        .unwrap();
    assert!(
        push.status.success(),
        "push failed: {}",
        String::from_utf8_lossy(&push.stderr)
    );

    // Doctor should detect "behind" (it does fetch --all --prune internally)
    gwt(&bare_dir)
        .arg("doctor")
        .assert()
        .success()
        .stderr(predicate::str::contains("Behind"));

    gwt(&bare_dir)
        .args(["remove", "test-doc-behind", "--force"])
        .assert()
        .success();
}

/// Doctor: detects stale worktree (directory deleted externally)
#[test]
fn doctor_stale_worktree() {
    let (_remote_tmp, _project_tmp, bare_dir) = init_fresh();
    let project_dir = bare_dir.parent().unwrap();

    gwt(&bare_dir)
        .args(["create", "test/doc-stale"])
        .assert()
        .success();

    // Delete the worktree directory externally (simulating manual rm -rf)
    let wt_dir = project_dir.join("test-doc-stale");
    std::fs::remove_dir_all(&wt_dir).unwrap();

    // Doctor should detect the stale worktree and prune it
    gwt(&bare_dir)
        .arg("doctor")
        .assert()
        .success()
        .stderr(predicate::str::contains("Stale").and(predicate::str::contains("Pruned")));
}
