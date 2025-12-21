# Destack Language Tests

Integration and fixture-based tests for the Destack language toolchain.

> See [TESTING.md](../../TESTING.md) for the overall testing philosophy and strategy.

## Test Suites

| Suite | Location | Description |
|-------|----------|-------------|
| **Smoke** | `fixtures/smoke/` | Parser and compiler don't crash, no errors on valid input |
| **Codegen** | `fixtures/codegen/` | Transpilation output matches expected snapshots |
| **Specification** | `fixtures/specification/` | MDTest-driven type checking and diagnostics |
| **Query** | `fixtures/query/` | MDTest-driven IDE/LSP queries (goto definition, completion, rename) |
| **Conformance** | `fixtures/conformance/` | Parser conformance against established test suites |
| **Formatter** | `fixtures/formatter/` | Format roundtrip stability |
| **Resolver** | `fixtures/resolver/` | Module resolution (from enhanced-resolve) |
| **Ecosystem** | `fixtures/ecosystem/` | Real-world package parsing |
| **Stress** | `fixtures/stress/` | Scale limits: large files, many modules, deep nesting |

## Running Tests

```bash
# run all tests via cargo
cargo test -p destack_test

# run specific test suites
cargo test --test smoke           # smoke tests only
cargo test --test codegen         # codegen tests only
cargo test --test specification   # type checking specification tests
cargo test --test query           # IDE query tests
cargo test --test stress          # stress tests (requires generated fixtures)

# filter by name
cargo test --test smoke -- parser      # only parser smoke tests
cargo test --test smoke -- compiler    # only compiler smoke tests
cargo test --test specification -- basics       # filter specification tests by path

# list tests without running
cargo test --test smoke -- --list

# verbose output
cargo test --test smoke -- --verbose
```

## Via Justfile

```bash
cd language
just test                # run all language tests
just test-smoke          # run smoke tests
just test-codegen        # run codegen tests
just test-specification  # run type checking specification tests
just test-query          # run IDE query tests
just test-conformance    # run conformance tests
just test-formatter      # run formatter tests
just test-stress         # run stress tests
```

## MDTest Framework

The MDTest framework powers both **Specification** and **Query** tests using markdown-driven test definitions.

### Specification Tests (`fixtures/specification/`)

Type checking specification tests. Tests are markdown files with code blocks and expected error messages.
See `fixtures/specification/README.md` for format details.

### Query Tests (`fixtures/query/`)

IDE/LSP query tests using marker annotations (`def:`, `use:`, `$0`) to specify locations and expected results.
Supports:
- `navigation/` - goto definition
- `assist/` - code completion
- `refactor/` - rename

## Stress Tests

Stress tests verify the toolchain handles extreme scale. Fixtures are generated on-demand:

```bash
# generate stress fixtures (not checked into git)
just generate-stress

# run stress tests
just test-stress
```

See `fixtures/stress/README.md` for categories and fixture details.
