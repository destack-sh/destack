# Target Support

This file is the canonical support policy for build and runtime targets across the repository.
Support is keyed by Rust target triple (for now).
See [TESTING.md](TESTING.md) for the operational gate, workflow, and script mapping that exercises this policy.

## Tiers

| Tier | Guarantee |
|-----------|--------|
| Tier 1 | Required on every pull request, release blocking, host checks are expected to pass with check, clippy, and tests. |
| Tier 2 | Required compile checks on pull requests, deeper host or integration checks may run on scheduled lanes. |
| Tier 3 | Best effort compile only, no release blocking guarantees. |

## Runtime

| Target triple | Tier | CI lane | Validation |
|-----------|--------|--------|--------|
| `aarch64-apple-darwin` | Tier 1 | `runtime-macos` | `cargo check`, `cargo clippy --all-targets`, and `runtime::host::macos` tests |
| `x86_64-pc-windows-msvc` | Tier 1 | `runtime-windows` | `cargo check`, `cargo clippy --all-targets`, and `runtime::host::windows` tests |
| `x86_64-pc-windows-gnu` | Tier 1 | `runtime-windows-gnu` | `cargo check`, target `clippy`, and runnable Windows tests through Wine |
| `x86_64-unknown-linux-gnu` | Tier 1 | `runtime-linux` | `cargo check`, `cargo clippy --all-targets`, and `runtime::host::linux` tests on x86_64 host runners |
| `aarch64-unknown-linux-gnu` | Tier 1 | `runtime-linux` | `cargo check`, `cargo clippy --all-targets`, and `runtime::host::linux` tests on aarch64 host runners |
| `aarch64-apple-ios` | Tier 2 | `runtime-ios` | SDK-aware compile checks with clippy and `--no-run` host tests |
| `aarch64-linux-android` | Tier 2 | `runtime-android` | NDK-aware compile checks with clippy and `--no-run` host tests |
| `wasm32-wasip1` | Tier 3 | `runtime-wasip1` | `cargo check --target wasm32-wasip1` |

### Local workflow

Run these commands from repository root when setting up or validating runtime target lanes.

| Goal | Command |
|-----------|--------|
| Install runtime toolchains and sdk prerequisites | `just language/install-toolchain` |
| Inspect host readiness for runtime target lanes | `just language/doctor-toolchain` |
| Lint runtime toolchain scripts | `just language/lint-toolchain` |
| Run host runtime lane on macOS | `just language/check-runtime-macos` |
| Run host runtime lane on Linux | `just language/check-runtime-linux` |
| Run host runtime lane on Windows | `just language/check-runtime-windows-msvc` |
| Run windows gnu runtime cross lane | `just language/check-runtime-windows-gnu` |
| Run iOS runtime target lane | `just language/check-runtime-ios` |
| Run Android runtime target lane | `just language/check-runtime-android` |
| Install Android SDK and NDK into the active sdk root | `just language/install-runtime-android-ndk` |
| Run wasm32-wasip1 runtime target lane | `just language/check-runtime-wasip1` |
| Run runtime cross-target compile lane | `just language/check-runtime-cross-targets` |

### Tier 1 required checks

Tier 1 branch protection should require these CI checks on `main`.

| Check |
|-----------|
| `CI Hygiene` |
| `Runtime Linux (x86_64)` |
| `Runtime Linux (aarch64)` |
| `Runtime macOS` |
| `Runtime Windows` |
| `Runtime Windows GNU` |
