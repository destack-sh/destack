use std::sync::atomic::{AtomicU64, Ordering};

/// Runtime clock sources and time policies.
#[derive(Debug, Clone, Default)]
pub struct Clock {
    /// Virtual clock used for deterministic scheduling.
    pub virtual_clock: VirtualClock,
    /// Monotonic clock used for duration measurements.
    pub monotonic_clock: MonotonicClock,
    /// Wall clock policy used for host time access.
    pub wall_clock_policy: WallClockPolicy,
}

/// Virtual clock state for deterministic time.
#[derive(Debug)]
pub struct VirtualClock {
    /// Epoch for virtual time in nanoseconds.
    pub epoch_nanos: u64,
    /// Tick size in nanoseconds.
    pub tick_nanos: u64,
    /// Current virtual time in nanoseconds.
    pub now_nanos: AtomicU64,
}

impl Default for VirtualClock {
    fn default() -> Self {
        Self {
            epoch_nanos: 0,
            tick_nanos: 1_000_000,
            now_nanos: AtomicU64::new(0),
        }
    }
}

impl Clone for VirtualClock {
    fn clone(&self) -> Self {
        Self {
            epoch_nanos: self.epoch_nanos,
            tick_nanos: self.tick_nanos,
            now_nanos: AtomicU64::new(self.now_nanos()),
        }
    }
}

/// Monotonic clock state for duration measurements.
#[derive(Debug, Default)]
pub struct MonotonicClock {
    /// Current monotonic time in nanoseconds.
    pub now_nanos: AtomicU64,
}

impl Clone for MonotonicClock {
    fn clone(&self) -> Self {
        Self {
            now_nanos: AtomicU64::new(self.now_nanos()),
        }
    }
}

impl MonotonicClock {
    /// Read the current monotonic time in nanoseconds.
    pub fn now_nanos(&self) -> u64 {
        self.now_nanos.load(Ordering::Relaxed)
    }

    /// Advance the monotonic clock by a delta.
    pub fn advance(&self, delta_nanos: u64) -> u64 {
        self.now_nanos.fetch_add(delta_nanos, Ordering::Relaxed) + delta_nanos
    }
}

impl VirtualClock {
    /// Read the current virtual time in nanoseconds.
    pub fn now_nanos(&self) -> u64 {
        self.now_nanos.load(Ordering::Relaxed)
    }

    /// Advance the virtual time by a delta.
    pub fn advance(&self, delta_nanos: u64) -> u64 {
        self.now_nanos.fetch_add(delta_nanos, Ordering::Relaxed) + delta_nanos
    }
}

impl Clock {
    /// Return the current virtual wall time in nanoseconds.
    pub fn wall_nanos(&self) -> u64 {
        self.virtual_clock.now_nanos()
    }

    /// Return the current monotonic time in nanoseconds.
    pub fn mono_nanos(&self) -> u64 {
        self.monotonic_clock.now_nanos()
    }

    /// Advance virtual and monotonic clocks for deterministic sleeps.
    pub fn sleep_nanos(&self, duration_nanos: u64) {
        let _ = self.virtual_clock.advance(duration_nanos);
        let _ = self.monotonic_clock.advance(duration_nanos);
    }

    /// Advance clocks until the virtual deadline is reached.
    pub fn sleep_until_nanos(&self, deadline_nanos: u64) {
        let now = self.virtual_clock.now_nanos();
        if deadline_nanos > now {
            self.sleep_nanos(deadline_nanos - now);
        }
    }
}

/// Policy for accessing wall clock time.
#[derive(Debug, Clone, Copy, Default)]
pub enum WallClockPolicy {
    /// Wall clock reads are forbidden.
    #[default]
    Disabled,
    /// Wall clock reads are allowed but logged.
    Logged,
    /// Wall clock reads pass through without logging.
    Passthrough,
}
