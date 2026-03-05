use serde::{Deserialize, Serialize};

/// Runtime diagnostic verbosity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum RuntimeDiagnosticLevel {
    /// Disable runtime diagnostics collection.
    Off,
    /// Record only error diagnostics.
    Error,
    /// Record warning and error diagnostics.
    #[default]
    Warn,
    /// Record informational diagnostics and above.
    Info,
    /// Record debug diagnostics and above.
    Debug,
    /// Record trace diagnostics and above.
    Trace,
}

/// Runtime diagnostics configuration.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RuntimeDiagnosticOptions {
    /// Minimum diagnostic level recorded by the runtime.
    pub level: RuntimeDiagnosticLevel,
    /// Maximum number of diagnostic entries retained in the runtime ring buffer.
    pub capacity: Option<u64>,
}

impl Default for RuntimeDiagnosticOptions {
    fn default() -> Self {
        Self {
            level: RuntimeDiagnosticLevel::Warn,
            capacity: Some(1024),
        }
    }
}

/// Runtime diagnostic verbosity for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum RuntimeDiagnosticLevelJson {
    /// Disable runtime diagnostics collection.
    Off,
    /// Record only error diagnostics.
    Error,
    /// Record warning and error diagnostics.
    Warn,
    /// Record informational diagnostics and above.
    Info,
    /// Record debug diagnostics and above.
    Debug,
    /// Record trace diagnostics and above.
    Trace,
}

impl From<RuntimeDiagnosticLevelJson> for RuntimeDiagnosticLevel {
    fn from(value: RuntimeDiagnosticLevelJson) -> Self {
        match value {
            RuntimeDiagnosticLevelJson::Off => RuntimeDiagnosticLevel::Off,
            RuntimeDiagnosticLevelJson::Error => RuntimeDiagnosticLevel::Error,
            RuntimeDiagnosticLevelJson::Warn => RuntimeDiagnosticLevel::Warn,
            RuntimeDiagnosticLevelJson::Info => RuntimeDiagnosticLevel::Info,
            RuntimeDiagnosticLevelJson::Debug => RuntimeDiagnosticLevel::Debug,
            RuntimeDiagnosticLevelJson::Trace => RuntimeDiagnosticLevel::Trace,
        }
    }
}

/// Runtime diagnostics configuration for JSON deserialization.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct RuntimeDiagnosticOptionsJson {
    /// Minimum diagnostic level recorded by the runtime.
    pub level: Option<RuntimeDiagnosticLevelJson>,
    /// Maximum number of diagnostic entries retained in the runtime ring buffer.
    pub capacity: Option<u64>,
}

impl RuntimeDiagnosticOptionsJson {
    /// Apply diagnostic overrides to one base set of options.
    pub fn apply_to(&self, options: &mut RuntimeDiagnosticOptions) {
        if let Some(level) = self.level {
            options.level = RuntimeDiagnosticLevel::from(level);
        }

        if let Some(capacity) = self.capacity {
            options.capacity = Some(capacity);
        }
    }
}
