# Toolchain

Host setup and target tooling for Destack runtime, bridge, and release workflows.

## Shape

This directory has three layers.

`runtime-toolchain.sh` owns environment readiness.
It installs, verifies, or lints required host tools and Rust targets.

`check-runtime-*.sh` owns one runtime target lane.
These scripts run Cargo checks and target-specific host validation.

## Testing

Run these from the repository root.

```sh
# environment
just language/doctor-toolchain
just language/ensure-toolchain

# focused runtime loops
just language/check-runtime-linux
just language/check-runtime-macos-rust
just language/check-runtime-windows-msvc

# full runtime gates
just language/check-runtime-linux
just language/check-runtime-macos
just language/check-runtime-windows-msvc
```
