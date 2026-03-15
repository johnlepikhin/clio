#![allow(deprecated)] // Command::cargo_bin — will migrate when assert_cmd stabilizes replacement

use assert_cmd::Command;
use std::fs;
use tempfile::TempDir;

/// Helper: create a temp dir with a config file pointing db_path into it.
fn setup_env() -> TempDir {
    let dir = TempDir::new().unwrap();
    let config_content = format!("db_path: {}/clio.db\n", dir.path().display());
    fs::write(dir.path().join("config.yaml"), &config_content).unwrap();
    dir
}

fn clio_cmd(dir: &TempDir) -> Command {
    let mut cmd = Command::cargo_bin("clio").unwrap();
    cmd.arg("--config").arg(dir.path().join("config.yaml"));
    cmd
}

#[test]
fn test_config_show() {
    let dir = setup_env();
    clio_cmd(&dir)
        .arg("config")
        .arg("show")
        .assert()
        .success();
}

#[test]
fn test_config_validate() {
    let dir = setup_env();
    clio_cmd(&dir)
        .arg("config")
        .arg("validate")
        .assert()
        .success();
}

#[test]
fn test_config_path() {
    let dir = setup_env();
    clio_cmd(&dir)
        .arg("config")
        .arg("path")
        .assert()
        .success();
}

#[test]
fn test_list_empty_db() {
    let dir = setup_env();
    clio_cmd(&dir).arg("list").assert().success();
}

#[test]
fn test_no_args_shows_help() {
    let dir = setup_env();
    clio_cmd(&dir).assert().failure();
}

#[test]
fn test_help_flag() {
    Command::cargo_bin("clio")
        .unwrap()
        .arg("--help")
        .assert()
        .success();
}

#[test]
fn test_version_flag() {
    Command::cargo_bin("clio")
        .unwrap()
        .arg("--version")
        .assert()
        .success();
}

#[test]
fn test_config_init_creates_file() {
    let dir = TempDir::new().unwrap();
    let output_path = dir.path().join("new-config.yaml");
    Command::cargo_bin("clio")
        .unwrap()
        .args(["config", "init", "--output"])
        .arg(&output_path)
        .assert()
        .success();
    assert!(output_path.exists(), "config init should create the file");
}

#[test]
fn test_malformed_config() {
    let dir = TempDir::new().unwrap();
    let bad_config = dir.path().join("bad.yaml");
    fs::write(&bad_config, "invalid: [yaml: {{broken").unwrap();
    Command::cargo_bin("clio")
        .unwrap()
        .arg("--config")
        .arg(&bad_config)
        .args(["config", "show"])
        .assert()
        .failure();
}

#[test]
fn test_copy_from_stdin() {
    let dir = setup_env();
    clio_cmd(&dir)
        .arg("copy")
        .write_stdin("hello from test")
        .assert()
        .success();

    // Verify entry was saved — list should show it
    let output = clio_cmd(&dir)
        .arg("list")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let stdout = String::from_utf8(output).unwrap();
    assert!(stdout.contains("hello from test"), "list should contain copied text");
}

#[test]
fn test_copy_empty_stdin_fails() {
    let dir = setup_env();
    clio_cmd(&dir)
        .arg("copy")
        .write_stdin("")
        .assert()
        .failure();
}

#[test]
fn test_select_nonexistent_id() {
    let dir = setup_env();
    clio_cmd(&dir)
        .args(["select", "999999"])
        .assert()
        .failure();
}
