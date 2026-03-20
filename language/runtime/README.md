# Runtime

The runtime is how Destack actually does anything interesting beyond pure computation.
The Destack runtime integrates VM and/or native execution with scheduling, platform / host bindings, simulation, telemetry, and all the other "runtime stuff".
Essentially, we integrate Node/Bun/Deno-level features and a V8/JSC-level execution, though we go much deeper and wider than the Node-family of runtimes.
As with "TypeScript++", the idea here is really "Web++/Node++", i.e., take the well known shape and good modern standards, and expand on them / refine as needed into a a more universal software engine.

## Runtime

Runtime behaviour is modelled along the three basic dimensions of engine ("where?"), execution ("how?") and world ("what?"):

| Dimension | Values                                      | Purpose                                           |
| --------- | ------------------------------------------- | ------------------------------------------------- |
| engine    | `vm`, `native`                              | chooses the execution engine                      |
| execution | `fast`, `deterministic`, `record`, `replay` | chooses determinism and replay behavior           |
| world     | `host`, `simulation`                        | chooses host-backed or simulation-backed bindings |

The runtime is organized around core `runtime`, `platform` bindings, and the underlying `host` integration:
 - `runtime/`: all the core runtime scaffolding and orchestration (world, topology, poller, scheduler/loop, etc.)
 - `platform/`: host implementations for the modules defined in the builtin ["platform"](language/builtin/library/platform) library
 - `host/`: host adapters, host event bridges, host ffi entrypoints, and host state integration

## World

The big all-encompassing container that owns the runtime is `World`, which controls ...
<!-- FUGU #Incomplete -->

**Determinism, simulation and replay**:
The big advantage of modeling the runtime against a single World concept is that - with some care, and when staying "inside our lanes" - we get to control all external effects, including timing, scheduling, randomness, and all runtime bindings that affect the runtime.

## Host

The `Host`s are the operating systems and deployment targets, like Linux, iOS, macOS, Android, Windows, and so on; they're basically the foundation of our platform, the last one/two words of the triplet.

Each host has its own capabilitiesaround when and how you get what state, which threads require what affinity in what order, and a bunch more fun stuff that the .
The basic bridge is the `HostBackend` that is implemented by each host to provide the platform-specific functionality needed by the runtime.

## Platform

The `platform` implements the "platform" bindings defined in `language/builtin/library/platform`, and the corresponding bindings and ABI surface are auto-generated in `language/runtime/src/generate` (into the not-to-be-edited `*.generated.rs` files).
The runtime generator wires as much of the native / VM data integration as possible, but unfortunately we still need to manually normalize and serialise / deserialise sometimes where no reliable automatic mapping exists.

### "ABI"

One of the more annoying parts of the runtime is that we need to support both native and VM execution even though they have pretty different underlying state and execution models.
They both share the `Heap` representation, but in-memory layouts and calling conventions are obviously different.

For each module (e.g. `fs` in `platform/fs/`), we generate and partially hand-wire:
 1) `vm.rs` and `native.rs` wrappers to forward into core implementation (usually native-shaped)
 2) `abi.generated.rs` codec for translating between different representations
 3) `bindings.generated.rs` wrappers for both native / VM gating either "host" and "simulation" with automatic replay/record

And more specifically, for every `T` in the source bindings (e.g. `OsPath` in `platform/fs/path.ds`), we generate in `abi.generated.rs` some:
 1) `TAbi`: borrowed ABI-generic view (over either `NativeAbi` or `VmAbi`)
 2) `T`: native view of `T`
 3) `TValue`: owned ("value") view of `T`
 4) `TVm`: VM view of `T`

For example:
```rust
pub enum OsPathAbi<A: BindingAbi> {
    OsPathBytes(platform_fs::OsPathBytesAbi<A>),
    OsPathUtf16(platform_fs::OsPathUtf16Abi<A>),
}

pub type OsPath = OsPathAbi<NativeAbi>;
pub type OsPathVm = OsPathAbi<VmAbi>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OsPathValue {
    OsPathBytes(OsPathBytesValue),
    OsPathUtf16(OsPathUtf16Value),
}
```

### Modules

The low-level `platform` bindings are not meant to be used _directly_ by general userland - though they are accessible to advanced users - but instead through the higher-level `destack:*` library, which is essentially a `node:*` shaped higher level API with all the same functionality.
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
| [`runtime`](../builtin/library/platform/runtime) | Low-level world control, lineage, pinned views, causal trace, observation streams, and snapshot export or restore. |
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
