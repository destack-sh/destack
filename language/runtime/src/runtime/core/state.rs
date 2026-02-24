use std::sync::Arc;

use crate::diagnostic::RuntimeErrorStore;
use crate::platform::{PlatformContext, ResourceTable};
use crate::runtime::RuntimeHooks;
use crate::runtime::bindings::BindingReplayPayload;
use crate::runtime::host::HostRuntime;
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
    /// Host adapter integration state.
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
    /// Resolve global and platform-specific module options for the current compile target.
    fn from_runtime_options(options: &RuntimeOptions) -> Self {
        // seed from global defaults first
        let mut resolved = Self {
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
        };

        // apply current target platform overrides
        #[cfg(target_os = "android")]
        {
            apply_module_overrides(&mut resolved, &options.platform.android);
        }
        #[cfg(target_os = "dragonfly")]
        {
            apply_module_overrides(&mut resolved, &options.platform.dragonfly);
        }
        #[cfg(target_os = "freebsd")]
        {
            apply_module_overrides(&mut resolved, &options.platform.freebsd);
        }
        #[cfg(target_os = "haiku")]
        {
            apply_module_overrides(&mut resolved, &options.platform.haiku);
        }
        #[cfg(target_os = "illumos")]
        {
            apply_module_overrides(&mut resolved, &options.platform.illumos);
        }
        #[cfg(target_os = "ios")]
        {
            apply_module_overrides(&mut resolved, &options.platform.ios);
        }
        #[cfg(target_os = "linux")]
        {
            apply_module_overrides(&mut resolved, &options.platform.linux);
        }
        #[cfg(target_os = "macos")]
        {
            apply_module_overrides(&mut resolved, &options.platform.macos);
        }
        #[cfg(target_os = "netbsd")]
        {
            apply_module_overrides(&mut resolved, &options.platform.netbsd);
        }
        #[cfg(target_os = "openbsd")]
        {
            apply_module_overrides(&mut resolved, &options.platform.openbsd);
        }
        #[cfg(target_os = "solaris")]
        {
            apply_module_overrides(&mut resolved, &options.platform.solaris);
        }
        #[cfg(windows)]
        {
            apply_module_overrides(&mut resolved, &options.platform.windows);
        }

        resolved
    }
}

/// Apply one platform module override set to one resolved target options value.
fn apply_module_overrides<T>(resolved: &mut ResolvedModuleOptions, platform: &T)
where
    T: ModuleOptionSource,
{
    // apply filesystem overrides
    merge_path_option(&mut resolved.fs.sandbox_root, &platform.fs().sandbox_root);
    merge_path_option(
        &mut resolved.fs.temporary_directory,
        &platform.fs().temporary_directory,
    );
    merge_path_option(
        &mut resolved.fs.cache_directory,
        &platform.fs().cache_directory,
    );

    // apply network overrides
    merge_vec_override(&mut resolved.net.dns_servers, &platform.net().dns_servers);
    merge_string_option(&mut resolved.net.proxy_url, &platform.net().proxy_url);
    merge_string_option(
        &mut resolved.net.bind_interface,
        &platform.net().bind_interface,
    );

    // apply process overrides
    merge_path_option(
        &mut resolved.process.default_working_directory,
        &platform.process().default_working_directory,
    );
    merge_bool_option(
        &mut resolved.process.inherit_environment,
        &platform.process().inherit_environment,
    );
    merge_vec_override(
        &mut resolved.process.environment_allowlist,
        &platform.process().environment_allowlist,
    );

    // apply audio overrides
    merge_string_option(&mut resolved.audio.backend, &platform.audio().backend);
    merge_string_option(
        &mut resolved.audio.output_device,
        &platform.audio().output_device,
    );
    merge_string_option(
        &mut resolved.audio.input_device,
        &platform.audio().input_device,
    );
    merge_copy_option(
        &mut resolved.audio.target_latency_frames,
        &platform.audio().target_latency_frames,
    );
    merge_copy_option(
        &mut resolved.audio.target_period_frames,
        &platform.audio().target_period_frames,
    );

    // apply input overrides
    merge_string_option(&mut resolved.input.backend, &platform.input().backend);
    merge_copy_option(
        &mut resolved.input.event_queue_capacity,
        &platform.input().event_queue_capacity,
    );

    // apply gpu overrides
    merge_string_option(&mut resolved.gpu.backend, &platform.gpu().backend);
    merge_string_option(&mut resolved.gpu.adapter_name, &platform.gpu().adapter_name);
    merge_string_option(
        &mut resolved.gpu.power_preference,
        &platform.gpu().power_preference,
    );
    merge_path_option(
        &mut resolved.gpu.shader_cache_directory,
        &platform.gpu().shader_cache_directory,
    );

    // apply tls overrides
    merge_path_option(
        &mut resolved.tls.trust_store_path,
        &platform.tls().trust_store_path,
    );
    merge_string_option(
        &mut resolved.tls.client_certificate_store,
        &platform.tls().client_certificate_store,
    );

    // apply security overrides
    merge_string_option(
        &mut resolved.security.sandbox_profile,
        &platform.security().sandbox_profile,
    );
    merge_string_option(
        &mut resolved.security.capability_profile,
        &platform.security().capability_profile,
    );

    // apply os service overrides
    merge_string_option(
        &mut resolved.os.default_locale,
        &platform.os().default_locale,
    );
    merge_path_option(
        &mut resolved.os.data_directory,
        &platform.os().data_directory,
    );
    merge_path_option(
        &mut resolved.os.state_directory,
        &platform.os().state_directory,
    );

    // apply device service overrides
    merge_vec_override(
        &mut resolved.device.allow_classes,
        &platform.device().allow_classes,
    );
    merge_vec_override(
        &mut resolved.device.deny_classes,
        &platform.device().deny_classes,
    );

    // apply crypto overrides
    merge_path_option(
        &mut resolved.crypto.host_store_paths.user,
        &platform.crypto().host_store_paths.user,
    );
    merge_path_option(
        &mut resolved.crypto.host_store_paths.machine,
        &platform.crypto().host_store_paths.machine,
    );

    // apply module overrides for placeholder modules
    merge_value(&mut resolved.debug, platform.debug());
    merge_value(&mut resolved.display, platform.display());
    merge_value(&mut resolved.error, platform.error());
    merge_value(&mut resolved.ffi, platform.ffi());
    merge_value(&mut resolved.io, platform.io());
    merge_value(&mut resolved.ipc, platform.ipc());
    merge_value(&mut resolved.memory, platform.memory());
    merge_value(&mut resolved.resource, platform.resource());
    merge_value(&mut resolved.thread, platform.thread());
    merge_value(&mut resolved.tty, platform.tty());
}

/// Source interface for module option values in platform option structs.
trait ModuleOptionSource {
    /// Borrow filesystem module options.
    fn fs(&self) -> &PlatformFsOptions;
    /// Borrow network module options.
    fn net(&self) -> &PlatformNetOptions;
    /// Borrow process module options.
    fn process(&self) -> &PlatformProcessOptions;
    /// Borrow audio module options.
    fn audio(&self) -> &PlatformAudioOptions;
    /// Borrow input module options.
    fn input(&self) -> &PlatformInputOptions;
    /// Borrow gpu module options.
    fn gpu(&self) -> &PlatformGpuOptions;
    /// Borrow tls module options.
    fn tls(&self) -> &PlatformTlsOptions;
    /// Borrow security module options.
    fn security(&self) -> &PlatformSecurityOptions;
    /// Borrow os service module options.
    fn os(&self) -> &PlatformOsOptions;
    /// Borrow device service module options.
    fn device(&self) -> &PlatformDeviceOptions;
    /// Borrow crypto module options.
    fn crypto(&self) -> &PlatformCryptoOptions;
    /// Borrow debug module options.
    fn debug(&self) -> &PlatformDebugOptions;
    /// Borrow display module options.
    fn display(&self) -> &PlatformDisplayOptions;
    /// Borrow error module options.
    fn error(&self) -> &PlatformErrorOptions;
    /// Borrow ffi module options.
    fn ffi(&self) -> &PlatformFfiOptions;
    /// Borrow io module options.
    fn io(&self) -> &PlatformIoOptions;
    /// Borrow ipc module options.
    fn ipc(&self) -> &PlatformIpcOptions;
    /// Borrow memory module options.
    fn memory(&self) -> &PlatformMemoryOptions;
    /// Borrow resource module options.
    fn resource(&self) -> &PlatformResourceOptions;
    /// Borrow thread module options.
    fn thread(&self) -> &PlatformThreadOptions;
    /// Borrow tty module options.
    fn tty(&self) -> &PlatformTtyOptions;
}

macro_rules! impl_module_option_source {
    ($type_name:ty) => {
        impl ModuleOptionSource for $type_name {
            fn fs(&self) -> &PlatformFsOptions {
                &self.fs
            }
            fn net(&self) -> &PlatformNetOptions {
                &self.net
            }
            fn process(&self) -> &PlatformProcessOptions {
                &self.process
            }
            fn audio(&self) -> &PlatformAudioOptions {
                &self.audio
            }
            fn input(&self) -> &PlatformInputOptions {
                &self.input
            }
            fn gpu(&self) -> &PlatformGpuOptions {
                &self.gpu
            }
            fn tls(&self) -> &PlatformTlsOptions {
                &self.tls
            }
            fn security(&self) -> &PlatformSecurityOptions {
                &self.security
            }
            fn os(&self) -> &PlatformOsOptions {
                &self.os
            }
            fn device(&self) -> &PlatformDeviceOptions {
                &self.device
            }
            fn crypto(&self) -> &PlatformCryptoOptions {
                &self.crypto
            }
            fn debug(&self) -> &PlatformDebugOptions {
                &self.debug
            }
            fn display(&self) -> &PlatformDisplayOptions {
                &self.display
            }
            fn error(&self) -> &PlatformErrorOptions {
                &self.error
            }
            fn ffi(&self) -> &PlatformFfiOptions {
                &self.ffi
            }
            fn io(&self) -> &PlatformIoOptions {
                &self.io
            }
            fn ipc(&self) -> &PlatformIpcOptions {
                &self.ipc
            }
            fn memory(&self) -> &PlatformMemoryOptions {
                &self.memory
            }
            fn resource(&self) -> &PlatformResourceOptions {
                &self.resource
            }
            fn thread(&self) -> &PlatformThreadOptions {
                &self.thread
            }
            fn tty(&self) -> &PlatformTtyOptions {
                &self.tty
            }
        }
    };
}

impl_module_option_source!(destack_workspace::PlatformAndroidOptions);
impl_module_option_source!(destack_workspace::PlatformDragonflyOptions);
impl_module_option_source!(destack_workspace::PlatformFreeBsdOptions);
impl_module_option_source!(destack_workspace::PlatformHaikuOptions);
impl_module_option_source!(destack_workspace::PlatformIllumosOptions);
impl_module_option_source!(destack_workspace::PlatformIosOptions);
impl_module_option_source!(destack_workspace::PlatformLinuxOptions);
impl_module_option_source!(destack_workspace::PlatformMacosOptions);
impl_module_option_source!(destack_workspace::PlatformNetBsdOptions);
impl_module_option_source!(destack_workspace::PlatformOpenBsdOptions);
impl_module_option_source!(destack_workspace::PlatformSolarisOptions);
impl_module_option_source!(destack_workspace::PlatformWindowsOptions);

/// Merge one optional string override into one target field.
fn merge_string_option(target: &mut Option<String>, override_value: &Option<String>) {
    if let Some(value) = override_value {
        *target = Some(value.clone());
    }
}

/// Merge one optional path override into one target field.
fn merge_path_option(
    target: &mut Option<std::path::PathBuf>,
    override_value: &Option<std::path::PathBuf>,
) {
    if let Some(value) = override_value {
        *target = Some(value.clone());
    }
}

/// Merge one optional copy-type override into one target field.
fn merge_copy_option<T: Copy>(target: &mut Option<T>, override_value: &Option<T>) {
    if let Some(value) = override_value {
        *target = Some(*value);
    }
}

/// Merge one optional boolean override into one target field.
fn merge_bool_option(target: &mut Option<bool>, override_value: &Option<bool>) {
    if let Some(value) = override_value {
        *target = Some(*value);
    }
}

/// Merge one full-value override into one target field.
fn merge_value<T: Clone>(target: &mut T, override_value: &T) {
    *target = override_value.clone();
}

/// Merge one vector override into one target field when the override is non-empty.
fn merge_vec_override<T: Clone>(target: &mut Vec<T>, override_value: &[T]) {
    if !override_value.is_empty() {
        *target = override_value.to_vec();
    }
}
