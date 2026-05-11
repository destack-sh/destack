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
| [**Smoke**](language/test/fixtures/smoke/) | Correctness | Quick | Broad no-crash and basic no-regression coverage for parser and compiler flows |
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
just language/test-smoke
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
just language/check-runtime-ios
just language/check-runtime-android
just language/install-runtime-android-ndk

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
