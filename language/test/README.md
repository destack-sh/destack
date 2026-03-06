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

| Suite | Family | Location | Purpose |
|-------|--------|----------|---------|
| **Unit** | Core correctness | crate local tests | Internal invariants and focused logic |
| **Smoke** | Core correctness | `fixtures/smoke/` | Broad sanity checks for parser and compiler flows |
| **Emit** | Core correctness | `fixtures/emit/` | Emitted output matches curated snapshots |
| **Specification** | Core correctness | `fixtures/specification/` | First-party language semantics and diagnostics |
| **Query** | Core correctness | `fixtures/query/` | IDE and LSP behavior |
| **Resolver** | Core correctness | `fixtures/resolver/` | Module and package resolution |
| **Formatter** | Core correctness | `fixtures/formatter/` | Formatting behavior on first-party fixtures |
| **Parser Conformance** | External conformance | `fixtures/parser/conformance/` | Behavior against pinned upstream parser suites |
| **Formatter Conformance** | External conformance | `fixtures/formatter/conformance/` | Behavior against pinned upstream formatter suites |
| **Ecosystem** | External ecosystem coverage | `fixtures/ecosystem/` | Curated TS-first Node, backend, and tooling packages |
| **Stress** | Robustness | `fixtures/stress/` | Correctness on very large and pathological inputs |

`node-conformance` will join the external conformance family once it lands.

## Gates

Run these from `language/` unless noted otherwise.

| Gate | Meaning |
|------|---------|
| **Quick** | Fast deterministic language verification |
| **Full** | `quick` plus ecosystem and stress coverage |

`just test` is the language test aggregate used by `just quick`.
It includes unit, smoke, emit, specification, query, resolver, formatter, grammar, parser conformance, and formatter conformance.
`just full` then adds `ecosystem` and `stress`.

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

## MDTest

The MDTest framework powers both `specification` and `query`.
Specification fixtures define language semantics and diagnostics in markdown.
Query fixtures define IDE and LSP behavior through source markers and expected results.

## Performance

This crate also houses MIR benchmark workloads through [`mirbench/`](mirbench/README.md).
Benchmarks are measurement tools, not ordinary correctness gates.
Stress is a correctness suite under extreme scale, not a benchmark suite.
