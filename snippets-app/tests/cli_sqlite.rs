use assert_cmd::prelude::*;
use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

fn bin() -> Command {
    Command::cargo_bin("snippets-app").expect("binary name 'snippets-app' not found; adjust in tests")
}

#[test]
fn sqlite_backend_roundtrip_create_list_remove() {
    let tmp = TempDir::new().unwrap();
    let db = tmp.path().join("snippets.sqlite3");

    // create
    let mut c = bin();
    c.env_clear();
    c.arg("--backend").arg("sqlite")
     .arg("--sqlite-path").arg(&db)
     .arg("--name").arg("sql")
     .arg("--lang").arg("sql");
    c.write_stdin("select 1;");
    c.assert().success().stdout(predicate::str::contains("Saved snippet 'sql'"));

    // list
    let mut l = bin();
    l.env_clear();
    l.arg("--backend").arg("sqlite").arg("--sqlite-path").arg(&db).arg("--list");
    let out = String::from_utf8(l.assert().success().get_output().stdout.clone()).unwrap();
    assert!(out.contains("name: sql"));

    // remove
    let mut r = bin();
    r.env_clear();
    r.arg("--backend").arg("sqlite").arg("--sqlite-path").arg(&db).arg("--remove").arg("sql");
    r.assert().success().stdout(predicate::str::contains("Removed 'sql'"));
}
