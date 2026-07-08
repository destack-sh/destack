use destack_heap as heap;
use destack_program as program;
use serde::{Deserialize, Serialize};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::runtime::machine::{Continuation, Machine};
use crate::runtime::worker::RunnableScope;

/// Runnable continuation queued by the event loop.
#[derive(Debug, Clone)]
pub struct Runnable {
    /// Runnable identifier used for ordering and logging.
    pub id: RunnableId,
    /// Continuation to resume.
    pub continuation: Continuation,
    /// Resume payload passed back into the machine.
    pub resume_value: program::Value,
}

/// Runnable stopped at one runtime stop point.
#[derive(Debug, Clone)]
pub struct StoppedRunnable {
    /// Runnable identifier used for ordering and logging.
    pub id: RunnableId,
    /// Continuation held out of the event loop until explicit resume.
    pub continuation: Continuation,
    /// Runnable scope active when execution stopped.
    pub scope: RunnableScope,
    /// Reason the runnable stopped.
    pub reason: program::StopReason,
}

/// Opaque runnable identifier used by the event loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct RunnableId(u64);

impl Runnable {
    /// Visit mutable heap root slots retained by this runnable.
    pub(crate) fn visit_root_slots(
        &mut self,
        machine: &mut Machine,
        visit: &mut dyn FnMut(heap::RootSlot<'_>) -> heap::HeapResult<()>,
    ) -> RuntimeResult<()> {
        machine.visit_continuation_root_slots(&mut self.continuation, visit)?;
        self.resume_value.visit_root_slot(visit)
    }
}

impl StoppedRunnable {
    /// Create one stopped runnable.
    pub const fn new(
        id: RunnableId,
        continuation: Continuation,
        scope: RunnableScope,
        reason: program::StopReason,
    ) -> Self {
        Self {
            id,
            continuation,
            scope,
            reason,
        }
    }

    /// Visit mutable heap root slots retained by this stopped runnable.
    pub(crate) fn visit_root_slots(
        &mut self,
        machine: &mut Machine,
        visit: &mut dyn FnMut(heap::RootSlot<'_>) -> heap::HeapResult<()>,
    ) -> RuntimeResult<()> {
        machine.visit_continuation_root_slots(&mut self.continuation, visit)
    }
}

impl RunnableId {
    /// Create a new runnable identifier.
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Return the raw runnable identifier value.
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Heap-root visiting behavior for scheduler-retained values.
pub(crate) trait RootValue {
    /// Visit the mutable heap root slot when the value carries a heap reference.
    fn visit_root_slot(
        &mut self,
        visit: &mut dyn FnMut(heap::RootSlot<'_>) -> heap::HeapResult<()>,
    ) -> RuntimeResult<()>;
}

impl RootValue for program::Value {
    fn visit_root_slot(
        &mut self,
        visit: &mut dyn FnMut(heap::RootSlot<'_>) -> heap::HeapResult<()>,
    ) -> RuntimeResult<()> {
        match self {
            Self::HeapReference(reference) => {
                visit(heap::RootSlot::HeapReference(reference)).map_err(Box::<RuntimeError>::from)
            }
            Self::SharedHeapReference(reference) => {
                visit(heap::RootSlot::SharedHeapReference(reference))
                    .map_err(Box::<RuntimeError>::from)
            }
            _ => Ok(()),
        }
    }
}
