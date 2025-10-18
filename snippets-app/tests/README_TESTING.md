# Test suite for `snippets-app`

This folder contains integration tests that exercise the CLI end-to-end, plus behavior-driven-style test names.

## Setup

1. Ensure your `Cargo.toml` includes the following dev-dependencies:

```
# Add this to your Cargo.toml
[dev-dependencies]
assert_cmd = "2.0"
predicates = "3.1"
tempfile = "3.10"
serde_json = "1.0"
httptest = "0.15"
wait-timeout = "0.2"
# Optional, useful for concurrency control in some envs
serial_test = "2.0"

```

2. If your binary is not named `snippets-app`, change the string in `cargo_bin("snippets-app")` accordingly.

## Run

```
cargo test
```

The tests isolate themselves using temporary directories and files.
