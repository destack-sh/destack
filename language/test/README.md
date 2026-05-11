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

The language suite taxonomy is:

| Suite | Family | Gate | Location | Purpose |
|-------|--------|------|----------|---------|
| **Unit** | Correctness | Quick | crate local tests | Internal invariants and focused logic |
| **Smoke** | Correctness | Quick | `fixtures/smoke/` | Broad sanity checks for parser and compiler flows |
| **Emit** | Correctness | Standalone | `fixtures/emit/` | Emitted output matches curated snapshots |
| **Specification** | Correctness | Quick | `fixtures/specification/` | First-party language semantics and diagnostics |
| **Conformance** | Conformance | Mixed | `fixtures/conformance/` | External parser and formatter corpora used as regression inputs, not product compatibility targets |
| **Query** | Correctness | Quick | `fixtures/query/` | Query-layer IDE behavior |
| **LSP** | Correctness | Quick | `fixtures/lsp/` | Applied editor scenarios over the real in-process LSP server |
| **Resolver** | Correctness | Quick | crate local tests | Module path resolution |
| **Formatter** | Correctness | Quick | `fixtures/formatter/` | Formatting behavior on first-party fixtures |
| **Stress** | Correctness | Full | `fixtures/stress/` | Shared hostile corpora and invariant-based consumers for parser, checker, query, and LSP |

The shared conformance catalog is generated from `suite.json` and `status.json`.

<!-- begin:conformance-catalog -->
| Domain | Suite | Title | Status | Origin Ref |
| --- | --- | --- | --- | --- |
| ecma | babel | ECMA Babel | ignore 7 | b8ef443e0a3ee202264fb40edc1cbce8f2352aaa |
| ecma | biome | ECMA Biome | known-fail 1, ignore 13 | 9f1b3b06586401b39e0aa886bf7c8484fd2a6ded |
| ecma | jsc | ECMA JSC | none | main |
| ecma | math | Math | none | 5c8206929d81b2d3d727ca6aac56c18358c8d790 |
| ecma | number | Number | translated 4, excluded 4 | 5c8206929d81b2d3d727ca6aac56c18358c8d790 |
| ecma | swc | ECMA SWC | ignore 3 | 5b9d77c1c89ade5772c6feee429386faf3b93a39 |
| ecma | temporal | Temporal | translated 3 | 5c8206929d81b2d3d727ca6aac56c18358c8d790 |
| ecma | test262 | ECMA Test262 | ignore 7 | 0e808c74fbec780646434cad17bb22dc52461003 |
| ecma | v8 | ECMA V8 | none | main |
| formatter | oxfmt | Formatter Oxfmt | known-fail-idempotence 4, ignore 12 | 8c3607060b7432d51bcd0b049cb77bed473d35e3 |
<!-- end:conformance-catalog -->

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
DESTACK_TEST_THREADS=4 DESTACK_TEST_JOBS=16 just test-conformance
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
just test-conformance
just test-conformance-ecma
just test-conformance-formatter
just update-conformance-catalog

# robustness
just generate-stress
just test-stress
just test-stress-query
just test-stress-lsp
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
