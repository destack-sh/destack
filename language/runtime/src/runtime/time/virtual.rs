use std::sync::atomic::{AtomicU64, Ordering};

use super::{Instant, Nanos};

/// Virtual clock state for deterministic time.
#[derive(Debug)]
pub(crate) struct VirtualClock {
    /// Epoch for virtual time in nanoseconds.
    epoch_nanos: u64,
    /// Current virtual wall time in nanoseconds.
    wall_nanos: AtomicU64,
    /// Current virtual monotonic time in nanoseconds.
    mono_nanos: AtomicU64,
}

impl VirtualClock {
    /// Create a new virtual clock.
    pub(crate) fn new(epoch_nanos: u64) -> Self {
        Self {
            epoch_nanos,
            wall_nanos: AtomicU64::new(epoch_nanos),
            mono_nanos: AtomicU64::new(0),
        }
    }

    /// Return the current virtual wall time in nanoseconds.
    pub(crate) fn wall(&self) -> Nanos {
        Nanos::new(self.wall_nanos.load(Ordering::Relaxed))
    }

    /// Return the current virtual monotonic time in nanoseconds.
    pub(crate) fn mono(&self) -> Nanos {
        Nanos::new(self.mono_nanos.load(Ordering::Relaxed))
    }

    /// Advance the virtual clock by a delta.
    pub(crate) fn advance(&self, delta: Nanos) -> Instant {
        let delta_nanos = delta.get();
        let wall = self.wall_nanos.fetch_add(delta_nanos, Ordering::Relaxed) + delta_nanos;
        let _ = self.mono_nanos.fetch_add(delta_nanos, Ordering::Relaxed) + delta_nanos;
        Instant::new(wall)
    }

    /// Advance the virtual clock to the provided deadline.
    pub(crate) fn advance_to(&self, deadline: Instant) -> Instant {
        // keep the current value if we are already past the deadline
        let now = Instant::from_nanos(self.wall());
        if deadline <= now {
            return now;
        }

        // advance by the remaining delta
        let delta = deadline.saturating_sub(now);
        self.advance(delta)
    }

    /// Restore one captured virtual clock state.
    pub(crate) fn restore_snapshot(&self, wall: Instant, mono: Instant) {
        self.wall_nanos.store(wall.get(), Ordering::Relaxed);
        self.mono_nanos.store(mono.get(), Ordering::Relaxed);
    }
}

impl Clone for VirtualClock {
    fn clone(&self) -> Self {
        Self {
            epoch_nanos: self.epoch_nanos,
            wall_nanos: AtomicU64::new(self.wall().get()),
            mono_nanos: AtomicU64::new(self.mono().get()),
        }
    }
}
