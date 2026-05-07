mod common;

use common::{gwt, init_fresh, init_fresh_with_second_remote};
use predicates::prelude::*;
use tempfile::TempDir;

/// Sync: pulls latest on a pushed branch
#[test]
fn sync_pulls_latest() {
    let (_remote_tmp, _project_tmp, bare_dir) = init_fresh();
    let project_dir = bare_dir.parent().unwrap();

    gwt(&bare_dir)
        .args(["create", "test/sync-pull"])
        .assert()
        .success();

    // Push so remote exists
    let push = std::process::Command::new("git")
        .args(["push", "--quiet", "origin", "test/sync-pull"])
        .current_dir(project_dir.join("test-sync-pull"))
        .output()
        .unwrap();
    assert!(
        push.status.success(),
        "push failed: {}",
        String::from_utf8_lossy(&push.stderr)
    );

    gwt(&bare_dir)
        .args(["sync", "test-sync-pull"])
        .assert()
        .success()
        .stderr(predicate::str::contains("Synced"));

    // cleanup
    gwt(&bare_dir)
        .args(["remove", "test-sync-pull", "--force"])
        .assert()
        .success();
}

/// Sync: fails when branch has no remote
#[test]
fn sync_fails_no_remote_branch() {
    let (_remote_tmp, _project_tmp, bare_dir) = init_fresh();

    gwt(&bare_dir)
        .args(["create", "test/sync-noremote"])
        .assert()
        .success();

    // Sync without pushing — remote ref doesn't exist
    gwt(&bare_dir)
        .args(["sync", "test-sync-noremote"])
        .assert()
        .failure();

    gwt(&bare_dir)
        .args(["remove", "test-sync-noremote", "--force"])
        .assert()
        .success();
}

/// Sync: rebases local commits on top of remote (no merge commit)
#[test]
fn sync_rebase_linear_history() {
    let (_remote_tmp, _project_tmp, bare_dir) = init_fresh();
    let project_dir = bare_dir.parent().unwrap();

    gwt(&bare_dir)
        .args(["create", "test/sync-rebase"])
        .assert()
        .success();

    let wt_dir = project_dir.join("test-sync-rebase");

    // Push to set up remote tracking
    let push = std::process::Command::new("git")
        .args(["push", "--quiet", "-u", "origin", "test/sync-rebase"])
        .current_dir(&wt_dir)
        .output()
        .unwrap();
    assert!(
        push.status.success(),
        "push failed: {}",
        String::from_utf8_lossy(&push.stderr)
    );

    // Advance the remote branch with a non-conflicting change
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
            "test/sync-rebase",
            &remote_url,
            "advance",
        ])
        .current_dir(advance_tmp.path())
        .output()
        .unwrap();
    let advance_dir = advance_tmp.path().join("advance");
    std::fs::write(advance_dir.join("remote-file.txt"), "from remote\n").unwrap();
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
            "remote commit",
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

    // Make a non-conflicting local commit (different file)
    std::fs::write(wt_dir.join("local-file.txt"), "from local\n").unwrap();
    std::process::Command::new("git")
        .args(["add", "."])
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
            "local commit",
        ])
        .current_dir(&wt_dir)
        .output()
        .unwrap();

    // Sync should succeed
    gwt(&bare_dir)
        .args(["sync", "test-sync-rebase"])
        .assert()
        .success()
        .stderr(predicate::str::contains("Synced"));

    // Verify linear history (no merge commits — every commit has exactly 1 parent)
    let log = std::process::Command::new("git")
        .args(["log", "--oneline", "--merges"])
        .current_dir(&wt_dir)
        .output()
        .unwrap();
    let merge_output = String::from_utf8_lossy(&log.stdout);
    assert!(
        merge_output.trim().is_empty(),
        "expected no merge commits, got: {merge_output}"
    );

    // Verify both commits are present
    let log = std::process::Command::new("git")
        .args(["log", "--oneline"])
        .current_dir(&wt_dir)
        .output()
        .unwrap();
    let log_output = String::from_utf8_lossy(&log.stdout);
    assert!(
        log_output.contains("local commit"),
        "missing local commit in log"
    );
    assert!(
        log_output.contains("remote commit"),
        "missing remote commit in log"
    );

    gwt(&bare_dir)
        .args(["remove", "test-sync-rebase", "--force"])
        .assert()
        .success();
}

/// Sync: rebase conflict produces helpful message
#[test]
fn sync_rebase_conflict() {
    let (_remote_tmp, _project_tmp, bare_dir) = init_fresh();
    let project_dir = bare_dir.parent().unwrap();

    gwt(&bare_dir)
        .args(["create", "test/sync-conflict"])
        .assert()
        .success();

    let wt_dir = project_dir.join("test-sync-conflict");

    // Push to set up remote tracking
    let push = std::process::Command::new("git")
        .args(["push", "--quiet", "-u", "origin", "test/sync-conflict"])
        .current_dir(&wt_dir)
        .output()
        .unwrap();
    assert!(
        push.status.success(),
        "push failed: {}",
        String::from_utf8_lossy(&push.stderr)
    );

    // Advance the remote branch with a conflicting change
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
            "test/sync-conflict",
            &remote_url,
            "advance",
        ])
        .current_dir(advance_tmp.path())
        .output()
        .unwrap();
    let advance_dir = advance_tmp.path().join("advance");
    std::fs::write(advance_dir.join("conflict.txt"), "remote content\n").unwrap();
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
            "remote change",
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

    // Make a conflicting local commit
    std::fs::write(wt_dir.join("conflict.txt"), "local content\n").unwrap();
    std::process::Command::new("git")
        .args(["add", "."])
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
            "local change",
        ])
        .current_dir(&wt_dir)
        .output()
        .unwrap();

    // Sync should fail with conflict message
    gwt(&bare_dir)
        .args(["sync", "test-sync-conflict"])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("Conflict")
                .and(predicate::str::contains("rebase --continue"))
                .and(predicate::str::contains("rebase --abort")),
        );

    // Abort the rebase so we can clean up
    std::process::Command::new("git")
        .args(["rebase", "--abort"])
        .current_dir(&wt_dir)
        .output()
        .unwrap();

    gwt(&bare_dir)
        .args(["remove", "test-sync-conflict", "--force"])
        .assert()
        .success();
}

/// Sync: auto-detect from cwd
#[test]
fn sync_auto_detect_cwd() {
    let (_remote_tmp, _project_tmp, bare_dir) = init_fresh();
    let project_dir = bare_dir.parent().unwrap();

    gwt(&bare_dir)
        .args(["create", "test/sync-cwd"])
        .assert()
        .success();

    // Push so remote exists
    let push = std::process::Command::new("git")
        .args(["push", "--quiet", "origin", "test/sync-cwd"])
        .current_dir(project_dir.join("test-sync-cwd"))
        .output()
        .unwrap();
    assert!(
        push.status.success(),
        "push failed: {}",
        String::from_utf8_lossy(&push.stderr)
    );

    // Sync with no name arg, using current_dir inside the worktree
    gwt(&bare_dir)
        .args(["sync"])
        .current_dir(project_dir.join("test-sync-cwd"))
        .assert()
        .success()
        .stderr(predicate::str::contains("Synced"));

    // cleanup
    gwt(&bare_dir)
        .args(["remove", "test-sync-cwd", "--force"])
        .assert()
        .success();
}

/// Sync: --remote pulls from a non-default remote
#[test]
fn sync_with_remote() {
    let (_remote_tmp, _upstream_tmp, _project_tmp, bare_dir) = init_fresh_with_second_remote();
    let project_dir = bare_dir.parent().unwrap();

    gwt(&bare_dir)
        .args(["create", "test/sync-up", "--remote", "upstream"])
        .assert()
        .success();

    // Push to upstream so remote tracking branch exists
    let push = std::process::Command::new("git")
        .args(["push", "--quiet", "upstream", "test/sync-up"])
        .current_dir(project_dir.join("test-sync-up"))
        .output()
        .unwrap();
    assert!(
        push.status.success(),
        "push failed: {}",
        String::from_utf8_lossy(&push.stderr)
    );

    gwt(&bare_dir)
        .args(["sync", "test-sync-up", "--remote", "upstream"])
        .assert()
        .success()
        .stderr(predicate::str::contains("Synced"));

    gwt(&bare_dir)
        .args(["remove", "test-sync-up", "--force"])
        .assert()
        .success();
}

/// Sync --all: syncs multiple worktrees at once
#[test]
fn sync_all() {
    let (_remote_tmp, _project_tmp, bare_dir) = init_fresh();
    let project_dir = bare_dir.parent().unwrap();

    // Create two worktrees and push them
    for name in &["test/sync-all-a", "test/sync-all-b"] {
        gwt(&bare_dir).args(["create", name]).assert().success();

        let dir_name = name.replace('/', "-");
        let push = std::process::Command::new("git")
            .args(["push", "--quiet", "-u", "origin", name])
            .current_dir(project_dir.join(&dir_name))
            .output()
            .unwrap();
        assert!(
            push.status.success(),
            "push failed: {}",
            String::from_utf8_lossy(&push.stderr)
        );
    }

    // Sync all
    gwt(&bare_dir)
        .args(["sync", "--all"])
        .assert()
        .success()
        .stderr(predicate::str::contains("Synced").count(2));

    // cleanup
    for name in &["test-sync-all-a", "test-sync-all-b"] {
        gwt(&bare_dir)
            .args(["remove", name, "--force"])
            .assert()
            .success();
    }
}

/// Sync --all: reports failures but continues syncing remaining worktrees
#[test]
fn sync_all_partial_failure() {
    let (_remote_tmp, _project_tmp, bare_dir) = init_fresh();
    let project_dir = bare_dir.parent().unwrap();

    // Create two worktrees: one pushed (will succeed), one not (will fail)
    gwt(&bare_dir)
        .args(["create", "test/sync-all-ok"])
        .assert()
        .success();
    let push = std::process::Command::new("git")
        .args(["push", "--quiet", "-u", "origin", "test/sync-all-ok"])
        .current_dir(project_dir.join("test-sync-all-ok"))
        .output()
        .unwrap();
    assert!(
        push.status.success(),
        "push failed: {}",
        String::from_utf8_lossy(&push.stderr)
    );

    gwt(&bare_dir)
        .args(["create", "test/sync-all-fail"])
        .assert()
        .success();
    // Don't push this one — sync will fail for it

    // Sync all — should fail overall but still sync the good one
    gwt(&bare_dir)
        .args(["sync", "--all"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Synced").and(predicate::str::contains("Failed")));

    // cleanup
    for name in &["test-sync-all-ok", "test-sync-all-fail"] {
        gwt(&bare_dir)
            .args(["remove", name, "--force"])
            .assert()
            .success();
    }
}

/// Sync --all: no worktrees prints message and succeeds
#[test]
fn sync_all_empty() {
    let (_remote_tmp, _project_tmp, bare_dir) = init_fresh();

    gwt(&bare_dir)
        .args(["sync", "--all"])
        .assert()
        .success()
        .stderr(predicate::str::contains("No worktrees"));
}

/// Sync: autostash preserves uncommitted changes during rebase
#[test]
fn sync_autostash_dirty_worktree() {
    let (_remote_tmp, _project_tmp, bare_dir) = init_fresh();
    let project_dir = bare_dir.parent().unwrap();

    gwt(&bare_dir)
        .args(["create", "test/autostash"])
        .assert()
        .success();

    let wt_dir = project_dir.join("test-autostash");

    // Push the branch so it has an upstream
    std::process::Command::new("git")
        .args(["push", "-u", "origin", "test/autostash"])
        .current_dir(&wt_dir)
        .output()
        .unwrap();

    // Advance the remote branch from another clone
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
            "test/autostash",
            &remote_url,
            "advance",
        ])
        .current_dir(advance_tmp.path())
        .output()
        .unwrap();
    let advance_dir = advance_tmp.path().join("advance");
    std::fs::write(advance_dir.join("remote-change.txt"), "from remote\n").unwrap();
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
            "-m",
            "remote advance",
        ])
        .current_dir(&advance_dir)
        .output()
        .unwrap();
    std::process::Command::new("git")
        .args(["push"])
        .current_dir(&advance_dir)
        .output()
        .unwrap();

    // Make a local dirty change (different file to avoid conflict)
    std::fs::write(wt_dir.join("local-wip.txt"), "work in progress\n").unwrap();

    // Sync should succeed despite dirty worktree
    gwt(&bare_dir)
        .args(["sync", "test-autostash"])
        .assert()
        .success()
        .stderr(predicate::str::contains("Synced"));

    // Verify the local dirty file is still there
    assert!(wt_dir.join("local-wip.txt").exists());
    assert_eq!(
        std::fs::read_to_string(wt_dir.join("local-wip.txt")).unwrap(),
        "work in progress\n"
    );

    // Verify the remote change was pulled
    assert!(wt_dir.join("remote-change.txt").exists());
}
