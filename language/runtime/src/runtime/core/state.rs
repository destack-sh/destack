use crate::diagnostic::RuntimeErrorStore;
use crate::platform::{PlatformContext, ResourceTable};
use crate::runtime::RuntimeRules;
use crate::runtime::bindings::BindingReplayPayload;
use crate::runtime::random::Random;
use crate::runtime::replay::{ReplayController, ReplayHeader};
use crate::runtime::time::Clock;
use crate::simulation::{SharedSimulationState, SimulationState};
use destack_workspace::{
    ExecutionMode, GcOptions, PlatformOptions, PlatformWindowsOptions, RandomMode, RandomOptions,
    ReplayLogOptions, ReplayPayloadMode, RuntimeOptions, TimeMode, TimeOptions,
};

/// Number of bytes in a megabyte for replay chunk sizing.
const BYTES_PER_MB: u64 = 1024 * 1024;

/// Shared runtime state for platform bindings and execution.
#[derive(Debug)]
pub struct RuntimeState {
    /// Platform context for host integrations.
    pub platform: PlatformContext,
    /// Platform runtime configuration options.
    pub platform_options: PlatformOptions,
    /// Virtual time and clock policy.
    pub time: Clock,
    /// Deterministic randomness streams.
    pub random: Random,
    /// Runtime GC options for heap policy.
    pub gc_options: GcOptions,
    /// External resource table and finalizers.
    pub resources: ResourceTable,
    /// Replay log and record/replay state.
    pub replay: ReplayController,
    /// Runtime rules and effect state.
    pub rules: RuntimeRules,
    /// Simulation world state shared across simulation bindings.
    pub simulation: SharedSimulationState,
    /// Runtime error storage for native bindings.
    pub errors: RuntimeErrorStore,
}

impl RuntimeState {
    /// Create runtime state from explicit platform context.
    pub fn new(platform: PlatformContext) -> Self {
        Self::from_runtime_options(platform, &RuntimeOptions::default())
    }

    /// Create runtime state from runtime options.
    pub fn from_runtime_options(platform: PlatformContext, options: &RuntimeOptions) -> Self {
        // build a replay header from options
        let header = Self::replay_header_from_options(options);

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
        // resolve time and random options for the execution mode
        let time_options = Self::resolve_time_options(options);
        let random_options = Self::resolve_random_options(options);

        // build runtime subsystems from options
        let time = Clock::from_options(&time_options);
        let random = Random::from_options(&random_options);
        let execution_mode = options.execution;
        let replay_payload = if options.execution == ExecutionMode::Replay {
            header.replay_payload
        } else {
            Self::resolve_replay_payload(options)
        };

        Self {
            platform,
            platform_options: options.platform.clone(),
            time,
            random,
            gc_options: options.gc.clone(),
            resources: ResourceTable::default(),
            replay: ReplayController::new(execution_mode, replay_payload, header),
            rules: RuntimeRules::from_runtime_options(options),
            simulation: SharedSimulationState::new(SimulationState::default()),
            errors: RuntimeErrorStore::default(),
        }
    }

    /// Return the runtime windows options.
    pub fn windows(&self) -> &PlatformWindowsOptions {
        &self.platform_options.windows
    }

    fn resolve_time_options(options: &RuntimeOptions) -> TimeOptions {
        // start from the configured time options
        let mut time_options = options.time.clone();

        // force virtual time during replay
        if options.execution == ExecutionMode::Replay {
            time_options.mode = TimeMode::Virtual;
        }

        time_options
    }

    fn resolve_random_options(options: &RuntimeOptions) -> RandomOptions {
        // start from the configured random options
        let mut random_options = options.random.clone();

        // force deterministic randomness during replay
        if options.execution == ExecutionMode::Replay {
            random_options.mode = RandomMode::Deterministic;
        }

        random_options
    }

    fn resolve_replay_payload(options: &RuntimeOptions) -> BindingReplayPayload {
        match options.replay_log.payload {
            ReplayPayloadMode::ResultsOnly => BindingReplayPayload::Results,
            ReplayPayloadMode::ArgumentsAndResults => BindingReplayPayload::ArgumentsAndResults,
        }
    }

    fn replay_header_from_options(options: &RuntimeOptions) -> ReplayHeader {
        // start from the default header
        let replay_payload = Self::resolve_replay_payload(options);
        let mut header = ReplayHeader {
            execution_mode: options.execution,
            replay_payload,
            ..ReplayHeader::default()
        };

        // apply replay log chunk sizing
        Self::apply_replay_log_options(&options.replay_log, &mut header);

        header
    }

    fn apply_replay_log_options(options: &ReplayLogOptions, header: &mut ReplayHeader) {
        // update chunk sizing from runtime options
        if let Some(chunk_size_mb) = options.chunk_size_mb {
            let chunk_bytes = chunk_size_mb.saturating_mul(BYTES_PER_MB);
            if chunk_bytes > 0 {
                header.max_chunk_bytes = chunk_bytes;
            }
        }
    }
}
