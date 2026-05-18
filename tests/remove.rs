mod common;

use common::{git_cmd, gwt, init_fresh};
use predicates::prelude::*;
use std::process::Command;

/// Remove: removes worktree directory and local branch
#[test]
fn remove_deletes_worktree_and_branch() {
    let (_remote_tmp, _project_tmp, bare_dir) = init_fresh();
    let project_dir = bare_dir.parent().unwrap();

    gwt(&bare_dir)
        .args(["create", "test/rm-basic"])
        .assert()
        .success();
    assert!(project_dir.join("test-rm-basic").exists());

    gwt(&bare_dir)
        .args(["remove", "test-rm-basic", "--force"])
        .assert()
        .success()
        .stderr(predicate::str::contains("Removed").and(predicate::str::contains("Deleted")));

    assert!(!project_dir.join("test-rm-basic").exists());

    // Verify branch is gone
    let output = git_cmd()
        .args(["branch", "--list", "test/rm-basic"])
        .current_dir(&bare_dir)
        .output()
        .unwrap();
    assert!(String::from_utf8_lossy(&output.stdout).trim().is_empty());
}

/// Remove: fails for nonexistent worktree
#[test]
fn remove_fails_nonexistent() {
    let (_remote_tmp, _project_tmp, bare_dir) = init_fresh();

    gwt(&bare_dir)
        .args(["remove", "no-such-worktree"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("does not exist"));
}

/// Remove: --force removes dirty worktree
#[test]
fn remove_force_dirty_worktree() {
    let (_remote_tmp, _project_tmp, bare_dir) = init_fresh();
    let project_dir = bare_dir.parent().unwrap();

    gwt(&bare_dir)
        .args(["create", "test/rm-dirty"])
        .assert()
        .success();

    // Make it dirty
    std::fs::write(
        project_dir.join("test-rm-dirty").join("dirty-file.txt"),
        "dirty",
    )
    .unwrap();

    // Without --force should fail
    gwt(&bare_dir)
        .args(["remove", "test-rm-dirty"])
        .assert()
        .failure();

    // With --force should succeed
    gwt(&bare_dir)
        .args(["remove", "test-rm-dirty", "--force"])
        .assert()
        .success()
        .stderr(predicate::str::contains("Removed"));
}

/// Remove: --delete-remote deletes the remote branch
#[test]
fn remove_delete_remote() {
    let (_remote_tmp, _project_tmp, bare_dir) = init_fresh();
    let project_dir = bare_dir.parent().unwrap();

    gwt(&bare_dir)
        .args(["create", "test/rm-remote"])
        .assert()
        .success();

    // Push so remote branch exists
    let push = git_cmd()
        .args(["push", "--quiet", "origin", "test/rm-remote"])
        .current_dir(project_dir.join("test-rm-remote"))
        .output()
        .unwrap();
    assert!(
        push.status.success(),
        "push failed: {}",
        String::from_utf8_lossy(&push.stderr)
    );

    // Remove with --delete-remote, piping "y" to stdin
    gwt(&bare_dir)
        .args(["remove", "test-rm-remote", "--force", "--delete-remote"])
        .write_stdin("y\n")
        .assert()
        .success()
        .stderr(
            predicate::str::contains("Removed")
                .and(predicate::str::contains("Deleted").and(predicate::str::contains("remote"))),
        );

    // Verify remote branch is gone
    let output = git_cmd()
        .args(["ls-remote", "--heads", "origin", "test/rm-remote"])
        .current_dir(&bare_dir)
        .output()
        .unwrap();
    assert!(String::from_utf8_lossy(&output.stdout).trim().is_empty());
}

/// Remove --merged: removes worktrees whose branches are merged into default
#[test]
fn remove_merged_removes_only_merged_worktrees() {
    let (_remote_tmp, _project_tmp, bare_dir) = init_fresh();
    let project_dir = bare_dir.parent().unwrap();

    // Create two worktrees
    gwt(&bare_dir)
        .args(["create", "feat-merged"])
        .assert()
        .success();
    gwt(&bare_dir)
        .args(["create", "feat-unmerged"])
        .assert()
        .success();

    // Add a commit to feat-unmerged so it's NOT merged
    let wt_unmerged = project_dir.join("feat-unmerged");
    Command::new("git")
        .args(["commit", "--allow-empty", "-m", "unmerged work"])
        .current_dir(&wt_unmerged)
        .output()
        .unwrap();

    // feat-merged has the same commits as main (no extra), so it IS merged
    // Run remove --merged
    gwt(&bare_dir)
        .args(["remove", "--merged"])
        .assert()
        .success()
        .stderr(predicate::str::contains("feat-merged").and(predicate::str::contains("Removed")));

    // Verify feat-merged worktree is gone
    assert!(!project_dir.join("feat-merged").exists());

    // Verify feat-unmerged worktree still exists
    assert!(wt_unmerged.exists());
}

/// Remove --merged: prints message when no merged worktrees exist
#[test]
fn remove_merged_no_merged_worktrees() {
    let (_remote_tmp, _project_tmp, bare_dir) = init_fresh();
    let project_dir = bare_dir.parent().unwrap();

    // Create a worktree with an unmerged commit
    gwt(&bare_dir)
        .args(["create", "feat-diverged"])
        .assert()
        .success();
    Command::new("git")
        .args(["commit", "--allow-empty", "-m", "diverged work"])
        .current_dir(project_dir.join("feat-diverged"))
        .output()
        .unwrap();

    gwt(&bare_dir)
        .args(["remove", "--merged"])
        .assert()
        .success()
        .stderr(predicate::str::contains("No merged worktrees"));
}

/// Remove: errors when neither name nor --merged is provided
#[test]
fn remove_requires_name_or_merged() {
    let (_remote_tmp, _project_tmp, bare_dir) = init_fresh();

    gwt(&bare_dir)
        .args(["remove"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("a worktree name is required"));
}
