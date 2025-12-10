# Conformance Tests

Parser conformance tests using external test suites.

## Test262 Parser Tests

ECMAScript parser conformance tests from [tc39/test262-parser-tests](https://github.com/tc39/test262-parser-tests).

### Setup

```sh
./test262-fetch.sh
```

The script pins to a specific commit for reproducibility. Version info is stored in `test262/VERSION`.

### Running

```sh
# run all test262 tests
cargo test --release --test conformance

# filter by name
cargo test --release --test conformance -- pass/arrow

# list tests without running
cargo test --release --test conformance -- --list

# update known failures file
cargo test --release --test conformance -- --update-known-failures
```

### Test Categories

| Directory | Expected Behavior |
|-----------|-------------------|
| `pass/` | Should parse successfully (script mode) |
| `pass-explicit/` | Should parse successfully (module mode) |
| `fail/` | Should fail to parse |
| `early/` | Should parse but have semantic errors |

### Regression Tracking

Each suite has a `<suite>-known-failures.txt` file listing tests expected to fail:

- **Regression**: test fails that is NOT in known-failures
- **Progress**: test passes that IS in known-failures

CI passes if there are no regressions. To make progress, fix parser bugs and remove passing tests from the known-failures file.

### Updating Test262 Version

1. Update `TEST262_COMMIT` in `test262-fetch.sh`
2. Update `TEST262_COMMIT` in `test262.rs`
3. Run `./test262-fetch.sh`
4. Run tests with `--update-known-failures` to capture new baseline
