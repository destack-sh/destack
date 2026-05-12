use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};

use serde::{Deserialize, Serialize};

use super::EventLoop;
use crate::diagnostic::RuntimeResult;
use crate::platform::ResourceId;
use crate::platform::time::TimerClock;
use crate::runtime::time::Nanos;

/// Handle for one event-loop timer owner.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TimerHandle {
    /// Timer owned by one platform resource.
    Resource(ResourceId),
    /// Timer owned by one internal runtime subsystem.
    Internal(u64),
}

impl TimerHandle {
    /// Return the resource id when this timer is resource owned.
    pub const fn resource_id(self) -> Option<ResourceId> {
        match self {
            Self::Resource(handle) => Some(handle),
            Self::Internal(_) => None,
        }
    }

    /// Return the internal id when this timer is runtime owned.
    pub const fn internal_id(self) -> Option<u64> {
        match self {
            Self::Resource(_) => None,
            Self::Internal(handle) => Some(handle),
        }
    }

    /// Return one deterministic sort key for this handle.
    pub const fn sort_key(self) -> (u8, u64, u64) {
        match self {
            Self::Resource(handle) => (0, handle.worker_id.0, handle.local_id),
            Self::Internal(handle) => (1, 0, handle),
        }
    }
}

impl From<ResourceId> for TimerHandle {
    fn from(handle: ResourceId) -> Self {
        Self::Resource(handle)
    }
}

/// Timer deadline in one explicit clock domain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimerDeadline {
    /// Clock domain used for this deadline.
    pub clock: TimerClock,
    /// Absolute deadline in the selected clock domain.
    pub at: Nanos,
}

/// Scheduled timer entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Timer {
    /// Handle for this timer.
    pub handle: TimerHandle,
    /// Next fire deadline.
    pub deadline: TimerDeadline,
    /// Interval for repeating timers.
    pub interval: Option<Nanos>,
}

/// Internal timer entry stored in the priority queue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TimerEntry {
    /// Fire deadline for the timer.
    deadline: TimerDeadline,
    /// Handle for this timer.
    handle: TimerHandle,
    /// Interval for repeating timers.
    interval: Option<Nanos>,
    /// Generation for stale entry detection.
    generation: u64,
}

impl Ord for TimerEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .deadline
            .at
            .cmp(&self.deadline.at)
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
    /// Latest live generation for each active timer handle.
    ///
    /// this lets the heap keep stale entries after reschedule or cancel,
    /// while dispatch only observes the newest live timer state
    active_generations: HashMap<TimerHandle, u64>,
    /// Next generation counter.
    next_generation: u64,
}

impl TimerQueue {
    /// Schedule a timer in the queue.
    pub fn schedule(&mut self, timer: Timer) {
        let generation = self.next_generation.wrapping_add(1);
        self.next_generation = generation;
        self.active_generations.insert(timer.handle, generation);
        self.heap_for_clock(timer.deadline.clock).push(TimerEntry {
            deadline: timer.deadline,
            handle: timer.handle,
            interval: timer.interval,
            generation,
        });
    }

    /// Cancel a timer by handle.
    pub fn cancel(&mut self, handle: TimerHandle) {
        self.active_generations.remove(&handle);
    }

    /// Drain ready timers that should fire at the given wall and monotonic instants.
    ///
    /// wall and monotonic timers are tracked in separate heaps because their deadlines
    /// are not directly comparable without a projection through the current clock state
    pub fn poll_ready(&mut self, wall_now: Nanos, mono_now: Nanos) -> Vec<Timer> {
        // collect timers that are ready to fire from both clock domains
        let mut ready = Vec::new();
        self.drain_ready_for_clock(TimerClock::Wall, wall_now, &mut ready);
        self.drain_ready_for_clock(TimerClock::Monotonic, mono_now, &mut ready);

        // deterministic ordering across clock domains
        ready.sort_by_key(|timer| {
            (
                self.clock_order(timer.deadline.clock),
                timer.deadline.at,
                timer.handle.sort_key(),
            )
        });

        ready
    }

    /// Return next wall and monotonic timer deadlines when they exist.
    pub fn next_deadlines(&mut self) -> (Option<Nanos>, Option<Nanos>) {
        let wall = self.peek_active_fire_at(TimerClock::Wall);
        let mono = self.peek_active_fire_at(TimerClock::Monotonic);

        (wall, mono)
    }

    /// Return true if any timers are active.
    pub fn has_pending_timers(&self) -> bool {
        !self.active_generations.is_empty()
    }

    /// Capture all currently active timers in deterministic order.
    pub fn image(&self) -> Vec<Timer> {
        // collect active timers from both heaps
        let mut active = Vec::new();
        self.collect_active_timers(&self.wall_timers, &mut active);
        self.collect_active_timers(&self.mono_timers, &mut active);

        // return them in the same stable order as dispatch
        active.sort_by_key(|timer| {
            (
                self.clock_order(timer.deadline.clock),
                timer.deadline.at,
                timer.handle.sort_key(),
            )
        });

        active
    }

    /// Restore active timers from one immutable timer image.
    pub fn restore_image(&mut self, timers: &[Timer]) {
        // reset queue state before rebuilding
        self.wall_timers.clear();
        self.mono_timers.clear();
        self.active_generations.clear();
        self.next_generation = 0;

        // reschedule active timers
        for timer in timers {
            self.schedule(*timer);
        }
    }

    /// Return the active heap for one clock domain.
    fn heap_for_clock(&mut self, clock: TimerClock) -> &mut BinaryHeap<TimerEntry> {
        match clock {
            TimerClock::Wall => &mut self.wall_timers,
            TimerClock::Monotonic => &mut self.mono_timers,
        }
    }

    /// Collect active timers from one heap.
    fn collect_active_timers(&self, heap: &BinaryHeap<TimerEntry>, output: &mut Vec<Timer>) {
        // collect only the latest active generation for each handle
        for entry in heap {
            let Some(current_generation) = self.active_generations.get(&entry.handle) else {
                continue;
            };
            if *current_generation != entry.generation {
                continue;
            }

            output.push(Timer {
                handle: entry.handle,
                deadline: entry.deadline,
                interval: entry.interval,
            });
        }
    }

    /// Drain ready timers for one clock domain.
    fn drain_ready_for_clock(&mut self, clock: TimerClock, now: Nanos, ready: &mut Vec<Timer>) {
        loop {
            let Some(entry) = self.peek_active_entry(clock) else {
                break;
            };

            if entry.deadline.at > now {
                break;
            }

            let entry = self
                .heap_for_clock(clock)
                .pop()
                .expect("timer entry should be present");
            let Some(current_generation) = self.active_generations.get(&entry.handle) else {
                continue;
            };
            if *current_generation != entry.generation {
                continue;
            }

            let timer = Timer {
                handle: entry.handle,
                deadline: entry.deadline,
                interval: entry.interval,
            };
            ready.push(timer);

            let Some(interval) = entry.interval else {
                self.active_generations.remove(&entry.handle);
                continue;
            };

            if interval.get() == 0 {
                self.active_generations.remove(&entry.handle);
                continue;
            }

            // coalesce missed intervals into one callback and schedule the next future deadline
            let mut next_fire = entry.deadline.at.saturating_add(interval);
            if next_fire <= now {
                let elapsed = now.saturating_sub(next_fire);
                let skipped_periods = elapsed.get() / interval.get() + 1;
                let skip_delta = interval.get().saturating_mul(skipped_periods);
                next_fire = next_fire.saturating_add(Nanos::new(skip_delta));
            }
            let generation = self.next_generation.wrapping_add(1);
            self.next_generation = generation;
            self.active_generations.insert(entry.handle, generation);
            self.heap_for_clock(entry.deadline.clock).push(TimerEntry {
                deadline: TimerDeadline {
                    clock: entry.deadline.clock,
                    at: next_fire,
                },
                handle: entry.handle,
                interval: entry.interval,
                generation,
            });
        }
    }

    /// Return one active timer entry for one clock domain.
    fn peek_active_entry(&mut self, clock: TimerClock) -> Option<TimerEntry> {
        loop {
            let entry = self.heap_for_clock(clock).peek().copied()?;
            let Some(current_generation) = self.active_generations.get(&entry.handle) else {
                self.heap_for_clock(clock).pop();
                continue;
            };
            if *current_generation != entry.generation {
                self.heap_for_clock(clock).pop();
                continue;
            }

            return Some(entry);
        }
    }

    /// Return one active timer fire timestamp for one clock domain.
    fn peek_active_fire_at(&mut self, clock: TimerClock) -> Option<Nanos> {
        self.peek_active_entry(clock).map(|entry| entry.deadline.at)
    }

    /// Return deterministic ordering for timer clock domains.
    fn clock_order(&self, clock: TimerClock) -> u8 {
        match clock {
            TimerClock::Monotonic => 0,
            TimerClock::Wall => 1,
        }
    }
}

impl EventLoop {
    /// Normalize one timer deadline using scheduler options.
    pub(super) fn normalize_deadline(&self, deadline: Nanos) -> Nanos {
        // quantize to timer resolution first
        let mut normalized = deadline;
        if let Some(timer_resolution_ns) = self.options.timer_resolution_ns {
            normalized = self.round_up_deadline(normalized, Nanos::new(timer_resolution_ns));
        }

        // then quantize to the configured coalescing window
        if let Some(max_timer_coalesce_ns) = self.options.max_timer_coalesce_ns {
            normalized = self.round_up_deadline(normalized, Nanos::new(max_timer_coalesce_ns));
        }

        normalized
    }

    /// Schedule a timer in the runtime queue.
    pub fn schedule_timer(&self, timer: Timer) -> RuntimeResult<()> {
        // normalize timer deadlines so scheduling stays deterministic
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
        let timer = Timer {
            handle: timer.handle,
            deadline,
            interval,
        };

        self.canceled_timers.lock().remove(&timer.handle);
        let mut queue = self.timers.lock();
        queue.schedule(timer);
        Ok(())
    }

    /// Cancel a timer by handle.
    pub fn cancel_timer(&self, handle: impl Into<TimerHandle>) -> RuntimeResult<()> {
        let handle = handle.into();
        let mut queue = self.timers.lock();
        queue.cancel(handle);
        self.canceled_timers.lock().insert(handle);
        Ok(())
    }

    /// Drain timers that are ready at the given time.
    pub fn poll_timers(&self, wall_now: Nanos, mono_now: Nanos) -> RuntimeResult<Vec<Timer>> {
        let mut queue = self.timers.lock();
        let ready = queue.poll_ready(wall_now, mono_now);
        Ok(ready)
    }

    /// Enqueue timers that are due at the current wall and monotonic timestamps.
    pub fn enqueue_due_timers(&mut self, wall_now: Nanos, mono_now: Nanos) -> RuntimeResult<()> {
        let ready = self.poll_timers(wall_now, mono_now)?;
        self.ready_timers.lock().extend(ready);
        Ok(())
    }

    /// Drain due timers and keep only matching ones out of the ready queue.
    pub fn take_due_timers_matching(
        &self,
        wall_now: Nanos,
        mono_now: Nanos,
        mut matches: impl FnMut(TimerHandle) -> bool,
    ) -> RuntimeResult<Vec<Timer>> {
        // drain due timers from the shared timer queue
        let ready = self.poll_timers(wall_now, mono_now)?;
        if ready.is_empty() {
            return Ok(Vec::new());
        }

        let mut matched = Vec::new();
        let mut ready_timers = self.ready_timers.lock();

        // keep non matching timers queued for the normal event loop path
        for timer in ready {
            if matches(timer.handle) {
                matched.push(timer);
            } else {
                ready_timers.push_back(timer);
            }
        }

        Ok(matched)
    }

    /// Round one deadline up to one deterministic quantum.
    fn round_up_deadline(&self, deadline: Nanos, quantum: Nanos) -> Nanos {
        if quantum.get() <= 1 {
            return deadline;
        }

        let remainder = deadline.get() % quantum.get();
        if remainder == 0 {
            return deadline;
        }

        deadline.saturating_add(Nanos::new(quantum.get().saturating_sub(remainder)))
    }
}

#[cfg(test)]
mod tests {
    use super::{Timer, TimerDeadline, TimerQueue};
    use crate::platform::ResourceId;
    use crate::platform::time::TimerClock;
    use crate::runtime::WorkerId;
    use crate::runtime::scheduler::TimerHandle;
    use crate::runtime::time::Nanos;

    const TEST_WORKER_ID: WorkerId = WorkerId(1);

    /// Ensures repeating timers reschedule correctly.
    #[test]
    fn test_repeating_timer_reschedules() {
        let mut queue = TimerQueue::default();
        queue.schedule(Timer {
            handle: TimerHandle::Resource(ResourceId::new(TEST_WORKER_ID, 1)),
            deadline: TimerDeadline {
                clock: TimerClock::Monotonic,
                at: Nanos::new(10),
            },
            interval: Some(Nanos::new(10)),
        });

        let first = queue.poll_ready(Nanos::new(0), Nanos::new(10));
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].handle.sort_key(), (0, TEST_WORKER_ID.0, 1));

        let second = queue.poll_ready(Nanos::new(0), Nanos::new(19));
        assert!(second.is_empty());

        let third = queue.poll_ready(Nanos::new(0), Nanos::new(20));
        assert_eq!(third.len(), 1);
        assert_eq!(third[0].handle.sort_key(), (0, TEST_WORKER_ID.0, 1));
    }

    /// Ensures repeating timers coalesce missed intervals into one fire.
    #[test]
    fn test_repeating_timer_coalesces_missed_intervals() {
        let mut queue = TimerQueue::default();
        queue.schedule(Timer {
            handle: TimerHandle::Resource(ResourceId::new(TEST_WORKER_ID, 2)),
            deadline: TimerDeadline {
                clock: TimerClock::Monotonic,
                at: Nanos::new(10),
            },
            interval: Some(Nanos::new(10)),
        });

        let ready = queue.poll_ready(Nanos::new(0), Nanos::new(100));
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].handle.sort_key(), (0, TEST_WORKER_ID.0, 2));

        let after_first = queue.poll_ready(Nanos::new(0), Nanos::new(100));
        assert!(after_first.is_empty());

        let (_, next_deadline) = queue.next_deadlines();
        let next_deadline = next_deadline.expect("repeating timer should remain scheduled");
        assert!(next_deadline > Nanos::new(100));
    }

    /// Ensures wall and monotonic timers dispatch against independent clocks.
    #[test]
    fn test_poll_ready_uses_clock_specific_deadlines() {
        let mut queue = TimerQueue::default();
        queue.schedule(Timer {
            handle: TimerHandle::Resource(ResourceId::new(TEST_WORKER_ID, 10)),
            deadline: TimerDeadline {
                clock: TimerClock::Wall,
                at: Nanos::new(100),
            },
            interval: None,
        });
        queue.schedule(Timer {
            handle: TimerHandle::Resource(ResourceId::new(TEST_WORKER_ID, 11)),
            deadline: TimerDeadline {
                clock: TimerClock::Monotonic,
                at: Nanos::new(50),
            },
            interval: None,
        });

        let first = queue.poll_ready(Nanos::new(0), Nanos::new(60));
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].handle.sort_key(), (0, TEST_WORKER_ID.0, 11));

        let second = queue.poll_ready(Nanos::new(120), Nanos::new(60));
        assert_eq!(second.len(), 1);
        assert_eq!(second[0].handle.sort_key(), (0, TEST_WORKER_ID.0, 10));
    }
}
