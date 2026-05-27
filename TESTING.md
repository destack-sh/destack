# Testing

Destack is a universal software engine for building correct, optimal, integrated software, so of course testing Destack's own correctness and performance is quite important.

## Checks

| Check | Purpose | Local command | CI usage |
|------|---------|---------------|----------|
| **Check Quick** | Fast, deterministic confidence for normal development and mainline validation | `just check`, `just check-quick` | `main` push validation, split across `Hygiene Check`, area `Check` jobs, and the Linux runtime checks |
| **Check Full** | Deep validation for broad local and release confidence | `just check-full` | Scheduled nightly validation, signed nightly packaging, and release validation |

## Terminology

| Command family | Meaning |
|----------------|---------|
| `format`, `format-check` | Rewrite or check formatting |
| `check`, `check-quick`, `check-full` | Aggregate confidence checks |
| `lint` | Static analysis and compile-time validation |
| `build` | Produce build artifacts |
| `test` | Run the normal deterministic test aggregate for that area |
| `install-*`, `fetch-*`, `generate-*` | Prepare prerequisites or generated inputs |
| `doctor-*`, `ensure-*` | Inspect or guarantee toolchain readiness |
| `validate-*` | Validate publish payloads or policy state |
| `publish-*` | Publish artifacts or packages |

## Suites

| Suite | Family | Check | Purpose |
|------|--------|------|---------|
| [**Unit**](language/test/README.md) | Correctness | Quick | Internal invariants in parser, compiler, runtime, resolver, and related crates |
| [**Emit**](language/test/fixtures/emit/) | Correctness | Standalone | Emitted output matches curated checked-in snapshots |
| [**Specification**](language/test/fixtures/specification/) | Correctness | Quick | First-party language semantics and diagnostics |
| [**Conformance**](language/test/fixtures/conformance/) | Conformance | Mixed | External parser and formatter corpora used as regression inputs, not product compatibility targets |
| [**Query**](language/test/fixtures/query/) | Correctness | Quick | Query-layer IDE behavior such as navigation, completion, rename, and diagnostics |
| [**LSP**](language/test/fixtures/lsp/) | Correctness | Quick | Applied LSP editor scenarios over the real in-process language server |
| [**Formatter**](language/test/fixtures/formatter/) | Correctness | Quick | Formatting behavior on first-party fixtures |
| [**Grammar**](language/grammar/README.md) | Correctness | Quick | Tree-sitter grammar routing, corpus coverage, and specification sweeps |
| [**Stress**](language/test/fixtures/stress/) | Correctness | Full | Very large or pathological inputs that should still complete correctly |

The shared conformance catalog is generated from `suite.json` and `status.json`.

<!-- begin:conformance-catalog -->
| Domain | Suite | Title | Status | Origin Ref |
| --- | --- | --- | --- | --- |
| ecma | babel | ECMA Babel | excluded 45 | b8ef443e0a3ee202264fb40edc1cbce8f2352aaa |
| ecma | biome | ECMA Biome | excluded 57 | 9f1b3b06586401b39e0aa886bf7c8484fd2a6ded |
| ecma | swc | ECMA SWC | excluded 23 | 5b9d77c1c89ade5772c6feee429386faf3b93a39 |
| ecma | test262 | ECMA Test262 | excluded 20 | 0e808c74fbec780646434cad17bb22dc52461003 |
| formatter | oxfmt | Formatter Oxfmt | excluded 19, known-fail-idempotence 4 | 8c3607060b7432d51bcd0b049cb77bed473d35e3 |
<!-- end:conformance-catalog -->

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
just language/test-emit
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
