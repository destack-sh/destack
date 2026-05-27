use std::cmp::Ordering;
use std::collections::BinaryHeap;

use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};

use super::EventLoop;
use crate::diagnostic::RuntimeResult;
use crate::host::ResourceId;
use crate::host::time::TimerClock;
use crate::runtime::time::Nanos;

/// Pending wake for one timer resource.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScheduledTimer {
    /// Host resource that owns this timer.
    pub resource_id: ResourceId,
    /// Next fire deadline.
    pub deadline: TimerDeadline,
    /// Interval for repeating timers.
    pub interval: Option<Nanos>,
}

impl ScheduledTimer {
    /// Return one deterministic sort key for this timer.
    pub const fn sort_key(&self) -> (u64, u64) {
        (self.resource_id.worker_id.0, self.resource_id.local_id)
    }
}

/// Timer deadline in one explicit clock domain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimerDeadline {
    /// Clock domain that owns the deadline timestamp.
    pub clock: TimerClock,
    /// Absolute deadline in the selected clock domain.
    pub at: Nanos,
}

impl TimerDeadline {
    /// Return deterministic ordering for timer clock domains.
    pub const fn clock_rank(self) -> u8 {
        match self.clock {
            TimerClock::Monotonic => 0,
            TimerClock::Wall => 1,
        }
    }
}

/// Timer queue for scheduled event-loop timers.
#[derive(Debug, Default)]
pub(super) struct TimerQueue {
    /// Pending wall-clock timers.
    wall: DeadlineQueue,
    /// Pending monotonic timers.
    monotonic: DeadlineQueue,
    /// Latest live generation for each active timer resource.
    active_generations: FxHashMap<ResourceId, u64>,
    /// Next generation counter.
    next_generation: u64,
}

impl TimerQueue {
    /// Schedule a timer in the queue.
    pub(super) fn schedule(&mut self, timer: ScheduledTimer) {
        let generation = self.next_generation.wrapping_add(1);
        self.next_generation = generation;
        self.active_generations
            .insert(timer.resource_id, generation);

        match timer.deadline.clock {
            TimerClock::Wall => self.wall.schedule(timer, generation),
            TimerClock::Monotonic => self.monotonic.schedule(timer, generation),
        }
    }

    /// Cancel a timer by resource id.
    pub(super) fn cancel(&mut self, resource_id: ResourceId) {
        self.active_generations.remove(&resource_id);
    }

    /// Pop the next ready timer for the current wall and monotonic instants.
    pub(super) fn pop_ready(&mut self, wall_now: Nanos, mono_now: Nanos) -> Option<ScheduledTimer> {
        let wall = self.wall.peek_ready(&self.active_generations, wall_now);
        let monotonic = self
            .monotonic
            .peek_ready(&self.active_generations, mono_now);

        let (clock, now) = match (wall, monotonic) {
            (Some(wall), Some(monotonic)) => {
                if wall.ready_key() <= monotonic.ready_key() {
                    (TimerClock::Wall, wall_now)
                } else {
                    (TimerClock::Monotonic, mono_now)
                }
            }
            (Some(_), None) => (TimerClock::Wall, wall_now),
            (None, Some(_)) => (TimerClock::Monotonic, mono_now),
            (None, None) => return None,
        };

        self.pop_ready_from_queue(clock, now)
    }

    /// Return next wall and monotonic timer deadlines when they exist.
    pub(super) fn next_deadlines(&mut self) -> (Option<Nanos>, Option<Nanos>) {
        let wall = self.wall.next_deadline(&self.active_generations);
        let monotonic = self.monotonic.next_deadline(&self.active_generations);

        (wall, monotonic)
    }

    /// Return true if any active timer is ready at the given instants.
    pub(super) fn has_ready(&mut self, wall_now: Nanos, mono_now: Nanos) -> bool {
        self.wall
            .peek_ready(&self.active_generations, wall_now)
            .is_some()
            || self
                .monotonic
                .peek_ready(&self.active_generations, mono_now)
                .is_some()
    }

    /// Return true if any timers are active.
    pub(super) fn has_pending_timers(&self) -> bool {
        !self.active_generations.is_empty()
    }

    /// Return whether one timer resource is still active.
    pub(super) fn has_active_timer(&self, resource_id: ResourceId) -> bool {
        self.active_generations.contains_key(&resource_id)
    }

    /// Capture all currently active timers in deterministic order.
    pub(super) fn image(&self) -> Vec<ScheduledTimer> {
        let mut active = Vec::new();
        self.wall
            .collect_active(&self.active_generations, &mut active);
        self.monotonic
            .collect_active(&self.active_generations, &mut active);

        active.sort_by_key(|timer| {
            (
                timer.deadline.clock_rank(),
                timer.deadline.at,
                timer.sort_key(),
            )
        });

        active
    }

    /// Restore active timers from one immutable timer image.
    pub(super) fn restore_image(&mut self, timers: &[ScheduledTimer]) {
        self.wall.clear();
        self.monotonic.clear();
        self.active_generations.clear();
        self.next_generation = 0;

        for timer in timers {
            self.schedule(*timer);
        }
    }

    /// Pop one ready entry from the selected clock domain.
    #[inline(never)]
    fn pop_ready_from_queue(&mut self, clock: TimerClock, now: Nanos) -> Option<ScheduledTimer> {
        let entry = match clock {
            TimerClock::Wall => self.wall.pop_active(&self.active_generations),
            TimerClock::Monotonic => self.monotonic.pop_active(&self.active_generations),
        }?;

        let timer = entry.scheduled_timer();
        let Some(interval) = entry.interval else {
            self.active_generations.remove(&entry.resource_id);
            return Some(timer);
        };

        if interval.get() == 0 {
            self.active_generations.remove(&entry.resource_id);
            return Some(timer);
        }

        self.reschedule_repeating_entry(entry, now, interval);
        Some(timer)
    }

    /// Reschedule one repeating timer after a dispatch.
    fn reschedule_repeating_entry(&mut self, entry: TimerEntry, now: Nanos, interval: Nanos) {
        let mut next_fire = entry.deadline.at.saturating_add(interval);
        if next_fire <= now {
            let elapsed = now.saturating_sub(next_fire);
            let skipped_periods = elapsed.get() / interval.get() + 1;
            let skip_delta = interval.get().saturating_mul(skipped_periods);
            next_fire = next_fire.saturating_add(Nanos::new(skip_delta));
        }

        let timer = ScheduledTimer {
            resource_id: entry.resource_id,
            deadline: TimerDeadline {
                clock: entry.deadline.clock,
                at: next_fire,
            },
            interval: entry.interval,
        };

        self.schedule(timer);
    }
}

/// Deadline queue for one clock domain.
#[derive(Debug, Default)]
struct DeadlineQueue {
    /// Pending timer entries ordered by deadline.
    entries: BinaryHeap<TimerEntry>,
}

impl DeadlineQueue {
    /// Insert one scheduled timer.
    fn schedule(&mut self, timer: ScheduledTimer, generation: u64) {
        self.entries.push(TimerEntry {
            deadline: timer.deadline,
            resource_id: timer.resource_id,
            interval: timer.interval,
            generation,
        });
    }

    /// Clear all queued timer entries.
    fn clear(&mut self) {
        self.entries.clear();
    }

    /// Collect active timers from this queue.
    fn collect_active(
        &self,
        active_generations: &FxHashMap<ResourceId, u64>,
        output: &mut Vec<ScheduledTimer>,
    ) {
        for entry in &self.entries {
            let Some(current_generation) = active_generations.get(&entry.resource_id) else {
                continue;
            };
            if *current_generation != entry.generation {
                continue;
            }

            output.push(entry.scheduled_timer());
        }
    }

    /// Return one ready active entry.
    fn peek_ready(
        &mut self,
        active_generations: &FxHashMap<ResourceId, u64>,
        now: Nanos,
    ) -> Option<TimerEntry> {
        let entry = self.peek_active(active_generations)?;
        if entry.deadline.at <= now {
            Some(entry)
        } else {
            None
        }
    }

    /// Return the next active fire timestamp.
    fn next_deadline(&mut self, active_generations: &FxHashMap<ResourceId, u64>) -> Option<Nanos> {
        self.peek_active(active_generations)
            .map(|entry| entry.deadline.at)
    }

    /// Return the next active entry and prune stale entries.
    fn peek_active(
        &mut self,
        active_generations: &FxHashMap<ResourceId, u64>,
    ) -> Option<TimerEntry> {
        loop {
            let entry = self.entries.peek().copied()?;
            let Some(current_generation) = active_generations.get(&entry.resource_id) else {
                self.entries.pop();
                continue;
            };
            if *current_generation != entry.generation {
                self.entries.pop();
                continue;
            }

            return Some(entry);
        }
    }

    /// Pop the next active entry.
    fn pop_active(
        &mut self,
        active_generations: &FxHashMap<ResourceId, u64>,
    ) -> Option<TimerEntry> {
        loop {
            let entry = self.entries.pop()?;
            let Some(current_generation) = active_generations.get(&entry.resource_id) else {
                continue;
            };
            if *current_generation != entry.generation {
                continue;
            }

            return Some(entry);
        }
    }
}

/// Internal timer entry stored in the priority queue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TimerEntry {
    /// Fire deadline for the timer.
    deadline: TimerDeadline,
    /// Host resource that owns this timer.
    resource_id: ResourceId,
    /// Interval for repeating timers.
    interval: Option<Nanos>,
    /// Generation for stale entry detection.
    generation: u64,
}

impl TimerEntry {
    /// Return one scheduled timer payload.
    const fn scheduled_timer(self) -> ScheduledTimer {
        ScheduledTimer {
            resource_id: self.resource_id,
            deadline: self.deadline,
            interval: self.interval,
        }
    }

    /// Return one deterministic readiness key.
    const fn ready_key(self) -> (u8, u64, u64, u64) {
        (
            self.deadline.clock_rank(),
            self.deadline.at.get(),
            self.resource_id.worker_id.0,
            self.resource_id.local_id,
        )
    }
}

impl Ord for TimerEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .deadline
            .at
            .cmp(&self.deadline.at)
            .then_with(|| {
                other
                    .resource_id
                    .worker_id
                    .0
                    .cmp(&self.resource_id.worker_id.0)
            })
            .then_with(|| other.resource_id.local_id.cmp(&self.resource_id.local_id))
            .then_with(|| other.generation.cmp(&self.generation))
    }
}

impl PartialOrd for TimerEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl EventLoop {
    /// Normalize one timer deadline using scheduler options.
    pub(super) fn normalize_deadline(&self, deadline: Nanos) -> Nanos {
        deadline
    }

    /// Schedule a timer in the runtime queue.
    pub fn schedule_timer(&mut self, timer: ScheduledTimer) -> RuntimeResult<()> {
        let deadline = TimerDeadline {
            clock: timer.deadline.clock,
            at: self.normalize_deadline(timer.deadline.at),
        };
        let interval = timer.interval.map(|interval| {
            if interval.get() <= 1 {
                return interval;
            }

            let interval = self.normalize_deadline(interval);
            if interval.get() == 0 {
                return Nanos::new(1);
            }

            interval
        });
        let timer = ScheduledTimer {
            resource_id: timer.resource_id,
            deadline,
            interval,
        };

        self.timers.schedule(timer);
        Ok(())
    }

    /// Cancel a timer by resource id.
    pub fn cancel_timer(&mut self, resource_id: ResourceId) -> RuntimeResult<()> {
        self.timers.cancel(resource_id);
        Ok(())
    }

    /// Pop one timer that is ready at the given time.
    pub fn pop_ready_timer(
        &mut self,
        wall_now: Nanos,
        mono_now: Nanos,
    ) -> RuntimeResult<Option<ScheduledTimer>> {
        Ok(self.timers.pop_ready(wall_now, mono_now))
    }
}

#[cfg(test)]
mod tests {
    use super::{ScheduledTimer, TimerDeadline, TimerQueue};
    use crate::host::ResourceId;
    use crate::host::time::TimerClock;
    use crate::runtime::WorkerId;
    use crate::runtime::time::Nanos;

    const TEST_WORKER_ID: WorkerId = WorkerId(1);

    /// Ensures repeating timers reschedule correctly.
    #[test]
    fn test_repeating_timer_reschedules() {
        let mut queue = TimerQueue::default();
        queue.schedule(ScheduledTimer {
            resource_id: ResourceId::new(TEST_WORKER_ID, 1),
            deadline: TimerDeadline {
                clock: TimerClock::Monotonic,
                at: Nanos::new(10),
            },
            interval: Some(Nanos::new(10)),
        });

        let first = queue
            .pop_ready(Nanos::new(0), Nanos::new(10))
            .expect("timer should fire");
        assert_eq!(first.sort_key(), (TEST_WORKER_ID.0, 1));

        let second = queue.pop_ready(Nanos::new(0), Nanos::new(19));
        assert!(second.is_none());

        let third = queue
            .pop_ready(Nanos::new(0), Nanos::new(20))
            .expect("timer should fire again");
        assert_eq!(third.sort_key(), (TEST_WORKER_ID.0, 1));
    }

    /// Ensures repeating timers coalesce missed intervals into one fire.
    #[test]
    fn test_repeating_timer_coalesces_missed_intervals() {
        let mut queue = TimerQueue::default();
        queue.schedule(ScheduledTimer {
            resource_id: ResourceId::new(TEST_WORKER_ID, 2),
            deadline: TimerDeadline {
                clock: TimerClock::Monotonic,
                at: Nanos::new(10),
            },
            interval: Some(Nanos::new(10)),
        });

        let ready = queue
            .pop_ready(Nanos::new(0), Nanos::new(100))
            .expect("timer should fire");
        assert_eq!(ready.sort_key(), (TEST_WORKER_ID.0, 2));

        let after_first = queue.pop_ready(Nanos::new(0), Nanos::new(100));
        assert!(after_first.is_none());

        let (_, next_deadline) = queue.next_deadlines();
        let next_deadline = next_deadline.expect("repeating timer should remain scheduled");
        assert!(next_deadline > Nanos::new(100));
    }

    /// Ensures wall and monotonic timers dispatch against independent clocks.
    #[test]
    fn test_pop_ready_uses_clock_specific_deadlines() {
        let mut queue = TimerQueue::default();
        queue.schedule(ScheduledTimer {
            resource_id: ResourceId::new(TEST_WORKER_ID, 10),
            deadline: TimerDeadline {
                clock: TimerClock::Wall,
                at: Nanos::new(100),
            },
            interval: None,
        });
        queue.schedule(ScheduledTimer {
            resource_id: ResourceId::new(TEST_WORKER_ID, 11),
            deadline: TimerDeadline {
                clock: TimerClock::Monotonic,
                at: Nanos::new(50),
            },
            interval: None,
        });

        let first = queue
            .pop_ready(Nanos::new(0), Nanos::new(60))
            .expect("monotonic timer should fire");
        assert_eq!(first.sort_key(), (TEST_WORKER_ID.0, 11));

        let second = queue
            .pop_ready(Nanos::new(120), Nanos::new(60))
            .expect("wall timer should fire");
        assert_eq!(second.sort_key(), (TEST_WORKER_ID.0, 10));
    }
}
