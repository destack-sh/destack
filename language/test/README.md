# Destack Language Tests

Integration and fixture-based tests for the Destack language toolchain.

## Test Types

| Type | Location | Description |
|------|----------|-------------|
| **Smoke** | `fixtures/smoke/` | Parser and compiler don't crash, no errors on valid input |
| **Codegen** | `fixtures/codegen/` | Transpilation output matches expected snapshots |
| **Runtime** | `fixtures/runtime/` | MIR interpreter and runtime execution |
| **MDTest** | `fixtures/mdtest/` | Markdown-driven type checking and diagnostic tests |
| **Resolver** | `fixtures/resolver/` | Module resolution tests (from enhanced-resolve) |

## Running Tests

```bash
# run all tests via cargo
cargo test -p destack_test

# run specific test suites
cargo test --test smoke           # smoke tests only
cargo test --test codegen         # codegen tests only

# filter by name
cargo test --test smoke -- parser      # only parser smoke tests
cargo test --test smoke -- compiler    # only compiler smoke tests
cargo test --test smoke -- smoke1      # filter by test name

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
```
