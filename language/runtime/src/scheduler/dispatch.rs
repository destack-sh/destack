use super::{TimerWake, Wake};
use crate::diagnostic::RuntimeResult;
use crate::world::time::{Instant, Nanos};

use super::EventLoop;

impl EventLoop {
    /// Pop the next wake from the event loop.
    pub(crate) fn next_wake(
        &mut self,
        wall_now: Nanos,
        mono_now: Nanos,
    ) -> RuntimeResult<Option<Wake>> {
        if let Some(timer) = self.pop_ready_timer(wall_now, mono_now)? {
            return Ok(Some(Wake::Timer(TimerWake::new(timer.resource_id))));
        }

        Ok(self.wakes.pop_front())
    }

    /// Report whether one runnable item is already ready without advancing time.
    pub(crate) fn has_ready_work(&self) -> bool {
        !self.tasks.is_empty() || !self.microtasks.is_empty() || !self.wakes.is_empty()
    }

    /// Report whether any work remains in the event loop, including future wakes.
    pub(crate) fn has_pending_work(&self) -> bool {
        if self.has_ready_work()
            || !self.wake_waiters.is_empty()
            || !self.waiters.is_empty()
            || self.task_table.has_pending_execution()
            || !self.drops.is_empty()
        {
            return true;
        }

        self.timers.has_pending_timers()
    }

    /// Return the next wall deadline when any timer can become runnable.
    pub(crate) fn next_deadline(&mut self, wall_now: Nanos, mono_now: Nanos) -> Option<Instant> {
        // due timers are runnable immediately
        if self.timers.has_ready(wall_now, mono_now) {
            return Some(Instant::from_nanos(wall_now));
        }

        // map wall and monotonic timer deadlines into one wall-clock wakeup
        let (wall_deadline, mono_deadline) = self.timers.next_deadlines();
        let wall_deadline = wall_deadline
            .filter(|deadline| *deadline > wall_now)
            .map(Instant::from_nanos);
        let mono_deadline = mono_deadline
            .filter(|deadline| *deadline > mono_now)
            .map(|deadline| {
                let delta = deadline.saturating_sub(mono_now);
                Instant::from_nanos(wall_now.saturating_add(delta))
            });

        match (wall_deadline, mono_deadline) {
            (Some(wall_deadline), Some(mono_deadline)) => Some(wall_deadline.min(mono_deadline)),
            (Some(wall_deadline), None) => Some(wall_deadline),
            (None, Some(mono_deadline)) => Some(mono_deadline),
            (None, None) => None,
        }
    }
}
