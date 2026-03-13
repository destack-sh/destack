# HANDOFF

## Purpose

This file is a deliberately exhaustive handoff for the current Destack runtime and heap workstream.

This handoff exists because the work has been large, intertwined, and repeatedly at risk of being over-compacted in conversation.

This document is intended to let the next session resume with full context and no hidden assumptions.

This document is intentionally long.

This document should be treated as the canonical narrative summary of what was discussed, what was changed, what was decided, what was deferred, and what still remains.

This document is written from the perspective of the current worktree at `/Users/florian/symbol/destack-3`.

## High level summary

The original workstream started as a half-complete heap and page refactor for the Destack runtime.

The heap refactor was closely tied to the broader DST branch, replay, restore, fork, and rewind architecture.

The initial goal was to finish the heap/page refactor and make the runtime substrate serious enough for real DST.

The scope then expanded in three major ways.

The first expansion was a full cleanup of the heap architecture and naming.

The second expansion was a cleanup and reduction of the runtime history, replay, branching, and query model.

The third expansion was a clarification of the long-term native-first-class constraint, which implies a later backend-neutral byte and layout based heap payload redesign.

At the current point in time, the heap-side structural refactor has been checked in as a major step.

The runtime history and query cleanup is still in the working tree and not yet committed.

The next major architectural pass after the runtime cleanup is the backend-neutral heap payload redesign.

## Original user goals

The user explicitly wanted:

- a serious language runtime design
- low or competitive memory amplification
- cheap allocation
- cheap CoW, forking, rewind, and sharing
- the fewest possible moving parts
- the cleanest, most meaningful nouns and verbs
- no transitional logic
- no legacy cruft
- no “for now” hedges in design docs
- final-system thinking

The user also explicitly wanted:

- DST and replay to be first-class
- native and VM to both be first-class
- stable managed references in the language model
- serious performance, not just correctness
- exact isolation, especially around memory limits

The user was also very clear that:

- recommendations must be challenged when they look wrong
- awkward naming should be called out
- the design should be hardened, not papered over
- docs should describe the final intended design, not transient status

## Initial review of the codebase and docs

The original files the user asked to review were:

- `README.md`
- `language/DESIGN.md`
- `language/SPECIFICATION.md`
- `language/runtime/README.md`
- `RUNTIME_PLAN.md`
- `HANDOFF.md`

The initial reading of the codebase found:

- a coherent but still settling `World` / `Runtime` / `Agent` split
- a coherent branch and revision model
- a cleaner trace and image story than before
- a half-complete heap and page refactor
- a still transitional managed heap path
- a still awkward managed/raw page and span naming model
- a still too-VM-shaped heap payload representation

The first concrete findings from the code review were:

- the managed heap GC path still copied slot vectors during tracing in places it should not
- resizing managed cells still did rebuild-and-replace instead of better in-place or page-backed growth behavior
- the VM still knew too much about inline versus overflow storage shape
- the crate split was not fully symmetrical and some naming still reflected old storage assumptions

At this stage, the recommendation was to finish the heap cut first, before doing broader DST substrate work.

## Early heap discussion

The user immediately pushed on whether page classes should be done now as part of the foundational refactor.

The response was that there were two distinct ideas hiding under “page classes”.

The first idea was semantic class separation such as managed pages, overflow pages, and raw pages.

The second idea was class-specific sizing and allocation policy.

The code already had part of the first idea, but not the final form of the second.

The key reasons given for not immediately doing the full class sizing work were:

- the managed storage and slot boundary was still settling
- pointer arithmetic and capacity assumptions were still too baked into the representation
- class tuning without stable object-shape data would be guesswork

The user then made it clear that performance was a hard priority and that cheap forking and CoW were a core requirement.

The user also noted that stable pointers matter because the language has a Go-like ability to take the pointer of a managed reference.

## Stable references and the pointer model

The early design discussion established a critical distinction:

- managed references should be stable logical references
- raw pointers should be physical or address-sensitive pointers

At that stage, the constraint was still being reasoned about in the context of a non-moving heap.

The key invariant articulated was:

- `ManagedPointer` at the time, later renamed to `ManagedReference`, should be stable object identity
- page slot should be stable storage location while live
- internal heap metadata can change
- object location should not move in the non-moving design

The heap was then changed to remove direct arithmetic derivation of page locations from pointer ids.

Instead:

- managed and raw ids resolve through heap-local location tables
- a `PageSlotAddress`, later renamed `PageSlot`, became the low-level page/slot coordinate
- the heap core stopped relying on single global page-capacity arithmetic

This was an important architectural step because it decoupled identity from page arithmetic.

## Broad heap structural pass

The early big heap pass implemented:

- heap-local `id -> page slot` address tables for managed and raw heaps
- a new page slot location type
- overflow span changes to use stable page slot starts instead of flat global indices
- managed GC traversal without allocating temporary vectors for child scanning
- managed overflow resize improvements for in-place shrink and some in-place growth
- movement of storage-shape knowledge out of the VM and back into `ManagedHeap`

This improved the heap shape materially and validated the direction.

At that point, the next stated target was:

- class-aware page directories
- allocator policy
- stronger heap image and trace structural sharing

## Class-aware storage and the question of what a pointer is

The next major design discussion was about class-aware storage and pointer identity.

The proposed final class-aware shape was:

- one persistent heap root
- one page directory under it
- page entries tagged by page class
- per-class allocators and free structures
- immutable page images as CoW leaves

The major candidate classes discussed were:

- managed cell pages
- overflow value pages
- raw value pages
- raw byte pages
- large runs or spans

At that point, the user asked what a pointer actually is for Destack.

The answer that settled out was:

- a managed pointer should not be a naked machine address
- it should be a stable managed object reference with provenance and optional offset
- a raw pointer should be the physical or address-sensitive form

This later evolved into:

- `ManagedReference`
- `RawPointer`

The important model was:

- managed reference equals stable identity
- raw pointer equals address-sensitive pointer capability
- dereference goes through heap metadata
- page class and allocator layout should not leak into the language-visible managed reference model

## Maximum performance discussion

The user asked directly whether this architecture could be made fast enough to be comparable to Go or JVM style runtimes.

The answer was:

- not as a scalar claim across all workloads
- but yes, it can be a serious high-performance runtime for the right constraints

The core point was that control over the full stack allows optimization in ways broader than choosing a GC algorithm.

The recommended performance strategy was:

- stack and register placement first
- inline layout second
- region or arena allocation where possible
- managed heap for the subset of objects that truly need identity and shared lifetime
- raw memory for explicit systems-level needs
- class-aware pages underneath all of it

The big insight was:

- the managed heap should not be the storage for everything
- the compiler and runtime should avoid creating managed heap pressure whenever possible
- the heap should be for the subset of values that really need managed identity and durable participation in the DST substrate

The discussion also covered:

- no need to cargo-cult a nursery
- possibility of hybrid storage classes
- importance of exact value placement
- eventual possibility of a more layout-directed runtime

## Managed logical references and pinning

There was then a deep discussion about whether managed references should be logical rather than physical.

The eventual consensus was:

- managed references should be logical references
- raw pointers should be physical pointers
- pinning should be the bridge where stable raw exposure is needed

The user worried whether that would compromise the “universal software” goal.

The answer was:

- no, not if raw and pinned memory remain first-class
- yes, if they are treated as second-class escape hatches

The recommendation was:

- managed references
- pinned managed references
- raw pointers

as one coherent ladder rather than two different worlds.

Later, after more discussion, pinning was simplified substantially for the non-moving design:

- in a non-moving heap, pinning mostly means keepalive plus explicit raw-address exposure
- pinning no longer needed to primarily mean “do not move”
- a small lexical pin model or internal pin capability was the right fit

The user then clarified that non-moving was indeed the current intended model.

So pinning settled into:

- keep object alive
- allow safe raw/native address exposure
- stay out of durable heap images

## Language, MIR, lowering, and runtime doc model

We then explicitly modeled the managed/raw/pinned split across the stack.

The affected docs were:

- `language/DESIGN.md`
- `language/SPECIFICATION.md`
- `language/mir/README.md`
- `language/compiler/src/lower/README.md`
- `language/runtime/README.md`

The semantic changes codified there were:

- managed references have stable identity, not guaranteed native-address semantics
- `&T` is a borrow/view, not a universally stable machine pointer
- raw pointers are the explicit physical pointer form
- pinning is a storage/address-exposure property, not a broad second pointer universe
- MIR `ref<managed ...>` must not imply physical-address semantics
- runtime and compiler lowering must agree on that

This was not just a runtime implementation detail.

It was explicitly framed as a language-semantic and compiler-semantic decision.

## Clarifying `&T`

There was a focused discussion on what the value of `&T` is in this system.

The agreed answer was:

- `&T` is a safe borrowed view
- it carries aliasing and lifetime meaning
- it is not a promise of raw-address permanence

This aligned the design more closely with:

- JVM/CLR logical references
- Rust `&T` versus `*T`
- selective pinning ideas

It also established a crucial distinction:

- managed reference is identity
- `&T` is borrowed access
- `*T` is raw pointer

## Prior art discussion

Prior art explicitly mentioned and used to guide the design included:

- JVM / HotSpot
- CLR
- V8
- Go
- mimalloc
- jemalloc
- persistent vectors and structurally shared collections
- rr
- PANDA
- FASTER
- ARC / CAR for cache-like adaptive policies
- OpenTelemetry style spans and tracing on the observability side

The key lessons drawn were:

- serious runtimes share one managed heap semantic model across execution tiers
- they do not necessarily share a single payload representation everywhere
- exact counters and allocation-path admission are the systems-grade way to do hard limits
- telemetry and replay traces must not be conflated

## Heap naming and noun cleanup

A large part of the workstream then became a systematic noun and verb cleanup of the heap.

The main renames and reductions included:

- `ManagedPointer` -> `ManagedReference`
- keep `RawPointer`
- `ScanDescriptorHandle` -> `ManagedLayoutId`, then later to `LayoutId` by reusing MIR’s canonical concept
- `PageSlotAddress` -> `PageSlot`
- `StackCell` -> `ValueCell`
- `cell/` module -> `allocation/`, then later allocation types moved under `managed/` and `raw/`
- `ManagedSpanPage` -> `ValuePage`
- `PageBacking`, `PageState`, `PlainPage`, and `MarkedPage` were all simplified away or reorganized
- `ImageIndex` -> initially discussed as `SharedIndex` / `SharedVector` / `ChunkedVector`, and then ultimately reduced to a private `Vector`

The current principal heap nouns are:

- `Heap`
- `HeapImage`
- `HeapLimits`
- `ManagedHeap`
- `RawHeap`
- `ManagedAllocation`
- `RawAllocation`
- `ManagedPage`
- `RawPage`
- `ManagedSpan`
- `RawSpan`
- `ManagedReference`
- `RawPointer`
- `PageSlot`
- `LayoutId`
- `Value`
- `ValueCell`

The explicit goal was to have the fewest meaningful nouns possible while preserving the real semantic distinctions.

## Heap module reorganization

The heap crate was significantly reorganized.

The current shape is:

- `language/heap/src/heap/`
- `language/heap/src/managed/`
- `language/heap/src/raw/`
- `language/heap/src/page/`
- `language/heap/src/value/`
- `language/heap/src/string/`

The intent was:

- shared heap composition in `heap/`
- managed-specific implementation in `managed/`
- raw-specific implementation in `raw/`
- page infrastructure in `page/`
- value-layer types in `value/`
- string-specific support in `string/`

Specific file and directory cleanup included:

- `heap/store.rs` renamed to `heap/heap.rs`
- `time/stamp.rs` was later identified for splitting but not yet done
- `page/storage.rs` was recognized as a bad fit for `Page` and later `Page` was moved to `page/live.rs`
- `string/string.rs` was renamed to `string/layout.rs`

The heap side is now much closer to the semantic structure of the runtime.

## Page and CoW design

The page layer was one of the big simplification targets.

The agreed model became:

- `PageImage` is the immutable durable leaf
- `Page` in `page/live.rs` is the live mutable CoW page
- `ManagedPage` wraps page storage plus managed-side metadata
- managed and raw heaps use page slots as the stable anchor points for allocations

The heap no longer treats page wrappers and storage wrappers as different public layers.

The CoW design now works like this:

- capture one immutable `HeapImage`
- fork or restore from that image
- keep sharing image leaves
- detach only the touched page or span on first mutation

This applies to:

- managed pages
- managed value pages
- raw pages
- raw spans
- large spans

## Stable locations and location tables

A central architectural step was introducing and then stabilizing location tables.

The managed heap now resolves:

- managed reference id -> `PageSlot`

The raw heap now resolves:

- raw allocation id -> `PageSlot`

The location tables are part of the durable image.

The stable slot model lets the runtime:

- separate identity from page arithmetic
- support stable references
- share page/span leaves structurally
- later evolve the payload model without changing reference identity

## Heap image and snapshot model

The heap image model was progressively refined.

The current shape is:

- `HeapImage`
- private `ManagedImage`
- private `RawImage`

The private image-side section nouns still exist because they own per-side image and snapshot logic.

However, the public semantic model centers on:

- `Heap`
- `HeapImage`

The image captures:

- managed page images
- raw page images
- managed and raw spans
- large spans
- location tables
- allocator/free state that must survive restore

The design intentionally keeps:

- GC mark state
- pin counts
- other live-only execution state

out of the durable image.

## Hard limits and exact accounting

This was one of the hardest and most important parts of the heap work.

The user explicitly wanted exact isolation-grade heap accounting.

The first attempt involved a `HeapBudget` object and post-mutation limit checks.

That was eventually rejected as not pristine and not systems-grade enough.

The final direction that was established is:

- `Heap` owns the mutation boundary
- exact heap usage should be tracked by exact running counters
- every growth-capable operation must plan exact retained-byte deltas
- hard limits must be admitted before commit
- post-checks are only verification

During the current heap pass, several correctness issues were found and fixed:

- cumulative managed allocation admission was previously comparing multiple substeps against one stale baseline
- `Heap::set_limits` previously mutated the active limits before validating them
- `HeapLimits` cloning was cleaned up by making `HeapLimits` `Copy`
- noisy `Arc<Mutex>`-style budget handling was removed

At the current checkpoint:

- hard-limit correctness is materially improved
- the remaining serious follow-up is the exact incremental retained-byte counter model on every mutation path

This was explicitly called out as still the final systems-grade thing to do later.

## Runtime heap config cleanup

The runtime heap config was collapsed into one surface.

Previously, heap config was split across:

- `runtime.heap`
- `runtime.limits.heap`

That was judged to be bloat.

It was collapsed into one runtime config surface in:

- `language/workspace/src/config/runtime/heap.rs`

The resulting runtime config includes:

- `heap_growth_percent`
- `heap_soft_limit_bytes`
- `heap_initial_bytes`
- `max_bytes`
- `max_managed_bytes`
- `max_raw_bytes`
- `managed_large_span_values`
- `raw_large_span_bytes`

The boolean `HeapOptions.enabled` was removed because it was judged to be a weak and misleading GC toggle.

The view taken from Go/JVM/V8 style runtimes was:

- one heap tuning surface
- pacing and limits together
- no fake `enabled` boolean

## MIR layout cleanup

One key cleanup was making `LayoutId` always valid.

Originally, MIR had:

- `LayoutId`
- `INVALID_LAYOUT_ID`
- validity checks on the MIR side

That was judged to be wrong layering.

The final cleaned-up model is:

- MIR owns `LayoutId` as a real canonical id
- absence of a layout is represented as `Option<LayoutId>` at the heap side

This removed a sentinel from the MIR layer and cleaned up the semantics.

This also fits the later native-ready layout-driven heap direction better.

## Fallible VM allocation

A major side-effect of the exact heap mutation boundary work was making VM allocation fallible.

This was explicitly discussed as the correct direction.

The reasoning was:

- heap allocation can fail under hard limits
- pretending it is infallible forces incorrect semantics
- exact failure semantics are the systems-grade answer

This led to:

- VM heap and string allocation becoming fallible
- runtime and platform code needing to propagate those failures
- generator fallout and test harness fallout

The checked-in commits around this were:

- `refactor(language/vm): make heap allocation fallible`
- `refactor(language/runtime): propagate fallible vm allocation through platform`

It was also explicitly discussed that:

- native should share the same semantic failure model for heap-participating allocations
- not every native allocation will actually enter the managed heap
- but those that do should still be fallible

## Runtime history model cleanup

The runtime history model underwent a large cleanup pass.

This was driven by the user’s unease with `WorldCommand` and `WorldInvocation`.

The key design judgment was:

- `WorldCommand` and `WorldInvocation` were splitting history by implementation path, not by semantic role

The reduction that was agreed on was:

- `TraceRecord::Input(Input)`
- `TraceRecord::Outcome(Outcome)`
- `TraceRecord::Anchor(String)`

And then:

- `Input`
- `Mutation`
- `Outcome`
- anchor strings

as the world-level trace payload nouns.

Removed:

- `WorldCommand`
- `WorldInvocation`
- `ReplayEntry`
- separate `Anchor` type
- separate `TraceValidator` type

Important subtleties:

- `Mutation` still earns itself because it is the one useful world-specific payload family
- `Checkpoint` and `Snapshot` are not foundational runtime nouns
- they are derived conveniences over revisions and images

## Runtime foundational nouns

The runtime side was explicitly re-audited for foundational nouns.

The final current set agreed on was:

- `World`
- `Runtime`
- `Agent`
- `Topology`
- `Policy`
- `Trace`
- `Branch`
- `Revision`
- `Moment`
- `Image`
- `Observation`

Derived or view nouns:

- `Event`
- `Transition`
- `Checkpoint`
- `Snapshot`

The idea is:

- `Trace`, `Image`, `Revision`, `Branch`, and `Moment` form the DST substrate
- `Observation` is the emitted observability stream
- `Event` and `Transition` are query views
- `Checkpoint` and `Snapshot` are convenient API-level concepts, not deep sources of truth

## Observation model

The observability side was intentionally separated from the causal replay side.

The agreed split is:

- `Trace` = causal replay witness
- `Observation` = emitted visibility stream
- telemetry tracing, metrics, logs, assertions = built on `Observation`

Observation carries:

- `moment`
- `scope`
- `category`
- `name`
- `tags`
- `data`

The current `Scope` families are:

- world
- runtime
- agent
- entity
- edge
- resource

The goal is:

- one emitted stream for runtime and userland observability
- one event-query surface over both trace projections and observations
- no duplication of runtime event systems

## Event and Transition model

The query model was also reduced and cleaned.

`Event` is:

- one query-visible fact at one `Moment`
- projected from `TraceRecord` or `Observation`

`Transition` is:

- one query-visible step between adjacent `Moment`s
- caused by one authoritative trace step

The final shape stayed intentionally small.

No diff log is stored.

No separate transition log is stored.

The power comes from:

- `Moment`
- `Event`
- `Transition`
- `lineage.view(moment)`

and derived state comparison.

This is intended to be stronger than an Antithesis-style event-only model while keeping the substrate minimal.

## Query surface cleanup

A lot of work went into making the query surface smaller and more direct.

The current preferred verbs are:

- `kind(...)`
- `name(...)`
- `category(...)`
- `branch(...)`
- `descendants_of(...)`
- `between(...)`
- `up_to(...)`
- `on(scope)`
- `tagged(...)`

Earlier forms that were removed or reduced included:

- `of_kind(...)`
- `named(...)`
- `in_category(...)`
- `on_branch(...)`
- `scoped_to_*`

The rule was:

- use the most direct noun-based verb where possible
- let `Scope` and the query nouns do the work

`LineageView` remains the committed-history query root.

`World` remains the live branch-local query root.

The factoring of query code was also improved:

- `event.rs` owns event query nouns and projection
- `transition.rs` owns transition query nouns and projection
- `query.rs` was reduced to lineage and moment query plumbing

## Runtime file factoring cleanup

The `runtime/world` and `runtime/replay` trees were re-factored substantially.

Notable file moves and splits:

- `checkpoint.rs` -> `history.rs`
- `access.rs` introduced for access guards
- `event.rs` split out
- `transition.rs` split out
- `topology.rs` split into `topology/`
- `input.rs` introduced
- `mutation.rs` introduced
- `moment.rs` introduced
- `view.rs` introduced

The intent was to align filenames with the principal nouns.

This still left some cleanup work, but it materially improved the shape.

## Remaining runtime cleanup concerns raised during review

During the latest runtime audit, the user explicitly raised:

1. `should_materialize_revision_image` could be simplified to a boolean.

2. There were still absolute references like `super::CommittedRevision`.

3. `runtime/time/stamp.rs` felt wrong and should likely be split into `nanos.rs` and `instant.rs`.

4. `world/` still felt too unwieldy and maybe the runtime tree should be reorganized further.

5. `ExclusiveAccessLease` was questioned and then renamed to `ExclusiveAccessGuard`.

6. `EntrypointRef` duplication was questioned because runtime already has an entry reference notion.

7. `TransitionQuery` and `WorldTransitionQuery` looked bloated.

8. `tell()` was a strange method name.

9. `TraceValidator` and the naming of `Outcome` and `Anchor` were questioned.

10. The whole query layer might want to be factored further.

11. The hardcoded `memory://checkpoint/...` path in checkpointing was odd.

Several of these items were already addressed.

Some were still pending in the current unstaged runtime diff at the time this handoff was written.

## Specific runtime cleanup already performed

Already fixed:

- `ExclusiveAccessLease` -> `ExclusiveAccessGuard`
- `tell()` -> `sequence()`
- `TraceValidator` removed and replaced by an internal `Validator`
- `checkpoint.rs` renamed to `history.rs`
- stale absolute-reference offenders cleaned in several places
- `World::resolve(...)` renamed to `World::accept(...)`
- telemetry category settled on `Telemetry`
- `World::policy()` and `Image::policy()` now expose public `Policy`
- `EntryRef` -> `EntryReference`
- `entry_ref()` -> `entry_reference()`
- `WorldEventQuery` and `WorldTransitionQuery` removed
- `time/stamp.rs` split into `time/nanos.rs` and `time/instant.rs`
- `runtime/replay/` renamed to `runtime/trace/`
- `runtime/core/` renamed to `runtime/process/`
- `TraceCheckpointIndex::memory_path(...)` added

Still in-flight or pending when this handoff is being written:

- deciding whether to rename `WorldInstant` -> `Instant`
- cleaning remaining `super::...` references systematically
- continuing to verify that the new `world/`, `history/`, `topology/`, `process/`, and `trace/` split is final enough

## Why `Trace` was kept for now

A discussion happened around whether the causal replay substrate should be renamed from `Trace`.

The concern was:

- “trace” collides badly with OpenTelemetry-style tracing in a fully integrated stack

Several alternatives were discussed:

- `History`
- `Journal`
- `Log`

The eventual conclusion was:

- keep `Trace` for now because it remains the clearest and most accurate word for the causal replay witness
- do not use “trace” in the core runtime model for telemetry
- use `Observation` plus spans and telemetry concepts on the observability side

This issue may be revisited later, but it is not the current focus.

## Userland telemetry, OpenTelemetry, and observations

Another major discussion clarified how telemetry tracing and logs should fit.

The current model is:

- causal `Trace` stays purely for replay-correctness
- telemetry traces, spans, metrics, logs, assertions, and diagnostics are all one family of `Observation`

The runtime should:

- emit its own observations
- allow libraries and userland to emit observations
- make them branch-aware and moment-aware automatically

OpenTelemetry-style tracing is expected to be:

- represented internally as observation data with span and trace identifiers
- optionally exported to OpenTelemetry
- not used as the runtime’s own canonical causal model

This preserves:

- replay correctness
- observability richness
- one unified query surface

## Plain world path versus DST path

The user wanted both:

- a boring plain-world runtime API
- a powerful DST API layered on top

The plain-world path should feel like:

- `World::new(...)`
- `spawn_runtime(...)`
- `run_entrypoint(...)`
- `tick()`
- `ingest(...)`
- `accept(...)`
- `observe(...)`

The DST path should layer on top with:

- `checkpoint(...)`
- `fork(...)`
- `rewind(...)`
- `snapshot(...)`
- `lineage()`
- `events()`
- `transitions()`
- `view(moment)`

This layering is one of the core design goals.

Normal runtime code should not need to think about revisions and branches constantly.

DST tooling and analysis should be able to go deep when needed.

## The heap native-first-class problem

A major late-stage realization was that the heap, although much improved, is still too VM-shaped.

The current managed heap still stores payloads fundamentally as `Value`-oriented payloads.

That means:

- inline managed payloads are `Value`-shaped
- `ManagedSpan` is value-oriented
- large managed spans are `Arc<[Value]>`
- the external page nouns still reflect VM-shaped payload expectations

The user correctly pointed out that if native is first-class, this is not a sufficient final heap model.

The agreed future direction is:

- keep one shared semantic heap for VM and native
- do not split into VM heap and native heap
- make managed payload storage backend-neutral and layout-driven
- make payload bytes plus `LayoutId` the real substrate
- keep VM slot access as one convenience view over some layouts
- let native use direct typed layout access over the same heap semantics

This is the next major architectural pass after the current runtime cleanup.

It is not yet implemented.

## Why the current heap is still worth checking in first

Although the heap is still too VM-shaped in payload representation, the current heap step was still worth checking in because it solved independent foundational problems:

- ownership boundaries
- page/span naming and factoring
- image and CoW structure
- limits and mutation boundary
- managed/raw split cleanup
- exactness and test coverage for many correctness paths

The byte/layout redesign will build on top of that more stable base.

The user explicitly accepted the strategy of checking in the current heap step first and then starting the backend-neutral payload redesign next.

## Current checked-in commits

The current checked-in commits on the branch, after the heap-first check-in and related follow-ups, are:

- `1369127241` `feat(language/heap): scaffold experimental CoW heap`
- `134625222d` `refactor(language/mir): remove invalid layout ids`
- `e410298162` `refactor(language/workspace): collapse runtime heap config into one surface`
- `4a4386b603` `refactor(language/vm): make heap allocation fallible`
- `0e9b1f7c85` `refactor(language/runtime): propagate fallible vm allocation through platform`

These are the most recent relevant commits in the branch at the time of writing.

## Current unstaged runtime diff

At the time this handoff is being written, the following runtime-side files are still modified or newly added and are not yet committed.

Modified files:

- `language/runtime/README.md`
- `language/runtime/src/platform/runtime/abi.generated.rs`
- `language/runtime/src/platform/runtime/descriptor.rs`
- `language/runtime/src/platform/runtime/request.rs`
- `language/runtime/src/runtime/core/agent.rs`
- `language/runtime/src/runtime/core/execute.rs`
- `language/runtime/src/runtime/core/runtime.rs`
- `language/runtime/src/runtime/engine/engine.rs`
- `language/runtime/src/runtime/engine/entry.rs`
- `language/runtime/src/runtime/engine/vm.rs`
- `language/runtime/src/runtime/memory/gc.rs`
- `language/runtime/src/runtime/memory/pacer.rs`
- `language/runtime/src/runtime/memory/root.rs`
- `language/runtime/src/runtime/policy/hook.rs`
- `language/runtime/src/runtime/random/vm.rs`
- `language/runtime/src/runtime/trace/chunk.rs`
- `language/runtime/src/runtime/trace/codec.rs`
- `language/runtime/src/runtime/trace/entropy.rs`
- `language/runtime/src/runtime/trace/event.rs`
- `language/runtime/src/runtime/trace/log.rs`
- `language/runtime/src/runtime/trace/mod.rs`
- `language/runtime/src/runtime/trace/random.rs`
- `language/runtime/src/runtime/trace/reader.rs`
- `language/runtime/src/runtime/trace/replay.rs`
- `language/runtime/src/runtime/trace/time.rs`
- `language/runtime/src/runtime/tests/replay.rs`
- `language/runtime/src/runtime/tests/scheduler.rs`
- `language/runtime/src/runtime/tests/tests.rs`
- `language/runtime/src/runtime/tests/world.rs`
- `language/runtime/src/runtime/world/apply.rs`
- `language/runtime/src/runtime/world/constants.rs`
- `language/runtime/src/runtime/world/lineage.rs`
- `language/runtime/src/runtime/world/mod.rs`
- `language/runtime/src/runtime/world/observe.rs`
- `language/runtime/src/runtime/world/runtime.rs`
- `language/runtime/src/runtime/world/snapshot.rs`
- `language/runtime/src/runtime/world/tick.rs`
- `language/runtime/src/runtime/world/world.rs`

Deleted files:

- `language/runtime/src/runtime/trace/validator.rs`
- `language/runtime/src/runtime/world/checkpoint.rs`
- `language/runtime/src/runtime/world/command.rs`
- `language/runtime/src/runtime/world/ingress.rs`
- `language/runtime/src/runtime/world/topology.rs`

New files:

- `RUNTIME_PLAN.md`
- `language/runtime/src/runtime/world/access.rs`
- `language/runtime/src/runtime/world/event.rs`
- `language/runtime/src/runtime/world/history.rs`
- `language/runtime/src/runtime/world/input.rs`
- `language/runtime/src/runtime/world/moment.rs`
- `language/runtime/src/runtime/world/mutation.rs`
- `language/runtime/src/runtime/world/query.rs`
- `language/runtime/src/runtime/world/transition.rs`
- `language/runtime/src/runtime/world/view.rs`
- `language/runtime/src/runtime/world/topology/`

This is the area where the current cleanup is still in progress.

## Open runtime cleanup items in concrete terms

The next concrete runtime cleanup tasks are:

1. Decide whether to keep `WorldInstant` or rename it to `Instant`.

2. Clean every remaining `super::...` absolute-ish reference in `runtime/world`, `runtime/history`, and `runtime/trace`.

3. Continue to verify that file names match the nouns they contain.

4. Continue to verify that `world/` is no longer hiding broad non-world concerns after the new sibling splits.

5. Begin the backend-neutral heap payload redesign from the now-cleaner runtime baseline.

## Specific code-level pending notes

### `EntryReference`

The user does not like the abbreviation `ref`.

The current type is in:

- `language/runtime/src/runtime/engine/entry.rs`

It should be renamed to:

- `EntryReference`

and the method:

- `entry_reference()`

should become:

- `entry_reference()`

This change should be propagated through:

- `runtime/engine/engine.rs`
- `runtime/engine/vm.rs`
- `runtime/core/runtime.rs`
- `runtime/core/execute.rs`
- `runtime/world/input.rs`
- `runtime/world/runtime.rs`
- tests and any platform ABI code that refers to the entry reference type

### `WorldEventQuery` and `WorldTransitionQuery`

These currently exist as separate wrappers for live branch-local queries.

The user correctly identified them as bloated.

The cleaner model is:

- `EventQuery`
- `TransitionQuery`

for lineage-rooted committed queries

and direct methods on `World` for live branch-local query ranges.

So:

- remove `WorldEventQuery`
- remove `WorldTransitionQuery`
- remove `World::events()` and `World::transitions()` if they only return those wrappers
- keep `events_up_to`, `events_between`, `transitions_up_to`, `transitions_between`

### `time/stamp.rs`

This file currently defines:

- `Nanos`
- `WorldInstant`

The user strongly disliked this shape.

The likely cleanup is:

- `time/nanos.rs`
- `time/instant.rs`

Then update:

- `time/mod.rs`

to re-export both.

The question of whether `WorldInstant` should become `Instant` is still open.

The tradeoff is:

- `Instant` is cleaner inside `runtime::time`
- `WorldInstant` avoids collision with `std::time::Instant`

This is still a design judgment call.

### Checkpoint path helper

The hardcoded:

`format!("memory://checkpoint/{}", checkpoint_id.get())`

inside checkpointing was explicitly called out as odd.

The proposed fix is:

- add `TraceCheckpointIndex::memory_path(checkpoint_id)`

to:

- `language/runtime/src/runtime/trace/header.rs`

Then use it in `history.rs` instead of formatting inline.

### `Outcome` naming

The user asked why `Outcome` and `Anchor` are not `TraceOutcome` and `TraceAnchor`.

The current answer is:

- inside `runtime/replay`, the module scope already establishes they are trace-facing payloads
- fewer prefixes is cleaner once the old duplicate world nouns are gone

No change has been made there yet.

### `Anchor`

The user questioned whether `Anchor` should exist at all, given `Observation`.

The current position is:

- `Anchor` exists because there is a difference between:
  - a causal history marker that participates in the replay/history model
  - an emitted observation

Right now, anchors are plain strings in `TraceRecord::Anchor(String)`.

This is simple, but may still be revisited if a more explicit naming or metadata shape becomes preferable.

## What remains correct and should not be undone

The following major decisions should be treated as settled unless a later deeper architectural pass overturns them deliberately:

- keep one managed heap semantic model for VM and native
- do not build separate VM and native heaps
- keep managed references and raw pointers as distinct concepts
- keep `Trace` for the causal replay witness for now
- keep `Observation` as the common emitted observability stream
- keep `Event` and `Transition` as derived query views
- keep `Moment` as the primary history coordinate
- keep the plain-world API and DST API layered, not blended
- keep heap mutation ownership on `Heap`
- keep exact limits and exact isolation as a hard requirement
- keep page/span/image CoW as the right heap substrate

## How the runtime is intended to be used

The intended normal runtime usage is:

```rust
let world = World::new(options)?;
let runtime_id = world.spawn_runtime(engine, runtime_options)?;
world.run_entrypoint(runtime_id, entry, args)?;
world.tick()?;
```

The intended host/platform usage is:

```rust
while let Some(input) = platform.next_input()? {
    world.ingest(input)?;
    world.tick()?;
}
```

The intended DST usage is:

```rust
let revision = world.checkpoint("baseline")?;
let child = world.fork(revision.revision_id)?;
world.rewind(revision.revision_id)?;
```

The intended query usage is:

```rust
let lineage = world.lineage();
let failures = lineage
    .events()
    .branch(branch.id)?
    .observations()
    .name("invariant.failure");
```

```rust
let overloaded = lineage
    .moments()
    .branch(branch.id)?
    .where(|moment| {
        let view = lineage.view(*moment)?;
        Ok(view.runtime(runtime_id)?.pending_task_count > 100)
    });
```

```rust
let transitions = lineage
    .transitions()
    .branch(branch.id)?
    .where(|transition| {
        let before = lineage.view(transition.before)?;
        let after = lineage.view(transition.after)?;
        Ok(before.runtime_count() == 1 && after.runtime_count() == 2)
    });
```

These examples reflect the intended final shape.

## Runtime versus topology versus history factoring

One lingering concern repeatedly raised by the user was that `runtime/world/` still feels broad.

The current position after cleanup is:

- `world/` still makes sense as the place for:
  - world operations
  - history
  - queries
  - topology integration
  - observations

but:

- `topology/` is now correctly split out under `world/topology/`
- `event.rs`, `transition.rs`, and `view.rs` are separate
- `history.rs` and `access.rs` are separate

This is an improvement, though further reorganization may still become attractive later.

The user explicitly floated whether:

- topology should be a sibling of world
- trace/snapshot/checkpoint should be grouped elsewhere

This has not been fully reworked yet.

The current belief is that the present factoring is acceptable enough to proceed.

## Why `Moment` lives in `world/` and `WorldInstant` lives in `time/`

This exact question was asked explicitly.

The current conceptual answer is:

- `Moment` is a history coordinate in the replayable world model
- `WorldInstant` is a time coordinate in the time subsystem

So:

- `Moment` belongs with world/history
- `Instant` or `WorldInstant` belongs with time

The naming may still be revisited, but the conceptual separation is correct.

## Final note on runtime tracing versus telemetry tracing

This was discussed several times because of the overloaded word “trace”.

The final current understanding is:

- causal `Trace` is the authoritative replay witness
- telemetry tracing, spans, metrics, and logs are all `Observation`
- OpenTelemetry-style spans should be represented internally as observations, not as the runtime’s causal trace
- exporters can map observations to OpenTelemetry

This should remain the guiding rule.

## What to do next

The immediate next steps after this handoff should be:

1. Finish the runtime cleanup that is still in the working tree.

2. Specifically do:
   - decide `WorldInstant` vs `Instant`
   - clean remaining `super::...` references
   - review whether the `world/` vs `history/` vs `topology/` vs `process/` vs `trace/` split should be committed as-is

3. Re-run:
   - `cargo fmt -p destack_runtime`
   - `CARGO_INCREMENTAL=0 cargo check -p destack_runtime`
   - likely world and trace tests

4. Review whether the runtime cleanup diff is coherent enough to commit.

5. Then begin the next major architectural pass:
   - backend-neutral heap payload redesign
   - byte and layout based managed payload storage
   - preserving shared `Heap`, `HeapImage`, `ManagedReference`, `RawPointer`, CoW, and DST semantics

## Concrete warnings for the next session

- Do not forget that the current heap is still too VM-shaped in payload representation.

- Do not accidentally regress the current exact limit and mutation-boundary discipline in the heap.

- Do not reintroduce `WorldCommand` / `WorldInvocation` style duplication.

- Do not let telemetry tracing bleed into causal `Trace`.

- Do not let `Checkpoint` and `Snapshot` drift back into looking foundational in the docs.

- Do not create separate native and VM managed heaps.

- Do not accidentally drift back toward `runtime/world` as one dumping ground after the new split.

- Do not use `crate::...` or `super::...` references lazily inside functions or where better imports should exist, because the user explicitly hates this and AGENTS requires better hygiene.

## Quick index of touched high-signal areas

Heap side:

- `language/heap/src/heap/`
- `language/heap/src/managed/`
- `language/heap/src/raw/`
- `language/heap/src/page/`
- `language/heap/src/value/`
- `language/workspace/src/config/runtime/heap.rs`
- `language/mir/src/metadata/layout.rs`

Runtime side:

- `language/runtime/src/runtime/world/`
- `language/runtime/src/runtime/trace/`
- `language/runtime/src/runtime/process/`
- `language/runtime/src/runtime/engine/`
- `language/runtime/src/runtime/tests/`
- `language/runtime/src/platform/runtime/`
- `language/runtime/README.md`
- `RUNTIME_PLAN.md`

## Final state assessment

Heap:

- ready and checked in as a major step
- coherent enough
- next major step is byte/layout payload redesign

Runtime world/replay:

- major noun and factoring cleanup done
- current working tree still contains additional cleanup not yet committed
- next immediate work is finishing the remaining runtime cleanup items listed above

Overall:

- the project is in a much better place than at the beginning of this session
- the principal architectural directions are clearer
- the next session should not need to rediscover the model from scratch

## End

If resuming from this handoff, start by checking the current unstaged runtime diff against the “Current unstaged runtime diff” section above, then continue with the “What to do next” list.
