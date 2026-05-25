# Destack Language Tests

This crate and fixture tree provide the language-specific test suites for Destack.
See [TESTING.md](../../TESTING.md) for the repository-wide model.

## Structure

Language testing uses two storage patterns.
Unit tests stay with the crates they exercise.
Fixture-driven suites live under `language/test/fixtures/` and are run by the `destack_test` harness.
There is no separate `unit/` fixture tree.
Unit tests enter the language check model through `just test-unit`.

## Checks

Run these from `language/` unless noted otherwise.

| Check | Meaning |
|------|---------|
| **Check Quick** | Fast deterministic language validation |
| **Check Full** | `check-quick` plus ecosystem and stress coverage |

`just test` is the language test aggregate used by `just check-quick`.
`just check-full` then adds the slower ecosystem and stress lanes.

## Concurrency

Use `DESTACK_TEST_THREADS` to control Rust `libtest` concurrency and the default custom harness worker count.
Set `DESTACK_TEST_JOBS` only when a custom harness should use a different worker count than `libtest`.

```bash
# cap everything to 4 workers
DESTACK_TEST_THREADS=4 just check-quick

# keep rust tests at 4 but let a custom harness fan out further
DESTACK_TEST_THREADS=4 DESTACK_TEST_JOBS=16 just test-conformance
```

## Commands

Use these commands when iterating on the language stack.

```bash
# aggregate lanes
just test
just check-quick
just check-full

# core suites
just test-unit
just test-specification
just test-query
just test-lsp
just test-formatter
just test-grammar

# external suites
just test-conformance
just test-conformance-ecma
just test-conformance-formatter
just update-conformance-catalog

# robustness
just generate-stress
just test-stress
just test-stress-parser
just test-stress-formatter
```

## Mdtest

The mdtest framework powers the specification, query, and LSP suites.
Specification fixtures define language semantics and diagnostics in markdown.
Query fixtures define lower-level query behavior through source markers and expected results.
LSP fixtures define applied editor scenarios through `ds:path` source blocks and `lsp ...` expectation blocks.

## Direct Entry Points

Use the justfile commands as the public interface.
The direct Cargo entry points are useful when working on one harness in isolation.

```bash
cargo test -p destack_test --test specification
cargo test -p destack_test --test query
cargo test -p destack_test --test lsp
```

## Performance

Benchmarks are measurement tools, not ordinary correctness gates.
Stress is a correctness suite under extreme scale, not a benchmark suite.
