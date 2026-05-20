//! Integration tests that exercise the full agwt workflow.
//!
//! All tests are self-contained: they create a local bare git repo as the
//! "remote" so no network access or credentials are needed.
//!
//! Run with:
//!   cargo test --test integration

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

/// Helper: build an `agwt` command with test-safe git configuration.
fn agwt_cmd() -> Command {
    let mut cmd = Command::cargo_bin("agwt").unwrap();
    cmd.env("GIT_CONFIG_COUNT", "1")
        .env("GIT_CONFIG_KEY_0", "safe.bareRepository")
        .env("GIT_CONFIG_VALUE_0", "all");
    cmd
}

/// Helper: build a `git` command with test-safe configuration applied.
fn git_cmd() -> std::process::Command {
    let mut cmd = std::process::Command::new("git");
    cmd.env("GIT_CONFIG_COUNT", "1")
        .env("GIT_CONFIG_KEY_0", "safe.bareRepository")
        .env("GIT_CONFIG_VALUE_0", "all");
    cmd
}

/// Helper: build an `agwt` command pointing at a specific bare dir.
fn gwt(bare_dir: &std::path::Path) -> Command {
    let mut cmd = agwt_cmd();
    cmd.arg("--bare-dir").arg(bare_dir);
    cmd
}

/// Helper: create a local bare repo seeded with a commit on main.
/// Returns (TempDir, remote_path). The TempDir must be kept alive.
fn setup_local_remote() -> (TempDir, std::path::PathBuf) {
    let remote_tmp = TempDir::new().unwrap();
    let remote_path = remote_tmp.path().join("remote.git");
    git_cmd()
        .args(["init", "--bare"])
        .arg(&remote_path)
        .output()
        .unwrap();

    // Seed it with a commit on main via a temporary working copy
    let seed_tmp = TempDir::new().unwrap();
    let seed_dir = seed_tmp.path().join("seed");
    git_cmd()
        .args(["clone", remote_path.to_str().unwrap(), "seed"])
        .current_dir(seed_tmp.path())
        .output()
        .unwrap();
    std::fs::write(seed_dir.join("README.md"), "# test\n").unwrap();
    git_cmd()
        .args(["add", "."])
        .current_dir(&seed_dir)
        .output()
        .unwrap();
    git_cmd()
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
    git_cmd()
        .args(["branch", "-M", "main"])
        .current_dir(&seed_dir)
        .output()
        .unwrap();
    git_cmd()
        .args(["push", "-u", "origin", "main"])
        .current_dir(&seed_dir)
        .output()
        .unwrap();
    // Set HEAD on the bare remote to main
    git_cmd()
        .args(["symbolic-ref", "HEAD", "refs/heads/main"])
        .current_dir(&remote_path)
        .output()
        .unwrap();

    (remote_tmp, remote_path)
}

/// Helper: init a fresh project using a local remote.
/// Returns (remote_tmp, project_tmp, bare_dir PathBuf).
/// Both TempDirs must be kept alive for the duration of the test.
fn init_fresh() -> (TempDir, TempDir, std::path::PathBuf) {
    let (remote_tmp, remote_path) = setup_local_remote();

    let project_tmp = TempDir::new().unwrap();
    agwt_cmd()
        .args(["init", remote_path.to_str().unwrap(), "--name", "agwt"])
        .current_dir(project_tmp.path())
        .assert()
        .success();

    let bare_dir = project_tmp.path().join("agwt").join(".bare");
    (remote_tmp, project_tmp, bare_dir)
}

/// Helper: init a fresh project with a second remote named "upstream".
/// Returns (remote_tmp, upstream_tmp, project_tmp, bare_dir PathBuf).
fn init_fresh_with_second_remote() -> (TempDir, TempDir, TempDir, std::path::PathBuf) {
    let (remote_tmp, project_tmp, bare_dir) = init_fresh();

    // Create a second bare repo as "upstream"
    let upstream_tmp = TempDir::new().unwrap();
    let upstream_path = upstream_tmp.path().join("upstream.git");
    git_cmd()
        .args(["init", "--bare"])
        .arg(&upstream_path)
        .output()
        .unwrap();
    // Seed it
    let seed_tmp = TempDir::new().unwrap();
    let seed_dir = seed_tmp.path().join("seed");
    git_cmd()
        .args(["clone", upstream_path.to_str().unwrap(), "seed"])
        .current_dir(seed_tmp.path())
        .output()
        .unwrap();
    std::fs::write(seed_dir.join("README.md"), "# upstream\n").unwrap();
    git_cmd()
        .args(["add", "."])
        .current_dir(&seed_dir)
        .output()
        .unwrap();
    git_cmd()
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
    git_cmd()
        .args(["branch", "-M", "main"])
        .current_dir(&seed_dir)
        .output()
        .unwrap();
    git_cmd()
        .args(["push", "-u", "origin", "main"])
        .current_dir(&seed_dir)
        .output()
        .unwrap();
    git_cmd()
        .args(["symbolic-ref", "HEAD", "refs/heads/main"])
        .current_dir(&upstream_path)
        .output()
        .unwrap();

    // Add it as a remote in the bare repo
    git_cmd()
        .args(["remote", "add", "upstream", upstream_path.to_str().unwrap()])
        .current_dir(&bare_dir)
        .output()
        .unwrap();
    git_cmd()
        .args(["fetch", "--quiet", "upstream"])
        .current_dir(&bare_dir)
        .output()
        .unwrap();

    (remote_tmp, upstream_tmp, project_tmp, bare_dir)
}

// =============================================================================
// init
// =============================================================================

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
    git_cmd()
        .args(["init", "--bare"])
        .arg(&remote_path)
        .output()
        .unwrap();
    let seed_tmp = TempDir::new().unwrap();
    let seed_dir = seed_tmp.path().join("seed");
    git_cmd()
        .args(["clone", remote_path.to_str().unwrap(), "seed"])
        .current_dir(seed_tmp.path())
        .output()
        .unwrap();
    std::fs::write(seed_dir.join("README.md"), "# test\n").unwrap();
    git_cmd()
        .args(["add", "."])
        .current_dir(&seed_dir)
        .output()
        .unwrap();
    git_cmd()
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
    git_cmd()
        .args(["branch", "-M", "main"])
        .current_dir(&seed_dir)
        .output()
        .unwrap();
    git_cmd()
        .args(["push", "-u", "origin", "main"])
        .current_dir(&seed_dir)
        .output()
        .unwrap();
    git_cmd()
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

// =============================================================================
// list
// =============================================================================

/// List: freshly-inited repo shows the default branch worktree
#[test]
fn list_empty() {
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
fn list_empty_after_remove() {
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
    assert!(stdout.contains("main"), "main should still be listed");
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
    let push = git_cmd()
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
    let push = git_cmd()
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

    // Fetch so local knows about the remote advance
    git_cmd()
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
    let lock = git_cmd()
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
    git_cmd()
        .args(["worktree", "unlock", wt_dir.to_str().unwrap()])
        .current_dir(&bare_dir)
        .output()
        .unwrap();

    gwt(&bare_dir)
        .args(["remove", "test-list-locked", "--force"])
        .assert()
        .success();
}

// =============================================================================
// create
// =============================================================================

/// Create: branch from default base
#[test]
fn create_default_base() {
    let (_remote_tmp, _project_tmp, bare_dir) = init_fresh();

    gwt(&bare_dir)
        .args(["create", "test/create-default"])
        .assert()
        .success()
        .stderr(predicate::str::contains("Created"));

    gwt(&bare_dir)
        .args(["remove", "test-create-default", "--force"])
        .assert()
        .success();
}

/// Create: slash in branch name becomes dash in directory name
#[test]
fn create_slash_to_dash() {
    let (_remote_tmp, _project_tmp, bare_dir) = init_fresh();
    let project_dir = bare_dir.parent().unwrap();

    gwt(&bare_dir)
        .args(["create", "feature/my-thing"])
        .assert()
        .success();

    assert!(project_dir.join("feature-my-thing").exists());

    gwt(&bare_dir)
        .args(["remove", "feature-my-thing", "--force"])
        .assert()
        .success();
}

/// Create: --name override
#[test]
fn create_name_override() {
    let (_remote_tmp, _project_tmp, bare_dir) = init_fresh();
    let project_dir = bare_dir.parent().unwrap();

    gwt(&bare_dir)
        .args(["create", "test/custom-dir", "--name", "my-custom-dir"])
        .assert()
        .success();

    assert!(project_dir.join("my-custom-dir").exists());

    gwt(&bare_dir)
        .args(["remove", "my-custom-dir", "--force"])
        .assert()
        .success();
}

/// Create: --base explicit base ref
#[test]
fn create_with_base() {
    let (_remote_tmp, _project_tmp, bare_dir) = init_fresh();

    gwt(&bare_dir)
        .args(["create", "test/from-main", "--base", "main"])
        .assert()
        .success()
        .stderr(predicate::str::contains("from main"));

    gwt(&bare_dir)
        .args(["remove", "test-from-main", "--force"])
        .assert()
        .success();
}

/// Create: fails if directory already exists
#[test]
fn create_fails_if_dir_exists() {
    let (_remote_tmp, _project_tmp, bare_dir) = init_fresh();
    let project_dir = bare_dir.parent().unwrap();

    std::fs::create_dir(project_dir.join("exists")).unwrap();

    gwt(&bare_dir)
        .args(["create", "test/whatever", "--name", "exists"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("already exists"));
}

/// Create: fails with invalid base ref
#[test]
fn create_fails_invalid_base() {
    let (_remote_tmp, _project_tmp, bare_dir) = init_fresh();

    gwt(&bare_dir)
        .args([
            "create",
            "test/bad-base",
            "--base",
            "nonexistent-branch-xyz",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found"));
}

// =============================================================================
// checkout
// =============================================================================

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
    let push = git_cmd()
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
    let push = git_cmd()
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
    let push = git_cmd()
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

// =============================================================================
// remove
// =============================================================================

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

    // Without --force: confirm "y" but git worktree remove still fails (dirty)
    gwt(&bare_dir)
        .args(["remove", "test-rm-dirty"])
        .write_stdin("y\n")
        .assert()
        .failure();

    // With --force should succeed (skips confirmation and forces removal)
    gwt(&bare_dir)
        .args(["remove", "test-rm-dirty", "--force"])
        .assert()
        .success()
        .stderr(predicate::str::contains("Removed"));
}

// =============================================================================
// sync
// =============================================================================

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
    let push = git_cmd()
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
    let push = git_cmd()
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
            "remote commit",
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

    // Make a non-conflicting local commit (different file)
    std::fs::write(wt_dir.join("local-file.txt"), "from local\n").unwrap();
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
    let log = git_cmd()
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
    let log = git_cmd()
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
    let push = git_cmd()
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
            "remote change",
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

    // Make a conflicting local commit
    std::fs::write(wt_dir.join("conflict.txt"), "local content\n").unwrap();
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
    git_cmd()
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
    let push = git_cmd()
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
// doctor
// =============================================================================

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

/// Shell-init: powershell output contains function and completion
#[test]
fn shell_init_powershell() {
    Command::cargo_bin("agwt")
        .unwrap()
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
    Command::cargo_bin("agwt")
        .unwrap()
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

// =============================================================================
// --remote option
// =============================================================================

/// Create: --remote uses a non-default remote
#[test]
fn create_with_remote() {
    let (_remote_tmp, _upstream_tmp, _project_tmp, bare_dir) = init_fresh_with_second_remote();

    gwt(&bare_dir)
        .args(["create", "test/from-upstream", "--remote", "upstream"])
        .assert()
        .success()
        .stderr(predicate::str::contains("Created"));

    gwt(&bare_dir)
        .args(["remove", "test-from-upstream", "--force"])
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
    let push = git_cmd()
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
    let push = git_cmd()
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

// =============================================================================
// doctor: additional cases
// =============================================================================

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
    // Get the remote URL from the bare repo
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

    // Doctor should detect "behind" (it does active-set fetch with prune internally)
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

// =============================================================================
// --version
// =============================================================================

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
