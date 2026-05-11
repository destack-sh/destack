use destack_artifact::Runtime;
use serde::{Deserialize, Serialize};

use crate::config::target::AppOptions;
use crate::{ExecutionMode, ReplayPayloadMode};

use super::random::{RandomMode, RandomOptions, RandomOptionsJson};
use super::replay::ReplayOptions;
use super::time::{TimeMode, TimeOptions, TimeOptionsJson};
use super::{
    HeapOptions, HeapOptionsJson, PlatformAudioOptions, PlatformAudioOptionsJson,
    PlatformCryptoOptions, PlatformCryptoOptionsJson, PlatformDeviceOptions,
    PlatformDeviceOptionsJson, PlatformDisplayOptions, PlatformDisplayOptionsJson,
    PlatformFsOptions, PlatformFsOptionsJson, PlatformGpuOptions, PlatformGpuOptionsJson,
    PlatformInputOptions, PlatformInputOptionsJson, PlatformIpcOptions, PlatformIpcOptionsJson,
    PlatformNetOptions, PlatformNetOptionsJson, PlatformOptions, PlatformOptionsJson,
    PlatformOsOptions, PlatformOsOptionsJson, PlatformProcessOptions, PlatformProcessOptionsJson,
    PlatformSecurityOptions, PlatformSecurityOptionsJson, PlatformTlsOptions,
    PlatformTlsOptionsJson, RuntimeDiagnosticOptions, RuntimeDiagnosticOptionsJson, SchedulerMode,
    SchedulerOptions, SchedulerOptionsJson, SimulationOptions, SimulationOptionsJson, TraceMode,
    TraceOptions, TraceOptionsJson,
};

/// Runtime configuration.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct RuntimeOptions {
    /// Runtime environment family.
    pub environment: Runtime,
    /// Stable runtime name for policy selection.
    pub name: Option<String>,
    /// App declaration used for host support checks.
    pub app: AppOptions,
    /// Runtime scheduler configuration.
    pub scheduler: SchedulerOptions,
    /// Runtime clock source configuration.
    pub time: TimeOptions,
    /// Runtime randomness source configuration.
    pub random: RandomOptions,
    /// Runtime simulation configuration.
    pub simulation: SimulationOptions,
    /// Runtime trace configuration.
    pub trace: TraceOptions,
    /// Runtime heap configuration.
    pub heap: HeapOptions,
    /// Runtime diagnostics configuration.
    pub diagnostic: RuntimeDiagnosticOptions,
    /// Global filesystem runtime defaults.
    pub fs: PlatformFsOptions,
    /// Global network runtime defaults.
    pub net: PlatformNetOptions,
    /// Global process runtime defaults.
    pub process: PlatformProcessOptions,
    /// Global audio runtime defaults.
    pub audio: PlatformAudioOptions,
    /// Global input runtime defaults.
    pub input: PlatformInputOptions,
    /// Global GPU runtime defaults.
    pub gpu: PlatformGpuOptions,
    /// Global TLS runtime defaults.
    pub tls: PlatformTlsOptions,
    /// Global security runtime defaults.
    pub security: PlatformSecurityOptions,
    /// Global OS service runtime defaults.
    pub os: PlatformOsOptions,
    /// Global device service runtime defaults.
    pub device: PlatformDeviceOptions,
    /// Display runtime options.
    pub display: PlatformDisplayOptions,
    /// IPC runtime options.
    pub ipc: PlatformIpcOptions,
    /// Global crypto runtime defaults.
    pub crypto: PlatformCryptoOptions,
    /// Platform-specific host runtime overrides.
    pub platform: PlatformOptions,
}

impl RuntimeOptions {
    /// Apply one collapsed execution summary onto the runtime planes.
    pub fn set_execution_mode(&mut self, mode: ExecutionMode) {
        // scheduler mode
        self.scheduler.mode = match mode {
            ExecutionMode::Fast => SchedulerMode::Parallel,
            ExecutionMode::Deterministic | ExecutionMode::Record | ExecutionMode::Replay => {
                SchedulerMode::Cooperative
            }
        };

        // trace mode
        self.trace.mode = match mode {
            ExecutionMode::Fast | ExecutionMode::Deterministic => TraceMode::Off,
            ExecutionMode::Record => TraceMode::Record,
            ExecutionMode::Replay => TraceMode::Replay,
        };

        // replay uses virtual facts supplied by trace playback
        if mode == ExecutionMode::Replay {
            self.time.mode = TimeMode::Virtual;
            self.random.mode = RandomMode::Deterministic;
        }
    }

    /// Apply one collapsed time summary onto the runtime planes.
    pub fn set_time_mode(&mut self, mode: TimeMode) {
        self.time.mode = mode;
    }

    /// Apply one collapsed randomness summary onto the runtime planes.
    pub fn set_random_mode(&mut self, mode: RandomMode) {
        self.random.mode = mode;
    }

    /// Return the collapsed execution summary.
    pub fn execution_mode(&self) -> ExecutionMode {
        // trace replay is always authoritative
        if self.trace.mode == TraceMode::Replay {
            return ExecutionMode::Replay;
        }

        // trace recording selects record execution
        if self.trace.mode == TraceMode::Record {
            return ExecutionMode::Record;
        }

        // cooperative worlds collapse to deterministic execution
        if self.scheduler.mode == SchedulerMode::Cooperative {
            return ExecutionMode::Deterministic;
        }

        ExecutionMode::Fast
    }

    /// Return the collapsed time-source summary.
    pub fn time_mode(&self) -> TimeMode {
        self.time.mode
    }

    /// Return the collapsed randomness summary.
    pub fn random_mode(&self) -> RandomMode {
        self.random.mode
    }

    /// Return the configured trace payload policy.
    pub fn replay_payload_mode(&self) -> ReplayPayloadMode {
        self.trace.payload
    }

    /// Return the configured trace chunk size in megabytes.
    pub fn trace_chunk_size_mb(&self) -> Option<u64> {
        self.trace.chunk_size_mb
    }

    /// Return derived clock configuration for runtime internals.
    pub fn time_options(&self) -> TimeOptions {
        let mut options = self.time.clone();
        options.epoch_ns = options.epoch_ns.or(self.simulation.time.epoch_ns);
        options.time_zone = options
            .time_zone
            .clone()
            .or_else(|| self.simulation.time.time_zone.clone());

        options
    }

    /// Return derived randomness configuration for runtime internals.
    pub fn random_options(&self) -> RandomOptions {
        let mut options = self.random.clone();
        options.seed = options.seed.or(self.simulation.random.seed);
        options.per_runnable |= self.simulation.random.per_runnable;

        options
    }

    /// Return derived trace-storage configuration for runtime internals.
    pub fn replay_options(&self) -> ReplayOptions {
        ReplayOptions {
            path: self.trace.path.clone(),
            template: self.trace.template.clone(),
            chunk_size_mb: self.trace.chunk_size_mb,
            payload: self.trace.payload,
        }
    }

    /// Return the configured scheduler options.
    pub fn scheduler_options(&self) -> &SchedulerOptions {
        &self.scheduler
    }
}

/// Runtime config JSON.
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(untagged)]
pub enum RuntimeConfigJson {
    /// Runtime environment shorthand.
    Environment(Runtime),
    /// Full runtime configuration object.
    Options(Box<RuntimeOptionsJson>),
}

impl Default for RuntimeConfigJson {
    fn default() -> Self {
        Self::Options(Box::default())
    }
}

impl RuntimeConfigJson {
    /// Return this runtime config as object form.
    pub fn as_options_json(&self) -> RuntimeOptionsJson {
        match self {
            Self::Environment(environment) => RuntimeOptionsJson {
                environment: Some(*environment),
                ..RuntimeOptionsJson::default()
            },
            Self::Options(options) => options.as_ref().clone(),
        }
    }
}

pub(crate) fn runtime_options_from_json(json: Option<&RuntimeConfigJson>) -> RuntimeOptions {
    runtime_options_with_base(&RuntimeOptions::default(), json)
}

/// Derive runtime options from a base set of options plus overrides.
pub(crate) fn runtime_options_with_base(
    base: &RuntimeOptions,
    overrides: Option<&RuntimeConfigJson>,
) -> RuntimeOptions {
    // start from the base options
    let mut options = base.clone();

    // apply overrides when present
    if let Some(overrides) = overrides {
        overrides.as_options_json().apply_to(&mut options);
    }

    options
}

/// Runtime options (object form).
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct RuntimeOptionsJson {
    /// Runtime environment family.
    pub environment: Option<Runtime>,
    /// Stable runtime name for policy selection.
    pub name: Option<String>,
    /// Runtime scheduler configuration.
    pub scheduler: Option<SchedulerOptionsJson>,
    /// Runtime clock source configuration.
    pub time: Option<TimeOptionsJson>,
    /// Runtime randomness source configuration.
    pub random: Option<RandomOptionsJson>,
    /// Runtime simulation configuration.
    pub simulation: Option<SimulationOptionsJson>,
    /// Runtime trace configuration.
    pub trace: Option<TraceOptionsJson>,
    /// Runtime heap configuration.
    pub heap: Option<HeapOptionsJson>,
    /// Runtime diagnostics configuration.
    pub diagnostic: Option<RuntimeDiagnosticOptionsJson>,
    /// Global filesystem runtime defaults.
    pub fs: Option<PlatformFsOptionsJson>,
    /// Global network runtime defaults.
    pub net: Option<PlatformNetOptionsJson>,
    /// Global process runtime defaults.
    pub process: Option<PlatformProcessOptionsJson>,
    /// Global audio runtime defaults.
    pub audio: Option<PlatformAudioOptionsJson>,
    /// Global input runtime defaults.
    pub input: Option<PlatformInputOptionsJson>,
    /// Global GPU runtime defaults.
    pub gpu: Option<PlatformGpuOptionsJson>,
    /// Global TLS runtime defaults.
    pub tls: Option<PlatformTlsOptionsJson>,
    /// Global security runtime defaults.
    pub security: Option<PlatformSecurityOptionsJson>,
    /// Global OS service runtime defaults.
    pub os: Option<PlatformOsOptionsJson>,
    /// Global device service runtime defaults.
    pub device: Option<PlatformDeviceOptionsJson>,
    /// Global crypto runtime defaults.
    pub crypto: Option<PlatformCryptoOptionsJson>,
    /// Global display runtime defaults.
    pub display: Option<PlatformDisplayOptionsJson>,
    /// Global ipc runtime defaults.
    pub ipc: Option<PlatformIpcOptionsJson>,
    /// Platform-specific host runtime overrides.
    pub platform: Option<PlatformOptionsJson>,
}

impl RuntimeOptionsJson {
    /// Inherit unset runtime settings from one parent config.
    pub fn extend_from(&mut self, parent: &Self) {
        if self.environment.is_none() {
            self.environment = parent.environment;
        }
        if self.name.is_none() {
            self.name = parent.name.clone();
        }
        if let Some(scheduler) = &mut self.scheduler {
            if let Some(parent_scheduler) = &parent.scheduler {
                scheduler.extend_from(parent_scheduler);
            }
        } else {
            self.scheduler = parent.scheduler.clone();
        }
        if let Some(time) = &mut self.time {
            if let Some(parent_time) = &parent.time {
                time.extend_from(parent_time);
            }
        } else {
            self.time = parent.time.clone();
        }
        if let Some(random) = &mut self.random {
            if let Some(parent_random) = &parent.random {
                random.extend_from(parent_random);
            }
        } else {
            self.random = parent.random.clone();
        }
        if let Some(simulation) = &mut self.simulation {
            if let Some(parent_simulation) = &parent.simulation {
                simulation.extend_from(parent_simulation);
            }
        } else {
            self.simulation = parent.simulation.clone();
        }
        if let Some(trace) = &mut self.trace {
            if let Some(parent_trace) = &parent.trace {
                trace.extend_from(parent_trace);
            }
        } else {
            self.trace = parent.trace.clone();
        }
        if let Some(heap) = &mut self.heap {
            if let Some(parent_heap) = &parent.heap {
                heap.extend_from(parent_heap);
            }
        } else {
            self.heap = parent.heap.clone();
        }
        if self.diagnostic.is_none() {
            self.diagnostic = parent.diagnostic.clone();
        }
        if self.fs.is_none() {
            self.fs = parent.fs.clone();
        }
        if self.net.is_none() {
            self.net = parent.net.clone();
        }
        if self.process.is_none() {
            self.process = parent.process.clone();
        }
        if self.audio.is_none() {
            self.audio = parent.audio.clone();
        }
        if self.input.is_none() {
            self.input = parent.input.clone();
        }
        if self.gpu.is_none() {
            self.gpu = parent.gpu.clone();
        }
        if self.tls.is_none() {
            self.tls = parent.tls.clone();
        }
        if self.security.is_none() {
            self.security = parent.security.clone();
        }
        if self.os.is_none() {
            self.os = parent.os.clone();
        }
        if self.device.is_none() {
            self.device = parent.device.clone();
        }
        if self.crypto.is_none() {
            self.crypto = parent.crypto.clone();
        }
        if self.display.is_none() {
            self.display = parent.display.clone();
        }
        if self.ipc.is_none() {
            self.ipc = parent.ipc.clone();
        }
        if self.platform.is_none() {
            self.platform = parent.platform.clone();
        }
    }

    /// Apply runtime option overrides to a base set of options.
    pub fn apply_to(&self, options: &mut RuntimeOptions) {
        // environment
        if let Some(environment) = self.environment {
            options.environment = environment;
        }

        // runtime identity
        if let Some(name) = &self.name {
            options.name = Some(name.clone());
        }

        // apply runtime planes
        if let Some(scheduler) = &self.scheduler {
            scheduler.apply_to(&mut options.scheduler);
        }

        if let Some(time) = &self.time {
            time.apply_to(&mut options.time);
        }

        if let Some(random) = &self.random {
            random.apply_to(&mut options.random);
        }

        if let Some(simulation) = &self.simulation {
            simulation.apply_to(&mut options.simulation);
        }

        if let Some(trace) = &self.trace {
            trace.apply_to(&mut options.trace);
        }

        // apply heap overrides
        if let Some(heap) = &self.heap {
            heap.apply_to(&mut options.heap);
        }

        // apply diagnostic overrides
        if let Some(diagnostic) = &self.diagnostic {
            diagnostic.apply_to(&mut options.diagnostic);
        }

        // apply filesystem defaults
        if let Some(fs) = &self.fs {
            fs.apply_to(&mut options.fs);
        }

        // apply network defaults
        if let Some(net) = &self.net {
            net.apply_to(&mut options.net);
        }

        // apply process defaults
        if let Some(process) = &self.process {
            process.apply_to(&mut options.process);
        }

        // apply audio defaults
        if let Some(audio) = &self.audio {
            audio.apply_to(&mut options.audio);
        }

        // apply input defaults
        if let Some(input) = &self.input {
            input.apply_to(&mut options.input);
        }

        // apply gpu defaults
        if let Some(gpu) = &self.gpu {
            gpu.apply_to(&mut options.gpu);
        }

        // apply tls defaults
        if let Some(tls) = &self.tls {
            tls.apply_to(&mut options.tls);
        }

        // apply security defaults
        if let Some(security) = &self.security {
            security.apply_to(&mut options.security);
        }

        // apply os defaults
        if let Some(os) = &self.os {
            os.apply_to(&mut options.os);
        }

        // apply device defaults
        if let Some(device) = &self.device {
            device.apply_to(&mut options.device);
        }

        // apply crypto defaults
        if let Some(crypto) = &self.crypto {
            crypto.apply_to(&mut options.crypto);
        }

        // apply display defaults
        if let Some(display) = &self.display {
            display.apply_to(&mut options.display);
        }

        // apply ipc defaults
        if let Some(ipc) = &self.ipc {
            ipc.apply_to(&mut options.ipc);
        }

        // apply platform overrides
        if let Some(platform) = &self.platform {
            platform.apply_to(&mut options.platform);
        }
    }
}
