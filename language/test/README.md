# Destack Language Tests

This crate and fixture tree provide the language-specific test suites for Destack.
See [TESTING.md](../../TESTING.md) for the repository-wide model.

## Structure

Language testing uses two storage patterns.
Unit tests stay with the crates they exercise.
Fixture-driven suites live under `language/test/fixtures/` and are run by the `destack_test` harness.
There is no separate `unit/` fixture tree.
Unit tests enter the language gate model through `just test-unit`.

## Suite Taxonomy

The current language suite taxonomy is:

| Suite | Family | Gate | Location | Purpose |
|-------|--------|------|----------|---------|
| **Unit** | Correctness | Quick | crate local tests | Internal invariants and focused logic |
| **Smoke** | Correctness | Quick | `fixtures/smoke/` | Broad sanity checks for parser and compiler flows |
| **Emit** | Correctness | Standalone | `fixtures/emit/` | Emitted output matches curated snapshots |
| **Specification** | Correctness | Quick | `fixtures/specification/` | First-party language semantics and diagnostics |
| **Query** | Correctness | Quick | `fixtures/query/` | Query-layer IDE behavior |
| **LSP** | Correctness | Quick | `fixtures/lsp/` | Applied editor scenarios over the real in-process LSP server |
| **Resolver** | Correctness | Quick | `fixtures/resolver/` | Module and package resolution |
| **Formatter** | Correctness | Quick | `fixtures/formatter/` | Formatting behavior on first-party fixtures |
| **Parser Conformance** | Conformance | Quick | `fixtures/parser/conformance/` | Behavior against pinned upstream parser suites |
| **Formatter Conformance** | Conformance | Quick | `fixtures/formatter/conformance/` | Behavior against pinned upstream formatter suites |
| **Ecosystem** | Conformance | Full | `fixtures/ecosystem/` | Curated TS-first Node, backend, and tooling packages |
| **Stress** | Correctness | Full | `fixtures/stress/` | Correctness on very large and pathological inputs |

`node-conformance` will join the external conformance family once it lands.

## Gates

Run these from `language/` unless noted otherwise.

| Gate | Meaning |
|------|---------|
| **Quick** | Fast deterministic language verification |
| **Full** | `quick` plus ecosystem and stress coverage |

`just test` is the language test aggregate used by `just quick`.
`just test-emit` stays standalone until the emit pipeline is mature enough to trust in the fast gate.
`just full` then adds the slower ecosystem and stress lanes.

## Concurrency

Use `DESTACK_TEST_THREADS` to control Rust `libtest` concurrency and the default custom harness worker count.
Set `DESTACK_TEST_JOBS` only when a custom harness should use a different worker count than `libtest`.

```bash
# cap everything to 4 workers
DESTACK_TEST_THREADS=4 just quick

# keep rust tests at 4 but let a custom harness fan out further
DESTACK_TEST_THREADS=4 DESTACK_TEST_JOBS=16 just test-parser-conformance
```

## Commands

Use these commands when iterating on the language stack.

```bash
# aggregate lanes
just test
just quick
just full

# core suites
just test-unit
just test-smoke
just test-emit
just test-specification
just test-query
just test-lsp
just test-resolver
just test-formatter
just test-grammar

# external suites
just test-parser-conformance
just test-formatter-conformance
just fetch-ecosystem
just test-ecosystem

# robustness
just generate-stress
just test-stress
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

This crate also houses MIR benchmark workloads through [`mirbench/`](mirbench/README.md).
Benchmarks are measurement tools, not ordinary correctness gates.
Stress is a correctness suite under extreme scale, not a benchmark suite.
