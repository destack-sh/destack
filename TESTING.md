# Testing

Destack is a universal software engine for building correct, optimal, integrated software, so of course testing Destack's own correctness and performance itself is critical.
Because of the breadth and depth of the project, testing is non-trivial, and some bigger platform tests cannot be run locally because they require specific hardware targets.

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
| [**Regression**](language/test/fixtures/regression/) | Correctness | Quick | Targeted bug reproductions that do not fit cleanly elsewhere |
| [**Conformance**](language/test/fixtures/conformance/) | Conformance | Mixed | External compatibility suites organized by domain, with `ecma` remaining corpus-first and `web` and `node` becoming feature-first |
| [**Query**](language/test/fixtures/query/) | Correctness | Quick | Query-layer IDE behavior such as navigation, completion, rename, and diagnostics |
| [**LSP**](language/test/fixtures/lsp/) | Correctness | Quick | Applied LSP editor scenarios over the real in-process language server |
| [**Resolver**](language/test/fixtures/resolver/) | Correctness | Quick | Node and TypeScript style module and package resolution |
| [**Formatter**](language/test/fixtures/formatter/) | Correctness | Quick | Formatting behavior on first-party fixtures |
| [**Grammar**](language/grammar/README.md) | Correctness | Quick | Tree-sitter grammar routing, corpus coverage, and specification sweeps |
| [**Ecosystem**](language/test/fixtures/ecosystem/) | Conformance | Full | Curated TS-first Node, backend, and tooling packages |
| [**Stress**](language/test/fixtures/stress/) | Correctness | Full | Very large or pathological inputs that should still complete correctly |

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
| formatter | oxfmt | Formatter Oxfmt | ignore 1 | 8c3607060b7432d51bcd0b049cb77bed473d35e3 |
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

## Targets

This table is the operational testing view: which workflows run, which commands back them, and how CI executes the lane.
Native Linux, macOS, and Windows lanes run on matching GitHub Actions runners.
iOS and Android lanes use SDK-backed cross compilation.
See [`TARGETS.md`](TARGETS.md) for the canonical support policy.

| Target triple | Tier | Primary workflow | Command | Backing implementation |
|---------------|------|------------------|---------|------------------------|
| `x86_64-unknown-linux-gnu` | Tier 1 | `runtime-linux-check.yml` | `just language/check-runtime-linux` | inline `cargo check`, `clippy`, and host tests in [language/justfile](/Users/florian/symbol/destack/language/justfile) |
| `aarch64-unknown-linux-gnu` | Tier 1 | `runtime-linux-check.yml` | `just language/check-runtime-linux` | same host lane on `ubuntu-24.04-arm` |
| `aarch64-apple-darwin` | Tier 1 | `runtime-macos-check.yml` | `just language/check-runtime-macos` | inline `cargo check`, `clippy`, and host tests in [language/justfile](/Users/florian/symbol/destack/language/justfile) |
| `x86_64-pc-windows-msvc` | Tier 1 | `runtime-windows-check.yml` | `just language/check-runtime-windows-msvc` | [check-runtime-windows-msvc.sh](/Users/florian/symbol/destack/dev/toolchain/check-runtime-windows-msvc.sh) |
| `aarch64-apple-ios` | Tier 2 | `runtime-ios-check.yml` | `just language/check-runtime-ios` | [check-runtime-ios.sh](/Users/florian/symbol/destack/dev/toolchain/check-runtime-ios.sh) |
| `aarch64-linux-android` | Tier 2 | `runtime-android-check.yml` | `just language/check-runtime-android` | [check-runtime-android.sh](/Users/florian/symbol/destack/dev/toolchain/check-runtime-android.sh) |

## Toolchains

Use the `just` commands as the public interface.
The scripts below are the backing implementation for target-specific lanes.
GitHub Actions Rust lanes use the shared setup actions, which also enable `sccache` through `setup-rust-just` and `setup-rust-bun-just`.

| Purpose | Public command | Backing script |
|---------|----------------|----------------|
| CI workflow policy checks | `just check-workflow-policy` | [check-workflow-policy.sh](/Users/florian/symbol/destack/dev/ci/check-workflow-policy.sh) |
| CI hygiene tools | `just install-hygiene-toolchain`, `just doctor-hygiene-toolchain`, `just ensure-hygiene-toolchain` | [hygiene-toolchain.sh](/Users/florian/symbol/destack/dev/ci/hygiene-toolchain.sh) |
| Runtime toolchain management | `just language/install-toolchain`, `just language/doctor-toolchain`, `just language/ensure-toolchain`, `just language/lint-toolchain` | [runtime-toolchain.sh](/Users/florian/symbol/destack/dev/toolchain/runtime-toolchain.sh) |
| Bridge toolchain management | `just bridge/install-toolchain`, `just bridge/doctor-toolchain`, `just bridge/ensure-toolchain` | [language-bridge-toolchain.sh](/Users/florian/symbol/destack/bridge/scripts/language-bridge-toolchain.sh) |
| Android host prerequisites | `just language/install-runtime-android-host-deps` | [install-runtime-android-host-deps.sh](/Users/florian/symbol/destack/dev/toolchain/install-runtime-android-host-deps.sh) |
| Android SDK and NDK install | `just language/install-runtime-android-ndk` | [install-android-ndk.sh](/Users/florian/symbol/destack/.github/scripts/install-android-ndk.sh) |
| Linux Wayland runtime lane | `just language/check-runtime-linux-wayland` | [check-runtime-linux-wayland.sh](/Users/florian/symbol/destack/dev/toolchain/check-runtime-linux-wayland.sh) |

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
just language/test-regression
just language/test-query
just language/test-lsp
just language/test-resolver
just language/test-formatter
just language/test-grammar
just language/test-conformance
just language/test-conformance-ecma
just language/test-conformance-formatter
just language/update-conformance-catalog
just language/fetch-ecosystem
just language/test-ecosystem
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
just language/fuzz
just language/fuzz-lexer
just language/fuzz-parser
just language/fuzz-formatter
```
