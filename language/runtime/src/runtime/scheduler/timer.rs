use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};

use crate::platform::ResourceId;
use crate::platform::time::TimerClock;

/// Scheduled timer entry.
#[derive(Debug, Clone, Copy)]
pub struct Timer {
    /// Clock domain used for this timer deadline.
    pub clock: TimerClock,
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
    /// Clock domain used for this timer deadline.
    clock: TimerClock,
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
    /// Pending wall-clock timers.
    wall_timers: BinaryHeap<TimerEntry>,
    /// Pending monotonic timers.
    mono_timers: BinaryHeap<TimerEntry>,
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
        self.heap_for_clock(timer.clock).push(TimerEntry {
            clock: timer.clock,
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
    pub fn poll_ready(&mut self, wall_now_nanos: u64, mono_now_nanos: u64) -> Vec<Timer> {
        // collect timers that are ready to fire from both clock domains
        let mut ready = Vec::new();
        self.drain_ready_for_clock(TimerClock::Wall, wall_now_nanos, &mut ready);
        self.drain_ready_for_clock(TimerClock::Monotonic, mono_now_nanos, &mut ready);

        // deterministic ordering across clock domains
        ready.sort_by_key(|timer| {
            (
                timer_clock_order(timer.clock),
                timer.fire_at_nanos,
                timer.handle.0,
            )
        });

        ready
    }

    /// Return next wall and monotonic timer deadlines when they exist.
    pub fn next_deadlines(&mut self) -> (Option<u64>, Option<u64>) {
        let wall = self.peek_active_fire_at(TimerClock::Wall);
        let mono = self.peek_active_fire_at(TimerClock::Monotonic);

        (wall, mono)
    }

    /// Return true if any timers are active.
    pub fn has_pending_timers(&self) -> bool {
        !self.generations.is_empty()
    }

    /// Return the active heap for one clock domain.
    fn heap_for_clock(&mut self, clock: TimerClock) -> &mut BinaryHeap<TimerEntry> {
        match clock {
            TimerClock::Wall => &mut self.wall_timers,
            TimerClock::Monotonic => &mut self.mono_timers,
        }
    }

    /// Drain ready timers for one clock domain.
    fn drain_ready_for_clock(&mut self, clock: TimerClock, now_nanos: u64, ready: &mut Vec<Timer>) {
        loop {
            let Some(entry) = self.peek_active_entry(clock) else {
                break;
            };

            if entry.fire_at_nanos > now_nanos {
                break;
            }

            let entry = self
                .heap_for_clock(clock)
                .pop()
                .expect("timer entry should be present");
            let Some(current_generation) = self.generations.get(&entry.handle) else {
                continue;
            };
            if *current_generation != entry.generation {
                continue;
            }

            let timer = Timer {
                clock: entry.clock,
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

            // coalesce missed intervals into one callback and schedule the next future deadline
            let mut next_fire = entry.fire_at_nanos.saturating_add(interval_nanos);
            if next_fire <= now_nanos {
                let elapsed = now_nanos.saturating_sub(next_fire);
                let skipped_periods = elapsed / interval_nanos + 1;
                let skip_delta = interval_nanos.saturating_mul(skipped_periods);
                next_fire = next_fire.saturating_add(skip_delta);
            }
            let generation = self.next_generation.wrapping_add(1);
            self.next_generation = generation;
            self.generations.insert(entry.handle, generation);
            self.heap_for_clock(entry.clock).push(TimerEntry {
                clock: entry.clock,
                fire_at_nanos: next_fire,
                handle: entry.handle,
                interval_nanos: entry.interval_nanos,
                generation,
            });
        }
    }

    /// Return one active timer entry for one clock domain.
    fn peek_active_entry(&mut self, clock: TimerClock) -> Option<TimerEntry> {
        loop {
            let entry = self.heap_for_clock(clock).peek().copied()?;
            let Some(current_generation) = self.generations.get(&entry.handle) else {
                let _ = self.heap_for_clock(clock).pop();
                continue;
            };
            if *current_generation != entry.generation {
                let _ = self.heap_for_clock(clock).pop();
                continue;
            }

            return Some(entry);
        }
    }

    /// Return one active timer fire timestamp for one clock domain.
    fn peek_active_fire_at(&mut self, clock: TimerClock) -> Option<u64> {
        self.peek_active_entry(clock)
            .map(|entry| entry.fire_at_nanos)
    }
}

/// Return deterministic ordering for timer clock domains.
fn timer_clock_order(clock: TimerClock) -> u8 {
    match clock {
        TimerClock::Monotonic => 0,
        TimerClock::Wall => 1,
    }
}

#[cfg(test)]
mod tests {
    use super::{Timer, TimerQueue};
    use crate::platform::ResourceId;
    use crate::platform::time::TimerClock;

    /// Ensures repeating timers reschedule correctly.
    #[test]
    fn test_repeating_timer_reschedules() {
        let mut queue = TimerQueue::default();

        queue.schedule(Timer {
            clock: TimerClock::Monotonic,
            handle: ResourceId(1),
            fire_at_nanos: 10,
            interval_nanos: Some(10),
        });

        let first = queue.poll_ready(0, 10);
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].handle.0, 1);

        let second = queue.poll_ready(0, 19);
        assert!(second.is_empty());

        let third = queue.poll_ready(0, 20);
        assert_eq!(third.len(), 1);
        assert_eq!(third[0].handle.0, 1);
    }

    /// Ensures repeating timers coalesce missed intervals into one fire.
    #[test]
    fn test_repeating_timer_coalesces_missed_intervals() {
        let mut queue = TimerQueue::default();

        queue.schedule(Timer {
            clock: TimerClock::Monotonic,
            handle: ResourceId(2),
            fire_at_nanos: 10,
            interval_nanos: Some(10),
        });

        let ready = queue.poll_ready(0, 100);
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].handle.0, 2);

        let after_first = queue.poll_ready(0, 100);
        assert!(after_first.is_empty());

        let (_, next_deadline) = queue.next_deadlines();
        let next_deadline = next_deadline.expect("repeating timer should remain scheduled");
        assert!(next_deadline > 100);
    }

    /// Ensures wall and monotonic timers dispatch against independent clocks.
    #[test]
    fn test_poll_ready_uses_clock_specific_deadlines() {
        let mut queue = TimerQueue::default();
        queue.schedule(Timer {
            clock: TimerClock::Wall,
            handle: ResourceId(10),
            fire_at_nanos: 100,
            interval_nanos: None,
        });
        queue.schedule(Timer {
            clock: TimerClock::Monotonic,
            handle: ResourceId(11),
            fire_at_nanos: 50,
            interval_nanos: None,
        });

        let first = queue.poll_ready(0, 60);
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].handle.0, 11);

        let second = queue.poll_ready(120, 60);
        assert_eq!(second.len(), 1);
        assert_eq!(second[0].handle.0, 10);
    }
}
