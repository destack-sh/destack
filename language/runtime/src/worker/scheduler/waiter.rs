use destack_heap as heap;
use destack_program as program;
use serde::{Deserialize, Serialize};

use super::{Callback, EventLoop, Invocation, Readiness, RunnableId, Wake, WakeKey};
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::{HostEventKind, ResourceId};

/// Suspended asynchronous continuations indexed by stable waiter ids.
#[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct WaiterTable {
    /// Dense reusable waiter slots.
    slots: Vec<WaiterSlot>,
    /// Vacant slot indices.
    vacant: Vec<u32>,
    /// Number of live waiters.
    len: usize,
}

/// One generation-checked waiter slot.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
struct WaiterSlot {
    /// Generation issued by this slot.
    generation: u32,
    /// Live continuation when this slot is occupied.
    continuation: Option<program::Continuation>,
    /// Task whose execution this waiter resumes when present.
    task: Option<program::Task>,
}

impl WaiterTable {
    /// Fork this waiter table for one forked World.
    pub(super) fn fork(&self) -> Self {
        Self {
            slots: self.slots.iter().map(WaiterSlot::fork).collect(),
            vacant: self.vacant.clone(),
            len: self.len,
        }
    }

    /// Insert one suspended continuation.
    pub(super) fn insert(
        &mut self,
        continuation: program::Continuation,
        task: Option<program::Task>,
    ) -> program::Waiter {
        let index = if let Some(index) = self.vacant.pop() {
            index
        } else {
            let index = self.slots.len() as u32;
            self.slots.push(WaiterSlot {
                generation: 1,
                continuation: None,
                task: None,
            });

            index
        };
        let slot = &mut self.slots[index as usize];
        slot.continuation = Some(continuation);
        slot.task = task;
        self.len += 1;

        program::Waiter::new(index, slot.generation)
    }

    /// Take one live suspended continuation.
    pub(super) fn take(
        &mut self,
        waiter: program::Waiter,
    ) -> Option<(program::Continuation, Option<program::Task>)> {
        let slot = self.slots.get_mut(waiter.index() as usize)?;
        if slot.generation != waiter.generation() {
            return None;
        }
        let continuation = slot.continuation.take()?;
        let task = slot.task.take();

        slot.generation = slot.generation.wrapping_add(1).max(1);
        self.vacant.push(waiter.index());
        self.len -= 1;

        Some((continuation, task))
    }

    /// Return whether one waiter names a live suspended continuation.
    pub(super) fn contains(&self, waiter: program::Waiter) -> bool {
        self.slots.get(waiter.index() as usize).is_some_and(|slot| {
            slot.generation == waiter.generation() && slot.continuation.is_some()
        })
    }

    /// Return whether no waiter is live.
    pub(super) const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Return the number of live waiters.
    pub(super) const fn len(&self) -> usize {
        self.len
    }

    /// Iterate over every live waiter and its suspended continuation.
    pub(super) fn iter(&self) -> impl Iterator<Item = (program::Waiter, &program::Continuation)> {
        self.slots.iter().enumerate().filter_map(|(index, slot)| {
            let continuation = slot.continuation.as_ref()?;
            let waiter = program::Waiter::new(index as u32, slot.generation);

            Some((waiter, continuation))
        })
    }

    /// Visit mutable heap roots retained by all live waiters.
    pub(super) fn visit_root_slots(
        &mut self,
        program: &program::Program,
        visit: &mut dyn FnMut(heap::RootSlot<'_>) -> heap::HeapResult<()>,
    ) -> RuntimeResult<()> {
        for slot in &mut self.slots {
            let Some(continuation) = &mut slot.continuation else {
                continue;
            };
            program
                .visit_continuation_root_slots(continuation, visit)
                .map_err(Box::<RuntimeError>::from)?;
        }

        Ok(())
    }
}

impl WaiterSlot {
    /// Fork this waiter slot for one forked World.
    fn fork(&self) -> Self {
        Self {
            generation: self.generation,
            continuation: self.continuation.as_ref().map(program::Continuation::fork),
            task: self.task,
        }
    }
}

impl EventLoop {
    /// Add one waiter for a timer resource.
    pub fn add_timer_waiter(&mut self, resource_id: ResourceId, callback: Callback) {
        self.wake_waiters
            .insert(WakeKey::Timer(resource_id), callback);
    }

    /// Remove the callback registered for one timer resource.
    pub fn remove_timer_waiter(&mut self, resource_id: ResourceId) -> Option<Callback> {
        self.wake_waiters.remove(&WakeKey::Timer(resource_id))
    }

    /// Add one waiter for one resource readiness.
    pub fn add_resource_waiter(
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
    pub fn has_resource_waiter(&self, resource_id: ResourceId, readiness: Readiness) -> bool {
        self.wake_waiters.contains_key(&WakeKey::Resource {
            resource_id,
            readiness,
        })
    }

    /// Add one waiter for a host event kind.
    pub fn add_host_waiter(&mut self, kind: HostEventKind, callback: Callback) {
        self.wake_waiters.insert(WakeKey::Host(kind), callback);
    }

    /// Return whether one waiter is registered for the given host event kind.
    pub fn has_host_waiter(&self, kind: HostEventKind) -> bool {
        self.wake_waiters.contains_key(&WakeKey::Host(kind))
    }

    /// Dispatch one wake into the task queue.
    pub fn dispatch(&mut self, wake: Wake) -> Option<RunnableId> {
        let key = wake.key();
        let is_inactive_timer = matches!(key, WakeKey::Timer(resource_id) if !self.timers.has_active_timer(resource_id));
        let invocation = if is_inactive_timer {
            self.wake_waiters.remove(&key)?.invoke()
        } else {
            self.wake_waiters.get(&key)?.invoke()
        };

        Some(self.enqueue_task(invocation))
    }

    /// Park one asynchronous continuation and return its waiter id.
    pub(crate) fn park(&mut self, continuation: program::Continuation) -> program::Waiter {
        self.waiters.insert(continuation, None)
    }

    /// Attempt to queue one waiter resumption as a microtask.
    pub(crate) fn queue_waiter(
        &mut self,
        waiter: program::Waiter,
        value: program::Value,
    ) -> program::Result<bool> {
        let Some((continuation, task)) = self.waiters.take(waiter) else {
            return Ok(false);
        };
        if let Some(task) = task {
            self.task_table.ready(task, false)?;
        }
        let invocation = Invocation::resume(task, continuation, value);
        self.enqueue_microtask(invocation);

        Ok(true)
    }

    /// Attempt to queue one waiter cancellation as a microtask.
    pub(crate) fn cancel_waiter(&mut self, waiter: program::Waiter) -> program::Result<bool> {
        let Some((continuation, task)) = self.waiters.take(waiter) else {
            return Ok(false);
        };
        if let Some(task) = task {
            self.task_table.ready(task, true)?;
        }
        let invocation = Invocation::cancel(task, continuation);
        self.enqueue_microtask(invocation);

        Ok(true)
    }
}
