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
| **Conformance** | Conformance | Mixed | `fixtures/conformance/` | External compatibility suites organized by domain, with `ecma` remaining corpus-first and `web` and `node` becoming feature-first |
| **Query** | Correctness | Quick | `fixtures/query/` | Query-layer IDE behavior |
| **LSP** | Correctness | Quick | `fixtures/lsp/` | Applied editor scenarios over the real in-process LSP server |
| **Resolver** | Correctness | Quick | `fixtures/resolver/` | Module and package resolution |
| **Formatter** | Correctness | Quick | `fixtures/formatter/` | Formatting behavior on first-party fixtures |
| **Ecosystem** | Conformance | Full | `fixtures/ecosystem/` | Curated TS-first Node, backend, and tooling packages |
| **Stress** | Correctness | Full | `fixtures/stress/` | Shared hostile corpora and invariant-based consumers for parser, checker, query, and LSP |

The shared conformance catalog is generated from `suite.json` and `status.json`.

<!-- begin:conformance-catalog -->
| Domain | Suite | Title | Status | Origin Ref |
| --- | --- | --- | --- | --- |
| ecma | babel | ECMA Babel | ignore 7 | b8ef443e0a3ee202264fb40edc1cbce8f2352aaa |
| ecma | biome | ECMA Biome | known-fail 1, ignore 7 | 9f1b3b06586401b39e0aa886bf7c8484fd2a6ded |
| ecma | jsc | ECMA JSC | none | main |
| ecma | math | Math | none | 5c8206929d81b2d3d727ca6aac56c18358c8d790 |
| ecma | number | Number | translated 4, excluded 4 | 5c8206929d81b2d3d727ca6aac56c18358c8d790 |
| ecma | swc | ECMA SWC | ignore 2 | 5b9d77c1c89ade5772c6feee429386faf3b93a39 |
| ecma | temporal | Temporal | translated 3 | 5c8206929d81b2d3d727ca6aac56c18358c8d790 |
| ecma | test262 | ECMA Test262 | ignore 7 | 0e808c74fbec780646434cad17bb22dc52461003 |
| ecma | v8 | ECMA V8 | none | main |
| formatter | oxfmt | Formatter Oxfmt | known-fail-idempotence 4, ignore 8 | 8c3607060b7432d51bcd0b049cb77bed473d35e3 |
| node | crypto | node:crypto | none | 7547e795ef700e1808702fc2851a0dcc3395a065 |
| node | fs | node:fs | none | 7547e795ef700e1808702fc2851a0dcc3395a065 |
| node | net | node:net | none | 7547e795ef700e1808702fc2851a0dcc3395a065 |
| node | os | node:os | none | 7547e795ef700e1808702fc2851a0dcc3395a065 |
| node | path | node:path | none | 7547e795ef700e1808702fc2851a0dcc3395a065 |
| node | process | node:process | none | 7547e795ef700e1808702fc2851a0dcc3395a065 |
| node | random | node:random | none | 7547e795ef700e1808702fc2851a0dcc3395a065 |
| node | streams | node:streams | none | 7547e795ef700e1808702fc2851a0dcc3395a065 |
| node | thread | node:thread | none | 7547e795ef700e1808702fc2851a0dcc3395a065 |
| node | time | node:time | none | 7547e795ef700e1808702fc2851a0dcc3395a065 |
| node | tls | node:tls | none | 7547e795ef700e1808702fc2851a0dcc3395a065 |
| node | tty | node:tty | translated 10, excluded 3 | 7547e795ef700e1808702fc2851a0dcc3395a065 |
| node | url | node:url | none | 7547e795ef700e1808702fc2851a0dcc3395a065 |
| web | bluetooth | Bluetooth | none | 55d076dca564300616a75eec5ec696e805c7bc3b |
| web | encoding | Encoding | none | 55d076dca564300616a75eec5ec696e805c7bc3b |
| web | fetch | fetch | translated 20, excluded 11 | 55d076dca564300616a75eec5ec696e805c7bc3b |
| web | fileapi | FileAPI | translated 5 | 55d076dca564300616a75eec5ec696e805c7bc3b |
| web | filesystem-access | File System Access | none | 55d076dca564300616a75eec5ec696e805c7bc3b |
| web | indexeddb | IndexedDB | none | 55d076dca564300616a75eec5ec696e805c7bc3b |
| web | streams | Streams | none | 55d076dca564300616a75eec5ec696e805c7bc3b |
| web | url | URL | none | 55d076dca564300616a75eec5ec696e805c7bc3b |
| web | webaudio | WebAudio | none | 55d076dca564300616a75eec5ec696e805c7bc3b |
| web | webcrypto | WebCrypto | none | 55d076dca564300616a75eec5ec696e805c7bc3b |
| web | webgpu | WebGPU | none | 9726cfe2893834c4bb42b435638c4e7362f4c258 |
| web | webserial | WebSerial | none | 55d076dca564300616a75eec5ec696e805c7bc3b |
| web | workers | Workers | none | 55d076dca564300616a75eec5ec696e805c7bc3b |
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
just fetch-ecosystem
just test-ecosystem

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
