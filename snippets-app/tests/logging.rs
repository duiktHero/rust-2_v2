use assert_cmd::prelude::*;
use assert_cmd::Command;
use std::fs;
use tempfile::TempDir;

fn bin() -> Command {
    Command::cargo_bin("snippets-app")
        .expect("binary name 'snippets-app' not found; adjust in tests")
}

#[test]
fn logging_to_file_when_env_set() {
    let tmp = TempDir::new().unwrap();
    let log = tmp.path().join("logs/app.log");
    let json_path = tmp.path().join("snippets.json");

    let mut c = bin();
    c.env_clear();
    c.env("SNIPPETS_APP_LOG_LEVEL", "debug");
    c.env("SNIPPETS_APP_LOG_PATH", log.to_string_lossy().to_string());
    c.arg("--backend")
        .arg("json")
        .arg("--json-path")
        .arg(&json_path)
        .arg("--name")
        .arg("lg")
        .arg("--lang")
        .arg("text");
    c.write_stdin("body");
    c.assert().success();

    let content = fs::read_to_string(&log).expect("log file created");
    assert!(content.contains("Snippet saved"));
    assert!(content.contains("backend=Json"));
}
