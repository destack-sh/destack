use serde::{Deserialize, Serialize};

/// Observation category for emitted runtime or user facts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ObservationCategory {
    /// Runtime lifecycle and machine diagnostics.
    Runtime,
    /// Topology mutation and graph diagnostics.
    Topology,
    /// Resource lifecycle and action diagnostics.
    Resource,
    /// Scheduler diagnostics.
    Scheduler,
    /// General diagnostic and policy notices.
    Diagnostic,
    /// Telemetry, tracing, and performance instrumentation.
    Telemetry,
    /// Domain-level user or library observations.
    Domain,
}
