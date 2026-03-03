use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use destack_heap as heap;
use serde::{Deserialize, Serialize};

use crate::diagnostic::{AgentDiagnosticStore, RuntimeResult};
use crate::host::{Host, HostEventKind};
use crate::platform::state::PlatformState;
use crate::platform::{PlatformContext, ResourceId, ResourceTable};
use crate::runtime::bindings::{BindingPolicy, BindingRegistry, BindingReplayPayload};
use crate::runtime::engine::EngineContinuation;
use crate::runtime::memory::Heap;
use crate::runtime::policy::{Policy, PolicyIdentity};
use crate::runtime::poller::PollerToken;
use crate::runtime::random::Random;
use crate::runtime::replay::{ReplayController, ReplayHeader};
use crate::runtime::scheduler::{EventLoop, EventLoopWatch};
use crate::runtime::snapshot::SnapshotStore;
use crate::runtime::time::{Clock, HostClockSource};
use crate::runtime::world::World;
use crate::runtime::{Hooks, RuntimeFinalizers};
use destack_workspace::{
    ExecutionMode, PlatformAudioOptions, PlatformCryptoOptions, PlatformDebugOptions,
    PlatformDeviceOptions, PlatformDisplayOptions, PlatformErrorOptions, PlatformFfiOptions,
    PlatformFsOptions, PlatformGpuOptions, PlatformInputOptions, PlatformIoOptions,
    PlatformIpcOptions, PlatformMemoryOptions, PlatformNetOptions, PlatformOptions,
    PlatformOsOptions, PlatformProcessOptions, PlatformResourceOptions, PlatformSecurityOptions,
    PlatformThreadOptions, PlatformTlsOptions, PlatformTtyOptions, RandomMode, ReplayOptions,
    ReplayPayloadMode, RuntimeOptions, TimeMode,
};

/// Number of bytes in a megabyte for replay chunk sizing.
const BYTES_PER_MB: u64 = 1024 * 1024;
/// Global agent-state id sequence for stable per-agent identity.
static NEXT_AGENT_INSTANCE_ID: AtomicU64 = AtomicU64::new(1);

/// Resolved module runtime options for the current compile target.
#[derive(Debug, Clone)]
pub struct ResolvedModuleOptions {
    /// Filesystem module options for this runtime target.
    pub fs: PlatformFsOptions,
    /// Network module options for this runtime target.
    pub net: PlatformNetOptions,
    /// Process module options for this runtime target.
    pub process: PlatformProcessOptions,
    /// Audio module options for this runtime target.
    pub audio: PlatformAudioOptions,
    /// Input module options for this runtime target.
    pub input: PlatformInputOptions,
    /// GPU module options for this runtime target.
    pub gpu: PlatformGpuOptions,
    /// TLS module options for this runtime target.
    pub tls: PlatformTlsOptions,
    /// Security module options for this runtime target.
    pub security: PlatformSecurityOptions,
    /// OS service module options for this runtime target.
    pub os: PlatformOsOptions,
    /// Device service module options for this runtime target.
    pub device: PlatformDeviceOptions,
    /// Crypto module options for this runtime target.
    pub crypto: PlatformCryptoOptions,
    /// Debug module options for this runtime target.
    pub debug: PlatformDebugOptions,
    /// Display module options for this runtime target.
    pub display: PlatformDisplayOptions,
    /// Error module options for this runtime target.
    pub error: PlatformErrorOptions,
    /// FFI module options for this runtime target.
    pub ffi: PlatformFfiOptions,
    /// I/O module options for this runtime target.
    pub io: PlatformIoOptions,
    /// IPC module options for this runtime target.
    pub ipc: PlatformIpcOptions,
    /// Memory module options for this runtime target.
    pub memory: PlatformMemoryOptions,
    /// Resource module options for this runtime target.
    pub resource: PlatformResourceOptions,
    /// Thread module options for this runtime target.
    pub thread: PlatformThreadOptions,
    /// TTY module options for this runtime target.
    pub tty: PlatformTtyOptions,
}

/// Stable identifier for one runtime-managed agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct AgentId(pub u64);

/// Primary agent lane for executing Destack programs.
pub struct Agent {
    /// Monotonic process-local agent identity.
    pub id: AgentId,
    /// Monotonic process-local agent state identity.
    pub instance_id: u64,
    /// Stable agent name for selector matching.
    pub name: String,
    /// Agent labels for selector matching.
    pub labels: BTreeMap<String, String>,
    /// Platform context for host integrations.
    pub platform: PlatformContext,
    /// Host integration state.
    pub host: Host,
    /// Shared deterministic world for policy, simulation, clock, and randomness.
    pub world: Arc<World>,
    /// Immutable runtime options.
    pub options: RuntimeOptions,
    /// Platform-specific runtime configuration options.
    pub platform_options: PlatformOptions,
    /// Resolved module options for this compile target.
    pub module_options: ResolvedModuleOptions,

    /// External resource table and finalizers.
    pub resources: ResourceTable,
    /// Agent hooks and effect state.
    pub hooks: Arc<Hooks>,
    /// Agent-level finalizer registry for module services.
    pub finalizers: RuntimeFinalizers,
    /// Agent-owned platform module state store.
    pub platform_state: PlatformState,
    /// Agent diagnostics storage for runtime errors and warning events.
    pub diagnostic: Arc<AgentDiagnosticStore>,
    /// External binding registry and policy enforcement.
    pub bindings: BindingRegistry,
    /// Managed heap and GC coordination.
    pub heap: Heap,
    /// Event loop for tasks, microtasks, and timers.
    pub event_loop: Box<EventLoop>,
}

impl std::fmt::Debug for Agent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Agent")
            .field("agent_id", &self.id)
            .field("name", &self.name)
            .field("labels", &self.labels)
            .field("platform", &self.platform)
            .field("options", &self.options)
            .field("platform_options", &self.platform_options)
            .field("module_options", &self.module_options)
            .field("resources", &self.resources)
            .field("hooks", &self.hooks)
            .field("finalizers", &self.finalizers)
            .field("platform_state", &self.platform_state)
            .field("host", &self.host)
            .field("world", &self.world)
            .field("diagnostic", &self.diagnostic)
            .field("bindings", &self.bindings)
            .field("heap", &self.heap)
            .field("event_loop", &self.event_loop)
            .finish()
    }
}

impl Agent {
    /// Create one agent with explicit runtime options.
    pub fn from_options(
        platform: PlatformContext,
        options: &RuntimeOptions,
    ) -> RuntimeResult<Self> {
        Self::from_options_internal(platform, options, None, None)
    }

    /// Create one agent with explicit runtime options in one shared world.
    #[allow(clippy::arc_with_non_send_sync)]
    pub fn from_options_in_world(
        platform: PlatformContext,
        options: &RuntimeOptions,
        world: Arc<World>,
    ) -> RuntimeResult<Self> {
        Self::from_options_internal(platform, options, None, Some(world))
    }

    /// Create one agent with explicit runtime options and host clock source.
    #[cfg(test)]
    pub(crate) fn from_options_with_host_clock_source(
        platform: PlatformContext,
        options: &RuntimeOptions,
        host_clock_source: Arc<dyn HostClockSource>,
    ) -> RuntimeResult<Self> {
        Self::from_options_internal(platform, options, Some(host_clock_source), None)
    }

    /// Create one agent from runtime options and optional world or host clock overrides.
    fn from_options_internal(
        platform: PlatformContext,
        options: &RuntimeOptions,
        host_clock_source: Option<Arc<dyn HostClockSource>>,
        world_override: Option<Arc<World>>,
    ) -> RuntimeResult<Self> {
        // runtime identity
        let runtime_name = options
            .name
            .clone()
            .unwrap_or_else(|| "runtime".to_string());
        let runtime_labels = options.labels.clone();

        // shared world
        let world = match world_override {
            Some(world) => world,
            None => {
                let replay_header = Self::replay_header_from_runtime_options(options);
                let is_replay = options.execution == ExecutionMode::Replay;
                let time_mode = if is_replay {
                    TimeMode::Virtual
                } else {
                    options.time.mode
                };
                let random_mode = if is_replay {
                    RandomMode::Deterministic
                } else {
                    options.random.mode
                };
                let replay_payload = if is_replay {
                    replay_header.replay_payload
                } else {
                    Self::resolved_replay_payload_from_options(options)
                };

                let clock = if let Some(host_clock_source) = host_clock_source {
                    Clock::from_mode_and_options_with_host_clock_source(
                        time_mode,
                        &options.time,
                        host_clock_source,
                    )
                } else {
                    Clock::from_mode_and_options(time_mode, &options.time)
                };
                let random = Random::new(options.random.seed.unwrap_or(0), random_mode);
                let policy = Policy::from_workspace_policy_rules(&options.rules);
                policy.validate()?;
                let replay =
                    ReplayController::new(options.execution, replay_payload, replay_header);

                Arc::new(World::for_runtime(policy, clock, random, replay))
            }
        };
        let agent_id = world.allocate_agent_id();

        // agent identity
        let agent_name = options
            .primary_agent
            .name
            .clone()
            .unwrap_or_else(|| format!("agent-{}", agent_id.0));
        let agent_labels = options.primary_agent.labels.clone();

        // hooks and resources
        let hooks = Arc::new(Hooks::new(
            world.clone(),
            agent_id,
            PolicyIdentity {
                runtime_name: runtime_name.clone(),
                runtime_labels: runtime_labels.clone(),
                agent_name: agent_name.clone(),
                agent_labels: agent_labels.clone(),
            },
            world.replay().mode(),
        ));
        let resources = ResourceTable::default();
        resources.set_hooks(hooks.clone());

        // bindings, heap, event loop
        let mut bindings = BindingRegistry::new();
        bindings.set_policy(BindingPolicy::new(world.replay().mode()));
        bindings.install_native_defaults();
        bindings.apply_runtime_defaults(options);

        let mut heap = Heap::default();
        heap.configure_gc(options.gc.clone());

        let mut event_loop = Box::new(EventLoop::default());
        event_loop.configure(options.scheduler.clone())?;

        // agent state
        Ok(Self {
            id: agent_id,
            instance_id: NEXT_AGENT_INSTANCE_ID.fetch_add(1, Ordering::Relaxed),
            name: agent_name,
            labels: agent_labels,
            platform,
            options: options.clone(),
            platform_options: options.platform.clone(),
            module_options: ResolvedModuleOptions::from_runtime_options(options),
            resources,
            hooks,
            finalizers: RuntimeFinalizers::default(),
            platform_state: PlatformState::default(),
            host: Host::from_runtime_options(options),
            world,
            diagnostic: Arc::new(AgentDiagnosticStore::from_options(&options.diagnostic)),
            bindings,
            heap,
            event_loop,
        })
    }

    /// Return one replay payload policy resolved from runtime options.
    fn resolved_replay_payload_from_options(options: &RuntimeOptions) -> BindingReplayPayload {
        match options.replay.payload {
            ReplayPayloadMode::ResultsOnly => BindingReplayPayload::Results,
            ReplayPayloadMode::ArgumentsAndResults => BindingReplayPayload::ArgumentsAndResults,
        }
    }

    /// Return one replay header synthesized from runtime options.
    fn replay_header_from_runtime_options(options: &RuntimeOptions) -> ReplayHeader {
        // start from the default header
        let replay_payload = Self::resolved_replay_payload_from_options(options);
        let mut header = ReplayHeader {
            execution_mode: options.execution,
            replay_payload,
            ..ReplayHeader::default()
        };

        // apply replay chunk sizing
        Self::apply_replay_overrides(&options.replay, &mut header);

        header
    }

    /// Apply replay chunk sizing overrides to one replay header.
    fn apply_replay_overrides(options: &ReplayOptions, header: &mut ReplayHeader) {
        // update chunk sizing from runtime options
        if let Some(chunk_size_mb) = options.chunk_size_mb {
            let chunk_bytes = chunk_size_mb.saturating_mul(BYTES_PER_MB);
            if chunk_bytes > 0 {
                header.max_chunk_bytes = chunk_bytes;
            }
        }
    }

    /// Borrow host integration.
    pub fn host(&self) -> &Host {
        &self.host
    }

    /// Return the callback agent id used by native host callback routing.
    pub fn host_callback_agent_id(&self) -> Option<u64> {
        self.host.callback_runtime_id()
    }

    /// Borrow the shared world attached to this agent.
    pub fn world(&self) -> &World {
        self.world.as_ref()
    }

    /// Register one timer watch.
    pub fn watch_timer(
        &mut self,
        handle: ResourceId,
        runnable: EngineContinuation,
        resume_value: heap::Value,
        priority: u8,
    ) -> RuntimeResult<()> {
        let watch = EventLoopWatch {
            runnable,
            resume_value,
            priority,
        };
        self.event_loop.watch_timer(handle, watch)
    }

    /// Remove the timer watch registered for one timer handle.
    pub fn unwatch_timer(&mut self, handle: ResourceId) -> Option<EventLoopWatch> {
        self.event_loop.unwatch_timer(handle)
    }

    /// Register one event watch.
    pub fn watch_event(
        &mut self,
        token: PollerToken,
        runnable: EngineContinuation,
        resume_value: heap::Value,
        priority: u8,
    ) -> RuntimeResult<()> {
        let watch = EventLoopWatch {
            runnable,
            resume_value,
            priority,
        };
        self.event_loop.watch_event(token, watch)
    }

    /// Remove the event watch registered for one poller token.
    pub fn unwatch_event(&mut self, token: PollerToken) -> Option<EventLoopWatch> {
        self.event_loop.unwatch_event(token)
    }

    /// Register one host semantic event watch.
    pub fn watch_host_event(
        &mut self,
        kind: HostEventKind,
        runnable: EngineContinuation,
        resume_value: heap::Value,
        priority: u8,
    ) -> RuntimeResult<()> {
        let watch = EventLoopWatch {
            runnable,
            resume_value,
            priority,
        };
        self.event_loop.watch_host_event(kind, watch)
    }

    /// Remove the host event watch registered for one host event kind.
    pub fn unwatch_host_event(&mut self, kind: HostEventKind) -> Option<EventLoopWatch> {
        self.event_loop.unwatch_host_event(kind)
    }

    /// Return the number of dropped events with no registered dispatch watch.
    pub fn dropped_unwatched_dispatch_events(&self) -> u64 {
        self.event_loop.dropped_unwatched_dispatch_events()
    }

    /// Return the number of dropped host queue events due to queue pressure.
    pub fn dropped_host_queue_events(&self) -> u64 {
        self.event_loop.dropped_host_queue_events()
    }

    /// Return the number of dropped dispatch events observed by the event loop.
    pub fn dropped_dispatch_events(&self) -> u64 {
        self.event_loop.dropped_dispatch_events()
    }

    /// Capture a runtime snapshot and record a checkpoint in the replay log.
    pub fn snapshot(&mut self, store: &SnapshotStore) -> RuntimeResult<()> {
        // allocate a new checkpoint id
        let checkpoint_id = store.allocate_checkpoint_id();

        // capture replay metadata
        let branch_id = self.world.replay().log().branch_id();
        let sequence = self.world.replay().log().next_sequence();

        // NOTE #Incomplete: snapshot payload capture is not implemented yet
        let payload = Vec::new();

        // write snapshot payload and register in the replay log
        let metadata = store.write_snapshot(checkpoint_id, branch_id, sequence, &payload)?;
        self.world
            .replay()
            .log()
            .record_checkpoint(metadata.into_checkpoint_index())?;

        Ok(())
    }
}

impl Default for Agent {
    fn default() -> Self {
        let options = RuntimeOptions::default();
        match Self::from_options(PlatformContext::new(Vec::new()), &options) {
            Ok(agent) => agent,
            Err(error) => {
                panic!("default runtime options should build an agent: {error:?}");
            }
        }
    }
}

impl Drop for Agent {
    /// Run agent-level finalizers when agent state is released.
    fn drop(&mut self) {
        self.finalizers.run_all();
    }
}

impl ResolvedModuleOptions {
    /// Resolve global module options for the current compile target.
    fn from_runtime_options(options: &RuntimeOptions) -> Self {
        Self {
            fs: options.fs.clone(),
            net: options.net.clone(),
            process: options.process.clone(),
            audio: options.audio.clone(),
            input: options.input.clone(),
            gpu: options.gpu.clone(),
            tls: options.tls.clone(),
            security: options.security.clone(),
            os: options.os.clone(),
            device: options.device.clone(),
            crypto: options.crypto.clone(),
            debug: options.debug.clone(),
            display: options.display.clone(),
            error: options.error.clone(),
            ffi: options.ffi.clone(),
            io: options.io.clone(),
            ipc: options.ipc.clone(),
            memory: options.memory.clone(),
            resource: options.resource.clone(),
            thread: options.thread.clone(),
            tty: options.tty.clone(),
        }
    }
}
