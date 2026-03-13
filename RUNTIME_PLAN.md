Runtime refactor plan.

This plan tracks the high-level runtime shape across world ownership, deterministic scheduling, replay, checkpoints, branching, rewind, topology, and simulation.
The goal is one coherent world-owned runtime model for serious deterministic systems testing and exploration.

## Performance goals
- The default fast path should stay as close to zero-overhead as possible when record, fork, rewind, and snapshot features are unused.
- Deterministic record, fork, rewind, and branching should add the minimum possible allocation, memory-retention, and cache overhead to the hot path.
- Shared-prefix trace backing and heap CoW should optimize for high branch fanout, low incremental fork cost, and low memory duplication.
- Extra metadata, cursors, and exported artifacts should stay derivative and should never force unrelated work into the fast path.

## Locked decisions
- `World` is the public runtime root.
- `Runtime` is a subordinate process-like container inside one world.
- `Agent` is a subordinate execution lane inside one runtime.
- `Topology` is the canonical metadata graph for world objects.
- `Simulation` is one world-owned environment backend, not a second runtime root.
- `Trace` is the durable causal history, and replay is one execution mode over trace.
- One `World` is always on exactly one active branch.
- `fork` is a verb that creates a new world from one checkpoint.
- `World`, `Runtime`, `Agent`, `Topology`, `Policy`, `Trace`, `Revision`, `Branch`, `Moment`, `Image`, and `Observation` are the foundational runtime nouns.
- `Checkpoint` and `Snapshot` are derived conveniences, not foundational nouns.
- `Checkpoint` is only one named retained revision.
- `Snapshot` is only one serialized image export plus lineage metadata.
- `World` is the live reconstructible object, not the file format.
- Checkpoints and rewind should move to one real pause and capture protocol, not one ad hoc readiness flag.
- `Capture` is the image boundary trait, and world quiescence is the temporal coordination protocol around it.
- `Capture` should stay one-phase: no `prepare_capture` hook in the main trait.
- `SnapshotCodec` is the serialized boundary beside `Capture`, not part of the quiescence protocol.
- VM must provide durable snapshots for full checkpoint, rewind, and fork support.
- Native does not get a fake checkpoint story: it must provide a durable snapshot representation or remain unsupported for that feature set.
- `dsconfig.runtime` only sets static world defaults and static routing rules.
- Userland test libraries own campaigns, workloads, search, and checkers.
- `Revision` is one lineage coordinate: branch, trace sequence, and clock instants, plus one optional retained image reference.
- `Checkpoint` is one named or indexed anchor to one retained revision.
- `Image` is immutable in-memory materialized state.
- `Snapshot` is one serialized or exported representation of one image, optionally bundled with trace metadata.
- `Lineage` is shared branch, checkpoint, and snapshot backing, not active branch ownership.
- `Moment` is the primary history coordinate for replay, restore, and query.
- `Observation` is the explicit emitted runtime and user event stream kept separate from authoritative replay data.
- `AgentSnapshot` owns agent-local execution state, including VM engine state.
- `VmSnapshot` lives in `language/vm`, not in runtime.
- VM snapshots should be immutable and shareable across forked worlds.
- Heap snapshotting should be the main copy-on-write target.
- Snapshot ownership follows causal ownership: world-shared state in `WorldSnapshot`, runtime coordination state in `RuntimeSnapshot`, and execution lane state in `AgentSnapshot`.

## Core model
- `Instant` is always temporal and never a lineage coordinate.
- `Moment` is one execution coordinate over one branch and one trace sequence.
- `Trace` is one world-global replay witness, not a universal event bus.
- `Revision` is one named lineage coordinate over one branch and one trace position.
- `Branch` is one named mutable head that points at one revision.
- `Image` is one immutable materialized world state retained for fast restore, rewind, branch or fork, and export.
- `Snapshot` is one serialized artifact derived from one image, plus optional trace or metadata payload.
- `Checkpoint` is one durable named anchor to one retained revision and usually forces image materialization.
- `Replay` is one consumer and execution mode over trace, not the storage model itself.
- `Observation` is one explicit queryable stream of emitted runtime or user events.

## Final history model
- `Trace` is the source of truth for replay.
- `Image` is one retained acceleration point over the revision DAG.
- `Revision` names durable history points in one lineage.
- `Moment` is the precise restore and query coordinate.
- `Observation` is one explicit emitted event stream and is not required for replay correctness.
- `Snapshot` externalizes one image and associated lineage metadata.
- Rewind is:
  - restore one retained image when the target revision already has one
  - otherwise restore the nearest retained ancestor image and replay the trace suffix to the target moment
- Fork is:
  - share lineage, trace prefixes, and retained images
  - detach only on later writes to touched heap leaves or new trace tail records
- Images are optional for correctness:
  - one root image plus authoritative trace must be sufficient for deterministic replay
  - retained images exist only to reduce replay latency and memory or CPU cost
- Live world operations should only capture structural spawn images when record mode actually needs them for later suffix replay.
- Fast and deterministic execution should not pay eager runtime or agent spawn capture cost.

## Final trace model
- `Trace` stores only irreducible replay witness data.
- The authoritative trace taxonomy is:
  - `Input`: something entered the world from outside the deterministic substrate
  - `Mutation`: one explicit structural world-state change carried as an `Input`
  - `Outcome`: something execution observed that was not derivable from prior state and prior trace
  - `Anchor`: one retained or user-visible history anchor, such as checkpoints or labels
- `Input` includes:
  - ticks
  - replayable entrypoint runs
  - runtime removal
  - topology or policy mutations
  - future ingress records like network or IPC delivery when they are not already represented elsewhere
- `Outcome` includes:
  - host or binding results
  - entropy reads
  - time outcomes when time is host-authoritative
  - runtime or agent spawn structural images
  - any nondeterministic arbitration outcome that remains after scheduler or simulation policy
- `Anchor` includes:
  - checkpoint anchors
  - image boundaries
  - explicit user or debugger labels
- Internal deterministic execution is not authoritative trace data.
- Scheduler choices only enter trace when they are not derivable from prior state, deterministic policy, and recorded inputs or outcomes.
- `Trace` must support both:
  - fully deterministic DST mode, where replay can derive almost everything from one root image and one compact witness stream
  - mixed host mode, where trace additionally records host boundary outcomes

## Final observation model
- `Observation` is not part of replay correctness.
- `Observation` is the explicit emitted stream for:
  - diagnostics
  - logs
  - metrics
  - topology or scheduler notices
  - user-defined structured observations
- User-facing `Event` and `EventSet` style APIs should project from:
  - observations
  - projected trace records
  - derived state facts
- The foundational runtime substrate is state and moments, not raw event logs.
- `Event` is one normalized query-visible fact at one `Moment`.
- `Transition` is one normalized query-visible step between two adjacent `Moment`s.
- The core world-query surface should expose:
  - `World::observe(...)`
  - `World::events().between(...)`
  - `World::events().up_to(...)`
  - `World::transitions().between(...)`
  - `World::transitions().up_to(...)`
- `ObservationRecord` should always carry the exact `Moment` where it was emitted so event queries can merge observations with projected trace records without guessing.
- `Event` projects from:
  - authoritative trace inputs
  - authoritative trace outcomes
  - authoritative trace anchors
  - explicit observations
- `Transition` projects from authoritative trace steps and should answer:
  - what happened between `before` and `after`
  - what authoritative record caused the step
- `Transition` should not become one stored diff log or one second authority source.
- `Moment` is the primary query coordinate:
  - `Instant` remains temporal only
  - many moments may share one instant
- `Observation` should be emitted by both:
  - runtime subsystems like scheduler, topology, policy, and resource lifecycle
  - userland and library facilities like logging, tracing spans, metrics, assertions, and domain events
- The full integrated stack should lower high-level telemetry libraries onto `Observation`, not `Trace`.
- `Trace` remains the minimal replay witness.
- `Observation` remains the explicit observable stream.
- `Event` remains the unified query fact type.
- `Transition` remains the unified query step type.
- Observation query ergonomics should center on:
  - `name(...)`
  - `category(...)`
  - `tagged(...)`
  - `on(Scope::...)`
  - ergonomic facades like `runtime(...)`, `agent(...)`, `entity(...)`, `edge(...)`, and `resource(...)`
- One live `World` queries:
  - its committed active-branch history through lineage
  - its uncommitted active-branch tail through live world state
- One lineage-wide query view must query committed multibranch history directly from `Lineage`.
- Forked worlds query their own descendant live tails over shared lineage rather than pretending one live world owns one universal multibranch mutable log.

## Final query model
- The query substrate is moment-centric, not event-centric.
- The foundational runtime nouns are:
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
- The derived nouns are:
  - `Checkpoint`
  - `Snapshot`
  - `Event`
  - `Transition`
- `State` and `Diff` may exist as later query results, but they are not foundational runtime nouns.
- The query ownership split is:
  - `World`: live branch-local execution plus uncommitted tail queries
  - `LineageView`: committed multibranch history queries
- `LineageView` should expose at least:
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
- Branch-local live queries on `World` should remain:
  - `events_up_to`
  - `events_between`
  - `transitions_up_to`
  - `transitions_between`
- Committed observations must be retained in lineage alongside committed trace and retained images.
- Live world observation buffers should only own the active uncommitted tail plus live subscription state.
- Restoring or forking a world must not pretend world-local observation buffers are lineage history.

## Final integrated telemetry model
- `Trace` is the authoritative replay witness for correctness.
- `Observation` is the explicit emitted stream for observability.
- The integrated stack should build:
  - logging
  - tracing spans
  - metrics
  - assertions
  - domain events
  on top of `Observation`.
- Runtime subsystems should also emit observations for:
  - scheduler progress
  - topology mutations
  - resource lifecycle
  - policy diagnostics
- Query APIs should merge projected trace records and observations into one `Event` view.
- Replay must never depend on logging, telemetry, or observation emission.

## Final query examples
The final query API should feel direct and branch-aware.

```rust
let lineage = world.lineage();
let branch = world.branch();
let now = world.moment();

let failures = lineage
    .events()
    .branch(branch.id)?
    .observations();

let topology_inputs = lineage
    .events()
    .branch(branch.id)?
    .inputs()
    .name("topology.define_entity_kind");

let view = lineage.view(lineage.branch_head_moment(branch.id)?)?;
let runtime_count = view.runtime_count();
let policy = view.policy();

let leadup = lineage
    .transitions()
    .between(lineage.branch_origin_moment(branch.id)?, now)?;

let leadup = leadup.filter(|transition| matches!(
    transition.kind,
    TransitionKind::Input | TransitionKind::Outcome
));

let divergence = lineage.divergence(branch.id, other_branch.id)?;
```

The live branch-local fast path should remain available on `World`.

```rust
let before = world.moment();
world.observe(Observation::world_summary(
    ObservationCategory::Diagnostic,
    "demo",
    "demo observation",
));
let moment = world.moment();

let events = world.events().up_to(moment)?;
let transitions = world.transitions().up_to(moment)?;
```

The final integrated telemetry surface should feel like structured observation emission instead of ad hoc text logging.

```rust
world.observe(
    Observation::summary(
        ObservationCategory::Domain,
        Scope::runtime(runtime_id),
        "http.request",
        "request received",
    )
    .tagged("route", "/checkout"),
);
```

And observation queries should read directly off the event surface.

```rust
let requests = lineage
    .events()
    .branch(branch.id)?
    .observations()
    .name("http.request")
    .category(ObservationCategory::Domain)
    .runtime(runtime_id)
    .tagged("route", "/checkout");
```

## Final image policy model
- `Lineage` owns revisions, retained images, checkpoints, and trace images.
- We do not need a separate image-cache architecture noun beyond lineage ownership.
- `RevisionImagePolicy` decides only:
  - admit: retain one image for a revision
  - protect: keep one retained image regardless of ordinary eviction pressure
  - evict: drop one retained image while keeping revision and trace history
- Image placement should be adaptive and budget driven:
  - replay distance or weighted replay cost
  - heap and scheduler churn since the nearest retained image
  - branch heat, including repeated fork, rewind, and debugger demand
  - incremental retained image bytes under current memory pressure
- `checkpoint` and `hibernate` are just revisions with mandatory image protection.

## Replacement map
- Old `Replay*` naming should be retired as the conceptual center:
  - `ReplayLog` -> `Trace`
  - `ReplayEvent` -> `TraceRecord`
  - `ReplayLogReader` -> `TraceCursor`
  - `ReplaySnapshot` -> one trace-state or cursor-state type, not one world-state concept
  - replay becomes one execution mode and one trace consumer
- Old flat trace-event thinking should be retired:
  - `TraceEvent` should go away as the conceptual center
  - the final role-based trace classes are `Input`, `Outcome`, and `Anchor`
  - top-level `Tick` records should go away as peers to commands and outcomes
  - explicit labels belong to anchors or observations, not one mixed catch-all event enum
- Old mutable replay-log snapshotting should be replaced by immutable shared-prefix trace storage:
  - trace storage should use immutable `TraceSegment`s
  - branches should share trace prefixes
  - branch-local divergence should happen in one tail segment or tail chain
- Old checkpoint readiness gating should be replaced:
  - `CheckpointState::{Ready, Blocked}` should go away
  - `require_checkpoint_ready` should go away
  - `require_image_ready` should go away
  - `with_checkpoint_blocked` should go away
  - replace them with one real world quiesce protocol and `Capture` implementations
- Old checkpoint semantics should be narrowed and clarified:
  - checkpoint is not a second state model
  - checkpoint is one named anchor to one revision
  - a checkpoint usually points at one image
- Old overloaded snapshot thinking should be removed:
  - `Image` is immutable in-memory state
  - `Snapshot` is one serialized artifact
  - runtime code, VM code, and heap code should all use that distinction consistently
- Old event-centric debugging thinking should be removed:
  - `Trace` is not the universal query bus
  - `Observation` is the explicit emitted event stream
  - `Event` and `Transition` should be higher-level query projections over moments, state, observations, and trace
- Old ad hoc fork and rewind wiring should be replaced by lineage-centered operations:
  - `Branch` owns the mutable head
  - `Revision` is the lineage coordinate
  - `Image` is the retained restore, rewind, branch or fork, and export acceleration point
  - `Snapshot` is export and import only
- Old blurred topology and state boundaries should be kept separate:
  - topology is identity, labels, kinds, and edges
  - state is payload and execution machinery
  - trace is causal history
- Old resource snapshot assumptions should be removed:
  - portability is not binding metadata
  - capture and portability belong to resource kinds, providers, and instances
  - bindings only keep static call semantics

## Migration targets
- `language/runtime/src/runtime/trace/*`
  - rename and reshape around `Trace*`
  - preserve existing event semantics where good: entropy and binding-call events
  - add explicit control and marker events
- `language/runtime/src/runtime/world/history.rs`
  - delete readiness-flag logic
  - introduce pause or capture coordination
  - move checkpoint metadata to revision and image terminology
- `language/runtime/src/runtime/world/*`
  - recenter world fork and rewind on branch, revision, checkpoint, image, and trace
  - stop treating replay state as the authoritative lineage model
- `language/runtime/src/runtime/process/*`
  - reflect the same image vs snapshot distinction in runtime and agent capture paths
  - move runtime, agent, and subsystem capture onto the shared `Capture` trait
- `language/vm/*` and `language/heap/*`
  - reflect the same image vs snapshot distinction in VM and heap naming
  - keep immutable in-memory images separate from serialized exports
  - implement the shared `Capture` and `SnapshotCodec` traits there too

## Outstanding DST cleanup
- The current trace-prefix representation still needs one real cleanup pass:
  - `TraceLogImage` currently uses `Arc<Vec<Arc<TraceSegment>>>` for shared prefix storage
  - that works, but it is not obviously the final pristine or highest-performance shape
  - revisit whether shared prefix and tail should be represented by one more explicit persistent segment chain or another lighter-weight immutable prefix structure
  - keep this tied to the broader DST follow-up on trace authority, prefix sharing, and branch fanout performance
- The current world ownership shape still needs one follow-up audit:
  - `World` still uses `Arc<World>` despite being intentionally owner-thread affine
  - that is acceptable for now, but it is still one architectural smell
  - revisit whether we keep `Arc` as one lifetime tool or move to a more explicit owner-thread shared-ownership model later
- Heap CoW is broadly good, but still needs one dedicated performance audit after trace-prefix cleanup:
  - page image retention
  - detach-on-write overhead
  - page metadata size
  - potential page grouping or image sharing improvements

## Heap evolution
- Keep page images as the coarse immutable CoW unit unless profiling proves a better sharing unit.
- Stable references are a hard constraint:
  - no moving GC
  - no compacting GC
  - managed object identity stays stable across mark, fork, rewind, and snapshot
  - managed storage slots stay stable for the lifetime of a live allocation
- The managed heap semantic model is:
  - managed references are logical object identities
  - raw pointers are physical or ABI-facing pointers
  - page layout and allocator policy are internal implementation details
- Native and VM must share the same managed heap semantics:
  - one `ManagedReference` model
  - one `HeapImage` model
  - one `LayoutId` space
  - one CoW, fork, rewind, and snapshot story
  - no separate native-only managed heap and no separate VM-only managed heap
- The heap must be backend-neutral at the storage layer:
  - managed payload storage is bytes plus `LayoutId`
  - scanning, size, alignment, and field interpretation come from runtime layout metadata
  - VM `Value` layouts are one family of layouts, not the universal heap payload model
  - native typed layouts are another family of layouts over the same managed heap
- The shared heap substrate should own:
  - managed and raw identity
  - page and span images
  - exact retained-byte accounting and hard-limit admission
  - layout-driven scan metadata
  - fork, rewind, and snapshot semantics
- VM-specific slot access should be one convenience view over layout-backed managed storage, not the core ontology of the heap.
- Native code generation should lower directly to typed field access over the same layout-backed managed storage, not through `Value`-centric payloads.
- The final page-class model should be:
  - `ManagedPage`: stable managed identity-anchor allocations
  - `ValuePage`: page-backed managed payload storage used by VM-oriented value layouts
  - `RawValuePage` or run: page-backed raw value storage
  - `RawBytePage` or run: page-backed raw byte storage
  - `PageImage`: immutable shared CoW backing for any page class
- The final live page model should be:
  - `PageImage`: immutable durable leaf state
  - `Page`: one live mutable page with copy on write storage
  - live mark bits and other page-local mutable metadata stay outside durable images
- The final managed allocation model should be:
  - small inline payload bytes when it fits
  - external `ManagedSpan` when it does not
  - no arbitrary host allocation embedded inside managed payload storage
  - no universal `Value` array payload assumption
- The final raw storage model should be:
  - no `Vec<u8>` or other host-owned side allocation embedded inside raw allocations
  - raw bytes and large raw payloads live in page-backed or run-backed storage under the same image model
- GC metadata should stay out of user payload headers:
  - boxed structs remain headerless payloads
  - GC state lives in page-local side metadata
  - dispatch metadata like vtable pointers may still live in class payloads when required for dispatch
- Durable page-image state should include only semantic allocation state:
  - payload contents
  - occupancy
  - durable `LayoutId` information
- Transient page-ref state should stay outside shared page images:
  - mark bits
  - mark queues
  - dirty or card state
  - live pin state
- Pinning is:
  - scoped keepalive plus raw-exposure permission for managed storage
  - live-only state, not durable image state
  - a capture barrier for fork, rewind, and snapshot when active external raw exposure exists
- The managed and raw heaps should share the same broad CoW law:
  - immutable shared page images
  - tiny mutable live frontier
  - first write detaches only the touched page or run
- The next page-directory evolution should make this explicit:
  - heap roots own class-aware page directories
  - page directories point at immutable page images
  - fork and rewind swap roots instead of copying heap vectors
- The current agreed noun set is:
  - `ManagedAllocation`
  - `ManagedPage`
  - `ValuePage`
  - `ManagedSpan`
  - `RawAllocation`
  - `RawPage`
  - `RawSpan`
  - `PageSlot`
  - `LayoutId`
  - `Page`
- The current agreed module split should be:
  - `managed/` at crate root for managed allocation, spans, and GC
  - `raw/` at crate root for raw allocation and span logic
  - `page/` at crate root for page classes, images, addresses, and storage mechanics
  - `value/` at crate root for values, references, pointers, and value cells
  - `heap/` for allocator, limits, image roots, and orchestration logic
- The concrete implementation order is:
  - remove stale pointer arithmetic assumptions from pointer helpers and callers
  - make raw byte storage page-backed or run-backed instead of `Vec<u8>`-backed
  - add explicit page-local side metadata for `LayoutId` and pin state
  - remove remaining VM/runtime storage-shape leakage from managed-allocation access
  - move heap images from flat page vectors toward class-aware page-directory roots
  - move managed payload storage from VM-value-oriented layouts toward byte and layout-driven storage shared by VM and native
- The major CoW and rewind rules are:
  - page images are the only sharing unit that matters semantically
  - transient GC or pin state must not cause image detaches
  - restore and rewind should normally be root swaps plus targeted live-state rebuild
  - external raw-address exposure must be quiesced before durable capture

## Event and scheduling model
- `World` owns trace, clocks, topology, simulation, lineage, and world-scoped policy.
- `Runtime` owns runtime coordination, host integration, and one set of agents.
- `Agent` owns event loop, heap, engine, resources, hooks, diagnostics, and other lane-local execution state.
- Event loops are per-agent local runnable state and are captured inside images, not modeled as peers to trace.
- Trace is world-global because forks, rewind, simulation, topology, and cross-agent ordering are world-global.
- Instants come from world clocks.
- Moments come from lineage plus trace sequence.
- Timer deadlines inside agent event loops reference instants, not moments.
- Scheduler choices only enter trace when they are not derivable from prior state, recorded inputs, and deterministic policy.
- The authoritative trace witness should stay small:
  - inputs
  - outcomes
  - anchors
- Scheduler and host observations that do not affect replay correctness belong in `Observation`, not `Trace`.

## Resource capture model
- Binding declarations already own static call semantics like effect, replay policy, payload policy, simulation support, scope, blocking, affinity, and capability requirements.
- Resource capture policy should live on resource kinds or runtime providers, not as one generic binding field.
- Concrete resource instances should provide capture payloads at snapshot time.
- Resource capture should stay orthogonal:
  - `ResourceBacking`: virtual or host
  - `ResourceCapture`: none, state, or recipe
  - `ResourcePortability`: local, portable, or external
- VM and heap should follow the same layering:
  - image for immutable in-memory state
  - snapshot for serialized export

## Userland control model
- Userland should be able to create and control worlds, branches, checkpoints, runtimes, agents, trace views, and snapshots through one stable control API.
- Raw runtime, VM, heap, event-loop, and host-handle internals should remain internal even when the control surface is powerful.
- `platform.*` remains the low-level binding layer and is not the main user-facing orchestration API.
- `destack.*` should be the higher-level user-facing control layer.
- The control surface should be capability-gated and policy-aware:
  - `runtime.world.create`
  - `runtime.world.control`
  - `runtime.runtime.create`
  - `runtime.agent.create`
  - `runtime.trace.read`
  - `runtime.trace.mark`
  - `runtime.snapshot.create`
  - `runtime.snapshot.restore`
  - `runtime.simulation.control`
- The preferred layering is:
  - kernel: runtime, VM, heap, scheduler internals
  - control: stable world and runtime control handles and operations
  - library: search, campaigns, workload runners, invariant checkers, debugger tooling

## Existing surface alignment
- `platform.debug.core.mark` should become one low-level implementation hook for higher-level history labels, not the primary user-facing trace API.
- `platform.debug.trace.*` should remain one low-level stream and sink control surface for exporting or mirroring runtime trace data.
- `platform.debug.inspector.*` should remain one low-level debugger transport surface.
- `destack.debug` should become the higher-level debugger and trace API over world, branch, revision, and trace concepts.
- `destack.snapshot` should become the higher-level image, checkpoint, export, and restore API.
- `destack.test` should become the higher-level search, campaign, fault, and invariant library surface built on world control.
- New world, runtime, and agent orchestration should not be introduced under `platform.debug`; it belongs in higher-level `destack` control namespaces.

## Low-level platform control shape
- We still need one low-level runtime control and introspection surface under `platform`.
- That surface should live under `platform.runtime`, not under `platform.debug`.
- The bulk of the implementation should not live under `platform.runtime`.
- `platform.runtime` should stay one thin ABI and binding layer that delegates into runtime-owned control infrastructure.
- `platform.debug` should stay focused on debugger transport, profiler sessions, trace sinks, and low-level marker emission.
- `platform.resource` should remain the low-level generic resource identity layer used by runtime-managed handles.
- `platform.runtime` should expose low-level control over:
  - world creation and destruction
  - runtime creation and destruction
  - agent creation and destruction
  - branch, revision, checkpoint, image, and snapshot handles
  - world advancement, pause, rewind, fork, and restore
  - trace access and history label insertion
  - topology and simulation inspection
- The low-level control surface should prefer typed ids and typed metadata over unstructured JSON strings.
- The low-level control surface should expose stable handles and selectors, not raw internal structs.

## Runtime control table
- Low-level runtime handles should resolve through one runtime-owned control table.
- The control table is not part of world topology and is not replayed, snapshotted, or exported.
- The control table should own opaque external handles for:
  - worlds
  - runtimes
  - agents
  - pinned world views
  - trace cursors
  - observation subscriptions
- The control table should be implemented in `runtime/control/*`, with `platform/runtime/*` staying one thin binding and ABI layer.
- Runtime-owned binding domains should use one flat layout:
  - generated ABI in `platform/<domain>/abi.generated.rs`
  - generated wrappers in `platform/<domain>/{native,vm}.rs`
  - handwritten shared helpers in `platform/<domain>/core.rs` when they are actually needed
  - no nested `platform/<domain>/runtime/`
  - no nested delegate-side `platform/<domain>/tests/`
- Runtime-owned binding tests should live at crate level under `language/runtime/src/tests/platform/<domain>/`.
- The control table should split stable metadata from live objects:
  - process-global metadata for opaque handle ids and handle kinds
  - owner-thread local live objects for worlds, pinned views, cursors, and subscriptions
- The control table should map each handle to one runtime-owned control object:
  - `Arc<World>` for worlds
  - one world handle plus stable local ids for runtimes and agents
  - one world handle plus one pinned `RevisionId` for world views
  - one world handle plus mutable reader state for trace cursors and observation subscriptions
- Thread-local handle registries are still the wrong model for this surface.
- The current honest low-level affinity is `owner`, not `any`, because live control objects stay owner-thread local today.
- Mutable reader handles like trace cursors and observation subscriptions should stay explicitly stateful and exclusive.
- World topology remains world-local semantic state and should stay separate from the process-global control table.
- If a root or ambient world exists, it should still be one normal world entry in the process-global control table.

## Runtime-owned binding cleanup
- The generator already owns the ABI shell for runtime-owned bindings, but it still leaves too much semantic codec boilerplate handwritten.
- Shared mechanical VM/native codec helpers should move into `platform/core/*` when they are genuinely cross-domain:
  - out-pointer helpers like `call_out`
  - VM string-handle and native-string conversion
  - common byte-slice and value-slice conversion
- `platform.runtime` is the forcing function for this cleanup, but the same improvements should be applied to other codec-heavy domains like `fs`, `net`, `tls`, `process`, and `crypto`.
- The handwritten per-domain layer should keep only:
  - actual runtime or host calls
  - domain-specific validation
  - domain-specific structural conversion
- The generator should eventually grow enough struct, optional, and array codec support that the handwritten domain files become mostly semantic glue instead of repetitive field plumbing.

## Introspection model
- Introspection should be able to answer nearly everything about live or captured runtime state efficiently.
- The low-level introspection API should support four access patterns:
  - point lookup by stable id
  - bulk enumeration by typed cursor or selector
  - read-consistent views pinned to one revision or one live read epoch
  - subscriptions for event and state-change streams
- Read-consistent views are important so userland can inspect topology, runtimes, agents, resources, queues, trace state, and snapshots without racing the live system.
- Trace should not be the only introspection channel.
- Introspection should include structured state queries for:
  - worlds
  - runtimes
  - agents
  - branches
  - revisions
  - checkpoints
  - images and snapshots
  - resources
  - topology nodes and edges
  - event-loop queues, watches, timers, and scheduling state
  - heap and VM summary state
  - diagnostics, hooks, policy, and simulation state
- Introspection should also support streaming channels for:
  - trace events
  - resource lifecycle events
  - topology change events
  - scheduler and execution markers
  - debugger and profiler events
- The primary low-level introspection payloads should be typed structs and binary blobs where needed, not stringified JSON.

## Relevant prior art
- V8 Inspector and Chrome DevTools Protocol: debugger transport and targeted object inspection.
- Perfetto and Chrome tracing: efficient append-only event timelines and streamed trace export.
- JVMTI and JFR: runtime control plus structured profiling and event capture in one VM.
- Erlang process inspection and tracing: cheap per-process state queries and event stream subscriptions.
- `/proc`, `perf`, `eBPF`, and DTrace: stable ids, point queries, bulk enumeration, and probes rather than one giant dump API.
- CRIU: image export, restore policy, and external resource classification.
- FoundationDB trace and simulation tooling: deterministic system-wide event capture and reproducible testing.

## Low-level platform namespace sketch
- `platform.runtime.core`
  - process-like creation and control for worlds, runtimes, agents
- `platform.runtime.lineage`
  - branch, revision, checkpoint, rewind, fork
- `platform.runtime.trace`
  - trace cursors, trace export, trace subscription, history labels
- `platform.runtime.inspect`
  - typed point lookup and bulk enumeration over runtime state
- `platform.runtime.snapshot`
  - image capture, snapshot export, import, restore
- `platform.runtime.topology`
  - topology queries and topology change feeds
- `platform.debug`
  - inspector transport, profile sessions, low-level debug break, trace sink wiring

## Completed work
- [x] Make `World` the public runtime root.
- [x] Move deterministic time advancement under `World`.
- [x] Trim replay to the core deterministic event set.
- [x] Make topology the canonical metadata graph.
- [x] Unify builtin topology kind classification under semantic facets.
- [x] Reduce simulation to one minimal world-owned event scaffold.

## Execution checklist
### Phase 1: capture model and image or snapshot split
- [x] Add shared `Capture` and `SnapshotCodec` traits in `destack_base`.
- [x] Implement `Capture` and `SnapshotCodec` for heap.
- [x] Implement `Capture` and `SnapshotCodec` for VM isolate.
- [x] Move runtime leaf subsystems from readiness gates to real capture paths.
- [x] Move agent and runtime image capture to the shared `Capture` model.
- [x] Make `World` implement `Capture` and `SnapshotCodec`.
- [ ] Remove the remaining ad hoc world-local capture seams where direct helpers still bypass the trait surface.

### Phase 2: lineage and checkpoint semantics
- [x] Introduce first-class `Branch`, `Revision`, `Checkpoint`, and `Image`.
- [x] Make checkpoints anchor revisions instead of acting as a parallel state model.
- [x] Make rewind and fork resolve through revision and image backing.
- [x] Move all remaining world fork and rewind entrypoints to revision-first semantics, with checkpoint as one indexing layer on top.
- [x] Make branch head movement explicit and uniform in the world lineage code.
- [x] Add durable branch and checkpoint metadata persistence beside trace data.

### Phase 3: trace substrate
- [x] Recenter the naming around `Trace` instead of `Replay`.
- [x] Keep trace world-global and event loops agent-local.
- [x] Move low-level runtime handle and inspect implementation out of `platform/runtime` into runtime-owned control-table and inspect modules.
- [x] Replace the thread-local runtime registry with one process-global control table plus owner-thread local live objects.
- [x] Keep `platform.runtime` thin and delegate shared control-table and inspect logic into runtime-owned modules.
- [x] Fix the low-level runtime surface review blockers:
  - honest handle affinity
  - one consistent `TraceDescriptor` meaning
  - no overstated VM/runtime snapshot surface
  - no overstated world resource owner model
- [x] Add direct low-level runtime snapshot coverage for create, describe, list, read, import, and restore.
- [x] Add revision-aware checkpoint indexing in the trace trailer.
- [x] Replace the old chunk naming with explicit immutable `TraceSegment` storage.
- [x] Make trace image and snapshot explicit instead of reusing one opaque trace state blob.
- [x] Make cursor validation use the live trace trailer instead of cached log-hash metadata.
- [x] Make revision-based rewind and fork restore trace from explicit lineage trace-image records.
- [x] Finish the shared-prefix trace model so branch-local divergence is explicit in lineage rather than implicit in cloned trace-store state.
- [x] Introduce real branch-local trace tails over shared immutable prefix segments.
- [x] Split causal `Trace` from high-volume runtime observation and instrumentation channels.
- [ ] Make restore and cursor state revision-aware instead of only raw trace-store-state aware.

### Phase 4: quiesce, suspend, and hibernate
- [x] Replace the old checkpoint flag with a world quiesce coordinator.
- [x] Keep capture one-phase and put temporal coordination in world quiescence, not in `Capture`.
- [x] Extend quiesce into suspend support for event-loop queues, timers, host events, and VM continuation images.
- [x] Define the current hibernate boundary explicitly for local and portable provider restore, and reject external rebinding loudly.
- [ ] Extend suspend across native continuations, finalizers, and initialized platform state.
- [ ] Add explicit external rebinding support for durable or cross-machine hibernate.
- [ ] Replace remaining idle-only assumptions with explicit capture barriers or richer capture support per subsystem.

### Phase 5: resources and platform state
- [x] Move resource capture to explicit resource kind and provider semantics.
- [x] Add the final resource portability and capture classification model.
- [x] Clean up `PlatformState` shape and remove the awkward cfg-gated structural drift.
- [x] Move platform state to the right module path and align it with the capture model.

### Phase 6: low-level runtime surface
- [ ] Finish the low-level `platform.runtime.*` surface around world, lineage, inspect, trace, and snapshot.
- [ ] Align the low-level debug surface with runtime trace and observation instead of conflating them.
- [ ] Add efficient pinned-view and structured introspection paths over topology, state, and trace.

## Current shape pass
- [x] Add world-owned `BranchId` and `Branch` types.
- [x] Add world-owned `CheckpointId` and `Checkpoint` types.
- [x] Add world-owned `SnapshotId` and `Snapshot` durable types.
- [x] Add world-owned lineage state with one real root branch.
- [x] Add `World` branch, checkpoint, and snapshot query methods.
- [x] Add `World::{checkpoint, rewind, fork}` methods with loud placeholder errors.
- [x] Move branch and checkpoint ids out of replay and into world-owned modules.
- [x] Delete the old top-level runtime snapshot module and `Agent::snapshot` path.
- [x] Add explicit world safepoint tracking.
- [x] Define durable world, runtime, and agent snapshot payload types.
- [x] Define the backend checkpoint boundary for engine-owned execution state.
- [x] Make `Agent` own live engine execution state.
- [x] Make `AgentSnapshot` own `EngineSnapshot`.

## Next implementation steps
- [x] Replace opaque VM snapshot bytes with one structured `VmSnapshot`.
- [x] Define one structured `HeapSnapshot` in `language/heap`.
- [x] Make VM heap snapshot backing immutable and shareable.
- [x] Make continuation, frame, and queued VM execution state snapshotable by construction.
- [x] Implement VM `Engine::snapshot` and `Engine::restore`.
- [x] Implement world snapshot capture from world, runtime, and agent state.
- [x] Implement world snapshot restore into fresh live state.
- [x] Implement world checkpoint creation with durable snapshot storage.
- [x] Implement world rewind from stored checkpoints.
- [x] Implement world fork with shared immutable replay and snapshot backing.
- [x] Add branch and checkpoint persistence metadata alongside replay logs.
- [x] Add world loading and reconstruction from persisted branch, checkpoint, snapshot, and replay state.

## Snapshot granularity
- Immutable code, modules, interned strings, and static constants should not be duplicated per world or per checkpoint.
- `WorldSnapshot` should capture shared causal state:
  - topology
  - policy
  - resources
  - simulation
  - clock
  - random
  - replay position
  - next ids and revision
- `RuntimeSnapshot` should capture runtime-local shared state:
  - runtime metadata
  - host and poller coordination state
- `AgentSnapshot` should capture agent-local execution state:
  - event loop
  - timers
  - watches
  - resource table
  - platform state
  - VM engine state
- `VmSnapshot` should be structured, not opaque:
  - heap snapshot
  - continuations
  - queued tasks and microtasks
  - execution-local interpreter state
- Heap sharing should happen at one page-local unit, not at one whole-heap blob and not at one object-per-copy granularity.
- `Heap` stays authoritative and agent-local.
- `ManagedHeap` and `RawHeap` own live `Page` values.
- `Page` owns page-local transient state like mark bits.
- `PageImage` is immutable snapshot-backed page state shared across checkpoints and forked worlds.
- `PageReference` is private heap implementation state and only models whether one live page slot is currently owned or image-backed.
- First write after fork or restore must detach only the touched shared page.
- Page-local mark bits replace any per-cell mark state in the managed heap.
- Live pin state and raw-exposure keepalive state stay outside `PageImage` and are rebuilt or re-established after restore.
- Resource payload snapshots should stay per-resource, not as one monolithic world resource blob.

## Validation goals
- [ ] Checkpoint then rewind restores identical world state.
- [ ] Forked worlds share replay prefix but isolate later mutations.
- [ ] Rewinding the same checkpoint twice is idempotent.
- [ ] Forked worlds diverge only after the fork point.
- [ ] VM-backed worlds restore exact execution state after rewind.
- [ ] Native worlds fail loudly until they provide durable backend snapshots.
- [ ] Forked VM worlds share immutable heap backing and only diverge on later writes.
- [ ] Page-local transient metadata like mark bits and live pins do not force CoW detaches or snapshot bloat.

## User-facing model
- `World` exposes deterministic execution, policy, topology, replay, checkpoints, rewind, and fork.
- `dsconfig.runtime` sets baseline execution defaults like time mode, random mode, replay payload mode, and static rules.
- Userland testing libraries layer workloads, campaigns, search, and invariants on top of those world APIs.
- Fast exploration comes from sparse checkpoints and many forked worlds running in parallel, not from many active branches inside one world.

## Generator cleanup
- The generator should stay module-agnostic.
- The generator should only own mechanical ABI shape and codec work, not domain semantics.
- The generator pipeline should stay split into:
  - `analyze/`
  - `model/`
  - `emit/`
- `platform/runtime` is the main calibration module for generator cleanup.
- `fs`, `net`, and `crypto` are the next calibration modules for shared mechanical codec cleanup.

### Remaining work

#### 1. `platform/runtime`
- [x] Thin `platform/runtime/native.rs` so it reads like one regular native binding layer.
- [x] Thin `platform/runtime/vm.rs` so it reads like one regular VM binding layer.
- [x] Keep `binding`, `descriptor`, `handle`, `request`, and `view` each scoped to one coherent job.
- [x] Push repeated structural request, filter, and descriptor flow out of `native.rs` and `vm.rs`.

#### 2. calibration modules
- [x] Apply the improved mechanical codec path to `fs`.
- [x] Apply the improved mechanical codec path to `net`.
- [x] Apply the improved mechanical codec path to `crypto`.
- [ ] Keep only semantic normalization local to each calibration module.

#### 3. generator
- [ ] Split the remaining oversized emit files into smaller renderer units with one clear job each.
- [ ] Keep moving naming and render mechanics onto `ModuleCodegen` and other real nouns.
- [ ] Push only structural encode/decode into shared generator or platform-core codec helpers.

## Next DST pass
- [x] Make the trace shared-prefix backing more explicit and persistent than `Arc<Vec<Arc<TraceSegment>>>`.
- [ ] Keep `Lineage` fully authoritative for revision and trace-image identity while treating `TraceImage` as materialized state only.
- [ ] Re-review fork and rewind costs after the trace backing change and measure the remaining shell/bootstrap overhead.
- [ ] Re-audit page image sharing granularity after the mark-bit split, especially whether per-page `Arc<PageImage<_>>` should remain the immutable sharing unit.

### Trace redesign target
- The trace fast path should optimize for:
  - one mode check when trace recording is off
  - append-only writes when trace recording is on
  - O(1) fork sharing
  - low branch fanout memory overhead
  - sequential replay over immutable recorded blocks
- The trace should use one coarse immutable sharing unit and one small mutable frontier:
  - `TraceBlock`: one immutable shared block of frozen `TraceSegment` values
  - `TraceTail`: one mutable append frontier with one active builder and one small local sealed segment buffer
  - `TraceSegment`: one immutable encoded event segment value, not one refcounted object
- The intended shape is:
  - `TraceLog { branch_id, next_sequence, head: Option<Arc<TraceBlock>>, tail: TraceTail, trailer }`
  - `TraceBlock { parent: Option<Arc<TraceBlock>>, segments: Box<[TraceSegment]>, segment_count, byte_count }`
  - `TraceTail { active: TraceSegmentBuilder, sealed: Vec<TraceSegment> }`
- The trace should not use:
  - `Arc<Vec<Arc<TraceSegment>>>`
  - one `Arc` per segment
  - one flat shared prefix array as the persistent history representation
- Fork should work by:
  - sharing the current `head`
  - materializing or freezing only the local tail
  - starting the child with one fresh empty mutable tail
- Rewind and restore should work by:
  - restoring one `TraceImage` that reattaches the shared immutable block chain
  - recreating one fresh mutable frontier for future divergence
- The trace-local cursor should:
  - cache the shared block path once per visible head
  - seek by segment/header boundaries instead of event-scanning from zero
- The trace redesign should stay aligned with the heap CoW/page-image design:
  - share coarse immutable units
  - keep one small mutable frontier
  - avoid nested refcounting and pointer chasing in hot paths

### Page CoW follow-up
- [x] Move managed page mark bits out of shared page backing so GC marking does not detach page contents.
- Heap/page CoW should now follow the same rule as trace:
  - immutable page backing is shared
  - mark bits are live frontier state
  - only real cell or occupancy writes detach shared pages
- The next heap-specific question is:
  - whether per-page `Arc<PageImage<_>>` is still the right immutable sharing granularity
  - or whether a coarser page-set image would reduce metadata and refcount overhead without hurting detach costs
