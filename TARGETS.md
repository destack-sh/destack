# Target Support

This file is the canonical support policy for build and runtime targets across the repository.
Support is keyed by Rust target triple (for now).
See [TESTING.md](TESTING.md) for the operational gate, workflow, and script mapping that exercises this policy.

## Tiers

| Tier | Guarantee |
|-----------|--------|
| Tier 1 | Release blocking, exercised on scheduled full lanes, and expected to pass with check, clippy, and tests on supported hosts. |
| Tier 2 | Validated on scheduled full lanes and release, with compile checks or host checks as appropriate. |
| Tier 3 | Best effort compile only, no release blocking guarantees. |

## Runtime

| Target triple | Tier | CI lane | Validation |
|-----------|--------|--------|--------|
| `aarch64-apple-darwin` | Tier 1 | `runtime-macos-check` | `cargo check`, `cargo clippy --all-targets`, and `runtime::host::macos` tests |
| `x86_64-pc-windows-msvc` | Tier 1 | `runtime-windows-check` | `cargo check`, `cargo clippy --all-targets`, and `runtime::host::windows` tests |
| `x86_64-unknown-linux-gnu` | Tier 1 | `runtime-linux-check` | `cargo check`, `cargo clippy --all-targets`, and `runtime::host::linux` tests on x86_64 host runners |
| `aarch64-unknown-linux-gnu` | Tier 1 | `runtime-linux-check` | `cargo check`, `cargo clippy --all-targets`, and `runtime::host::linux` tests on aarch64 host runners |
| `aarch64-apple-ios` | Tier 2 | `runtime-ios-check` | SDK-aware compile checks with clippy and `--no-run` host tests |
| `aarch64-linux-android` | Tier 2 | `runtime-android-check` | NDK-aware compile checks with clippy and `--no-run` host tests |

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
| Run iOS runtime target lane | `just language/check-runtime-ios` |
| Run Android runtime target lane | `just language/check-runtime-android` |
| Install Android SDK and NDK into the active sdk root | `just language/install-runtime-android-ndk` |

### Mainline required checks

These are the cheap required checks for direct pushes and optional branch protection on `main`.

| Check |
|-----------|
| `Hygiene Check` |
| `Language Resolver Check (Windows)` |
| `Runtime Check (Linux, x86_64)` |
| `Runtime Check (Linux, aarch64)` |
