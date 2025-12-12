# Conformance Tests

Parser conformance tests using external test suites.

## Quick Start

```sh
# install all test fixtures
just language/install-fixtures

# run all conformance tests
cargo test --release --test conformance

# run a specific suite
cargo test --release --test conformance -- --test262
```

## Test Suites

| Suite | Source | Tests | Description |
|-------|--------|-------|-------------|
| babel | [babel/babel](https://github.com/babel/babel) | ~4,016 | TypeScript, JSX/TSX |
| biome | [biomejs/biome](https://github.com/biomejs/biome) | ~637 | JS/TS |
| swc | [swc-project/swc](https://github.com/swc-project/swc) | ~685 | TypeScript, JSX/TSX |
| test262 | [tc39/test262-parser-tests](https://github.com/tc39/test262-parser-tests) | ~5,363 | ECMAScript |

## Regression Tracking

Each suite has a `<suite>-known-failures.txt` file listing tests expected to fail:

- **Regression**: test fails that is NOT in known-failures
- **Progress**: test passes that IS in known-failures

The overall conformance test suite passes if there are no *regressions*. 
To make progress, fix bugs and remove newly passing tests from the known-failures files.

### Updating Baselines

```sh
# update known failures for all suites
cargo test --release --test conformance -- --update-known-failures

# update for a specific suite
cargo test --release --test conformance -- --test262 --update-known-failures
```

## Updating Suite Versions

Each suite is pinned to a specific commit for reproducibility.

1. Update the commit SHA in `<suite>-fetch.sh`
2. Update the commit SHA in `<suite>.rs`
3. Run the fetch script
4. Run tests with `--update-known-failures` to capture new baseline

## Manual Setup

If you need to fetch suites individually:

```sh
./test262-fetch.sh
./babel-fetch.sh
./swc-fetch.sh
./biome-fetch.sh
```
