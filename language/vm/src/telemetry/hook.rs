use crate::memory::GcStats;

use super::Statistics;

/// Callback for reporting execution statistics.
pub type TelemetryStatsHook = fn(&Statistics);
/// Callback for reporting garbage collection statistics.
pub type TelemetryGcHook = fn(&GcStats);

/// Telemetry hook configuration for runtime integration.
#[derive(Debug, Clone, Default)]
pub struct TelemetryHook {
    /// Hook invoked when execution statistics are updated.
    pub on_stats: Option<TelemetryStatsHook>,
    /// Hook invoked after a garbage collection cycle completes.
    pub on_gc: Option<TelemetryGcHook>,
}
