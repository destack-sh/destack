use serde::{Deserialize, Serialize};

/// Host clock identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ClockId {
    /// Wall clock.
    Wall = 1,
    /// Monotonic clock.
    Monotonic = 2,
    /// Process CPU clock.
    ProcessCpu = 3,
    /// Thread CPU clock.
    ThreadCpu = 4,
    /// Boot-relative clock.
    Boot = 5,
    /// Raw monotonic clock.
    MonotonicRaw = 6,
}

/// Host clock source kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ClockSource {
    /// Realtime source.
    Realtime = 1,
    /// Monotonic source.
    Monotonic = 2,
    /// Performance counter source.
    PerformanceCounter = 3,
    /// Virtual source.
    Virtual = 4,
}

/// Host clock properties.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClockProperties {
    /// Clock identifier.
    pub id: ClockId,
    /// Host clock source kind.
    pub source: ClockSource,
    /// Reported resolution in nanoseconds.
    pub resolution_ns: u64,
    /// Whether the clock is monotonic.
    pub is_monotonic: bool,
}
