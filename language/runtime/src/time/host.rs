use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Host clock state for wall and monotonic time.
#[derive(Debug, Clone)]
pub struct HostClock {
    /// Monotonic origin for elapsed time.
    origin: Instant,
}

impl HostClock {
    /// Create a host clock.
    pub fn new() -> Self {
        Self {
            origin: Instant::now(),
        }
    }

    /// Return the current wall time in nanoseconds.
    pub fn wall_nanos(&self) -> u64 {
        // convert wall time to nanoseconds
        let duration = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_else(|_| Duration::from_secs(0));
        nanos_from_duration(duration)
    }

    /// Return the current monotonic time in nanoseconds.
    pub fn mono_nanos(&self) -> u64 {
        // convert elapsed time to nanoseconds
        nanos_from_duration(self.origin.elapsed())
    }

    /// Sleep for the given duration in nanoseconds.
    pub fn sleep_nanos(&self, duration_nanos: u64) {
        // skip zero length sleeps
        if duration_nanos == 0 {
            return;
        }

        // sleep using the host timer
        std::thread::sleep(Duration::from_nanos(duration_nanos));
    }

    /// Sleep until the provided deadline in nanoseconds.
    pub fn sleep_until_nanos(&self, deadline_nanos: u64) {
        // skip if the deadline has passed
        let now = self.wall_nanos();
        if deadline_nanos <= now {
            return;
        }

        // sleep for the remaining delta
        let delta = deadline_nanos.saturating_sub(now);
        self.sleep_nanos(delta);
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
    // clamp duration to u64 nanoseconds
    let nanos = duration.as_nanos();
    u64::try_from(nanos).unwrap_or(u64::MAX)
}
