# Runtime Host And State Refactor Plan

This document captures the current architectural audit and the target refactor plan for the native runtime host boundary.
The goal is to make the runtime less fussy, more explicit about ownership and threading, and easier to extend across iOS, macOS, Android, Windows, and Linux.

## Goals

The refactor should:
- keep one shared semantic host ABI
- keep one mechanical host generator pipeline
- split host shells by actual OS application model
- separate process-global native services from runtime state and resource state
- make async completions and resource event streams explicit
- remove mixed-layer platform state objects that currently own too much
- centralize runtime policy constants instead of scattering magic timing and queue values across backend files

The refactor should not:
- force identical shell thickness across all platforms
- move plain syscall-shaped capability logic into host shells
- treat shell as a dumping ground for all platform code
- preserve current layering just because it already exists

## Prior Art Direction

The architecture should follow a blend of prior-art patterns:
- Flutter and Chromium for engine plus embedder layering
- React Native and Expo for typed generated native module boundaries
- Godot and Unity for stable native/plugin edges where external ABI is required
- Electron mostly as a warning about central coordinator bloat

The key prior-art lesson is:
- keep one shared semantic core
- put OS lifecycle, callbacks, activation, and thread ownership into per-platform shells
- keep native ABI stable only at true external boundaries

## Core Definitions

### Canonical Nouns

These are the final top-level nouns the runtime should use:
- `Shell`
- `Session`
- `Service`
- `Request`
- `Event`
- `RuntimeState`
- `ResourceState`
- `Backend`
- `Transport`

They mean:
- `Shell`: the per-OS boundary where the OS or framework is in charge
- `Session`: one runtime attached to one shell
- `Service`: one process-global native service with explicit execution policy
- `Request`: one runtime-to-host operation
- `Event`: one host-to-runtime semantic event
- `RuntimeState`: pure semantic runtime-owned subsystem state
- `ResourceState`: one long-lived stream, watch, handle, or session state
- `Backend`: one low-level implementation engine such as IOCP, io_uring, XInput, CoreAudio, WASAPI, or ALSA
- `Transport`: one ABI-safe generated carrier used only at external boundaries

These nouns should be sufficient.
We should avoid introducing more architectural nouns unless we discover a missing concept.

### Host ABI

`host/abi` is the semantic source of truth.
It defines requests, events, results, optionals, tagged payloads, and host support metadata.
It does not define lifecycle glue, callback registration, thread rules, or ABI layout hacks.

### Host Shell

A shell is the per-OS subsystem that owns the boundary between runtime semantics and the OS application model.
The shell owns only the parts where the OS or framework is in charge.

A shell owns:
- native lifecycle and activation entrypoints
- native callback registration
- process-global host-thread or event-loop services
- session attach and detach
- request mediation when native APIs are lifecycle-bound, callbacked, or stateful
- translation from native callbacks into semantic `HostEvent`s

A shell does not own:
- plain syscalls
- straightforward synchronous capability logic
- semantic ABI definitions
- generic ABI lowering
- pure runtime state

### Session

A session binds one runtime to one shell.
It owns:
- session routing identity
- host queue
- request ids
- request context
- session capability view
- session-scoped ingress delivery

The current `Session` concept is fundamentally correct and should be retained.

### Host Service

A host service is a process-global native service with explicit execution policy and affinity.
Examples:
- AppKit main-thread service
- Win32 message-loop service
- notification delegate service
- background scheduler service
- XInput poll service

### Resource State

Resource state is per-handle or per-watch state.
Examples:
- text input session
- location watch
- display subscription
- audio stream
- background execution

### Platform Module

`platform/*` owns semantic capability logic, runtime state, and resource state.
It should not own ambient callback registration or shell bootstrap.

## Scope Lattice

The runtime should explicitly revolve around these scopes:

### Process

Process-global scope owns:
- native callback registrations
- framework delegates
- message loops
- main-thread services
- scheduler hooks
- process-global service registries

### World

World scope owns:
- runtime orchestration
- replay and determinism boundaries
- shared runtime infrastructure

### Runtime

Runtime scope owns:
- host attachment
- request ids
- runtime policy and declaration context
- runtime-scoped event handling

### Agent

Agent scope owns:
- interaction-facing platform state
- display, input, and audio state that is conceptually tied to one live agent
- agent finalizers

### Session

Session scope owns:
- one runtime attached to one host shell
- session queue
- session routing
- session-scoped host capabilities

### Resource

Resource scope owns:
- per-handle lifecycle
- per-resource event queues
- per-resource state machines
- finalization

There should not be a separate fuzzy "host scope".
For one compiled host target, host-global is just process-global.

## Target Layout

The target layout should be:

- `language/runtime/src/host/abi`
- `language/runtime/src/generate/host`
- `language/runtime/src/host/apple`
- `language/runtime/src/host/ios`
- `language/runtime/src/host/macos`
- `language/runtime/src/host/android`
- `language/runtime/src/host/windows`
- `language/runtime/src/host/linux`
- `language/runtime/abi/src/ios`
- `language/runtime/abi/src/macos`
- `language/runtime/abi/src/android`
- `language/runtime/abi/src/windows`
- `language/runtime/abi/src/linux`

The meaning of these directories should be:

- `host/apple`: shared Apple support only
- `host/ios`: iOS shell
- `host/macos`: macOS shell
- `host/android`: Android shell
- `host/windows`: Windows shell
- `host/linux`: thin Linux shell

The shared Apple emitter may remain under `generate/host/emit/apple`, but it should target both iOS and macOS explicitly rather than treating Apple as one shell.

## Full Target Map

The architecture should be mapped explicitly like this.

### Current host root layout

After the current slice-1 cleanup, the internal host tree should read like this:

- `language/runtime/src/host/mod.rs`: noun-only host root
- `language/runtime/src/host/core/session.rs`: `Session`
- `language/runtime/src/host/core/*`: internal host machinery such as driver, queue, registry, request, event, status, target selection, and session wiring
- `language/runtime/src/host/os.rs`: compile-target OS family switchboard
- `language/runtime/src/host/unsupported.rs`: unsupported compile-target fallback
- `language/runtime/src/host/<os>/*`: concrete OS-family host implementations

This is the intended current shape until the shell-family rewrite lands.

### Semantic and generation layer

- `language/runtime/src/host/abi`: semantic host truth and support matrix
- `language/runtime/src/generate/host`: mechanical lowering from semantic ABI into platform-specific transport and bridge code

### Internal shell layer

- `language/runtime/src/host/ios`: iOS shell
- `language/runtime/src/host/macos`: macOS shell
- `language/runtime/src/host/android`: Android shell
- `language/runtime/src/host/windows`: Windows shell
- `language/runtime/src/host/linux`: thin Linux shell
- `language/runtime/src/host/apple`: shared Apple support only

### Public ABI facade layer

- `language/runtime/abi/src/ios`: public iOS ABI facade
- `language/runtime/abi/src/macos`: public macOS ABI facade
- `language/runtime/abi/src/android`: public Android ABI facade
- `language/runtime/abi/src/windows`: public Windows ABI facade
- `language/runtime/abi/src/linux`: public Linux ABI facade

### Runtime and subsystem layer

- `language/runtime/src/runtime/process/service/*`: generic process-service and executor substrate
- `language/runtime/src/platform/*`: semantic capability logic, runtime state, resource state, and backend integration

### Shell-owned responsibilities

The shells should own:
- lifecycle and activation ingress
- native callback registration
- host-thread and loop mediation
- shell-scoped request mediation
- translation from native callbacks into semantic events

### Platform-owned responsibilities

The platform modules should own:
- semantic capability logic
- runtime state
- resource state
- direct syscall-shaped operations
- backend integration that does not require shell mediation

## Platform Coverage Strategy

The shell strategy should be:

- iOS: full shell
- macOS: full shell
- Android: full shell
- Windows: full shell
- Linux: thin shell

The reason Linux should be thinner is that Linux desktop integration is fragmented across Wayland, X11, DBus, portals, and service managers.
Linux should not be forced into a thick shell just for symmetry.

## Support Truth

The runtime needs three distinct truths:

### Semantic truth

This lives in `host/abi`.
It answers what a capability means.

### Host support truth

This should also live in authored host ABI metadata.
It answers which host shells project which modules, requests, and ingress lanes.

### Runtime availability truth

This lives below the ABI layer in host policy and platform logic.
It answers whether a capability is actually usable on this device, on this OS version, with these declarations and permissions.

The support matrix should be explicit in authored host ABI metadata and should drive generation.
The support matrix should not be inferred from shell code or name heuristics.

## Control Flow

There are only four important flow families.

### 1. Immediate request

Flow:
- runtime or platform binding
- semantic `HostRequest`
- `Session::submit`
- shell mediation if needed
- direct platform logic
- immediate result

Use this for synchronous or straightforward host calls.

### 2. Deferred one-shot request completion

Flow:
- runtime submits request
- shell accepts request
- OS completes later
- shell ingress publishes `RequestCompleted { request_id, result }`
- runtime resolves the pending request transaction

Use this for permission prompts, document pickers, notification permission prompts, and other one-shot async operations.

### 3. Opened resource stream

Flow:
- runtime opens one resource
- resource handle is returned immediately
- later native updates arrive through shell ingress
- runtime routes them to that resource queue
- caller reads until close

Use this for text sessions, location watches, display subscriptions, background execution handles, audio streams, and similar long-lived resources.

### 4. Ambient host ingress

Flow:
- OS callback or activation enters shell ingress
- shell marshals onto required host service or loop if needed
- shell translates to semantic `HostEvent`
- session queue receives the event
- runtime, agent, or resource state consumes it

Use this for lifecycle, activation, intents, notifications, background launch, and other OS-owned callbacks.

## Examples Of Final Control Flow

The abstract flow families above should map to concrete subsystem behavior like this.

### Text input on one main-thread shell

For text input on iOS or macOS:
- runtime opens one text resource through `Session::submit`
- shell mediates because the OS owns focus and text editing lifecycle
- resource handle returns immediately
- shell registers or attaches native text callbacks on the main thread through one process-global service
- native text updates arrive through shell ingress
- shell publishes semantic text events into the session queue
- runtime drains those events and routes them to the text `ResourceState`
- resource close tears down the native session through the shell

The important point is:
- main-thread callback registration belongs to shell plus service
- text session contents and queues belong to `ResourceState`
- platform input state should not also own shell bootstrap and callback replay

### Background task launch

For background tasks on iOS, macOS, Android, or Windows:
- runtime submits one background registration or scheduling request through the session
- shell mediates when native scheduler or lifecycle integration is required
- process-global background service owns native registration and callback threading
- when the OS launches or wakes the app, shell ingress receives the callback first
- shell translates launch metadata into semantic background events
- session queue receives those events
- runtime routes them to background runtime or resource state

The important point is:
- native launch entrypoints belong to the shell
- scheduler registration belongs to one process-global service
- semantic execution bookkeeping belongs to background runtime or resource state

### Audio monitor and streams

For audio:
- runtime or agent accesses audio state directly
- `PlatformAudioState` lazily resolves one process-global audio monitor service
- runtime-owned `AudioRuntimeState` tracks semantic stream and event state
- backend details such as WASAPI, ASIO, CoreAudio, or ALSA remain backend concerns

The important point is:
- audio already mostly follows the target model
- `AudioMonitorService` is one `Service`
- `AudioRuntimeState` is one `RuntimeState`
- backend poll timing and queue policy still need cleanup

### Network helper state

For network helpers such as Windows UDS runtime state or macOS route state:
- runtime owns helper state directly
- no shell mediation is required unless native lifecycle or callback ownership appears

The important point is:
- `platform/net` is mostly `RuntimeState` plus backend helpers
- it is not one of the main shell-boundary problems

### Proactor and I/O completion

For IOCP, io_uring, or Unix fallback proactors:
- runtime opens one completion resource
- resource owns one concrete proactor backend instance
- runtime submits I/O operations directly to that backend
- completions are polled from the backend and merged into one completion queue

The important point is:
- proactor is `Backend` plus `ResourceState`
- it is not a shell concept
- it should stay out of the host-shell refactor except where shell-owned native ingress must feed into it

## Current To Target Mapping

The current runtime can be mapped onto the target nouns like this.

### Session

- `host/core/session.rs`: current seed of the final `Session` concept
- `host/core/request.rs`: request ids and session context
- `host/core/queue.rs`: session event queue mechanics
- `host/core/registry.rs`: current session registry and session routing substrate

### Shell internals

- `host/core/driver.rs`: current narrow runtime-facing shell implementation seam
- `host/windows/*`: already the clearest current shell implementation
- `host/android/*`: current shell implementation that needs to be aligned with the new model
- `host/ios/*`: current shell implementation that should become explicitly iOS rather than implicitly Apple-family
- `host/apple/*`: should become shared Apple support plus any remaining shell code that moves into `ios` and `macos`
- `host/linux/*`: current thin shell seed
- `host/macos/*`: current macOS host code should be folded into the explicit macOS shell model

### Services

- `runtime/process/service.rs`: current `GlobalService` concept, which should survive as the architecture's `Service`
- `runtime/process/service/registry.rs`: process-global typed service registry
- `runtime/process/service/executor/host.rs`: host-affine executor substrate
- `runtime/process/service/executor/periodic.rs`: low-level polling helper, not a top-level architecture concept
- display services on AppKit and Win32: healthy examples of `Service`
- `platform/audio/core/monitor.rs`: healthy example of `Service`
- `platform/input/windows/xinput.rs`: legitimate `Service`, but with suspicious policy constants

### Runtime state

- `platform/fs/state.rs`: healthy example of pure runtime state
- `platform/audio/core/runtime.rs`: healthy example of runtime state
- `platform/net/state.rs`: mostly runtime state plus backend helpers
- much of `platform/os/state/platform.rs`: should remain runtime state after decomposition
- much of `platform/input/state.rs`: should remain runtime state after decomposition

### Resource state

- text session state under `platform/input`
- location watches under `platform/os`
- display subscriptions under `platform/display`
- audio streams under `platform/audio`
- completion queues and poll handles under `platform/io`
- background execution handles under `platform/os/background`

### Backends

- `platform/proactor/windows/iocp.rs`
- `platform/proactor/unix/uring.rs`
- `platform/proactor/unix/unix.rs`
- `platform/input/windows/xinput.rs`
- `platform/audio/windows/wasapi/*`
- `platform/audio/windows/asio/*`
- `platform/audio/unix/coreaudio/*`
- `platform/audio/unix/alsa/*`
- `platform/audio/unix/opensles/*`

### Overgrown mixed bags that need decomposition

- `platform/os/state/platform.rs`
- `platform/input/state.rs`
- `platform/os/background/*` as a whole, because shell ingress and semantic state are still too interwoven
- `platform/os/notification/*` where ingress and runtime state are still not cleanly separated

## Request Return Model

The request return model should be explicit and minimal.

Requests should only return one of:
- immediate result
- accepted deferred request
- opened resource handle

Async information should only come back through:
- `RequestCompleted(request_id, result)`
- `ResourceEvent(resource_id, payload)`

This means long-lived async interactions like text keyboard or text input should be modeled as resource streams, not as ad hoc async request completions.

## Threading Model

The runtime already has a good substrate in:
- `runtime/process/service/*`
- `ExecutionPolicy`
- `ExecutionAffinity`
- `HostExecutor`

We should keep and formalize that model.

Process-global host services should declare explicit affinity such as:
- main thread
- message loop
- MTA
- polling

Platform modules should not smuggle thread-affine callback setup into their own runtime state objects.
That orchestration belongs in shell services.

The clearest examples are:
- AppKit and iOS main-thread UI or text integration: shell plus service
- Win32 display and activation message loops: shell plus service
- background scheduler callbacks: shell plus service
- direct syscalls or direct backend operations: platform logic or resource state, not shell

## Current Audit Findings

### Healthy patterns

These areas mostly align with the target design:
- `Session` as the runtime-shell boundary
- process-global display services on AppKit and Win32
- audio monitor service
- Windows XInput service concept
- pure runtime state in `platform/fs/state.rs`

### Mixed or overgrown patterns

These are the main problem areas:

#### `platform/os/state/platform.rs`

This object currently mixes:
- runtime semantic state
- event stream registries
- watch registries
- pending transactions keyed by `HostRequestId`
- host queue bootstrap state
- host event reconciliation logic

It should be split into:
- runtime semantic state
- resource registry state
- request transaction state
- shell/session-owned queue bootstrap and reconciliation

#### `platform/input/state.rs`

This object currently mixes:
- text session resource state
- host queue bootstrap
- observer registration and replay
- host finalization close plumbing
- unix and windows monitor service handles
- runtime state handles

It should be split into:
- shell/session text ingress bootstrap
- text resource state
- input monitor service state

#### background ingress and runtime state

Background logic is conceptually close to the right shape but the shell boundary is not explicit enough.
Launch markers, ingress servicing, storage, wrapper generation, and semantic background state are too spread out across layers.

### Other subsystem findings

#### `platform/audio`

`platform/audio` is comparatively healthy.
`PlatformAudioState` mostly acts as one agent-owned access point for:
- one process-global `AudioMonitorService`
- one runtime-owned `AudioRuntimeState`

The remaining audio issues are mostly:
- policy constants
- backend polling defaults
- making service ownership more explicit in naming and docs

#### `platform/net`

`platform/net` is also comparatively healthy.
It mostly contains runtime-owned helper state such as:
- `WindowsUdsRuntimeState`
- `MacosRouteRuntimeState`

This is runtime state plus backend helper state, not one shell-boundary problem.

#### `platform/proactor` and `platform/io`

`platform/proactor` and `platform/io` are not shell architecture.
They are backend and resource architecture.

The important classification is:
- IOCP, io_uring, and Unix fallback polling are `Backend`
- completion queues and poll handles are `ResourceState`

These modules should not be forced into the shell model.
They should be kept separate from the host-shell refactor.

### Process-global state sweep

The sweep found:
- service registries and host session registries are expected
- many backend statics are harmless facts or API caches
- the true architectural smell is not raw statics, but mixed-scope runtime state objects

## Constants And Policy Audit

Constants need classification into:
- fact
- limit
- policy

### Facts

These are fine near backend code:
- OS error codes
- HID usage constants
- ABI bitmasks
- fixed path names
- resource labels

### Limits

These are usually okay but should be documented:
- backend max channels
- supported sample rate ranges
- required queue depths

### Policy

These are the dangerous ones and should be centralized:
- polling intervals
- wait slices
- queue capacities
- retry timing
- default deadlines

The most suspicious current policy constants include:
- `XINPUT_POLL_INTERVAL = 4ms`
- `OS_READ_WAIT_SLICE_NS`
- `NETWORK_WATCH_SLICE_NS`
- audio event poll defaults and min or max poll intervals
- raw input queue limits where they define overflow policy
- wait timeouts in audio backends
- desktop background deadlines

The refactor should move policy constants out of backend files into central per-subsystem policy modules.
Those policies should become configurable from runtime, agent, or stream options where appropriate.

### Concrete policy review list

The following current constants should be explicitly reviewed and either justified as backend policy or moved into central subsystem policy modules.

- `platform/os/state/core.rs`: `OS_READ_WAIT_SLICE_NS`
- `platform/os/network/core.rs`: `NETWORK_WATCH_SLICE_NS`
- `platform/input/windows/xinput.rs`: `XINPUT_POLL_INTERVAL`
- `platform/input/unix/event.rs`: `INPUT_MONITOR_NATIVE_WAIT`
- `platform/input/unix/event.rs`: `INPUT_MONITOR_SYNTHETIC_INTERVAL`
- `platform/input/windows/raw.rs`: `RAW_INPUT_QUEUE_LIMIT`
- `platform/input/windows/raw.rs`: `RAW_MONITOR_QUEUE_LIMIT`
- `platform/input/windows/raw.rs`: `RAW_HID_QUEUE_LIMIT`
- `platform/input/windows/raw.rs`: `RAW_TOUCH_QUEUE_LIMIT`
- `platform/audio/core/constants.rs`: `DEFAULT_EVENT_QUEUE_CAPACITY`
- `platform/audio/core/constants.rs`: `EVENT_POLL_INTERVAL_NS`
- `platform/audio/core/constants.rs`: `MIN_EVENT_POLL_INTERVAL_NS`
- `platform/audio/core/constants.rs`: `MAX_EVENT_POLL_INTERVAL_NS`
- `platform/audio/core/constants.rs`: `MIN_WORKER_POLL_INTERVAL_NS`
- `platform/device/midi/core/queue.rs`: `DEFAULT_INPUT_QUEUE_CAPACITY`
- `platform/device/midi/core/queue.rs`: `DEFAULT_EVENT_QUEUE_CAPACITY`
- `platform/device/midi/core/queue.rs`: `DEFAULT_EVENT_POLL_INTERVAL`
- `platform/device/camera/linux/service.rs`: `CAMERA_WATCH_POLL_INTERVAL`
- `platform/device/camera/macos/service.rs`: `CAMERA_WATCH_POLL_INTERVAL`
- `platform/device/camera/android/watch.rs`: `CAMERA_WATCH_POLL_INTERVAL_NS`
- `platform/proactor/windows/iocp.rs`: `DEFAULT_PENDING_POLL_SLICE_NS`
- `host/unix/request/location/linux.rs`: `GEOCLUE_LAST_KNOWN_TIMEOUT`
- `host/unix/request/location/linux.rs`: `GEOCLUE_POLL_SLICE`
- `host/windows/request/location/service.rs`: `LOCATION_LAST_KNOWN_TIMEOUT_NS`
- `runtime/process/service/executor/periodic.rs`: `PERIODIC_IDLE_WAIT`
- `platform/core/unix/sync.rs`: `APPLE_SEMAPHORE_WAIT_SLICE`
- `host/apple/core/message.rs`: `APPLE_THREAD_MESSAGE_WAIT_SLICE_SECONDS`

The first cleanup pass should classify each of these as one of:
- backend fact
- backend limit
- subsystem policy
- shell or host-loop policy

Anything in the last two classes should move out of backend files.

### Concrete process-global state review list

The following process-global statics and registries are architecturally important and should be classified explicitly as process services, shell registries, backend caches, or test-only globals.

These are expected and likely remain:
- `runtime/process/service/registry.rs`: `SERVICE_REGISTRY`
- `host/core/registry.rs`: `HOST_SESSION_REGISTRY`
- `runtime/process/service/windows/message.rs`: `WINDOWS_LOOP_REGISTRY`

These are important native callback or backend globals that should be folded into the shell or service model more explicitly:
- `platform/input/unix/macos.rs`: `MACOS_TAP_STATE`
- `platform/input/unix/macos.rs`: `MACOS_TAP_WORKER`
- `platform/input/unix/macos.rs`: `MACOS_TAP_PORT`
- `platform/net/windows/packet.rs`: `PACKET_SOCKET_STATES`
- `platform/net/unix/packet.rs`: `PACKET_SOCKET_STATES`
- `platform/os/credentials/core.rs`: `CREDENTIAL_CREATE_GUARDS`
- `host/android/abi/registry.rs`: `ANDROID_BINDINGS_REGISTRY`
- `host/apple/abi/registry.rs`: `IOS_BINDINGS_REGISTRY`

Most test-only globals, one-time capability probes, and immutable protocol constants do not need architectural action beyond normal hygiene.

### Additional process-global state worth reviewing

The following globals are also relevant because they represent real process-global behavior or host integration, even if they may ultimately remain.

- `platform/input/windows/event.rs`: `WINDOWS_MONITOR_STREAMS`
- `platform/input/windows/core.rs`: `WINDOWS_CONSOLE_STREAMS`
- `platform/os/notification/runtime/state.rs`: `DESKTOP_NOTIFICATION_TEST_MODE`
- `platform/os/background/runtime/state.rs`: `DESKTOP_BACKGROUND_TEST_MODE`
- `platform/input/unix/macos.rs`: `MACOS_TAP_STATE`
- `platform/input/unix/macos.rs`: `MACOS_TAP_WORKER`
- `platform/input/unix/macos.rs`: `MACOS_TAP_PORT`
- `host/windows/request/notification/core.rs`: `WINDOWS_NOTIFICATION_ACTIVATOR`
- `host/windows/request/notification/core.rs`: `WINDOWS_NOTIFICATION_ACTIVATOR_FACTORY`
- `host/windows/request/notification/core.rs`: `WINDOWS_TOAST_IDENTITY`
- `host/macos/request/notification/core.rs`: macOS notification center delegate global
- `platform/device/bluetooth/macos/core.rs`: global Apple dispatch queue
- `platform/core/unix/apple/dispatch.rs`: global Apple dispatch queue

These should each be classified as one of:
- service state
- shell registry
- backend cache
- host callback singleton
- test-only global

### Concrete current state objects to decompose

The following state objects are the main structural decomposition targets.

#### `platform/os/state/platform.rs`

Current responsibilities:
- lifecycle and semantic OS state
- permission and location cache state
- event stream and watch registries
- deferred request transaction tracking
- queue bootstrap and host-event reconciliation

Target decomposition:
- `PlatformOsRuntimeState`
- `PlatformOsResourceState`
- `PlatformOsRequestState`
- shell-owned or session-owned OS bridge state

The queue bootstrap and host-event reconciliation logic should leave `platform/os` state and move toward shell plus session ownership.

#### `platform/input/state.rs`

Current responsibilities:
- text session resource state
- monitor runtime state
- lazy access to process services
- host queue bootstrap and observer replay
- host close plumbing during resource teardown

Target decomposition:
- `PlatformInputTextState`
- `PlatformInputMonitorState`
- `PlatformInputServices`
- shell-owned or session-owned input bridge state

The host queue bootstrap and observer replay logic should leave `platform/input` state and move toward shell plus session ownership.

#### `platform/os/background/*`

Current responsibilities are spread across:
- shell-like launch ingress
- process-global runtime bookkeeping
- storage and wrapper generation
- semantic background state

Target decomposition:
- shell-owned background ingress and launch integration
- one process-global background service
- runtime and resource state for semantic background execution
- backend storage and wrapper support

#### `platform/os/notification/*`

Current responsibilities are spread across:
- host ingress
- posted-notification bookkeeping
- per-session runtime state

Target decomposition:
- shell-owned notification ingress
- one notification service for process-global native integration where needed
- runtime state for posted notification bookkeeping and semantic delivery

## Full Systematic Fix

The systematic fix for the whole runtime is:

1. make the canonical nouns explicit in code and docs
2. make `Session` the only runtime-facing host entrypoint
3. keep `Service` as the process-global native integration concept
4. move shell-owned bootstrap and ingress logic out of platform state bags
5. keep backend and resource architecture separate from shell architecture
6. classify all constants into fact, limit, and policy
7. classify all globals into service, shell registry, backend cache, callback singleton, or test-only
8. centralize subsystem policy and make polling demand-driven where possible

This is the root fix.
It is not a patchwork of local cleanups.

## Services And Executors

The process-service substrate is still the right basic mechanism.
The problem is not that `GlobalService` exists.
The problem is that its role is not named clearly enough, and polling policy currently leaks through helper APIs into backend modules.

### `GlobalService`

The current `GlobalService` trait is conceptually valid.
It represents one process-global native service with one execution policy.

That concept should survive.
It should gradually become the architectural `Service` noun.

The main changes should be:
- use it more explicitly for process-global native integration points
- stop letting platform state bags impersonate services
- eventually consider renaming it to `Service` or `HostService` once the architecture is cleaner

### `ServiceHandle`

`ServiceHandle<S>` is also conceptually valid.
It is one lazy typed handle for one process-global service.

This should survive.
It is useful in agent or runtime state as one access path to a process-global service.

### `HostExecutor`

`HostExecutor` is a real architectural concept.
It is how thread-affine services should marshal onto:
- main thread
- message loop
- other host-owned execution contexts

This should stay central for UI, activation, display, and similar host-thread work.

### `periodic_service_executor()`

The periodic executor should survive only as low-level plumbing.
It is useful for polling-based services, but it should not be a primary architectural noun.

The important cleanup is:
- polling services should own their own policy
- policy should not be hidden as ad hoc constants in backend files
- demand should decide whether one polling service is active

Examples that currently need this cleanup include:
- XInput polling
- audio monitor polling
- MIDI watch polling
- camera watch polling

So the intended end state is:
- `GlobalService` survives in concept as `Service`
- `HostExecutor` stays as the thread-affine service primitive
- `periodic_service_executor()` remains one implementation helper used by some services

## Shell Definition In Code

The runtime-facing shell surface should stay narrow.
The clean entrypoint should be `Session`.

That means the runtime should conceptually interact like this:
- `Shell::attach(...) -> Session`
- `Session::submit(request) -> outcome`
- `Session::poll() -> events`
- `Session::detach()`

The current `Session` idea is already close to this target.
The current `SessionDriver` idea is also useful, but it should remain a narrow internal shell implementation detail rather than the main architectural noun.

The runtime-facing session surface should do only:
- report platform
- report capabilities
- submit requests
- pump or drain ingress
- expose any needed session-local context

Shell internals do not need one giant shared trait.
They can stay concrete per OS.

## Ownership Rules

A code path belongs in the shell world if:
- the OS calls us first
- lifecycle matters
- activation matters
- callback registration matters
- host-thread affinity matters
- runtime-session routing from ambient callbacks matters

A code path belongs in `platform/*` if:
- it is a direct syscall or direct capability call
- it is synchronous or straightforward
- it owns semantic runtime state
- it owns resource state
- it does not require shell mediation

## Concrete Refactor Actions

### Add

- explicit `RequestCompleted` event for deferred one-shot request completion
- explicit shell-owned session bootstrap layer
- explicit per-subsystem policy modules for timing and queue defaults
- explicit shell or service directories under each OS host
- explicit support matrix expansion for macOS and Windows in host generation

### Remove

- reliance on vague `service_*` verbs as top-level architecture language
- platform modules owning ambient host callback bootstrap
- mixed giant state bags that combine runtime, resource, transaction, and shell bootstrap state
- the current `host/apple` shell identity

### Keep

- shared semantic host ABI
- shared generator pipeline
- `Session` as the runtime-facing host attachment
- process-global service substrate
- per-OS shell direction
- direct platform calls for syscall-shaped capability logic
- backend and resource architecture in `platform/proactor` and `platform/io`

### Refactor

- split `PlatformOsState`
- split `PlatformInputState`
- move background ingress toward shell ownership
- keep `platform/audio` mostly structurally intact, but centralize policy and clarify service ownership
- keep `platform/net` mostly structurally intact
- keep `platform/proactor` and `platform/io` separate from shell refactors
- split Apple shell into iOS and macOS shells
- retain `host/apple` for shared support only
- formalize Windows shell around the pattern it already has
- keep Linux shell thin and capability-oriented

## Proposed Phases

### Phase 1: naming and model cleanup

- write down the nouns and scopes in code comments and docs
- make `Shell`, `Session`, `Service`, `RuntimeState`, `ResourceState`, and `Backend` the canonical architecture nouns
- keep `SessionDriver` narrow and internal
- stop using fuzzy "service ingress" language at architectural boundaries

### Phase 2: async completion cleanup

- add explicit deferred request completion events
- classify host requests into immediate, deferred, and resource-opening groups
- move document and permission transaction handling onto that explicit model

### Phase 3: state decomposition

- split `PlatformOsState` into semantic runtime state, resource registries, and transactions
- split `PlatformInputState` into shell bootstrap, resource state, and service state
- review `platform/audio` and keep its healthier service pattern as the model
- explicitly classify `platform/net` as runtime and backend helper state
- explicitly classify `platform/proactor` and `platform/io` as backend and resource architecture

### Phase 4: shell and service extraction

- create `host/ios`, `host/macos`, `host/android`, `host/windows`, and `host/linux` shell directories
- move process-global native services under those OS host trees where appropriate
- keep `host/apple` only for shared support code

### Phase 5: generator support expansion

- extend host generator platform model to include macOS and Windows
- keep shared ABI semantic truth
- generate only from explicit support metadata
- keep Apple-family emitter backend shared, but target iOS and macOS separately

### Phase 6: policy centralization

- classify constants into fact, limit, and policy
- centralize policy constants by subsystem
- make polling and queue defaults demand-driven or configurable where appropriate
- classify process-global statics into service, shell registry, backend cache, or test-only buckets
- fold callback-heavy globals into explicit shell or service ownership where possible

## Immediate Concrete Follow-Up Tasks

The next practical tasks should be:

1. write the exact target decomposition for `PlatformOsState`
2. write the exact target decomposition for `PlatformInputState`
3. classify remaining service-backed polling modules into fact, limit, and policy
4. write one noun-to-current-code mapping table for runtime comments and docs
5. begin the changeset 2 ownership rewrite against the now-stable `Session` and `Service` model

## Landing Plan

The refactor should land in three large changesets.
Each changeset should be coherent enough to review on its own and complete enough to move the architecture materially.
These changesets should avoid transitional naming and should move directly toward the final model.

### Changeset 1: host core rewrite

Goal:
- make `Session` the explicit runtime-facing host concept
- rewrite the host core around the final request and event model
- rename, remove, and reshape stale host-core architecture vocabulary immediately
- finalize `Service` as the process-global singleton concept and keep executor mechanics as private implementation detail
- remove the fake shared wait and polling substrate and move any unavoidable polling under concrete service ownership

Primary nouns:
- `Session`
- `Service`
- `Request`
- `Event`
- `RuntimeState`
- `ResourceState`
- `Backend`
- `Transport`

Primary files and submodules:
- `language/runtime/src/host/core/session.rs`
- `language/runtime/src/host/core/request.rs`
- `language/runtime/src/host/core/queue.rs`
- `language/runtime/src/host/core/registry.rs`
- `language/runtime/src/host/core/driver.rs`
- `language/runtime/src/host/mod.rs`
- `language/runtime/src/host/mod.rs`
- `language/runtime/src/host/abi/core/module.rs`
- `language/runtime/src/host/abi/*`
- `language/runtime/src/generate/host/model/*`
- `language/runtime/src/generate/host/collect/*`
- `language/runtime/src/generate/host/emit/*` where needed for the rewritten request or event model
- `language/runtime/abi/src/*` where needed to reflect the rewritten completion and session model
- `language/runtime/src/runtime/process/service.rs`
- `language/runtime/src/runtime/process/service/registry.rs`
- `language/runtime/src/runtime/process/service/executor/host.rs`
- `language/runtime/src/runtime/process/service/executor/periodic.rs`
- `language/runtime/src/runtime/process/policy.rs`
- `language/runtime/src/platform/audio/core/constants.rs`
- `language/runtime/src/platform/device/midi/core/queue.rs`
- `language/runtime/src/platform/os/state/core.rs`
- `language/runtime/src/platform/os/network/core.rs`
- `language/runtime/src/platform/input/windows/xinput.rs`
- `language/runtime/src/platform/input/unix/event.rs`
- `language/runtime/src/platform/input/windows/raw.rs`
- `language/runtime/src/platform/device/camera/*/service.rs`
- `language/runtime/src/platform/device/camera/android/watch.rs`
- `language/runtime/src/platform/proactor/windows/iocp.rs`
- `language/runtime/src/host/unix/request/location/linux.rs`
- `language/runtime/src/host/windows/request/location/service.rs`
- `language/runtime/src/platform/core/unix/sync.rs`
- `language/runtime/src/host/apple/core/message.rs`

Main work:
- keep `Session` as the runtime-facing noun and confine `HostSession*` naming to ABI edge handles only where needed
- remove stale top-level host-core vocabulary where it no longer matches the final architecture
- make `poll` the actual top-level session event verb
- make request outcomes explicit: immediate, deferred completion, or opened resource
- add the explicit deferred completion event shape
- make session-owned request correlation and event routing explicit
- keep any remaining trait surfaces narrow and internal only when they still earn their keep
- rename `GlobalService` into the final `Service` concept and strip execution helpers off the trait surface
- keep the host-affinity executor as private service plumbing rather than an ambient runtime concept
- remove shared polling singletons and free periodic helpers from the runtime architecture surface
- move thread, loop, and periodic spawning into concrete service-owned implementation code
- classify constants into fact, limit, and policy
- classify process-global statics into service state, shell registry, backend cache, callback singleton, or test-only
- remove unexplained cross-cutting timing and queue policy from backend files where the root behavior should instead be event or wake driven

Acceptance criteria:
- one runtime-facing `Session` concept is obvious in code and docs
- stale top-level host-core naming is removed rather than deferred
- deferred one-shot completions are explicit in the event model
- resource-opening requests are clearly distinct from deferred one-shot requests
- `poll` is the obvious top-level session event verb
- process-global native services are explicit and read as `Service` ownership rather than incidental helpers
- `Service` means the process-global singleton concept only, not a bag of static execution helpers
- executor mechanics are private plumbing owned by concrete services
- fake shared wait slices and shared polling infrastructure are removed from the public architecture model
- suspicious timing and queue constants are either removed, localized to real backend or service needs, or explicitly justified
- no backend file silently defines cross-cutting subsystem policy without being called out
- no new name heuristics or ad hoc callback conventions are introduced
- `cargo fmt --all` passes
- `cargo check -p destack_runtime` passes
- `cargo test -p destack_runtime --no-run` passes

Status:
- completed

Outcome:
- `Session` is now the runtime-facing host attachment with `poll` as the public ingress verb
- deferred request completion is explicit in the host event model
- stale host-core ingress and completion vocabulary has been removed
- `Service` is now only the process-global singleton concept
- concrete services own thread, loop, and periodic execution directly through private plumbing
- free periodic helpers, shared polling singletons, and shared wait-slice behavior have been removed from the runtime architecture surface
- `host/mod.rs` is reduced to the real host root surface, with low-level machinery under `host/core` and OS-family selection under `host/os.rs`
- `generator` and `execution` no longer shape the main runtime, host, platform, or diagnostic roots as architectural split points

### Changeset 2: ownership rewrite

Goal:
- finalize the ownership model across runtime state, resource state, services, shells, and sessions
- remove mixed ownership bags throughout the runtime
- move shell and session bridge logic out of platform state everywhere

Primary nouns:
- `RuntimeState`
- `ResourceState`
- `Session`
- `Service`
- `Backend`

Primary files and submodules:
- `language/runtime/src/platform/os/state/platform.rs`
- `language/runtime/src/platform/os/state/core.rs`
- `language/runtime/src/platform/os/state/lifecycle.rs`
- `language/runtime/src/platform/os/state/intent.rs`
- `language/runtime/src/platform/os/state/notification.rs`
- `language/runtime/src/platform/os/state/background.rs`
- `language/runtime/src/platform/os/state/location.rs`
- `language/runtime/src/platform/os/document/state.rs`
- `language/runtime/src/platform/os/permission/state.rs`
- `language/runtime/src/platform/input/state.rs`
- `language/runtime/src/platform/input/host.rs`
- `language/runtime/src/platform/input/windows/text.rs`
- `language/runtime/src/platform/input/unix/text.rs`
- `language/runtime/src/platform/os/background/*`
- `language/runtime/src/platform/os/notification/*`
- the rest of `language/runtime/src/platform/*` wherever ownership classification is still ambiguous
- `language/runtime/src/host/*` where session bridge ownership needs to move
- `language/runtime/src/host/<os>/*` where ingress bootstrap and replay move upward

Main work:
- fully decompose `PlatformOsState`
- fully decompose `PlatformInputState`
- move queue bootstrap, observer replay, ingress reconciliation, and related bridge logic out of platform state bags
- clean up background and notification organization under the same ownership rules
- review the rest of `platform/*` against the same ownership model and clean up any remaining mixed ownership bags
- keep `audio`, `net`, `proactor`, and `io` only where their current structure already matches the final ownership model

Acceptance criteria:
- no state object remains architecturally ambiguous about whether it owns shell, session, service, runtime, or resource responsibilities
- shell and session bridge logic lives with shell or session code
- resource queues and runtime state ownership are explicit
- the remaining `platform/*` modules are explicitly classified as runtime state, resource state, service access, shell bridge, or backend
- `cargo check -p destack_runtime --quiet` passes
- `cargo test -p destack_runtime --quiet` passes, or any blocked lanes are explicitly documented

### Changeset 3: shell family rewrite

Goal:
- land the entire final platform shell layout and support projection cleanly
- finish the new architecture completely, including the final shell family and public ABI facades

Primary nouns:
- `Shell`
- `Session`
- `Service`
- `Transport`

Primary files and submodules:
- `language/runtime/src/host/apple/*`
- `language/runtime/src/host/ios/*`
- `language/runtime/src/host/macos/*`
- `language/runtime/src/host/android/*`
- `language/runtime/src/host/windows/*`
- `language/runtime/src/host/linux/*`
- `language/runtime/src/generate/host/emit/apple/*`
- `language/runtime/src/generate/host/emit/android/*`
- `language/runtime/src/generate/host/emit/windows/*` where needed
- `language/runtime/src/generate/host/emit/linux/*` where needed
- `language/runtime/src/generate/host/model/*`
- `language/runtime/abi/src/ios/*`
- `language/runtime/abi/src/macos/*`
- `language/runtime/abi/src/android/*`
- `language/runtime/abi/src/windows/*`
- `language/runtime/abi/src/linux/*`

Main work:
- split Apple shell ownership into explicit iOS and macOS shells
- retain `host/apple` only for shared Apple support
- align Android, Windows, and Linux to the same shell and session model
- make public ABI facades explicit for the full shell family
- extend generator support and support-matrix projection across iOS, macOS, Android, Windows, and Linux
- remove any remaining transitional shell naming or layout

Acceptance criteria:
- `host/apple` is support only, not the shell identity
- `host/ios`, `host/macos`, `host/android`, `host/windows`, and `host/linux` are the explicit final shell homes
- public ABI facades exist and are coherent for the full host family
- generator output is driven by authored support metadata rather than shell-name folklore
- no transitional shell naming or structure remains
- `cargo check -p destack_runtime --quiet` passes
- `cargo check -p destack_runtime --features generator --quiet` passes
- host binding regeneration and diff checks are clean
4. define the shell directory split for Apple into iOS and macOS
5. audit and centralize suspicious timing and queue policy constants, starting with XInput and OS watch slices

## Summary

The runtime already contains the right primitives.
The main problem is not missing infrastructure.
The main problem is that process-global shell services, runtime-session state, resource state, and semantic platform state are currently mixed together in several key modules.

The refactor should make those boundaries explicit, keep shells small and honest, keep direct capability logic in `platform/*`, and centralize policy instead of hiding it inside backend constants.
