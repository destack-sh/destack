use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};

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

/// Internal timer entry stored in the priority queue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TimerEntry {
    /// Fire time for the timer.
    fire_at_nanos: u64,
    /// Handle for this timer.
    handle: ResourceId,
    /// Interval for repeating timers.
    interval_nanos: Option<u64>,
    /// Generation for stale entry detection.
    generation: u64,
}

impl Ord for TimerEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .fire_at_nanos
            .cmp(&self.fire_at_nanos)
            .then_with(|| other.generation.cmp(&self.generation))
    }
}

impl PartialOrd for TimerEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Timer queue used by platform bindings.
#[derive(Debug, Default)]
pub struct TimerQueue {
    /// Pending timers.
    timers: BinaryHeap<TimerEntry>,
    /// Current generation for active timers.
    generations: HashMap<ResourceId, u64>,
    /// Next generation counter.
    next_generation: u64,
}

impl TimerQueue {
    /// Schedule a timer in the queue.
    pub fn schedule(&mut self, timer: Timer) {
        let generation = self.next_generation.wrapping_add(1);
        self.next_generation = generation;
        self.generations.insert(timer.handle, generation);
        self.timers.push(TimerEntry {
            fire_at_nanos: timer.fire_at_nanos,
            handle: timer.handle,
            interval_nanos: timer.interval_nanos,
            generation,
        });
    }

    /// Cancel a timer by handle.
    pub fn cancel(&mut self, handle: ResourceId) {
        self.generations.remove(&handle);
    }

    /// Drain ready timers that should fire at the given time.
    pub fn poll_ready(&mut self, now_nanos: u64) -> Vec<Timer> {
        // collect timers that are ready to fire
        let mut ready = Vec::new();
        loop {
            let Some(entry) = self.timers.peek().copied() else {
                break;
            };

            if entry.fire_at_nanos > now_nanos {
                break;
            }

            let entry = self.timers.pop().expect("timer entry should be present");
            let Some(current_generation) = self.generations.get(&entry.handle) else {
                continue;
            };
            if *current_generation != entry.generation {
                continue;
            }

            let timer = Timer {
                handle: entry.handle,
                fire_at_nanos: entry.fire_at_nanos,
                interval_nanos: entry.interval_nanos,
            };
            ready.push(timer);

            let Some(interval_nanos) = entry.interval_nanos else {
                self.generations.remove(&entry.handle);
                continue;
            };

            if interval_nanos == 0 {
                self.generations.remove(&entry.handle);
                continue;
            }

            let next_fire = entry.fire_at_nanos.saturating_add(interval_nanos);
            let generation = self.next_generation.wrapping_add(1);
            self.next_generation = generation;
            self.generations.insert(entry.handle, generation);
            self.timers.push(TimerEntry {
                fire_at_nanos: next_fire,
                handle: entry.handle,
                interval_nanos: entry.interval_nanos,
                generation,
            });
        }

        ready
    }

    /// Return true if any timers are active.
    pub fn has_pending_timers(&self) -> bool {
        !self.generations.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::{Timer, TimerQueue};
    use crate::platform::ResourceId;

    /// Ensures repeating timers reschedule correctly.
    #[test]
    fn test_repeating_timer_reschedules() {
        let mut queue = TimerQueue::default();

        queue.schedule(Timer {
            handle: ResourceId(1),
            fire_at_nanos: 10,
            interval_nanos: Some(10),
        });

        let first = queue.poll_ready(10);
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].handle.0, 1);

        let second = queue.poll_ready(19);
        assert!(second.is_empty());

        let third = queue.poll_ready(20);
        assert_eq!(third.len(), 1);
        assert_eq!(third[0].handle.0, 1);
    }
}
