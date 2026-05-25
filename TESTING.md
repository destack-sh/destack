# Testing

Destack is a universal software engine for building correct, optimal, integrated software, so of course testing Destack's own correctness and performance is quite important.

## Checks

| Check | Purpose | Local command | CI usage |
|------|---------|---------------|----------|
| **Check Quick** | Fast, deterministic confidence for normal development and mainline validation | `just check`, `just check-quick` | `main` push validation, split across `Hygiene Check`, area `Check` jobs, and the Linux runtime checks |
| **Check Full** | Deep validation for broad local and release confidence | `just check-full` | Scheduled nightly validation, signed nightly packaging, and release validation |

## Commands

Run these commands from the repository root.

```bash
# checks
just check
just check-quick
just check-full

# language correctness suites
just language/test
just language/test-unit
just language/test-specification
just language/test-query
just language/test-lsp
just language/test-formatter
just language/test-grammar
just language/test-conformance
just language/test-conformance-ecma
just language/test-conformance-formatter
just language/update-conformance-catalog
just language/generate-stress
just language/test-stress
just language/test-stress-parser
just language/test-stress-formatter

# runtime and toolchain lanes
just language/install-toolchain
just language/doctor-toolchain
just language/ensure-toolchain
just language/lint-toolchain
just language/check-runtime-linux
just language/check-runtime-macos
just language/check-runtime-windows-msvc

# language performance tools
just language/bench
just language/bench-parser
just language/bench-lexer
just language/bench-compiler
just language/bench-compiler-stats
just language/bench-linter-stats
just language/fuzz
just language/fuzz-lexer
just language/fuzz-parser
just language/fuzz-formatter
just language/fuzz-list
```
