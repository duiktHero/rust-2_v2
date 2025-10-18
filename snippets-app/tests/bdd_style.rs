// Behavior-Driven style test names

use assert_cmd::prelude::*;
use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

fn bin() -> Command {
    Command::cargo_bin("snippets-app")
        .expect("binary name 'snippets-app' not found; adjust in tests")
}

#[test]
fn given_empty_store_when_list_then_prints_nothing() {
    let tmp = TempDir::new().unwrap();
    let json = tmp.path().join("snippets.json");
    let mut l = bin();
    l.env_clear();
    l.arg("--backend")
        .arg("json")
        .arg("--json-path")
        .arg(&json)
        .arg("--list");
    let out = String::from_utf8(l.assert().success().get_output().stdout.clone()).unwrap();
    assert!(out.trim().is_empty(), "expected no output for empty store");
}

#[test]
fn given_existing_snippet_when_create_with_same_name_then_overwrites() {
    let tmp = TempDir::new().unwrap();
    let json = tmp.path().join("snippets.json");

    for body in ["one", "two"] {
        let mut c = bin();
        c.env_clear();
        c.arg("--backend")
            .arg("json")
            .arg("--json-path")
            .arg(&json)
            .arg("--name")
            .arg("dup")
            .arg("--lang")
            .arg("text");
        c.write_stdin(body);
        c.assert().success();
    }

    let txt = std::fs::read_to_string(&json).unwrap();
    let v: serde_json::Value = serde_json::from_str(&txt).unwrap();
    assert_eq!(v.as_array().unwrap()[0]["code"], "two");
}
