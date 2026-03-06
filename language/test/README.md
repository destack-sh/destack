# Destack Language Tests

Integration and fixture-based tests for the Destack language toolchain.

> See [TESTING.md](../../TESTING.md) for the overall testing philosophy and strategy.

## Test Suites

| Suite | Location | Description |
|-------|----------|-------------|
| **Smoke** | `fixtures/smoke/` | Parser and compiler don't crash, no errors on valid input |
| **Codegen** | `fixtures/codegen/` | Transpilation output matches expected snapshots |
| **Specification** | `fixtures/specification/` | MDTest-driven type checking and diagnostics |
| **Query** | `fixtures/query/` | MDTest-driven IDE/LSP queries |
| **Conformance** | `fixtures/parser/conformance/` | Parser conformance against established test suites |
| **Formatter** | `fixtures/formatter/` | Format roundtrip stability |
| **Resolver** | `fixtures/resolver/` | Module resolution from Node-style and TypeScript-style layouts |
| **Interop canary** | `fixtures/ecosystem/` | Curated TS-first Node, backend, and tooling packages |
| **Stress** | `fixtures/stress/` | Scale limits: large files, many modules, deep nesting |

## Testing

Run these from `language/` unless noted otherwise.
Use `just test` as an alias for `just quick`.
`quick` is the main high-signal correctness lane.
`commit` currently reuses that deterministic lane.
`nightly` adds ecosystem canaries and stress.

```bash
# shared gate lanes
just test
just quick
just commit
just nightly

# run all tests via cargo
cargo test -p destack_test

# run specific test suites
cargo test --test smoke
cargo test --test codegen
cargo test --test specification
cargo test --test query
cargo test --test parser-conformance
cargo test --test formatter
cargo test --test ecosystem
cargo test --test stress

# resolver tests
cargo test -p destack_resolver

# filter by name
cargo test --test smoke -- parser
cargo test --test smoke -- compiler
cargo test --test specification -- basics

# list tests without running
cargo test --test smoke -- --list

# verbose output
cargo test --test smoke -- --verbose
```

## MDTest Framework

The MDTest framework powers both **Specification** and **Query** tests using markdown-driven test definitions.

### Specification Tests

Type checking specification tests live in `fixtures/specification/`.
They are markdown files with code blocks and expected error messages.
See `fixtures/specification/README.md` for format details.

### Query Tests

IDE and LSP query tests live in `fixtures/query/`.
They use marker annotations such as `def:`, `use:`, and `$0` to specify locations and expected results.
They cover navigation, completion, and rename workflows.

## Stress Tests

Stress tests verify that the toolchain handles extreme scale.
Fixtures are generated on-demand:

```bash
just generate-stress
just test-stress
```

See `fixtures/stress/README.md` for categories and fixture details.
