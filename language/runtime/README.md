# Runtime

The runtime is how Destack actually does anything interesting beyond pure computation.
The Destack runtime integrates VM and or native execution with scheduling, platform and host bindings, simulation, telemetry, and all the other runtime machinery (the cool people call this "effects").

## Testing

Run these from the repository root.

```sh
# focused local loop
cargo test -p destack_runtime
just language/check-runtime-macos # or linux/windows on matching hosts

# clean check
just language/check-quick

# exhaustive check
just language/check-full

# toolchain and target coverage
just language/doctor-toolchain
just language/lint-toolchain
just language/check-runtime-macos # and/or linux/windows-msvc on matching hosts
```
