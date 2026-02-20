use crate::diagnostic::RuntimeErrorStore;
use crate::platform::{PlatformContext, ResourceTable};
use crate::runtime::RuntimeHooks;
use crate::runtime::bindings::BindingReplayPayload;
use crate::runtime::random::Random;
use crate::runtime::replay::{ReplayController, ReplayHeader};
use crate::runtime::time::Clock;
use crate::simulation::{SharedSimulationState, SimulationState};
use destack_workspace::{
    ExecutionMode, GcOptions, PlatformWindowsOptions, RandomMode, ReplayLogOptions,
    ReplayPayloadMode, RuntimeOptions, TimeMode,
};

/// Number of bytes in a megabyte for replay chunk sizing.
const BYTES_PER_MB: u64 = 1024 * 1024;

/// Shared runtime state for platform bindings and execution.
#[derive(Debug)]
pub struct RuntimeState {
    /// Platform context for host integrations.
    pub platform: PlatformContext,
    /// Runtime GC options for heap policy.
    pub gc: GcOptions,
    /// Windows runtime configuration options.
    pub windows: PlatformWindowsOptions,
    /// Virtual time and clock policy.
    pub time: Clock,
    /// Deterministic randomness streams.
    pub random: Random,
    /// External resource table and finalizers.
    pub resources: ResourceTable,
    /// Replay log and record/replay state.
    pub replay: ReplayController,
    /// Runtime hooks and effect state.
    pub hooks: RuntimeHooks,
    /// Simulation world state shared across simulation bindings.
    pub simulation: SharedSimulationState,
    /// Runtime error storage for native bindings.
    pub errors: RuntimeErrorStore,
}

impl RuntimeState {
    /// Create runtime state from explicit platform context.
    pub fn new(platform: PlatformContext) -> Self {
        Self::from_options(platform, &RuntimeOptions::default())
    }

    /// Create runtime state from runtime options.
    pub fn from_options(platform: PlatformContext, options: &RuntimeOptions) -> Self {
        let header = Self::replay_header_from_runtime_options(options);
        Self::from_runtime_options_and_header(platform, options, header)
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

        Self::from_runtime_options_and_header(platform, &options, header)
    }

    /// Create runtime state from runtime options and replay header.
    pub fn from_runtime_options_and_header(
        platform: PlatformContext,
        options: &RuntimeOptions,
        header: ReplayHeader,
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

        let time = Clock::from_mode_and_options(resolved_time_mode, &options.time);
        let random = Random::new(options.random.seed.unwrap_or(0), resolved_random_mode);
        let execution_mode = options.execution;
        let replay_payload = if replay_mode {
            header.replay_payload
        } else {
            Self::resolved_replay_payload_from_options(options)
        };

        Self {
            platform,
            gc: options.gc.clone(),
            windows: options.platform.windows.clone(),
            time,
            random,
            resources: ResourceTable::default(),
            replay: ReplayController::new(execution_mode, replay_payload, header),
            hooks: RuntimeHooks::from_runtime_options(options),
            simulation: SharedSimulationState::new(SimulationState::default()),
            errors: RuntimeErrorStore::default(),
        }
    }

    fn resolved_replay_payload_from_options(options: &RuntimeOptions) -> BindingReplayPayload {
        match options.replay_log.payload {
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

        // apply replay log chunk sizing
        Self::apply_replay_log_overrides(&options.replay_log, &mut header);

        header
    }

    fn apply_replay_log_overrides(options: &ReplayLogOptions, header: &mut ReplayHeader) {
        // update chunk sizing from runtime options
        if let Some(chunk_size_mb) = options.chunk_size_mb {
            let chunk_bytes = chunk_size_mb.saturating_mul(BYTES_PER_MB);
            if chunk_bytes > 0 {
                header.max_chunk_bytes = chunk_bytes;
            }
        }
    }
}
