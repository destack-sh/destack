use serde::{Deserialize, Serialize};

/// Runtime diagnostic verbosity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
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
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
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
