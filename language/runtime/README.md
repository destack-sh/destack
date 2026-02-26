# Runtime

The runtime is how Destack actually does anything interesting beyond pure computation.
The Destack runtime wraps VM and/or native execution with scheduling, bindings, host integration, record/replay, telemetry, and all the other "runtime stuff".
Essentially, the runtime is where we integrate Node/Bun/Deno-level semantics with V8/JSC-runtime features .. and a bunch more stuff; it's really more like a universal game engine than a regular JS/TS runtime.

Runtime behavior is modeled along the three basic dimensions of engine ("where?"), execution ("how?") and world ("what?"):

| Dimension | Values | Purpose |
|-----------|--------|---------|
| engine | `vm`, `native` | chooses the execution engine |
| execution | `fast`, `deterministic`, `record`, `replay` | chooses determinism and replay behavior |
| world | `host`, `simulation` | chooses host-backed or simulation-backed bindings |

Component-wise, the runtime has a few main areas:
 - `platform/`: host implementations for the modules defined in the builtin ["platform"](language/builtin/lib/platform) lib
 - `runtime/`: all the core runtime scaffolding and orchestration (poller, scheduler/loop, etc.)

## Targets and Hosts

Runtime target support policy is defined in the repo-wide `TARGETS.md`.
`workspace` build `Platform` configuration selects product build behavior, while target support tiers define runtime and CI guarantees by Rust target triple.

<!-- FUGU: move runtime/host into host/? -->

## Modules

The platform module scope matrix is listed below.
Counts come from `bindings.generated.rs` and represent unique binding descriptors per module.
Scope is declared per binding descriptor in builtin metadata.
Generator validation enforces one effective scope per module.

| Module | Description |
|-----------|--------|
| [`audio`](./src/platform/audio) | Audio clocks, devices, streams, events, and MIDI I/O. |\| [`crypto`](./src/platform/crypto) | Cryptographic algorithms, keys, stores, certificates, and randomness. |
| [`debug`](./src/platform/debug) | Runtime tracing, profiling, inspector, and debug control hooks. |
| [`device`](./src/platform/device) | Host peripheral buses and device classes: serial, USB, Bluetooth, and camera. |
| [`display`](./src/platform/display) | Monitor discovery, display topology, and native window integration. |
| [`error`](./src/platform/error) | Runtime error bridge and structured host error conversion helpers. |
| [`ffi`](./src/platform/ffi) | Dynamic library loading, symbol lookup, pointer primitives, and foreign calls. |\| [`fs`](./src/platform/fs) | Filesystem paths, files, directories, metadata, watches, mapping, and extended attributes. |
| [`gpu`](./src/platform/gpu) | GPU adapters, devices, resources, pipelines, commands, presentation, and synchronization. |
| [`input`](./src/platform/input) | Input devices and streams: keyboard, pointer, touch, gamepad, raw HID, sensors, and text. |
| [`io`](./src/platform/io) | Generic host I/O primitives: control, polling, events, completions, and device endpoints. |
| [`ipc`](./src/platform/ipc) | Intra-host process communication: pipes, shared memory, local sockets, sync, and messages. |
| [`memory`](./src/platform/memory) | Virtual memory map, protect, advise, and lock operations. |
| [`net`](./src/platform/net) | Network addresses, sockets, listeners, interfaces, routes, and protocol operations. |
| [`os`](./src/platform/os) | OS services and host integration primitives: lifecycle, permissions, notifications, media, and credentials. |
| [`process`](./src/platform/process) | Process lifecycle, environment, identity, scheduling, limits, signals, and wait operations. |
| [`random`](./src/platform/random) | Secure entropy and deterministic random stream generation. |
| [`resource`](./src/platform/resource) | Runtime resource identifiers and handle lifecycle operations. |
| [`security`](./src/platform/security) | Capability checks, policy state, sandbox controls, and enforcement hooks. |
| [`thread`](./src/platform/thread) | Thread creation, synchronization, local storage, affinity, and priority controls. |
| [`time`](./src/platform/time) | Clock reads, sleep primitives, and timer scheduling operations. |\| [`tls`](./src/platform/tls) | TLS context and session operations for transport security and certificate flows. |
| [`tty`](./src/platform/tty) | Terminal I/O, mode management, pseudo-terminal pairs, and size control. |


## Testing

Run these from the repository root.
Quick check:
```sh
cargo test -p destack_runtime
```

Target coverage:
```sh
just language/test-runtime-* # (see justfile)
```
