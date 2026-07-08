use destack_heap as heap;

use super::{EventLoop, RootValue};
use crate::diagnostic::RuntimeResult;
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
            runnable.visit_root_slots(machine, visit)?;
        }

        // queued microtasks
        for runnable in &mut self.microtasks {
            runnable.visit_root_slots(machine, visit)?;
        }

        // suspended continuations
        for waiter in self.waiters.values_mut() {
            machine.visit_continuation_root_slots(&mut waiter.continuation, visit)?;
            waiter.resume_value.visit_root_slot(visit)?;
        }

        Ok(())
    }
}
