# Testing

Destack is a universal software engine for building correct, optimal, integrated software, so of course testing Destack's own correctness and performance itself is critical.
Because of the breadth and depth of the project, testing is non-trivial, and some bigger platform tests cannot be run locally because they require specific hardware targets.

## Gates

| Gate | Purpose | Local command | CI usage |
|------|---------|---------------|----------|
| **Quick** | Fast, deterministic confidence for normal development and pull requests | `just quick` | Pull request and `main` push verification, split across `CI Hygiene`, scoped `format-check`, scoped `check`, scoped `test`, and tiered runtime jobs |
| **Full** | Deep verification for broad local validation and release depth | `just full` | Nightly integration, release verification, and release blocking runtime smoke lanes |

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
| [**Emit**](language/test/fixtures/emit/) | Correctness | Quick | Emitted output matches curated checked-in snapshots |
| [**Specification**](language/test/fixtures/specification/) | Correctness | Quick | First-party language semantics and diagnostics |
| [**Query**](language/test/fixtures/query/) | Correctness | Quick | IDE and LSP behavior such as navigation, completion, rename, and diagnostics |
| [**Resolver**](language/test/fixtures/resolver/) | Correctness | Quick | Node and TypeScript style module and package resolution |
| [**Formatter**](language/test/fixtures/formatter/) | Correctness | Quick | Formatting behavior on first-party fixtures |
| [**Grammar**](language/grammar/README.md) | Correctness | Quick | Tree-sitter grammar routing, corpus coverage, and specification sweeps |
| [**Parser Conformance**](language/test/fixtures/parser/conformance/) | Conformance | Quick | Behavior against pinned upstream parser suites |
| [**Formatter Conformance**](language/test/fixtures/formatter/conformance/) | Conformance | Quick | Behavior against pinned upstream formatter suites |
| [**Ecosystem**](language/test/fixtures/ecosystem/) | Conformance | Full | Curated TS-first Node, backend, and tooling packages |
| [**Stress**](language/test/fixtures/stress/) | Correctness | Full | Very large or pathological inputs that should still complete correctly |

## Targets

This table is the operational testing view: which workflows run, which commands back them, and how CI executes the lane.
Native Linux, macOS, and Windows lanes run on matching GitHub Actions runners.
iOS and Android lanes use SDK-backed cross compilation.
Windows GNU uses a cross build plus Wine execution model.
See [`TARGETS.md`](TARGETS.md) for the canonical support policy.

| Target triple | Tier | Primary workflow | Command | Backing implementation |
|---------------|------|------------------|---------|------------------------|
| `x86_64-unknown-linux-gnu` | Tier 1 | `runtime-linux.yml` | `just language/check-runtime-linux` | inline `cargo check`, `clippy`, host tests, and `_smoke-runtime-host` in [language/justfile](/Users/florian/symbol/destack/language/justfile) |
| `aarch64-unknown-linux-gnu` | Tier 1 | `runtime-linux.yml` | `just language/check-runtime-linux` | same host lane on `ubuntu-24.04-arm` |
| `aarch64-apple-darwin` | Tier 1 | `runtime-macos.yml` | `just language/check-runtime-macos` | inline `cargo check`, `clippy`, host tests, and `_smoke-runtime-host` in [language/justfile](/Users/florian/symbol/destack/language/justfile) |
| `x86_64-pc-windows-msvc` | Tier 1 | `runtime-windows.yml` | `just language/check-runtime-windows-msvc` | [check-runtime-windows-msvc.sh](/Users/florian/symbol/destack/scripts/toolchain/check-runtime-windows-msvc.sh) |
| `x86_64-pc-windows-gnu` | Tier 1 | `runtime-windows-gnu.yml` | `just language/check-runtime-windows-gnu` | [check-runtime-windows-gnu.sh](/Users/florian/symbol/destack/scripts/toolchain/check-runtime-windows-gnu.sh) |
| `aarch64-apple-ios` | Tier 2 | `runtime-ios.yml` | `just language/check-runtime-ios` | [check-runtime-ios.sh](/Users/florian/symbol/destack/scripts/toolchain/check-runtime-ios.sh) |
| `aarch64-linux-android` | Tier 2 | `runtime-android.yml` | `just language/check-runtime-android` | [check-runtime-android.sh](/Users/florian/symbol/destack/scripts/toolchain/check-runtime-android.sh) |
| `wasm32-wasip1` | Tier 3 | `runtime-wasip1.yml` | `just language/check-runtime-wasip1` | [check-runtime-wasip1.sh](/Users/florian/symbol/destack/scripts/toolchain/check-runtime-wasip1.sh) |
| `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`, `x86_64-pc-windows-gnu` | Policy/support cross check | `runtime-cross.yml` | `just language/check-runtime-cross-targets` | [check-runtime-cross-targets.sh](/Users/florian/symbol/destack/scripts/toolchain/check-runtime-cross-targets.sh) |

## Toolchains

Use the `just` commands as the public interface.
The scripts below are the backing implementation for target-specific lanes.

| Purpose | Public command | Backing script |
|---------|----------------|----------------|
| CI workflow policy checks | `just check-workflow-policy` | [check-workflow-policy.sh](/Users/florian/symbol/destack/scripts/ci/check-workflow-policy.sh) |
| CI hygiene tools | `just install-hygiene-toolchain`, `just doctor-hygiene-toolchain`, `just ensure-hygiene-toolchain` | [hygiene-toolchain.sh](/Users/florian/symbol/destack/scripts/ci/hygiene-toolchain.sh) |
| Runtime toolchain management | `just language/install-toolchain`, `just language/doctor-toolchain`, `just language/ensure-toolchain`, `just language/lint-toolchain` | [runtime-toolchain.sh](/Users/florian/symbol/destack/scripts/toolchain/runtime-toolchain.sh) |
| Bridge toolchain management | `just bridge/install-toolchain`, `just bridge/doctor-toolchain`, `just bridge/ensure-toolchain` | [language-bridge-toolchain.sh](/Users/florian/symbol/destack/bridge/scripts/language-bridge-toolchain.sh) |
| Android host prerequisites | `just language/install-runtime-android-host-deps` | [install-runtime-android-host-deps.sh](/Users/florian/symbol/destack/scripts/toolchain/install-runtime-android-host-deps.sh) |
| Android SDK and NDK install | `just language/install-runtime-android-ndk` | [install-android-ndk.sh](/Users/florian/symbol/destack/.github/scripts/install-android-ndk.sh) |
| Linux Wayland runtime lane | `just language/check-runtime-linux-wayland` | [check-runtime-linux-wayland.sh](/Users/florian/symbol/destack/scripts/toolchain/check-runtime-linux-wayland.sh) |
| Native host release smoke | `just language/smoke-runtime-release "<target>"` | [smoke-runtime-release.sh](/Users/florian/symbol/destack/scripts/toolchain/smoke-runtime-release.sh) |
| Windows GNU runtime lane | `just language/check-runtime-windows-gnu` | [check-runtime-windows-gnu.sh](/Users/florian/symbol/destack/scripts/toolchain/check-runtime-windows-gnu.sh) |
| Runtime cross-target lane | `just language/check-runtime-cross-targets` | [check-runtime-cross-targets.sh](/Users/florian/symbol/destack/scripts/toolchain/check-runtime-cross-targets.sh) |

## Commands

Run these commands from the repository root.

```bash
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
just language/test-resolver
just language/test-formatter
just language/test-grammar
just language/test-parser-conformance
just language/test-formatter-conformance
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
just language/check-runtime-windows-gnu
just language/check-runtime-ios
just language/check-runtime-android
just language/install-runtime-android-ndk
just language/smoke-runtime-release
just language/check-runtime-wasip1
just language/check-runtime-cross-targets

# language performance tools
just language/benchmark
just language/benchmark-parser
just language/benchmark-lexer
just language/benchmark-compiler
just language/fuzz
just language/fuzz-lexer
just language/fuzz-parser
just language/fuzz-formatter
```
