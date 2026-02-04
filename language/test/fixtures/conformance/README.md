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

# run a specific suite with an inner filter
cargo test --release --test conformance -- --babel --suite-filter type-only-import-export-specifiers
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

## Test Semantics

Conformance suites are parser-first unless a suite explicitly marks early-error tests.

- babel, biome, swc: parse-only for both passing and failing tests.
- test262 pass and pass-explicit: parse-only.
- test262 fail: parse-only and must fail to parse.
- test262 early: early checks are enabled and must produce an error.

We do not attempt to match external error messages or error codes.
We only require that an error is surfaced in the relevant category.
The parser is intentionally lenient for binding identifiers, including reserved words like `yield`.
Reserved binding diagnostics belong in analysis, not parse errors, so parser suites should not expect failures here.

## Support Boundaries

Conformance expectations follow `language/INTEROPERABILITY.md`.
All files parse as strict modules and script mode is out of scope.
TypeScript syntax is rejected in `.js` and `.jsx` by default.
JSDoc typing and `@ts-check` semantics are out of scope.
JSX is only enabled in `.jsx` and `.tsx`.
Decorators are only enabled in `.ts`, `.tsx`, and `.ds`.
Import attributes and `using` are supported across file types.
Annex B and other sloppy mode behaviors are treated as expected failures.

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
