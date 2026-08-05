use super::{Callback, TimerWake, Wake, WakeKey};
use crate::diagnostic::RuntimeResult;
use crate::host::{HostEventKind, ResourceId};
use crate::scheduler::{Readiness, RunnableId};
use crate::world::time::{Instant, Nanos};

use super::EventLoop;

impl EventLoop {
    /// Add one waiter for a timer resource.
    pub(crate) fn add_timer_waiter(&mut self, resource_id: ResourceId, callback: Callback) {
        self.wake_waiters
            .insert(WakeKey::Timer(resource_id), callback);
    }

    /// Remove the callback registered for one timer resource.
    pub(crate) fn remove_timer_waiter(&mut self, resource_id: ResourceId) -> Option<Callback> {
        self.wake_waiters.remove(&WakeKey::Timer(resource_id))
    }

    /// Add one waiter for one resource readiness.
    pub(crate) fn add_resource_waiter(
        &mut self,
        resource_id: ResourceId,
        readiness: Readiness,
        callback: Callback,
    ) {
        self.wake_waiters.insert(
            WakeKey::Resource {
                resource_id,
                readiness,
            },
            callback,
        );
    }

    /// Return whether one waiter is registered for one resource readiness.
    pub(crate) fn has_resource_waiter(
        &self,
        resource_id: ResourceId,
        readiness: Readiness,
    ) -> bool {
        self.wake_waiters.contains_key(&WakeKey::Resource {
            resource_id,
            readiness,
        })
    }

    /// Add one waiter for a host event kind.
    pub(crate) fn add_host_waiter(&mut self, kind: HostEventKind, callback: Callback) {
        self.wake_waiters.insert(WakeKey::Host(kind), callback);
    }

    /// Return whether one waiter is registered for the given host event kind.
    pub(crate) fn has_host_waiter(&self, kind: HostEventKind) -> bool {
        self.wake_waiters.contains_key(&WakeKey::Host(kind))
    }

    /// Dispatch one wake into the task queue.
    pub(crate) fn dispatch(&mut self, wake: Wake) -> Option<RunnableId> {
        let key = wake.key();
        let is_inactive_timer = matches!(key, WakeKey::Timer(resource_id) if !self.timers.has_active_timer(resource_id));
        let invocation = if is_inactive_timer {
            self.wake_waiters.remove(&key)?.invoke()
        } else {
            self.wake_waiters.get(&key)?.invoke()
        };

        Some(self.enqueue_task(invocation))
    }

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
            || !self.fibers.is_empty()
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
