use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::host::core as host_core;

/// Source of host wall and monotonic clock behavior.
pub(crate) trait HostClockSource: std::fmt::Debug + Send + Sync {
    /// Return one wall-clock sample in nanoseconds.
    fn wall_nanos(&self) -> u64;

    /// Return one monotonic-clock sample in nanoseconds.
    fn mono_nanos(&self) -> u64;

    /// Sleep for one duration in nanoseconds.
    fn sleep_nanos(&self, duration_nanos: u64);

    /// Sleep until one wall-clock deadline in nanoseconds.
    fn sleep_until_wall_nanos(&self, deadline_nanos: u64) {
        // skip if the deadline has already passed
        let now = self.wall_nanos();
        if deadline_nanos <= now {
            return;
        }

        // sleep for the remaining wall delta
        let duration_nanos = deadline_nanos.saturating_sub(now);
        self.sleep_nanos(duration_nanos);
    }
}

/// Host-backed clock source using system wall and monotonic clocks.
#[derive(Debug, Clone)]
pub(crate) struct SystemHostClockSource;

impl SystemHostClockSource {
    /// Create one system host clock source.
    pub(crate) fn new() -> Self {
        Self
    }
}

impl HostClockSource for SystemHostClockSource {
    /// Return one wall-clock sample in nanoseconds.
    fn wall_nanos(&self) -> u64 {
        let duration = match SystemTime::now().duration_since(UNIX_EPOCH) {
            Ok(duration) => duration,
            Err(_) => Duration::from_secs(0),
        };

        nanos_from_duration(duration)
    }

    /// Return one monotonic-clock sample in nanoseconds.
    fn mono_nanos(&self) -> u64 {
        host_core::monotonic_now_ns()
    }

    /// Sleep for one duration in nanoseconds.
    fn sleep_nanos(&self, duration_nanos: u64) {
        // skip zero length sleeps
        if duration_nanos == 0 {
            return;
        }

        // sleep using the host timer
        std::thread::sleep(Duration::from_nanos(duration_nanos));
    }
}

/// Host clock state for wall and monotonic time.
#[derive(Debug, Clone)]
pub(crate) struct HostClock {
    /// Source for wall and monotonic clock operations.
    source: Arc<dyn HostClockSource>,
}

impl HostClock {
    /// Create a host clock.
    pub(crate) fn new() -> Self {
        Self::with_source(Arc::new(SystemHostClockSource::new()))
    }

    /// Create a host clock from one explicit source.
    pub(crate) fn with_source(source: Arc<dyn HostClockSource>) -> Self {
        Self { source }
    }

    /// Return the current wall time in nanoseconds.
    pub(crate) fn wall_nanos(&self) -> u64 {
        self.source.wall_nanos()
    }

    /// Return the current monotonic time in nanoseconds.
    pub(crate) fn mono_nanos(&self) -> u64 {
        self.source.mono_nanos()
    }

    /// Sleep for the given duration in nanoseconds.
    pub(crate) fn sleep_nanos(&self, duration_nanos: u64) {
        self.source.sleep_nanos(duration_nanos);
    }

    /// Sleep until the provided deadline in nanoseconds.
    pub(crate) fn sleep_until_nanos(&self, deadline_nanos: u64) {
        self.source.sleep_until_wall_nanos(deadline_nanos);
    }
}

impl Default for HostClock {
    /// Create the default host clock.
    fn default() -> Self {
        Self::new()
    }
}

/// Convert a duration to nanoseconds, saturating on overflow.
fn nanos_from_duration(duration: Duration) -> u64 {
    let nanos = duration.as_nanos();

    nanos.min(u128::from(u64::MAX)) as u64
}
