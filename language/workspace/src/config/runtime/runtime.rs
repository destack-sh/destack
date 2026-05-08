use std::collections::BTreeMap;

use destack_artifact::Runtime;
use serde::{Deserialize, Serialize};

use crate::config::target::AppOptions;
use crate::{ExecutionMode, ReplayPayloadMode};

use super::random::{RandomMode, RandomOptions};
use super::replay::ReplayOptions;
use super::time::{TimeMode, TimeOptions};
use super::{
    EffectOptions, EffectOptionsJson, EffectSource, HeapOptions, HeapOptionsJson,
    PlatformAudioOptions, PlatformAudioOptionsJson, PlatformCryptoOptions,
    PlatformCryptoOptionsJson, PlatformDeviceOptions, PlatformDeviceOptionsJson,
    PlatformDisplayOptions, PlatformDisplayOptionsJson, PlatformFsOptions, PlatformFsOptionsJson,
    PlatformGpuOptions, PlatformGpuOptionsJson, PlatformInputOptions, PlatformInputOptionsJson,
    PlatformIpcOptions, PlatformIpcOptionsJson, PlatformNetOptions, PlatformNetOptionsJson,
    PlatformOptions, PlatformOptionsJson, PlatformOsOptions, PlatformOsOptionsJson,
    PlatformProcessOptions, PlatformProcessOptionsJson, PlatformSecurityOptions,
    PlatformSecurityOptionsJson, PlatformTlsOptions, PlatformTlsOptionsJson,
    RuntimeDiagnosticOptions, RuntimeDiagnosticOptionsJson, SchedulerMode, SchedulerOptions,
    SchedulerOptionsJson, SimulationOptions, SimulationOptionsJson, TraceMode, TraceOptions,
    TraceOptionsJson,
};

/// Default identity options for one runtime primary worker.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct RuntimeWorkerOptions {
    /// Default primary worker name for policy selection.
    pub name: Option<String>,
    /// Default primary worker labels for policy selection.
    pub labels: BTreeMap<String, String>,
}

/// Runtime configuration.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct RuntimeOptions {
    /// Execution host semantics for this runtime.
    pub host: Runtime,
    /// Runtime version for versioned library selection.
    pub version: Option<String>,
    /// Stable runtime name for policy selection.
    pub name: Option<String>,
    /// Runtime labels for policy selection.
    pub labels: BTreeMap<String, String>,
    /// Default primary worker identity for policy selection.
    pub primary_worker: RuntimeWorkerOptions,
    /// Resolved app model for host availability checks.
    pub app: AppOptions,
    /// Runtime scheduler configuration.
    pub scheduler: SchedulerOptions,
    /// Runtime effect configuration.
    pub effect: EffectOptions,
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

        // effect sources
        if mode == ExecutionMode::Replay {
            self.effect.time = EffectSource::Trace;
            self.effect.random = EffectSource::Trace;
        }
    }

    /// Apply one collapsed time summary onto the runtime planes.
    pub fn set_time_mode(&mut self, mode: TimeMode) {
        self.effect.time = match mode {
            TimeMode::Host => EffectSource::Host,
            TimeMode::Virtual => EffectSource::Simulation,
        };
    }

    /// Apply one collapsed randomness summary onto the runtime planes.
    pub fn set_random_mode(&mut self, mode: RandomMode) {
        self.effect.random = match mode {
            RandomMode::Host => EffectSource::Host,
            RandomMode::Deterministic => EffectSource::Simulation,
        };
    }

    /// Return the collapsed execution summary.
    pub fn execution_mode(&self) -> ExecutionMode {
        // trace replay is always authoritative
        if self.trace.mode == TraceMode::Replay {
            return ExecutionMode::Replay;
        }

        // trace recording keeps the old record-facing behavior
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
        match self.effect.time {
            EffectSource::Host => TimeMode::Host,
            EffectSource::Trace | EffectSource::Simulation | EffectSource::Deny => {
                TimeMode::Virtual
            }
        }
    }

    /// Return the collapsed randomness summary.
    pub fn random_mode(&self) -> RandomMode {
        match self.effect.random {
            EffectSource::Host => RandomMode::Host,
            EffectSource::Trace | EffectSource::Simulation | EffectSource::Deny => {
                RandomMode::Deterministic
            }
        }
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
        let mode = self.time_mode();
        let epoch_ns = self.simulation.time.epoch_ns;
        let time_zone = self.simulation.time.time_zone.clone();

        TimeOptions {
            mode,
            epoch_ns,
            time_zone,
        }
    }

    /// Return derived randomness configuration for runtime internals.
    pub fn random_options(&self) -> RandomOptions {
        let mode = self.random_mode();
        let seed = self.simulation.random.seed;
        let per_runnable = self.simulation.random.per_runnable;

        RandomOptions {
            mode,
            seed,
            per_runnable,
        }
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
    /// Runtime contract shorthand.
    Host(Runtime),
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
            Self::Host(host) => RuntimeOptionsJson {
                host: Some(*host),
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
    /// Execution host semantics for this runtime.
    pub host: Option<Runtime>,
    /// Runtime version for versioned library selection.
    pub version: Option<String>,
    /// Stable runtime name for policy selection.
    pub name: Option<String>,
    /// Runtime labels for policy selection.
    pub labels: Option<BTreeMap<String, String>>,
    /// Default primary worker identity for policy selection.
    pub primary_worker: Option<RuntimeWorkerOptionsJson>,
    /// Runtime scheduler configuration.
    pub scheduler: Option<SchedulerOptionsJson>,
    /// Runtime effect configuration.
    pub effect: Option<EffectOptionsJson>,
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

/// Primary runtime worker options for JSON deserialization.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct RuntimeWorkerOptionsJson {
    /// Primary worker name for policy selection.
    pub name: Option<String>,
    /// Primary worker labels for policy selection.
    pub labels: Option<BTreeMap<String, String>>,
}

impl RuntimeWorkerOptionsJson {
    /// Apply primary worker overrides to one base set of runtime worker options.
    pub fn apply_to(&self, options: &mut RuntimeWorkerOptions) {
        // apply primary worker name override
        if let Some(name) = &self.name {
            options.name = Some(name.clone());
        }

        // apply primary worker label override
        if let Some(labels) = &self.labels {
            options.labels = labels.clone();
        }
    }
}

impl RuntimeOptionsJson {
    /// Inherit unset runtime settings from one parent config.
    pub fn extend_from(&mut self, parent: &Self) {
        if self.host.is_none() {
            self.host = parent.host.clone();
        }
        if self.version.is_none() {
            self.version = parent.version.clone();
        }
        if self.name.is_none() {
            self.name = parent.name.clone();
        }
        if self.labels.is_none() {
            self.labels = parent.labels.clone();
        }
        if self.primary_worker.is_none() {
            self.primary_worker = parent.primary_worker.clone();
        }
        if let Some(scheduler) = &mut self.scheduler {
            if let Some(parent_scheduler) = &parent.scheduler {
                scheduler.extend_from(parent_scheduler);
            }
        } else {
            self.scheduler = parent.scheduler.clone();
        }
        if let Some(effect) = &mut self.effect {
            if let Some(parent_effect) = &parent.effect {
                effect.extend_from(parent_effect);
            }
        } else {
            self.effect = parent.effect.clone();
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
        // host and version
        if let Some(host) = self.host {
            options.host = host;
        }
        if let Some(version) = &self.version {
            options.version = Some(version.clone());
        }

        // apply runtime identity overrides
        if let Some(name) = &self.name {
            options.name = Some(name.clone());
        }
        if let Some(labels) = &self.labels {
            options.labels = labels.clone();
        }

        // apply default primary worker identity overrides
        if let Some(primary_worker) = &self.primary_worker {
            primary_worker.apply_to(&mut options.primary_worker);
        }

        // apply runtime planes
        if let Some(scheduler) = &self.scheduler {
            scheduler.apply_to(&mut options.scheduler);
        }

        if let Some(effect) = &self.effect {
            effect.apply_to(&mut options.effect);
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
