# Runtime

The runtime is how Destack actually does anything interesting beyond pure computation.
The Destack runtime wraps VM and/or native execution with scheduling, bindings, host integration, simulation, telemetry, and all the other "runtime stuff".
Essentially, the runtime is where we integrate Node/Bun/Deno-level semantics with V8/JSC-runtime features, though we go much deeper and wider - it's really more like a universal software engine than it is a Node-derived runtime.

## Runtime

Runtime behaviour is modelled along the three basic dimensions of engine ("where?"), execution ("how?") and world ("what?"):

| Dimension | Values                                      | Purpose                                           |
| --------- | ------------------------------------------- | ------------------------------------------------- |
| engine    | `vm`, `native`                              | chooses the execution engine                      |
| execution | `fast`, `deterministic`, `record`, `replay` | chooses determinism and replay behavior           |
| world     | `host`, `simulation`                        | chooses host-backed or simulation-backed bindings |

The runtime is organized around core `runtime`, `platform` bindings, and the underlying `host` integration:
 - `runtime/`: all the core runtime scaffolding and orchestration (world, topology, poller, scheduler/loop, etc.)
 - `platform/`: host implementations for the modules defined in the builtin ["platform"](language/builtin/lib/platform) lib
 - `host/`: host adapters, host event bridges, host ffi entrypoints, and host state integration
 
Compared to other related runtimes like Chromium, Node, or even Unity or Godot, the Destack runtime has a peculiar shape with its explicit model of both "engine", "execution", and "world" semantics.
The point of explicitly modeling execution like this is to enable end-to-end simulation and replay as a sort of software laboratory. 
 
## World

The runtime lives in one `World`, which owns the root clocks, topology, simulation state, policy and rules, trace, observation, and lineage.
Each `World` contains 1-n `Runtime`s, and one `Runtime` contains 1-n `Agent`s.
Each `Agent` has its own execution lane with one `EventLoop`, one `Heap`, one execution `Engine`, one platform resource table, and the rest of its lane-local machinery.
The `Lineage` owns durable history metadata and retained images for revisions.
One `Image` materializes world, runtime, and agent state only for fast restore, rewind, branch or fork, and export.
Managed memory is part of this agent-local runtime state.
The runtime owns the concrete managed backend for an agent, while MIR and Lower define the semantic contract for managed references, barriers, stack maps, and runtime type metadata.
The current VM and native bring-up path uses the `destack_heap` managed heap backend.
Other engines, including WasmGC-backed engines, may realize the same managed semantics with a different collector and object representation.

| Noun | Meaning |
|-----------|--------|
| `World` | The live deterministic root and global coordination boundary. |
| `Runtime` | Process-like container inside one world. |
| `Agent` | Execution lane inside one runtime. |
| `Topology` | Canonical structural graph for world objects, runtimes, agents, resources, entities, and edges. |
| `Policy` | Active control, rule, and fault-injection state over one world. |
| `Branch` | Named mutable lineage head. |
| `Revision` | Named lineage coordinate over one branch and one trace position. |
| `Moment` | Precise execution coordinate over one branch and one trace sequence. |
| `Image` | Immutable in-memory materialized world, runtime, and agent state retained for fast restore, rewind, branch or fork, and export. |
| `Trace` | World-global replay witness. |
| `Observation` | Explicit emitted stream kept separate from replay witness data. |
| `Instant` | Time coordinate only, never a lineage coordinate. |

## History model

The final history model is intentionally small.
`Trace` is the source of truth for replay.
`Image` is one retained acceleration point over the revision DAG.
`Revision` names durable history points in lineage.
`Moment` is the precise restore and query coordinate.
`Observation` is one explicit emitted stream for debugging, metrics, logging, and higher-level analysis.

`Checkpoint` and `Snapshot` are derived conveniences, not foundational runtime nouns.
One checkpoint is only one named retained revision.
One snapshot is only one serialized image export plus lineage metadata.
One checkpoint is just one named or protected retained revision.
One snapshot is just one serialized export of one retained image and associated lineage metadata.

Images are not required for correctness.
One root image plus authoritative trace must be sufficient for deterministic replay across all runtimes, agents, event loops, and scheduler activity.
Retained images exist only to reduce replay latency and memory or CPU cost for rewind, fork, and export.
Live runtime and agent spawn operations should only capture structural spawn images when record mode actually needs suffix replay witnesses.
Fast and deterministic execution should not pay eager spawn-capture cost.

Rewind restores one retained image when the target revision already has one.
Otherwise it restores the nearest retained ancestor image and replays the trace suffix to the target moment.
Fork shares lineage, trace prefixes, and retained images, and then diverges only on later heap writes and trace tail appends.

## Trace model

`Trace` is not the universal runtime event bus.
It is the authoritative replay witness, so it stores only irreducible replay data.
The final role-based taxonomy is:

- `Input`: something entered the world from outside the deterministic substrate
- `Mutation`: one explicit structural world-state change carried as an `Input`
- `Outcome`: something execution observed that was not derivable from prior state and prior trace
- `Anchor`: one retained or user-visible history anchor, such as checkpoints or labels

`Input` includes world ticks, replayable entrypoint runs, runtime removal, topology or policy mutations, and future ingress records like network or IPC delivery when they are not already represented elsewhere.
`Outcome` includes host or binding results, entropy reads, time outcomes when time is host-authoritative, runtime or agent spawn structural images, and any nondeterministic arbitration outcome that remains after scheduler or simulation policy.
`Anchor` includes checkpoint anchors, image boundaries, and explicit user or debugger labels.

Internal deterministic execution is not authoritative trace data.
Scheduler choices only enter trace when they are not derivable from prior state, deterministic policy, and recorded inputs or outcomes.
This same model must work in both full DST mode, where replay can derive almost everything from one root image and one compact witness stream, and mixed host mode, where trace additionally records host boundary outcomes.

## Observation model

`Observation` is not part of replay correctness.
It is the explicit emitted stream for diagnostics, logs, metrics, topology or scheduler notices, and user-defined structured observations.
`Observation` is emitted by both runtime subsystems and userland or library code.
Runtime subsystems should emit observations for scheduler progress, topology mutation, resource lifecycle, and policy diagnostics.
Userland and libraries should emit observations for logging, tracing spans, metrics, assertions, and domain events.
Higher-level `Event` and `EventSet` style APIs should project from observations and projected trace records.
The foundational runtime substrate is moments, trace, images, and observations, not raw event logs.
`Event` is one normalized query-visible fact at one moment.
`Transition` is one normalized query-visible step between two adjacent moments.
`Transition` is derived from authoritative trace steps and never becomes one second storage substrate or one stored diff log.

In the runtime API this means:

- `World::observe(...)` emits one explicit observation at the current moment
- `World::events().between(...)` and `World::events().up_to(...)` project one normalized `EventSet`
- `World::transitions().between(...)` and `World::transitions().up_to(...)` derive one `TransitionSet`

These queries are branch-aware through `Moment`.
One live `World` instance queries its own active branch history and state, including the uncommitted live tail after the current head revision.
One lineage-wide query view queries committed multibranch history directly from shared lineage state.
Forked worlds query their own descendant live tails over the same lineage instead of pretending one live world is one universal multibranch observation log.

Projected events merge:

- trace inputs
- trace outcomes
- trace anchors
- moment-stamped observations

Derived transitions are built from authoritative trace steps between adjacent moments.
They are not a second storage substrate and should stay derivable from trace and world state.

## Query model

The query model is moment-centric and branch-aware.
`Moment` is the primary query coordinate.
`Instant` remains temporal only.
Many moments may share one virtual instant, so time and history stay distinct.

The foundational nouns are:

- `World`
- `Runtime`
- `Agent`
- `Topology`
- `Policy`
- `Branch`
- `Revision`
- `Moment`
- `Trace`
- `Image`
- `Observation`

The derived nouns are:

- `Checkpoint`
- `Snapshot`
- `Event`
- `Transition`

`Event` answers:

- what authoritative record landed here
- what observation was emitted here

`Transition` answers:

- what authoritative step happened between two adjacent moments
- what state moved from one moment to the next

The ownership split is:

- `World`: live branch-local queries, including uncommitted tail state
- `LineageView`: committed multibranch history queries

The baseline lineage query surface should expose:

- `branches`
- `moments_on`
- `moments_up_to`
- `moments_between`
- `events_on`
- `events_up_to`
- `events_between`
- `transitions_on`
- `transitions_up_to`
- `transitions_between`
- `view`
- `divergence_moment`

Committed observations belong to lineage history beside committed trace and retained images.
Live world observation buffers only own the active uncommitted tail and live subscriptions.

One committed observation record carries:

- one exact `Moment`
- one structural `Scope`
- one `ObservationCategory`
- one stable `name`
- optional `tags`
- one structured payload

## Integrated telemetry

The integrated Destack stack should build tracing, logging, metrics, assertions, and domain-event helpers on top of `Observation`.
Those facilities should not write directly to `Trace`.
`Trace` is the minimal replay witness.
`Observation` is the explicit observable stream.
`Event` is the normalized query view over both.

This split keeps replay cheap while making observability first-class across the whole stack.
Every observation should automatically carry exact moment and branch context, and may additionally carry runtime, agent, task, or span context as the higher-level telemetry libraries grow.

## Query examples

The final query surface should feel direct and branch-aware.

```rust
let lineage = world.lineage();
let branch = world.branch();

let failures = lineage
    .events()
    .branch(branch.id)?
    .observations();

let checkpoints = lineage
    .events()
    .branch(branch.id)?
    .anchors();

let topology_inputs = lineage
    .events()
    .branch(branch.id)?
    .inputs()
    .name("topology.define_entity_kind");

let view = lineage.view(lineage.branch_head_moment(branch.id)?)?;
let runtime_count = view.runtime_count();
let policy = view.policy();
```

Branch-local live queries should remain available on `World`.

```rust
let before = world.moment();
world.observe(Observation::world_summary(
    ObservationCategory::Diagnostic,
    "demo.control",
    "demo control observation",
));
let moment = world.moment();

let events = world.events().up_to(moment)?;
let transitions = world.transitions().up_to(moment)?;
```

Observation queries should feel direct and scope-aware.

```rust
let clicks = lineage
    .events()
    .branch(branch.id)?
    .observations()
    .name("ui.click")
    .category(ObservationCategory::Domain)
    .runtime(runtime_id)
    .tagged("screen", "checkout");

let render_summary = Observation::runtime_summary(
    ObservationCategory::Telemetry,
    runtime_id,
    "ui.render.commit",
    "render committed",
)
.tagged("route", "/checkout");
```

Transitions should expose adjacent history steps rather than a stored diff log.

```rust
let transitions = lineage.transitions().branch(branch.id)?;
let first_input = transitions
    .inputs()
    .first()
    .cloned();

let divergence = lineage.divergence(branch.id, other_branch.id)?;
```

## Memory model

The runtime uses one memory model family with multiple placement classes, not one universal storage representation.
This is what lets Destack support ordinary application code, replayable state, and address sensitive systems code without splitting into separate runtimes.

- Managed references are logical object identities.
- Raw pointers are physical or ABI facing pointers.
- Pinning is the bridge from managed lifetime to stable physical address exposure.
- Value, stack, region, and other ephemeral placements are preferred whenever identity is unnecessary.

Managed references are the default for ordinary reference types.
They identify an object in the current heap image, but they do not require that object to live at one permanent host address forever.
The runtime may back a managed reference with a direct pointer, compact handle, page directory entry, or another equivalent representation.

Raw pointers are for explicit address semantics.
They are used for FFI, MMIO, shared memory layouts, intrusive structures, allocator internals, and other code that truly depends on physical address behavior.
Pinned managed objects sit between the two.
They keep managed lifetime and tracing semantics, but additionally guarantee stable physical address exposure while pinned.
Pinning is mainly scoped keepalive plus explicit raw-address exposure for managed storage.
Live pin state should stay outside durable heap images and should act as a capture barrier when external raw exposure is active.
At the heap leaf level, immutable `PageImage` values are the durable CoW units, while live mutation happens through page descriptors owned by page arenas.
That keeps durable image sharing separate from live mark state, pin state, and detach-on-write mechanics.

This distinction is also what keeps the execution modes coherent.
`fast`, `deterministic`, `record`, and `replay` should share the same object model and pointer model.
The modes differ in trace and image policy, not in what a managed reference means.

This same rule must hold across engines.
VM and native are both first-class execution engines, so they must share one managed heap model rather than evolving separate managed runtimes.
They should share:

- `ManagedReference`
- `RawPointer`
- `HeapImage`
- `LayoutId`
- exact retained-byte accounting and hard-limit admission
- CoW, fork, rewind, and snapshot semantics

What they do not need to share bit-for-bit is payload representation.
The heap storage layer should be backend-neutral and layout-driven.
Managed allocations should fundamentally be payload bytes plus `LayoutId`, with scanning, size, alignment, and field interpretation coming from runtime layout metadata.
VM `Value` layouts are one family of layouts over that storage.
Native typed layouts are another family of layouts over the same storage.
This means VM slot access is one convenience view over managed storage, not the ontology of the heap itself.
Native code generation should lower directly to typed field access over the same managed heap and image model instead of routing everything through `Value` payload arrays.

## State authority

Not all runtime state needs the same capture strategy.
The runtime splits state into three broad authority classes.

- Image authoritative state: durable heap and runtime state that should restore directly from an image root.
- Trace authoritative state: causal history that can be reconstructed from trace roots and replay.
- Ephemeral execution state: stack, registers, scratch regions, and other transient state that is rebuilt or quiescently captured when needed.

Fork and rewind operate on roots, not raw process memory copies.
The important roots are the heap root for durable object state and the trace root for causal history.
Cheap branch and rewind come from sharing those roots structurally and only detaching touched leaves on mutation.
Within the heap root, the durable side should stay image-backed while the live side stays descriptor-backed, so page images remain immutable leaves and live page arenas remain the mutable frontier.

## Image policy

`Lineage` owns revisions, retained images, checkpoints, and trace images.
It does not need a second image-cache architecture noun beyond that ownership boundary.
`RevisionImagePolicy` decides only whether to admit, protect, or evict retained images.
Image placement should be adaptive and budget driven from replay distance, weighted replay cost, churn since the nearest retained image, branch heat, and incremental retained image bytes under memory pressure.
`checkpoint` and `hibernate` are simply revisions with mandatory image protection.

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
