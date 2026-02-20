use std::sync::atomic::{AtomicU64, Ordering};

/// Virtual clock state for deterministic time.
#[derive(Debug)]
pub(crate) struct VirtualClock {
    /// Epoch for virtual time in nanoseconds.
    epoch_nanos: u64,
    /// Tick size in nanoseconds.
    tick_nanos: u64,
    /// Current virtual wall time in nanoseconds.
    wall_nanos: AtomicU64,
    /// Current virtual monotonic time in nanoseconds.
    mono_nanos: AtomicU64,
}

impl VirtualClock {
    /// Create a new virtual clock.
    pub(crate) fn new(epoch_nanos: u64, tick_nanos: u64) -> Self {
        let tick_nanos = tick_nanos.max(1);
        Self {
            epoch_nanos,
            tick_nanos,
            wall_nanos: AtomicU64::new(epoch_nanos),
            mono_nanos: AtomicU64::new(0),
        }
    }

    /// Return the current virtual wall time in nanoseconds.
    pub(crate) fn wall_nanos(&self) -> u64 {
        self.wall_nanos.load(Ordering::Relaxed)
    }

    /// Return the current virtual monotonic time in nanoseconds.
    pub(crate) fn mono_nanos(&self) -> u64 {
        self.mono_nanos.load(Ordering::Relaxed)
    }

    /// Advance the virtual clock by a delta.
    pub(crate) fn advance(&self, delta_nanos: u64) -> u64 {
        // align the delta to the tick resolution
        let delta = align_to_tick(delta_nanos, self.tick_nanos);
        let wall = self.wall_nanos.fetch_add(delta, Ordering::Relaxed) + delta;
        let _ = self.mono_nanos.fetch_add(delta, Ordering::Relaxed) + delta;
        wall
    }

    /// Advance the virtual clock to the provided deadline.
    pub(crate) fn advance_to(&self, deadline_nanos: u64) -> u64 {
        // keep the current value if we are already past the deadline
        let now = self.wall_nanos();
        if deadline_nanos <= now {
            return now;
        }

        // advance by the remaining delta
        let delta = deadline_nanos.saturating_sub(now);
        self.advance(delta)
    }
}

impl Clone for VirtualClock {
    fn clone(&self) -> Self {
        Self {
            epoch_nanos: self.epoch_nanos,
            tick_nanos: self.tick_nanos,
            wall_nanos: AtomicU64::new(self.wall_nanos()),
            mono_nanos: AtomicU64::new(self.mono_nanos()),
        }
    }
}

/// Align a delta to the virtual tick interval.
fn align_to_tick(delta_nanos: u64, tick_nanos: u64) -> u64 {
    // align the delta to the tick interval
    let tick = tick_nanos.max(1);
    let remainder = delta_nanos % tick;
    if remainder == 0 {
        return delta_nanos;
    }

    delta_nanos.saturating_add(tick - remainder)
}
