use destack_program as program;
use serde::{Deserialize, Serialize};

use super::{EventLoop, Readiness, Task, Wake, WakeKey};
use crate::diagnostic::RuntimeResult;
use crate::host::{HostEventKind, ResourceId};
use crate::runtime::machine::{Continuation, ContinuationImage, Machine};

/// Suspended continuation that resumes when one wake source fires.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Waiter {
    /// Runnable continuation image to restore when dispatched.
    pub runnable: ContinuationImage,
    /// Resume value passed into the continuation.
    pub resume_value: program::Value,
    /// Task priority used when queueing the resumed task.
    pub priority: u8,
}

impl EventLoop {
    /// Add one waiter for a timer resource.
    pub fn add_timer_waiter(
        &mut self,
        resource_id: ResourceId,
        runnable: Continuation,
        resume_value: program::Value,
        priority: u8,
        machine: &mut Machine,
    ) -> RuntimeResult<()> {
        let waiter = self.capture_waiter(runnable, resume_value, priority, machine)?;
        self.waiters.insert(WakeKey::Timer(resource_id), waiter);

        Ok(())
    }

    /// Remove the waiter registered for one timer resource.
    pub fn remove_timer_waiter(&mut self, resource_id: ResourceId) -> Option<Waiter> {
        self.waiters.remove(&WakeKey::Timer(resource_id))
    }

    /// Add one waiter for one resource readiness.
    pub fn add_resource_waiter(
        &mut self,
        resource_id: ResourceId,
        readiness: Readiness,
        runnable: Continuation,
        resume_value: program::Value,
        priority: u8,
        machine: &mut Machine,
    ) -> RuntimeResult<()> {
        let waiter = self.capture_waiter(runnable, resume_value, priority, machine)?;
        self.waiters.insert(
            WakeKey::Resource {
                resource_id,
                readiness,
            },
            waiter,
        );

        Ok(())
    }

    /// Return whether one waiter is registered for one resource readiness.
    pub fn has_resource_waiter(&self, resource_id: ResourceId, readiness: Readiness) -> bool {
        self.waiters.contains_key(&WakeKey::Resource {
            resource_id,
            readiness,
        })
    }

    /// Add one waiter for a host event kind.
    pub fn add_host_waiter(
        &mut self,
        kind: HostEventKind,
        runnable: Continuation,
        resume_value: program::Value,
        priority: u8,
        machine: &mut Machine,
    ) -> RuntimeResult<()> {
        let waiter = self.capture_waiter(runnable, resume_value, priority, machine)?;
        self.waiters.insert(WakeKey::Host(kind), waiter);

        Ok(())
    }

    /// Return whether one waiter is registered for the given host event kind.
    pub fn has_host_waiter(&self, kind: HostEventKind) -> bool {
        self.waiters.contains_key(&WakeKey::Host(kind))
    }

    /// Build one task for one wake.
    pub fn task_for_wake(
        &mut self,
        wake: Wake,
        machine: &mut Machine,
    ) -> RuntimeResult<Option<Task>> {
        let key = wake.key();
        let is_inactive_timer = matches!(key, WakeKey::Timer(resource_id) if !self.timers.has_active_timer(resource_id));
        let Some(waiter) = self.waiters.get(&key) else {
            return Ok(None);
        };
        let runnable = machine.restore_continuation_image(&waiter.runnable)?;
        let resume_value = waiter.resume_value.clone();
        let priority = waiter.priority;

        if is_inactive_timer {
            self.waiters.remove(&key);
        }

        let task_id = self.next_task_id()?;

        Ok(Some(Task {
            id: task_id,
            runnable,
            resume_value,
            priority,
        }))
    }

    /// Capture one immutable waiter payload for repeatable dispatch.
    fn capture_waiter(
        &self,
        runnable: Continuation,
        resume_value: program::Value,
        priority: u8,
        machine: &mut Machine,
    ) -> RuntimeResult<Waiter> {
        let runnable = machine.continuation_image(&runnable)?;

        Ok(Waiter {
            runnable,
            resume_value,
            priority,
        })
    }
}
