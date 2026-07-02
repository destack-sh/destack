use destack_heap as heap;
use destack_program as program;

use super::EventLoop;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::runtime::machine::Machine;

impl EventLoop {
    /// Visit mutable heap root slots retained by queued scheduler state.
    pub(crate) fn visit_root_slots(
        &mut self,
        machine: &mut Machine,
        visit: &mut dyn FnMut(heap::RootSlot<'_>) -> heap::HeapResult<()>,
    ) -> RuntimeResult<()> {
        // queued tasks
        for runnable in &mut self.tasks {
            machine.visit_continuation_root_slots(&mut runnable.continuation, visit)?;
            visit_resume_value_root_slot(&mut runnable.resume_value, visit)?;
        }

        // queued microtasks
        for runnable in &mut self.microtasks {
            machine.visit_continuation_root_slots(&mut runnable.continuation, visit)?;
            visit_resume_value_root_slot(&mut runnable.resume_value, visit)?;
        }

        // suspended continuations
        for waiter in self.waiters.values_mut() {
            machine.visit_continuation_root_slots(&mut waiter.continuation, visit)?;
            visit_resume_value_root_slot(&mut waiter.resume_value, visit)?;
        }

        Ok(())
    }
}

/// Visit the mutable heap root slot in one value.
fn visit_resume_value_root_slot(
    value: &mut program::Value,
    visit: &mut dyn FnMut(heap::RootSlot<'_>) -> heap::HeapResult<()>,
) -> RuntimeResult<()> {
    match value {
        program::Value::HeapReference(reference) => {
            visit(heap::RootSlot::HeapReference(reference)).map_err(Box::<RuntimeError>::from)
        }
        program::Value::SharedHeapReference(reference) => {
            visit(heap::RootSlot::SharedHeapReference(reference)).map_err(Box::<RuntimeError>::from)
        }
        _ => Ok(()),
    }
}
