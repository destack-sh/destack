use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::diagnostic::RuntimeErrorStore;
use crate::host::HostRuntime;
use crate::platform::{PlatformContext, ResourceTable};
use crate::runtime::RuntimeHooks;
use crate::runtime::bindings::BindingReplayPayload;
use crate::runtime::random::Random;
use crate::runtime::replay::{ReplayController, ReplayHeader};
use crate::runtime::time::{Clock, HostClockSource};
use crate::simulation::{SharedSimulationState, SimulationState};
use destack_workspace::{
    ExecutionMode, GcOptions, PlatformAudioOptions, PlatformCryptoOptions, PlatformDebugOptions,
    PlatformDeviceOptions, PlatformDisplayOptions, PlatformErrorOptions, PlatformFfiOptions,
    PlatformFsOptions, PlatformGpuOptions, PlatformInputOptions, PlatformIoOptions,
    PlatformIpcOptions, PlatformMemoryOptions, PlatformNetOptions, PlatformOptions,
    PlatformOsOptions, PlatformProcessOptions, PlatformResourceOptions, PlatformSecurityOptions,
    PlatformThreadOptions, PlatformTlsOptions, PlatformTtyOptions, RandomMode, ReplayLogOptions,
    ReplayPayloadMode, RuntimeOptions, TimeMode,
};

/// Number of bytes in a megabyte for replay chunk sizing.
const BYTES_PER_MB: u64 = 1024 * 1024;
/// Global runtime-state id sequence for stable per-runtime identity.
static NEXT_RUNTIME_STATE_ID: AtomicU64 = AtomicU64::new(1);

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

/// Shared runtime state for platform bindings and execution.
#[derive(Debug)]
pub struct RuntimeState {
    /// Monotonic process-local runtime identity.
    pub instance_id: u64,
    /// Platform context for host integrations.
    pub platform: PlatformContext,
    /// Runtime GC options for heap policy.
    pub gc: GcOptions,
    /// Platform-specific runtime configuration options.
    pub platform_options: PlatformOptions,
    /// Resolved module options for the current compile target.
    pub module_options: ResolvedModuleOptions,
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
    /// Host integration state.
    pub host: HostRuntime,
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
        Self::from_runtime_options_and_header_with_host_clock_source(
            platform, options, header, None,
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
        Self::from_runtime_options_and_header_with_host_clock_source(
            platform,
            options,
            header,
            Some(host_clock_source),
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

        Self::from_runtime_options_and_header_with_host_clock_source(
            platform, &options, header, None,
        )
    }

    /// Create runtime state from runtime options and replay header.
    pub fn from_runtime_options_and_header(
        platform: PlatformContext,
        options: &RuntimeOptions,
        header: ReplayHeader,
    ) -> Self {
        Self::from_runtime_options_and_header_with_host_clock_source(
            platform, options, header, None,
        )
    }

    /// Create runtime state from options, header, and optional host clock source.
    fn from_runtime_options_and_header_with_host_clock_source(
        platform: PlatformContext,
        options: &RuntimeOptions,
        header: ReplayHeader,
        host_clock_source: Option<Arc<dyn HostClockSource>>,
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

        let time = if let Some(host_clock_source) = host_clock_source {
            Clock::from_mode_and_options_with_host_clock_source(
                resolved_time_mode,
                &options.time,
                host_clock_source,
            )
        } else {
            Clock::from_mode_and_options(resolved_time_mode, &options.time)
        };
        let random = Random::new(options.random.seed.unwrap_or(0), resolved_random_mode);
        let execution_mode = options.execution;
        let replay_payload = if replay_mode {
            header.replay_payload
        } else {
            Self::resolved_replay_payload_from_options(options)
        };

        Self {
            instance_id: NEXT_RUNTIME_STATE_ID.fetch_add(1, Ordering::Relaxed),
            platform,
            gc: options.gc.clone(),
            platform_options: options.platform.clone(),
            module_options: ResolvedModuleOptions::from_runtime_options(options),
            time,
            random,
            resources: ResourceTable::default(),
            replay: ReplayController::new(execution_mode, replay_payload, header),
            hooks: RuntimeHooks::from_runtime_options(options),
            host: HostRuntime::from_runtime_options(options),
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
