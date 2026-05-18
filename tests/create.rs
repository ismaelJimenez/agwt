mod common;

use common::{git_cmd, gwt, init_fresh, init_fresh_with_second_remote};
use predicates::prelude::*;

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

/// Create: stores base branch in git config
#[test]
fn create_stores_base_in_config() {
    let (_remote_tmp, _project_tmp, bare_dir) = init_fresh();

    gwt(&bare_dir)
        .args(["create", "test/base-stored", "--base", "main"])
        .assert()
        .success();

    // Verify agwt-base is stored in git config
    let output = git_cmd()
        .args(["config", "--get", "branch.test/base-stored.agwt-base"])
        .current_dir(&bare_dir)
        .output()
        .unwrap();
    assert!(output.status.success(), "agwt-base config should be set");
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "main");

    // Verify list hides "(from main)" since main is the default branch
    let output = gwt(&bare_dir).arg("list").assert().success();
    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    assert!(
        !stdout.contains("(from main)"),
        "should hide base when it is the default branch, got: {stdout}"
    );

    gwt(&bare_dir)
        .args(["remove", "test-base-stored", "--force"])
        .assert()
        .success();
}

/// Create: stores default branch as base when --base is omitted
#[test]
fn create_stores_default_base_in_config() {
    let (_remote_tmp, _project_tmp, bare_dir) = init_fresh();

    gwt(&bare_dir)
        .args(["create", "test/implicit-base"])
        .assert()
        .success();

    // Verify agwt-base is stored with the default branch name
    let output = git_cmd()
        .args(["config", "--get", "branch.test/implicit-base.agwt-base"])
        .current_dir(&bare_dir)
        .output()
        .unwrap();
    assert!(output.status.success(), "agwt-base config should be set");
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "main");

    gwt(&bare_dir)
        .args(["remove", "test-implicit-base", "--force"])
        .assert()
        .success();
}
