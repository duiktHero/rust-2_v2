use assert_cmd::prelude::*;
use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;
use httptest::{Server, Expectation, matchers::*, responders::*};

fn bin() -> Command {
    Command::cargo_bin("snippets-app").expect("binary name 'snippets-app' not found; adjust in tests")
}

#[test]
fn download_uses_http_body_as_snippet_code() {
    let server = Server::run();
    server.expect(
        Expectation::matching(all_of![request::method("GET"), request::path("/snippet")])
            .respond_with(status_code(200).body("from-http")),
    );

    let tmp = TempDir::new().unwrap();
    let json_path = tmp.path().join("snippets.json");

    let mut c = bin();
    c.env_clear();
    c.arg("--backend").arg("json")
     .arg("--json-path").arg(&json_path)
     .arg("--name").arg("net")
     .arg("--download").arg(server.url("/snippet").to_string());
    c.assert().success();

    let txt = std::fs::read_to_string(&json_path).unwrap();
    let v: serde_json::Value = serde_json::from_str(&txt).unwrap();
    assert_eq!(v.as_array().unwrap()[0]["code"], "from-http");
}

#[test]
fn download_non_200_fails() {
    let server = Server::run();
    server.expect(
        Expectation::matching(request::path("/oops"))
            .respond_with(status_code(404)),
    );

    let tmp = TempDir::new().unwrap();
    let db = tmp.path().join("snippets.sqlite3");

    let mut c = bin();
    c.env_clear();
    c.arg("--backend").arg("sqlite")
     .arg("--sqlite-path").arg(&db)
     .arg("--name").arg("net")
     .arg("--download").arg(server.url("/oops").to_string());
    c.assert().failure().stderr(predicate::str::contains("HTTP 404"));
}
