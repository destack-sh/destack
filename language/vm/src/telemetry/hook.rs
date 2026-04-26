use destack_heap::GcStats;

/// Callback for reporting garbage collection statistics.
pub type TelemetryGcHook = fn(&GcStats);

/// Telemetry hook configuration for runtime integration.
#[derive(Debug, Clone, Default)]
pub struct TelemetryHook {
    /// Hook invoked after a garbage collection cycle completes.
    pub on_gc: Option<TelemetryGcHook>,
}
