mod common;

use common::{git_cmd, gwt, init_fresh};
use predicates::prelude::*;
use tempfile::TempDir;

/// Rebase: successfully rebases onto configured base branch
#[test]
fn rebase_onto_configured_base() {
    let (_remote_tmp, _project_tmp, bare_dir) = init_fresh();
    let project_dir = bare_dir.parent().unwrap();

    // Create a feature branch with base=main
    gwt(&bare_dir)
        .args(["create", "test/rebase-ok", "--base", "main"])
        .assert()
        .success();

    let wt_dir = project_dir.join("test-rebase-ok");

    // Make a local commit on the feature branch
    std::fs::write(wt_dir.join("feature.txt"), "feature work\n").unwrap();
    git_cmd()
        .args(["add", "."])
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
            "feature commit",
        ])
        .current_dir(&wt_dir)
        .output()
        .unwrap();

    // Advance main on the remote
    let remote_url = git_cmd()
        .args(["remote", "get-url", "origin"])
        .current_dir(&bare_dir)
        .output()
        .unwrap();
    let remote_url = String::from_utf8_lossy(&remote_url.stdout)
        .trim()
        .to_string();

    let advance_tmp = TempDir::new().unwrap();
    git_cmd()
        .args(["clone", "--quiet", "-b", "main", &remote_url, "advance"])
        .current_dir(advance_tmp.path())
        .output()
        .unwrap();
    let advance_dir = advance_tmp.path().join("advance");
    std::fs::write(advance_dir.join("main-update.txt"), "main update\n").unwrap();
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
            "main advance",
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

    // Rebase the feature branch onto main
    gwt(&bare_dir)
        .args(["rebase", "test-rebase-ok"])
        .assert()
        .success()
        .stderr(predicate::str::contains("Rebased"));

    // Verify main-update.txt is visible (rebased on top of latest main)
    assert!(wt_dir.join("main-update.txt").exists());

    // Verify linear history
    let log = git_cmd()
        .args(["log", "--oneline", "--merges"])
        .current_dir(&wt_dir)
        .output()
        .unwrap();
    assert!(
        String::from_utf8_lossy(&log.stdout).trim().is_empty(),
        "expected no merge commits"
    );

    gwt(&bare_dir)
        .args(["remove", "test-rebase-ok", "--force"])
        .assert()
        .success();
}

/// Rebase: fails with helpful message on conflict
#[test]
fn rebase_conflict() {
    let (_remote_tmp, _project_tmp, bare_dir) = init_fresh();
    let project_dir = bare_dir.parent().unwrap();

    gwt(&bare_dir)
        .args(["create", "test/rebase-conflict", "--base", "main"])
        .assert()
        .success();

    let wt_dir = project_dir.join("test-rebase-conflict");

    // Make conflicting local commit
    std::fs::write(wt_dir.join("conflict.txt"), "local version\n").unwrap();
    git_cmd()
        .args(["add", "."])
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
            "local conflict",
        ])
        .current_dir(&wt_dir)
        .output()
        .unwrap();

    // Advance main with a conflicting change
    let remote_url = git_cmd()
        .args(["remote", "get-url", "origin"])
        .current_dir(&bare_dir)
        .output()
        .unwrap();
    let remote_url = String::from_utf8_lossy(&remote_url.stdout)
        .trim()
        .to_string();

    let advance_tmp = TempDir::new().unwrap();
    git_cmd()
        .args(["clone", "--quiet", "-b", "main", &remote_url, "advance"])
        .current_dir(advance_tmp.path())
        .output()
        .unwrap();
    let advance_dir = advance_tmp.path().join("advance");
    std::fs::write(advance_dir.join("conflict.txt"), "remote version\n").unwrap();
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
            "remote conflict",
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

    // Rebase should fail with conflict message
    gwt(&bare_dir)
        .args(["rebase", "test-rebase-conflict"])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("Conflict")
                .and(predicate::str::contains("rebase --continue"))
                .and(predicate::str::contains("rebase --abort")),
        );

    // Abort rebase for cleanup
    git_cmd()
        .args(["rebase", "--abort"])
        .current_dir(&wt_dir)
        .output()
        .unwrap();

    gwt(&bare_dir)
        .args(["remove", "test-rebase-conflict", "--force"])
        .assert()
        .success();
}

/// Rebase: errors out when no base configured and stdin is not a terminal
#[test]
fn rebase_no_base_not_tty() {
    let (_remote_tmp, _project_tmp, bare_dir) = init_fresh();

    // Create a branch without specifying --base (it still gets a base set to default)
    // So we need to manually unset it
    gwt(&bare_dir)
        .args(["create", "test/rebase-nobase"])
        .assert()
        .success();

    // Unset the agwt-base config
    git_cmd()
        .args(["config", "--unset", "branch.test/rebase-nobase.agwt-base"])
        .current_dir(&bare_dir)
        .output()
        .unwrap();

    // Rebase without a tty (assert_cmd pipes stdin)
    gwt(&bare_dir)
        .args(["rebase", "test-rebase-nobase"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("no base configured"));

    gwt(&bare_dir)
        .args(["remove", "test-rebase-nobase", "--force"])
        .assert()
        .success();
}

/// Rebase: auto-detect worktree from cwd
#[test]
fn rebase_auto_detect_cwd() {
    let (_remote_tmp, _project_tmp, bare_dir) = init_fresh();
    let project_dir = bare_dir.parent().unwrap();

    gwt(&bare_dir)
        .args(["create", "test/rebase-cwd", "--base", "main"])
        .assert()
        .success();

    let wt_dir = project_dir.join("test-rebase-cwd");

    // Rebase from within the worktree directory
    gwt(&bare_dir)
        .args(["rebase"])
        .current_dir(&wt_dir)
        .assert()
        .success()
        .stderr(predicate::str::contains("Rebased"));

    gwt(&bare_dir)
        .args(["remove", "test-rebase-cwd", "--force"])
        .assert()
        .success();
}

/// Rebase: autostash preserves uncommitted changes
#[test]
fn rebase_autostash() {
    let (_remote_tmp, _project_tmp, bare_dir) = init_fresh();
    let project_dir = bare_dir.parent().unwrap();

    gwt(&bare_dir)
        .args(["create", "test/rebase-stash", "--base", "main"])
        .assert()
        .success();

    let wt_dir = project_dir.join("test-rebase-stash");

    // Advance main on the remote
    let remote_url = git_cmd()
        .args(["remote", "get-url", "origin"])
        .current_dir(&bare_dir)
        .output()
        .unwrap();
    let remote_url = String::from_utf8_lossy(&remote_url.stdout)
        .trim()
        .to_string();

    let advance_tmp = TempDir::new().unwrap();
    git_cmd()
        .args(["clone", "--quiet", "-b", "main", &remote_url, "advance"])
        .current_dir(advance_tmp.path())
        .output()
        .unwrap();
    let advance_dir = advance_tmp.path().join("advance");
    std::fs::write(advance_dir.join("new-main.txt"), "from main\n").unwrap();
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
            "main update",
        ])
        .current_dir(&advance_dir)
        .output()
        .unwrap();
    git_cmd()
        .args(["push", "--quiet"])
        .current_dir(&advance_dir)
        .output()
        .unwrap();

    // Make a local dirty file (not committed)
    std::fs::write(wt_dir.join("dirty-wip.txt"), "work in progress\n").unwrap();

    // Rebase should succeed and preserve the dirty file
    gwt(&bare_dir)
        .args(["rebase", "test-rebase-stash"])
        .assert()
        .success()
        .stderr(predicate::str::contains("Rebased"));

    // Dirty file should still be there
    assert!(wt_dir.join("dirty-wip.txt").exists());
    assert_eq!(
        std::fs::read_to_string(wt_dir.join("dirty-wip.txt")).unwrap(),
        "work in progress\n"
    );

    gwt(&bare_dir)
        .args(["remove", "test-rebase-stash", "--force"])
        .assert()
        .success();
}
