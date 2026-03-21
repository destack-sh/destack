# Toolchain

Host setup and target tooling for Destack runtime, bridge, and release workflows.

## Shape

This directory has three layers.

`runtime-toolchain.sh` owns environment readiness.
It installs, verifies, or lints required host tools, SDKs, NDKs, and Rust targets.

`check-runtime-*-rust.sh` owns Rust target validation for one runtime target.
These scripts run Cargo checks, clippy, and non-executing test builds for that target.

`check-runtime-*-host.sh` owns native host shell validation.
These scripts run Gradle, SwiftPM, or Xcode-facing host package checks.

`check-runtime-*.sh` composes the Rust target lane with the relevant host shell lane.
These are the full gates that local full checks and CI should prefer.

## Testing

Run these from the repository root.

```sh
# environment
just language/doctor-toolchain
just language/ensure-toolchain

# focused runtime loops
just language/check-runtime-android-rust
just language/check-runtime-android-host
just language/check-runtime-ios-rust
just language/check-runtime-ios-host
just language/check-runtime-macos-rust
just language/check-runtime-apple-host

# full runtime gates
just language/check-runtime-android
just language/check-runtime-ios
just language/check-runtime-macos
```
