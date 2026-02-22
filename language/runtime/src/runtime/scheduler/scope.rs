#![allow(clippy::missing_const_for_thread_local)]

use std::cell::Cell;

use crate::runtime::random::RandomStreamId;

use super::{MicrotaskId, TaskId};

thread_local! {
    /// TLS slot for the current event loop scope.
    static BINDING_EVENT_LOOP_SCOPE: Cell<EventLoopScope> = const {
        Cell::new(EventLoopScope {
            task_id: None,
            microtask_id: None,
            microtask_depth: 0,
        })
    };
}

/// Event loop scope for runtime execution.
#[derive(Debug, Clone, Copy)]
pub struct EventLoopScope {
    /// Current task identifier, if any.
    task_id: Option<TaskId>,
    /// Current microtask identifier, if any.
    microtask_id: Option<MicrotaskId>,
    /// Current nested microtask execution depth.
    microtask_depth: usize,
}

impl EventLoopScope {
    /// Create an empty event loop scope.
    pub const fn empty() -> Self {
        Self {
            task_id: None,
            microtask_id: None,
            microtask_depth: 0,
        }
    }

    /// Create a task event loop scope.
    pub const fn for_task(task_id: TaskId) -> Self {
        Self {
            task_id: Some(task_id),
            microtask_id: None,
            microtask_depth: 0,
        }
    }

    /// Create a microtask event loop scope.
    pub const fn for_microtask(microtask_id: MicrotaskId, depth: usize) -> Self {
        Self {
            task_id: None,
            microtask_id: Some(microtask_id),
            microtask_depth: depth,
        }
    }

    /// Return the current task identifier.
    pub const fn task_id(self) -> Option<TaskId> {
        self.task_id
    }

    /// Return the current microtask identifier.
    pub const fn microtask_id(self) -> Option<MicrotaskId> {
        self.microtask_id
    }

    /// Return the current microtask nesting depth.
    pub const fn microtask_depth(self) -> usize {
        self.microtask_depth
    }

    /// Return the random stream identifier for this scope.
    pub const fn random_stream_id(self) -> RandomStreamId {
        // use tagged stream ids to avoid collisions between event loop sources
        const STREAM_ID_MASK: u64 = (1u64 << 62) - 1;
        const TASK_TAG: u64 = 1u64 << 62;
        const MICROTASK_TAG: u64 = 2u64 << 62;
        match (self.task_id, self.microtask_id) {
            (_, Some(microtask_id)) => {
                RandomStreamId::new(MICROTASK_TAG | (microtask_id.get() & STREAM_ID_MASK))
            }
            (Some(task_id), None) => {
                RandomStreamId::new(TASK_TAG | (task_id.get() & STREAM_ID_MASK))
            }
            (None, None) => RandomStreamId::DEFAULT,
        }
    }
}

/// Guard that restores the previous event loop scope.
#[derive(Debug)]
pub struct EventLoopScopeGuard {
    /// Previous event loop scope.
    previous: EventLoopScope,
}

impl Drop for EventLoopScopeGuard {
    /// Restore the previous event loop scope.
    fn drop(&mut self) {
        BINDING_EVENT_LOOP_SCOPE.with(|slot| slot.set(self.previous));
    }
}

/// Enter an event loop scope for runtime execution.
#[inline]
pub fn enter_event_loop_scope(scope: EventLoopScope) -> EventLoopScopeGuard {
    let previous = BINDING_EVENT_LOOP_SCOPE.with(|slot| {
        let previous = slot.get();
        slot.set(scope);
        previous
    });

    EventLoopScopeGuard { previous }
}

/// Return the current event loop scope.
#[inline]
pub(crate) fn current_event_loop_scope() -> EventLoopScope {
    BINDING_EVENT_LOOP_SCOPE.with(|slot| slot.get())
}
