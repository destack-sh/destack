# Runtime

The runtime is how Destack actually does anything interesting beyond pure computation.
The Destack runtime wraps VM and/or native execution with scheduling, bindings, host integration, trace, telemetry, and all the other "runtime stuff".
Essentially, the runtime is where we integrate Node/Bun/Deno-level semantics with V8/JSC-runtime features, though we go much deeper and wider - it's really more like a universal software engine than it is another Node-derived runtime.

## Runtime

Runtime behavior is modeled along the three basic dimensions of engine ("where?"), execution ("how?") and world ("what?"):

| Dimension | Values | Purpose |
|-----------|--------|---------|
| engine | `vm`, `native` | chooses the execution engine |
| execution | `fast`, `deterministic`, `record`, `replay` | chooses determinism and replay behavior |
| world | `host`, `simulation` | chooses host-backed or simulation-backed bindings |

The runtime is organized around core `runtime`, `platform` bindings, and the underlying `host` integration:
 - `runtime/`: all the core runtime scaffolding and orchestration (world, topology, poller, scheduler/loop, etc.)
 - `platform/`: host implementations for the modules defined in the builtin ["platform"](language/builtin/lib/platform) lib
 - `host/`: host adapters, host event bridges, host ffi entrypoints, and host state integration
 
Compared to other related runtimes like Chromium, Node, or even Unity or Godot, the Destack runtime has a peculiar shape with its explicit model of both "engine", "execution", and "world" semantics.
The point of explicitly modeling execution like this is to enable end-to-end simulation and replay as a sort of software laboratory. 
 
## World

The runtime lives in a main `World`, which owns the root clocks, topology, simulation state, policy / rules, trace / replay, and lineage ("history").
 - Each `World` contains 1-n `Runtime`s, and one `Runtime` contains 1-n `Agent`s.
 - Each `Agent` has its own execution lane with one `EventLoop`, one `Heap`, one execution `Engine`, one platform resource table, etc..
 - The `Lineage` owns the authoritative `Trace` images for each `Revision`.
 - One `Image` only materializes world, runtime, and agent state for fast restore.
Managed memory is part of this agent-local runtime state.
The runtime owns the concrete managed backend for an agent, while MIR and Lower define the semantic contract for managed references, barriers, stack maps, and runtime type metadata.
The current VM and native bring-up path uses the `destack_heap` managed heap backend.
Other engines, including WasmGC-backed engines, may realize the same managed semantics with a different collector and object representation.

| Noun | Meaning |
|-----------|--------|
| `World` | The live deterministic root and global coordination boundary. |
| `Runtime` | Process-like container inside one world. |
| `Agent` | Execution lane inside one runtime. |
| `Branch` | Named mutable lineage head. |
| `Revision` | World-global lineage coordinate over one branch and one trace position. |
| `Checkpoint` | Durable named or indexed anchor to one revision. |
| `Image` | Immutable in-memory materialized world, runtime, and agent state at one revision. |
| `Snapshot` | Serialized export of one image plus lineage metadata. |
| `Trace` | World-global causal history. |
| `Instant` | Time coordinate only, never a branch or checkpoint concept. |

## Host

The `Host`s are the operating systems and deployment targets, like Linux, iOS, macOS, Android, Windows, and so on; they're basically the foundation of our platform, the last one/two words of the triplet.

Each host has its own capabilities (and idiosyncrasies) around when and how you get what state, which threads require what affinity, and a bunch more fun stuff.
The basic bridge is the `HostBackend` that is implemented by each host to provide the platform-specific functionality needed by the runtime.

## Platform

The `platform` bindings implement the "platform" builtin library bindings defined in `language/builtin/lib/platform`, the corresponding bindings and ABI stuff is automatically generated in `language/runtime/src/generate` (see all the `*.generated.rs` files).
We have successively expanded the runtime generator to automatically wire as much of the native / VM data integration as possible, though unfortunately in some places we still need to manually normalize and serialise / deserialise because no reliable automatic mapping exists (or we couldn't find one). 

The low-level `platform` bindings are not meant to be used by general userland - though they are accessible to advanced users - but instead through the higher-level `destack:*` library, which is essentially a `node:*` shaped higher level API with all the same functionality.
And because Destack tries to follow web standards closely, all the low level binding modules are also organized around the same concepts, even though they go much deeper (and wider).

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
| [`runtime`](../builtin/lib/platform/runtime) | Low-level world control, lineage, pinned views, causal trace, observation streams, and snapshot export or restore. |
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
