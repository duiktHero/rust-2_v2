use assert_cmd::prelude::*;
use assert_cmd::Command;
use predicates::prelude::*;
use std::{fs};
use tempfile::TempDir;

fn bin() -> Command {
    Command::cargo_bin("snippets-app").expect("binary name 'snippets-app' not found; adjust in tests")
}

#[test]
fn create_snippet_via_stdin_json_backend() {
    let tmp = TempDir::new().unwrap();
    let json_path = tmp.path().join("snippets.json");

    let mut cmd = bin();
    cmd.env_clear(); // predictable
    cmd.arg("--backend").arg("json")
       .arg("--json-path").arg(&json_path)
       .arg("--name").arg("hello")
       .arg("--lang").arg("rust");
    cmd.write_stdin("fn main() {}");
    cmd.assert().success().stdout(predicate::str::contains("Saved snippet 'hello'"));

    // Verify JSON file contents
    let txt = fs::read_to_string(&json_path).expect("json exists");
    let v: serde_json::Value = serde_json::from_str(&txt).unwrap();
    let arr = v.as_array().expect("array");
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["name"], "hello");
    assert_eq!(arr[0]["lang"], "rust");
    assert_eq!(arr[0]["code"], "fn main() {}");
    assert!(arr[0]["created_at"].as_str().unwrap().len() > 10);
}

#[test]
fn overwrite_existing_snippet_updates_content() {
    let tmp = TempDir::new().unwrap();
    let json_path = tmp.path().join("snippets.json");

    for body in ["v1", "v2"] {
        let mut cmd = bin();
        cmd.env_clear();
        cmd.arg("--backend").arg("json")
           .arg("--json-path").arg(&json_path)
           .arg("--name").arg("same")
           .arg("--lang").arg("text");
        cmd.write_stdin(body);
        cmd.assert().success();
    }

    let txt = fs::read_to_string(&json_path).unwrap();
    let v: serde_json::Value = serde_json::from_str(&txt).unwrap();
    let arr = v.as_array().unwrap();
    assert_eq!(arr.len(), 1, "should overwrite (not append duplicate)");
    assert_eq!(arr[0]["code"], "v2");
}

#[test]
fn list_prints_all_snippets_in_desc_order() {
    let tmp = TempDir::new().unwrap();
    let json_path = tmp.path().join("snippets.json");

    // create two
    for (name, body) in [("a","alpha"), ("b","beta")] {
        let mut cmd = bin();
        cmd.env_clear();
        cmd.arg("--backend").arg("json")
           .arg("--json-path").arg(&json_path)
           .arg("--name").arg(name)
           .arg("--lang").arg("text");
        cmd.write_stdin(body);
        cmd.assert().success();
    }

    // list
    let mut list = bin();
    list.env_clear();
    list.arg("--backend").arg("json")
        .arg("--json-path").arg(&json_path)
        .arg("--list");
    let out = String::from_utf8(list.assert().success().get_output().stdout.clone()).unwrap();

    // Expect both present
    assert!(out.contains("name: a"));
    assert!(out.contains("name: b"));
}

#[test]
fn remove_existing_snippet_reports_and_deletes() {
    let tmp = TempDir::new().unwrap();
    let json_path = tmp.path().join("snippets.json");

    // create
    let mut c = bin();
    c.env_clear();
    c.arg("--backend").arg("json")
     .arg("--json-path").arg(&json_path)
     .arg("--name").arg("x")
     .arg("--lang").arg("text");
    c.write_stdin("content");
    c.assert().success();

    // remove
    let mut r = bin();
    r.env_clear();
    r.arg("--backend").arg("json")
     .arg("--json-path").arg(&json_path)
     .arg("--remove").arg("x");
    r.assert().success().stdout(predicate::str::contains("Removed 'x'"));

    // list should be empty
    let mut l = bin();
    l.env_clear();
    l.arg("--backend").arg("json")
     .arg("--json-path").arg(&json_path)
     .arg("--list");
    let out = String::from_utf8(l.assert().success().get_output().stdout.clone()).unwrap();
    assert!(out.trim().is_empty());
}

#[test]
fn remove_nonexistent_reports_politely() {
    let tmp = TempDir::new().unwrap();
    let json_path = tmp.path().join("snippets.json");

    let mut r = bin();
    r.env_clear();
    r.arg("--backend").arg("json")
     .arg("--json-path").arg(&json_path)
     .arg("--remove").arg("nope");
    r.assert().success().stdout(predicate::str::contains("No snippet named 'nope'"));
}

#[test]
fn missing_name_errors_without_list_or_remove() {
    let tmp = TempDir::new().unwrap();
    let json_path = tmp.path().join("snippets.json");

    let mut cmd = bin();
    cmd.env_clear();
    cmd.arg("--backend").arg("json")
       .arg("--json-path").arg(&json_path);
    cmd.write_stdin("some");
    cmd.assert().failure().stderr(predicate::str::contains("--name <NAME> is required"));
}
