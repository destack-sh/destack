# Runtime

The runtime is how Destack actually does anything interesting beyond pure computation.
The Destack runtime integrates VM and or native execution with scheduling, platform and host bindings, simulation, telemetry, and all the other runtime machinery (the cool people call this "effects").

## Runtime

Runtime behaviour is modelled along the three basic dimensions of engine ("where?"), execution ("how?") and world ("what?"):

| Dimension | Values                                      | Purpose                                           |
| --------- | ------------------------------------------- | ------------------------------------------------- |
| engine    | `vm`, `native`                              | chooses the execution engine                      |
| execution | `fast`, `deterministic`, `record`, `replay` | chooses determinism and replay behavior           |
| world     | `host`, `simulation`                        | chooses host-backed or simulation-backed bindings |

The runtime is organized around core `runtime`, `platform` bindings, and the underlying `host` integration:
 - `runtime/`: all the core runtime scaffolding and orchestration (world, topology, poller, scheduler/loop, etc.)
 - `platform/`: host implementations for the modules defined in the language [`platform`](../library/platform) library
 - `host/`: host adapters, host ingress bridges, host FFI entrypoints, and host state integration

## World

The big all-encompassing container that owns the runtime is `World`, which controls ...
<!-- FUGU #Incomplete -->

**Determinism, simulation and replay**:
The big advantage of modeling the runtime against a single World concept is that - with some care, and when staying "inside our lanes" - we get to control all external effects, including timing, scheduling, randomness, and all runtime bindings that affect the runtime.

## Host

The hosts are the operating systems and deployment targets, like Linux, iOS, macOS, Android, and Windows.
Each host has its own capabilities, lifecycle constraints, framework ownership rules, and thread-affinity requirements.

## Platform

The `platform` implements the bindings defined in `language/library/platform`, and the corresponding bindings and ABI surface are auto-generated in `language/runtime/src/generate` (into the not-to-be-edited `*.generated.rs` files).
The runtime generator wires as much of the native / VM data integration as possible, but unfortunately we still need to manually normalize and serialise / deserialise sometimes where no reliable automatic mapping exists.

### Modules

The low-level `platform` bindings are not meant to be used _directly_ by general userland - though they are accessible to advanced users - but instead through the higher-level `destack:*` library.
And because Destack tries to follow web standards closely, all the low level binding modules are also organized around the same concepts, even though they go much deeper (and wider), and differ in some ways for better performance and ergonomics.

| Module | Description |
|-----------|--------|
| [`audio`](./src/platform/audio) | Audio clocks, devices, streams, events, and MIDI I/O. |
| [`crypto`](./src/platform/crypto) | Cryptographic algorithms, keys, stores, certificates, and randomness. |
| [`debug`](./src/platform/debug) | Low-level debugger transport, profiling, trace sinks, and debug control hooks. |
| [`device`](./src/platform/device) | Host peripheral buses and device classes: serial, USB, Bluetooth, and camera. |
| [`display`](./src/platform/display) | Monitor discovery, display topology, and native window integration. |
| [`error`](./src/platform/error) | Runtime error bridge and structured host error conversion helpers. |
| [`ffi`](./src/platform/ffi) | Dynamic library loading, symbol lookup, pointer primitives, and foreign calls. |
| [`fs`](./src/platform/fs) | Filesystem paths, files, directories, metadata, watches, mapping, and extended attributes. |
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
| [`runtime`](../library/platform/runtime) | Low-level world control, lineage, pinned views, causal trace, observation streams, and snapshot export or restore. |
| [`security`](./src/platform/security) | Capability checks, policy state, sandbox controls, and enforcement hooks. |
| [`thread`](./src/platform/thread) | Thread creation, synchronization, local storage, affinity, and priority controls. |
| [`time`](./src/platform/time) | Clock reads, sleep primitives, and timer scheduling operations. |
| [`tls`](./src/platform/tls) | TLS context and session operations for transport security and certificate flows. |
| [`tty`](./src/platform/tty) | Terminal I/O, mode management, pseudo-terminal pairs, and size control. |


## Testing

Run these from the repository root.

```sh
# focused local loop
cargo test -p destack_runtime
just language/check-runtime-macos # or linux/windows on matching hosts

# clean gate
just language/quick

# exhaustive gate
just language/full

# toolchain and target coverage
just language/doctor-toolchain
just language/lint-toolchain
just language/check-runtime-macos # and/or linux/windows-msvc on matching hosts
just language/check-runtime-ios
just language/check-runtime-android
```
