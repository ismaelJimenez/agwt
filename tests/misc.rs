mod common;

use common::{agwt_cmd, git_cmd, gwt, init_fresh, setup_local_remote};
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
// transfer progress
// =============================================================================

/// Init: clone works with file:// URL (pack protocol, triggers transfer progress on TTY)
#[test]
fn init_clone_with_file_url() {
    let (_remote_tmp, remote_path) = setup_local_remote();
    let tmp = TempDir::new().unwrap();
    let url = format!("file://{}", remote_path.display());
    agwt_cmd()
        .args(["init", &url, "--name", "progress-test"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stderr(predicate::str::contains("Cloning"));

    let bare_dir = tmp.path().join("progress-test").join(".bare");
    assert!(bare_dir.exists());
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
    agwt_cmd()
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
    agwt_cmd()
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
    agwt_cmd()
        .args(["shell-init", "fish"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("function agwt")
                .and(predicate::str::contains("cd $dir"))
                .and(predicate::str::contains("COMPLETE=fish")),
        );
}

/// Shell-init: powershell output contains function and completion
#[test]
fn shell_init_powershell() {
    agwt_cmd()
        .args(["shell-init", "powershell"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("function agwt")
                .and(predicate::str::contains("Set-Location"))
                .and(predicate::str::contains("Register-ArgumentCompleter")),
        );
}

/// Shell-init: invalid shell rejected
#[test]
fn shell_init_invalid_shell() {
    agwt_cmd()
        .args(["shell-init", "nushell"])
        .assert()
        .failure();
}

// =============================================================================
// global options
// =============================================================================

/// --bare-dir with nonexistent path fails
#[test]
fn bare_dir_nonexistent_fails() {
    agwt_cmd()
        .args(["--bare-dir", "/tmp/no-such-bare-dir-xyz", "list"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("does not exist"));
}

/// No --bare-dir outside a project fails with helpful message
#[test]
fn no_bare_dir_outside_project_fails() {
    let tmp = TempDir::new().unwrap();
    agwt_cmd()
        .arg("list")
        .current_dir(tmp.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("Could not find"));
}

/// --version prints version info
#[test]
fn version_flag() {
    agwt_cmd()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("agwt"));
}

/// --verbose flag is accepted and fetch still succeeds
#[test]
fn verbose_flag_fetch() {
    let (_remote_tmp, _project_tmp, bare_dir) = init_fresh();
    gwt(&bare_dir)
        .args(["--verbose", "fetch"])
        .assert()
        .success()
        .stderr(predicate::str::contains("Fetched"));
}

/// --verbose flag works with create
#[test]
fn verbose_flag_create() {
    let (_remote_tmp, _project_tmp, bare_dir) = init_fresh();
    gwt(&bare_dir)
        .args(["--verbose", "create", "test/verbose-branch"])
        .assert()
        .success()
        .stderr(predicate::str::contains("Created"));
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
    agwt_cmd()
        .args(["init", remote_path.to_str().unwrap(), "--name", "agwt"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stderr(predicate::str::contains("Cloning"));

    let bare_dir = project_dir.join(".bare");
    assert!(bare_dir.exists());

    // --- list (default branch present) ---
    gwt(&bare_dir)
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("main"));

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
    let push = git_cmd()
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

    // --- list (only default branch remains) ---
    let output = gwt(&bare_dir).arg("list").assert().success();
    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    assert!(stdout.contains("main"), "main should still be listed");
    assert!(
        !stdout.contains("test-integration"),
        "removed branch should be gone"
    );
}
