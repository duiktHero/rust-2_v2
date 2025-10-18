use assert_cmd::prelude::*;
use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

fn bin() -> Command {
    Command::cargo_bin("snippets-app").expect("binary name 'snippets-app' not found; adjust in tests")
}

#[test]
fn empty_stdin_errors_with_hint() {
    let tmp = TempDir::new().unwrap();
    let db = tmp.path().join("snippets.sqlite3");

    let mut c = bin();
    c.env_clear();
    c.arg("--backend").arg("sqlite").arg("--sqlite-path").arg(&db)
     .arg("--name").arg("zzz");
    // DO NOT write to stdin
    c.assert().failure().stderr(predicate::str::contains("no data in STDIN; pass --download <URL> or pipe content"));
}
