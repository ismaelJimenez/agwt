mod common;

use common::{gwt, init_fresh, init_fresh_with_second_remote};
use predicates::prelude::*;

/// Checkout: tracks existing remote branch
#[test]
fn checkout_existing_branch() {
    let (_remote_tmp, _project_tmp, bare_dir) = init_fresh();
    let project_dir = bare_dir.parent().unwrap();

    // Create and push a branch
    gwt(&bare_dir)
        .args(["create", "test/checkout-target"])
        .assert()
        .success();
    let push = std::process::Command::new("git")
        .args(["push", "--quiet", "origin", "test/checkout-target"])
        .current_dir(project_dir.join("test-checkout-target"))
        .output()
        .unwrap();
    assert!(
        push.status.success(),
        "push failed: {}",
        String::from_utf8_lossy(&push.stderr)
    );

    // Remove locally
    gwt(&bare_dir)
        .args(["remove", "test-checkout-target", "--force"])
        .assert()
        .success();

    // Checkout should track it
    gwt(&bare_dir)
        .args(["checkout", "test/checkout-target"])
        .assert()
        .success()
        .stderr(predicate::str::contains("Created").and(predicate::str::contains("tracking")));

    // list shows it
    gwt(&bare_dir)
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("test-checkout-target"));

    // cleanup
    gwt(&bare_dir)
        .args(["remove", "test-checkout-target", "--force"])
        .assert()
        .success();
}

/// Checkout: --name override
#[test]
fn checkout_name_override() {
    let (_remote_tmp, _project_tmp, bare_dir) = init_fresh();
    let project_dir = bare_dir.parent().unwrap();

    // Create, push, remove
    gwt(&bare_dir)
        .args(["create", "test/co-name"])
        .assert()
        .success();
    let push = std::process::Command::new("git")
        .args(["push", "--quiet", "origin", "test/co-name"])
        .current_dir(project_dir.join("test-co-name"))
        .output()
        .unwrap();
    assert!(
        push.status.success(),
        "push failed: {}",
        String::from_utf8_lossy(&push.stderr)
    );
    gwt(&bare_dir)
        .args(["remove", "test-co-name", "--force"])
        .assert()
        .success();

    // Checkout with custom name
    gwt(&bare_dir)
        .args(["checkout", "test/co-name", "--name", "custom-co-dir"])
        .assert()
        .success();

    assert!(project_dir.join("custom-co-dir").exists());

    // cleanup
    gwt(&bare_dir)
        .args(["remove", "custom-co-dir", "--force"])
        .assert()
        .success();
}

/// Checkout: fails for nonexistent remote branch
#[test]
fn checkout_fails_nonexistent_branch() {
    let (_remote_tmp, _project_tmp, bare_dir) = init_fresh();

    gwt(&bare_dir)
        .args(["checkout", "no-such-branch-xyz"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found"));
}

/// Checkout: fails if directory already exists
#[test]
fn checkout_fails_if_dir_exists() {
    let (_remote_tmp, _project_tmp, bare_dir) = init_fresh();
    let project_dir = bare_dir.parent().unwrap();

    gwt(&bare_dir)
        .args(["create", "test/co-exists"])
        .assert()
        .success();
    let push = std::process::Command::new("git")
        .args(["push", "--quiet", "origin", "test/co-exists"])
        .current_dir(project_dir.join("test-co-exists"))
        .output()
        .unwrap();
    assert!(
        push.status.success(),
        "push failed: {}",
        String::from_utf8_lossy(&push.stderr)
    );
    gwt(&bare_dir)
        .args(["remove", "test-co-exists", "--force"])
        .assert()
        .success();

    // Create blocking directory
    std::fs::create_dir(project_dir.join("test-co-exists")).unwrap();

    gwt(&bare_dir)
        .args(["checkout", "test/co-exists"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("already exists"));
}

/// Checkout: succeeds when local branch already exists (no -b)
#[test]
fn checkout_existing_local_branch() {
    let (_remote_tmp, _project_tmp, bare_dir) = init_fresh();
    let project_dir = bare_dir.parent().unwrap();

    // Create and push a branch
    gwt(&bare_dir)
        .args(["create", "test/local-exists"])
        .assert()
        .success();
    let push = std::process::Command::new("git")
        .args(["push", "--quiet", "origin", "test/local-exists"])
        .current_dir(project_dir.join("test-local-exists"))
        .output()
        .unwrap();
    assert!(
        push.status.success(),
        "push failed: {}",
        String::from_utf8_lossy(&push.stderr)
    );

    // Remove worktree but keep local branch (--force removes worktree, branch persists
    // because it's not merged; we manually remove only the worktree directory)
    gwt(&bare_dir)
        .args(["remove", "test-local-exists", "--force"])
        .assert()
        .success();

    // Re-create the local branch ref pointing at the remote ref (simulating a local branch
    // that already exists, e.g. after a bare clone that maps remote refs to local branches)
    let create_branch = std::process::Command::new("git")
        .args(["branch", "test/local-exists", "origin/test/local-exists"])
        .current_dir(&bare_dir)
        .output()
        .unwrap();
    assert!(
        create_branch.status.success(),
        "branch create failed: {}",
        String::from_utf8_lossy(&create_branch.stderr)
    );

    // Verify the local branch exists
    let show_ref = std::process::Command::new("git")
        .args(["show-ref", "--verify", "refs/heads/test/local-exists"])
        .current_dir(&bare_dir)
        .output()
        .unwrap();
    assert!(show_ref.status.success(), "local branch should exist");

    // Checkout should succeed despite the local branch already existing
    gwt(&bare_dir)
        .args(["checkout", "test/local-exists"])
        .assert()
        .success()
        .stderr(predicate::str::contains("Created"));

    // Verify worktree exists
    assert!(project_dir.join("test-local-exists").exists());

    // cleanup
    gwt(&bare_dir)
        .args(["remove", "test-local-exists", "--force"])
        .assert()
        .success();
}

/// Checkout: --remote uses a non-default remote
#[test]
fn checkout_with_remote() {
    let (_remote_tmp, _upstream_tmp, _project_tmp, bare_dir) = init_fresh_with_second_remote();
    let project_dir = bare_dir.parent().unwrap();

    // Create a branch and push it to "upstream"
    gwt(&bare_dir)
        .args(["create", "test/co-upstream", "--remote", "upstream"])
        .assert()
        .success();
    let push = std::process::Command::new("git")
        .args(["push", "--quiet", "upstream", "test/co-upstream"])
        .current_dir(project_dir.join("test-co-upstream"))
        .output()
        .unwrap();
    assert!(
        push.status.success(),
        "push failed: {}",
        String::from_utf8_lossy(&push.stderr)
    );

    // Remove locally
    gwt(&bare_dir)
        .args(["remove", "test-co-upstream", "--force"])
        .assert()
        .success();

    // Checkout from upstream
    gwt(&bare_dir)
        .args(["checkout", "test/co-upstream", "--remote", "upstream"])
        .assert()
        .success()
        .stderr(predicate::str::contains("tracking"));

    gwt(&bare_dir)
        .args(["remove", "test-co-upstream", "--force"])
        .assert()
        .success();
}
