use tspp_heap as heap;
use tspp_program as program;

use super::EventLoop;
use crate::diagnostic::{RuntimeError, RuntimeResult};

impl EventLoop {
    /// Visit mutable heap root slots retained by queued scheduler state.
    pub(crate) fn visit_root_slots(
        &mut self,
        program: &program::Program,
        visit: &mut dyn FnMut(heap::RootSlot<'_>) -> heap::HeapResult<()>,
    ) -> RuntimeResult<()> {
        // queued tasks
        for runnable in &mut self.tasks {
            runnable.visit_root_slots(program, visit)?;
        }

        // queued microtasks
        for runnable in &mut self.microtasks {
            runnable.visit_root_slots(program, visit)?;
        }

        // external wake waiters
        for callback in self.wake_waiters.values_mut() {
            callback.visit_root_slots(program, visit)?;
        }

        // wakes buffered for running fibers
        self.fibers.visit_root_slots(program, visit)?;

        // values awaiting generated destruction
        for value in &mut self.drops {
            program
                .visit_value_root_slots(value, visit)
                .map_err(Box::<RuntimeError>::from)?;
        }

        Ok(())
    }
}
