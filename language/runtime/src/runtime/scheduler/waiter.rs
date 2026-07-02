use destack_program as program;
use serde::{Deserialize, Serialize};

use super::{EventLoop, Readiness, Runnable, Wake, WakeKey};
use crate::diagnostic::RuntimeResult;
use crate::host::{HostEventKind, ResourceId};
use crate::runtime::machine::Continuation;

/// Suspended continuation that resumes when one wake source fires.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Waiter {
    /// Continuation to dispatch when the wake source fires.
    pub continuation: Continuation,
    /// Resume value passed into the continuation.
    pub resume_value: program::Value,
}

impl EventLoop {
    /// Add one waiter for a timer resource.
    pub fn add_timer_waiter(
        &mut self,
        resource_id: ResourceId,
        runnable: Continuation,
        resume_value: program::Value,
    ) {
        let waiter = Waiter {
            continuation: runnable,
            resume_value,
        };
        self.waiters.insert(WakeKey::Timer(resource_id), waiter);
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
    ) {
        let waiter = Waiter {
            continuation: runnable,
            resume_value,
        };
        self.waiters.insert(
            WakeKey::Resource {
                resource_id,
                readiness,
            },
            waiter,
        );
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
    ) {
        let waiter = Waiter {
            continuation: runnable,
            resume_value,
        };
        self.waiters.insert(WakeKey::Host(kind), waiter);
    }

    /// Return whether one waiter is registered for the given host event kind.
    pub fn has_host_waiter(&self, kind: HostEventKind) -> bool {
        self.waiters.contains_key(&WakeKey::Host(kind))
    }

    /// Build one runnable for one wake.
    pub fn runnable_for_wake(&mut self, wake: Wake) -> RuntimeResult<Option<Runnable>> {
        let key = wake.key();
        let is_inactive_timer = matches!(key, WakeKey::Timer(resource_id) if !self.timers.has_active_timer(resource_id));
        let Some(waiter) = self.waiters.get(&key) else {
            return Ok(None);
        };
        let continuation = waiter.continuation.clone();
        let resume_value = waiter.resume_value.clone();

        if is_inactive_timer {
            self.waiters.remove(&key);
        }

        let id = self.next_runnable_id()?;

        Ok(Some(Runnable {
            id,
            continuation,
            resume_value,
        }))
    }
}
