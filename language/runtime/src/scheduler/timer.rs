use crate::platform::ResourceId;

/// Scheduled timer entry.
#[derive(Debug, Clone, Copy)]
pub struct Timer {
    /// Handle for this timer.
    pub handle: ResourceId,
    /// Next fire time in nanoseconds.
    pub fire_at_nanos: u64,
    /// Interval in nanoseconds for repeating timers.
    pub interval_nanos: Option<u64>,
}

/// Timer queue used by platform bindings.
#[derive(Debug, Default)]
pub struct TimerQueue {
    /// Pending timers.
    pub timers: Vec<Timer>,
}

impl TimerQueue {
    /// Schedule a timer in the queue.
    pub fn schedule(&mut self, timer: Timer) {
        self.timers.push(timer);
    }

    /// Cancel a timer by handle.
    pub fn cancel(&mut self, handle: ResourceId) {
        self.timers.retain(|timer| timer.handle != handle);
    }

    /// Drain ready timers that should fire at the given time.
    pub fn poll_ready(&mut self, now_nanos: u64) -> Vec<Timer> {
        // collect timers that are ready to fire
        let mut ready = Vec::new();
        self.timers.retain(|timer| {
            if timer.fire_at_nanos <= now_nanos {
                ready.push(*timer);
                return false;
            }

            true
        });

        ready
    }
}
