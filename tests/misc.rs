mod common;

use assert_cmd::Command;
use common::{gwt, init_fresh, setup_local_remote};
use predicates::prelude::*;
use tempfile::TempDir;

// =============================================================================
// fetch
// =============================================================================

/// Fetch: fetches all remotes
#[test]
fn fetch_works() {
    let (_remote_tmp, _project_tmp, bare_dir) = init_fresh();
    gwt(&bare_dir)
        .arg("fetch")
        .assert()
        .success()
        .stderr(predicate::str::contains("Fetched"));
}

// =============================================================================
// cd
// =============================================================================

/// Cd: prints worktree path to stdout
#[test]
fn cd_outputs_path() {
    let (_remote_tmp, _project_tmp, bare_dir) = init_fresh();

    gwt(&bare_dir)
        .args(["create", "test/cd-path"])
        .assert()
        .success();

    gwt(&bare_dir)
        .args(["cd", "test-cd-path"])
        .assert()
        .success()
        .stdout(predicate::str::contains("test-cd-path"));

    gwt(&bare_dir)
        .args(["remove", "test-cd-path", "--force"])
        .assert()
        .success();
}

/// Cd: fails for nonexistent worktree
#[test]
fn cd_fails_nonexistent() {
    let (_remote_tmp, _project_tmp, bare_dir) = init_fresh();

    gwt(&bare_dir)
        .args(["cd", "no-such-worktree"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("does not exist"));
}

// =============================================================================
// open
// =============================================================================

/// Open: opens worktree with specified editor
#[test]
fn open_worktree() {
    let (_remote_tmp, _project_tmp, bare_dir) = init_fresh();

    gwt(&bare_dir)
        .args(["create", "test/open-wt"])
        .assert()
        .success();

    // Use "true" as the editor (no-op command that exits 0)
    gwt(&bare_dir)
        .args(["open", "test-open-wt", "--editor", "true"])
        .assert()
        .success()
        .stderr(predicate::str::contains("Opened"));
}

/// Open: fails for nonexistent worktree
#[test]
fn open_fails_nonexistent() {
    let (_remote_tmp, _project_tmp, bare_dir) = init_fresh();

    gwt(&bare_dir)
        .args(["open", "no-such-worktree", "--editor", "true"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("does not exist"));
}

/// Open: falls back to $EDITOR env var
#[test]
fn open_uses_editor_env() {
    let (_remote_tmp, _project_tmp, bare_dir) = init_fresh();

    gwt(&bare_dir)
        .args(["create", "test/open-env"])
        .assert()
        .success();

    gwt(&bare_dir)
        .args(["open", "test-open-env"])
        .env("VISUAL", "true")
        .assert()
        .success()
        .stderr(predicate::str::contains("Opened"));
}

// =============================================================================
// move
// =============================================================================

/// Move: renames a worktree directory
#[test]
fn move_worktree() {
    let (_remote_tmp, _project_tmp, bare_dir) = init_fresh();
    let project_dir = bare_dir.parent().unwrap();

    gwt(&bare_dir)
        .args(["create", "test/move-src"])
        .assert()
        .success();

    gwt(&bare_dir)
        .args(["move", "test-move-src", "moved-dest"])
        .assert()
        .success()
        .stderr(predicate::str::contains("Moved"));

    assert!(!project_dir.join("test-move-src").exists());
    assert!(project_dir.join("moved-dest").exists());
}

/// Move: fails for nonexistent source
#[test]
fn move_fails_nonexistent() {
    let (_remote_tmp, _project_tmp, bare_dir) = init_fresh();

    gwt(&bare_dir)
        .args(["move", "no-such-wt", "new-name"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("does not exist"));
}

/// Move: fails if destination already exists
#[test]
fn move_fails_dest_exists() {
    let (_remote_tmp, _project_tmp, bare_dir) = init_fresh();

    gwt(&bare_dir)
        .args(["create", "test/move-a"])
        .assert()
        .success();
    gwt(&bare_dir)
        .args(["create", "test/move-b"])
        .assert()
        .success();

    gwt(&bare_dir)
        .args(["move", "test-move-a", "test-move-b"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("destination already exists"));
}

// =============================================================================
// lock / unlock
// =============================================================================

/// Lock: locks a worktree, list shows locked indicator
#[test]
fn lock_worktree() {
    let (_remote_tmp, _project_tmp, bare_dir) = init_fresh();

    gwt(&bare_dir)
        .args(["create", "test/lock-wt"])
        .assert()
        .success();

    gwt(&bare_dir)
        .args(["lock", "test-lock-wt", "--reason", "external drive"])
        .assert()
        .success()
        .stderr(predicate::str::contains("Locked"));

    // list should show locked indicator
    gwt(&bare_dir)
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("locked"));
}

/// Unlock: unlocks a previously locked worktree
#[test]
fn unlock_worktree() {
    let (_remote_tmp, _project_tmp, bare_dir) = init_fresh();

    gwt(&bare_dir)
        .args(["create", "test/unlock-wt"])
        .assert()
        .success();

    gwt(&bare_dir)
        .args(["lock", "test-unlock-wt"])
        .assert()
        .success();

    gwt(&bare_dir)
        .args(["unlock", "test-unlock-wt"])
        .assert()
        .success()
        .stderr(predicate::str::contains("Unlocked"));

    // list should no longer show locked indicator for this worktree
    gwt(&bare_dir)
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("locked").not());
}

/// Lock: fails for nonexistent worktree
#[test]
fn lock_fails_nonexistent() {
    let (_remote_tmp, _project_tmp, bare_dir) = init_fresh();

    gwt(&bare_dir)
        .args(["lock", "no-such-wt"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("does not exist"));
}

// =============================================================================
// shell-init
// =============================================================================

/// Shell-init: bash output contains function and completion
#[test]
fn shell_init_bash() {
    Command::cargo_bin("agwt")
        .unwrap()
        .args(["shell-init", "bash"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("agwt()")
                .and(predicate::str::contains("cd \"$dir\""))
                .and(predicate::str::contains("COMPLETE=bash")),
        );
}

/// Shell-init: zsh output contains function and completion
#[test]
fn shell_init_zsh() {
    Command::cargo_bin("agwt")
        .unwrap()
        .args(["shell-init", "zsh"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("agwt()")
                .and(predicate::str::contains("cd \"$dir\""))
                .and(predicate::str::contains("COMPLETE=zsh")),
        );
}

/// Shell-init: fish output contains function and completion
#[test]
fn shell_init_fish() {
    Command::cargo_bin("agwt")
        .unwrap()
        .args(["shell-init", "fish"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("function agwt")
                .and(predicate::str::contains("cd $dir"))
                .and(predicate::str::contains("COMPLETE=fish")),
        );
}

/// Shell-init: invalid shell rejected
#[test]
fn shell_init_invalid_shell() {
    Command::cargo_bin("agwt")
        .unwrap()
        .args(["shell-init", "powershell"])
        .assert()
        .failure();
}

// =============================================================================
// global options
// =============================================================================

/// --bare-dir with nonexistent path fails
#[test]
fn bare_dir_nonexistent_fails() {
    Command::cargo_bin("agwt")
        .unwrap()
        .args(["--bare-dir", "/tmp/no-such-bare-dir-xyz", "list"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("does not exist"));
}

/// No --bare-dir outside a project fails with helpful message
#[test]
fn no_bare_dir_outside_project_fails() {
    let tmp = TempDir::new().unwrap();
    Command::cargo_bin("agwt")
        .unwrap()
        .arg("list")
        .current_dir(tmp.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("Could not find"));
}

/// --version prints version info
#[test]
fn version_flag() {
    Command::cargo_bin("agwt")
        .unwrap()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("agwt"));
}

// =============================================================================
// full lifecycle
// =============================================================================

/// Full lifecycle: init → list → create → list → push → sync → doctor → remove → list
#[test]
fn full_workflow() {
    let (_remote_tmp, remote_path) = setup_local_remote();

    let tmp = TempDir::new().unwrap();
    let project_dir = tmp.path().join("agwt");

    // --- init ---
    Command::cargo_bin("agwt")
        .unwrap()
        .args(["init", remote_path.to_str().unwrap(), "--name", "agwt"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stderr(predicate::str::contains("Cloning"));

    let bare_dir = project_dir.join(".bare");
    assert!(bare_dir.exists());

    // --- list (empty) ---
    gwt(&bare_dir)
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("No worktrees"));

    // --- create ---
    gwt(&bare_dir)
        .args(["create", "test/integration"])
        .assert()
        .success()
        .stderr(predicate::str::contains("Created"));

    // --- list (shows it) ---
    gwt(&bare_dir)
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("test-integration"));

    // --- push + sync ---
    let push = std::process::Command::new("git")
        .args(["push", "--quiet", "origin", "test/integration"])
        .current_dir(project_dir.join("test-integration"))
        .output()
        .unwrap();
    assert!(
        push.status.success(),
        "push failed: {}",
        String::from_utf8_lossy(&push.stderr)
    );

    gwt(&bare_dir)
        .args(["sync", "test-integration"])
        .assert()
        .success()
        .stderr(predicate::str::contains("Synced"));

    // --- doctor ---
    gwt(&bare_dir).arg("doctor").assert().success();

    // --- remove ---
    gwt(&bare_dir)
        .args(["remove", "test-integration", "--force"])
        .assert()
        .success()
        .stderr(predicate::str::contains("Removed"));

    // --- list (empty again) ---
    gwt(&bare_dir)
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("No worktrees"));
}
