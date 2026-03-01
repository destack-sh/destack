use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::diagnostic::RuntimeErrorStore;
use crate::host::Host;
use crate::platform::{PlatformContext, ResourceTable};
use crate::runtime::Hooks;
use crate::runtime::bindings::BindingReplayPayload;
use crate::runtime::policy::Policy;
use crate::runtime::random::Random;
use crate::runtime::replay::{ReplayController, ReplayHeader};
use crate::runtime::time::{Clock, HostClockSource};
use crate::runtime::world::World;
use destack_workspace::{
    ExecutionMode, RandomMode, ReplayOptions, ReplayPayloadMode, RuntimeOptions, TimeMode,
};

/// Number of bytes in a megabyte for replay chunk sizing.
const BYTES_PER_MB: u64 = 1024 * 1024;
/// Global agent id sequence for stable per-agent identity.
static NEXT_AGENT_ID: AtomicU64 = AtomicU64::new(1);

/// Shared runtime state for platform bindings and execution.
#[derive(Debug)]
pub struct RuntimeContext {
    /// Monotonic process-local agent identity.
    pub agent_id: u64,
    /// Platform context for host integrations.
    pub platform: PlatformContext,
    /// Immutable runtime options.
    pub options: RuntimeOptions,
    /// External resource table and finalizers.
    pub resources: ResourceTable,
    /// Replay log and record/replay state.
    pub replay: ReplayController,
    /// Runtime hooks and effect state.
    pub hooks: Hooks,
    /// Host integration state.
    pub host: Host,
    /// Shared deterministic world for policy, simulation, clock, and randomness.
    pub world: World,
    /// Runtime error storage for native bindings.
    pub errors: RuntimeErrorStore,
}

impl RuntimeContext {
    /// Create runtime state from explicit platform context.
    pub fn new(platform: PlatformContext) -> Self {
        Self::from_options(platform, &RuntimeOptions::default())
    }

    /// Create runtime state from runtime options.
    pub fn from_options(platform: PlatformContext, options: &RuntimeOptions) -> Self {
        let header = Self::replay_header_from_runtime_options(options);
        Self::from_runtime_options_and_header_with_host_clock_source_and_world(
            platform, options, header, None, None,
        )
    }

    /// Create runtime state from runtime options and one shared world.
    pub fn from_options_in_world(
        platform: PlatformContext,
        options: &RuntimeOptions,
        world: World,
    ) -> Self {
        let header = Self::replay_header_from_runtime_options(options);
        Self::from_runtime_options_and_header_with_host_clock_source_and_world(
            platform,
            options,
            header,
            None,
            Some(world),
        )
    }

    /// Create runtime state from runtime options and one explicit host clock source.
    #[cfg(test)]
    pub(crate) fn from_options_with_host_clock_source(
        platform: PlatformContext,
        options: &RuntimeOptions,
        host_clock_source: Arc<dyn HostClockSource>,
    ) -> Self {
        let header = Self::replay_header_from_runtime_options(options);
        Self::from_runtime_options_and_header_with_host_clock_source_and_world(
            platform,
            options,
            header,
            Some(host_clock_source),
            None,
        )
    }

    /// Create runtime state from an explicit replay header.
    pub fn from_replay_header(
        platform: PlatformContext,
        mode: ExecutionMode,
        header: ReplayHeader,
    ) -> Self {
        // seed runtime options from the execution mode
        let options = RuntimeOptions {
            execution: mode,
            ..RuntimeOptions::default()
        };

        Self::from_runtime_options_and_header_with_host_clock_source_and_world(
            platform, &options, header, None, None,
        )
    }

    /// Create runtime state from runtime options and replay header.
    pub fn from_runtime_options_and_header(
        platform: PlatformContext,
        options: &RuntimeOptions,
        header: ReplayHeader,
    ) -> Self {
        Self::from_runtime_options_and_header_with_host_clock_source_and_world(
            platform, options, header, None, None,
        )
    }

    /// Create runtime state from options, header, and optional host clock source and world.
    fn from_runtime_options_and_header_with_host_clock_source_and_world(
        platform: PlatformContext,
        options: &RuntimeOptions,
        header: ReplayHeader,
        host_clock_source: Option<Arc<dyn HostClockSource>>,
        world: Option<World>,
    ) -> Self {
        // build runtime subsystems from options
        let replay_mode = options.execution == ExecutionMode::Replay;
        let resolved_time_mode = if replay_mode {
            TimeMode::Virtual
        } else {
            options.time.mode
        };
        let resolved_random_mode = if replay_mode {
            RandomMode::Deterministic
        } else {
            options.random.mode
        };

        let execution_mode = options.execution;
        let replay_payload = if replay_mode {
            header.replay_payload
        } else {
            Self::resolved_replay_payload_from_options(options)
        };
        let agent_id = NEXT_AGENT_ID.fetch_add(1, Ordering::Relaxed);
        let (world, policy) = if let Some(world) = world {
            let policy = world.policy();
            (world, policy)
        } else {
            let clock = if let Some(host_clock_source) = host_clock_source {
                Clock::from_mode_and_options_with_host_clock_source(
                    resolved_time_mode,
                    &options.time,
                    host_clock_source,
                )
            } else {
                Clock::from_mode_and_options(resolved_time_mode, &options.time)
            };
            let random = Random::new(options.random.seed.unwrap_or(0), resolved_random_mode);
            let policy = Policy::from_workspace_policy_rules(&options.rules);
            let world = World::for_runtime(policy.clone(), clock, random);
            (world, policy)
        };

        Self {
            agent_id,
            platform,
            options: options.clone(),
            resources: ResourceTable::default(),
            replay: ReplayController::new(execution_mode, replay_payload, header),
            hooks: Hooks::from_runtime_options_and_policy_in_world(
                options,
                &policy,
                world.clone(),
                agent_id,
            ),
            host: Host::from_runtime_options(options),
            world,
            errors: RuntimeErrorStore::default(),
        }
    }

    fn resolved_replay_payload_from_options(options: &RuntimeOptions) -> BindingReplayPayload {
        match options.replay.payload {
            ReplayPayloadMode::ResultsOnly => BindingReplayPayload::Results,
            ReplayPayloadMode::ArgumentsAndResults => BindingReplayPayload::ArgumentsAndResults,
        }
    }

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

    fn apply_replay_overrides(options: &ReplayOptions, header: &mut ReplayHeader) {
        // update chunk sizing from runtime options
        if let Some(chunk_size_mb) = options.chunk_size_mb {
            let chunk_bytes = chunk_size_mb.saturating_mul(BYTES_PER_MB);
            if chunk_bytes > 0 {
                header.max_chunk_bytes = chunk_bytes;
            }
        }
    }
}
