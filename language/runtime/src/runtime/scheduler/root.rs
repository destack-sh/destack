use destack_engine as engine;
use destack_heap as heap;

use super::EventLoop;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::runtime::engine::Engine;

impl EventLoop {
    /// Visit mutable heap root slots retained by queued scheduler state.
    pub(crate) fn visit_root_slots(
        &mut self,
        engine: &mut Engine,
        visit: &mut dyn FnMut(heap::RootSlot<'_>) -> heap::HeapResult<()>,
    ) -> RuntimeResult<()> {
        // queued tasks
        for task in &mut self.tasks {
            engine.visit_continuation_root_slots(&mut task.runnable, visit)?;
            visit_resume_value_root_slot(&mut task.resume_value, visit)?;
        }

        // queued microtasks
        for microtask in &mut self.microtasks {
            engine.visit_continuation_root_slots(&mut microtask.continuation, visit)?;
            visit_resume_value_root_slot(&mut microtask.resume_value, visit)?;
        }

        // suspended continuations
        for waiter in self.waiters.values_mut() {
            engine.visit_continuation_image_root_slots(&mut waiter.runnable, visit)?;
            visit_resume_value_root_slot(&mut waiter.resume_value, visit)?;
        }

        Ok(())
    }
}

/// Visit the mutable heap root slot in one value.
fn visit_resume_value_root_slot(
    value: &mut engine::Value,
    visit: &mut dyn FnMut(heap::RootSlot<'_>) -> heap::HeapResult<()>,
) -> RuntimeResult<()> {
    match value {
        engine::Value::HeapReference(reference) => {
            visit(heap::RootSlot::HeapReference(reference)).map_err(Box::<RuntimeError>::from)
        }
        engine::Value::SharedHeapReference(reference) => {
            visit(heap::RootSlot::SharedHeapReference(reference)).map_err(Box::<RuntimeError>::from)
        }
        _ => Ok(()),
    }
}
