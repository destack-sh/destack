# Testing

Destack is a universal software engine for building correct, optimal, integrated software, so of course testing Destack's own correctness and performance is quite important.

## Gates

| Gate | Purpose | Local command | CI usage |
|------|---------|---------------|----------|
| **Quick** | Fast, deterministic confidence for normal development and mainline verification | `just quick` | `main` push verification, split across `Hygiene Check`, area `Check` jobs, the Windows resolver check, and the Linux runtime checks |
| **Full** | Deep verification for broad local validation and release depth | `just full` | Scheduled nightly verification, signed nightly canary packaging, and release verification |

When you need to tune test concurrency, set `DESTACK_TEST_THREADS`.
That knob drives Rust `libtest` concurrency and also feeds custom language harness jobs by default.
Set `DESTACK_TEST_JOBS` only when a custom harness should use a different worker count than Rust `libtest`.

## Terminology

| Command family | Meaning |
|----------------|---------|
| `format`, `format-check` | Rewrite or verify formatting |
| `check` | Static analysis and compile-time validation |
| `build` | Produce build artifacts |
| `test` | Run the normal deterministic test aggregate for that area |
| `install-*`, `fetch-*`, `generate-*` | Prepare prerequisites or generated inputs |
| `doctor-*`, `ensure-*` | Inspect or guarantee toolchain readiness |
| `validate-*` | Validate publish payloads or policy state |
| `publish-*` | Publish artifacts or packages |

## Suites

| Suite | Family | Gate | Purpose |
|------|--------|------|---------|
| [**Unit**](language/test/README.md) | Correctness | Quick | Internal invariants in parser, compiler, runtime, resolver, and related crates |
| [**Emit**](language/test/fixtures/emit/) | Correctness | Standalone | Emitted output matches curated checked-in snapshots |
| [**Specification**](language/test/fixtures/specification/) | Correctness | Quick | First-party language semantics and diagnostics |
| [**Conformance**](language/test/fixtures/conformance/) | Conformance | Mixed | External parser and formatter corpora used as regression inputs, not product compatibility targets |
| [**Query**](language/test/fixtures/query/) | Correctness | Quick | Query-layer IDE behavior such as navigation, completion, rename, and diagnostics |
| [**LSP**](language/test/fixtures/lsp/) | Correctness | Quick | Applied LSP editor scenarios over the real in-process language server |
| **Resolver** | Correctness | Quick | Crate-local module path resolution tests |
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
| formatter | oxfmt | Formatter Oxfmt | excluded 17, known-fail-idempotence 4 | 8c3607060b7432d51bcd0b049cb77bed473d35e3 |
<!-- end:conformance-catalog -->

## Commands

Run these commands from the repository root.

```bash
# tune test concurrency when needed
DESTACK_TEST_THREADS=8 just quick
DESTACK_TEST_THREADS=4 DESTACK_TEST_JOBS=16 just language/test-conformance

# gates
just quick
just full

# language correctness suites
just language/test
just language/test-unit
just language/test-emit
just language/test-specification
just language/test-query
just language/test-lsp
just language/test-resolver
just language/test-formatter
just language/test-grammar
just language/test-conformance
just language/test-conformance-ecma
just language/test-conformance-formatter
just language/update-conformance-catalog
just language/generate-stress
just language/test-stress

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
just language/list-fuzz-targets
```
