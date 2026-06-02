use std::sync::Arc;

use destack_engine as engine;

use crate::diagnostic::RuntimeError;

use super::{Continuation, Machine, MachineImage, Outcome};

impl engine::Engine for Machine {
    type Continuation = Continuation;
    type Image = Arc<MachineImage>;
    type Error = RuntimeError;

    fn initialize(&mut self, context: engine::EngineMemory<'_>) -> Result<(), Self::Error> {
        Machine::initialize(
            self,
            context.heap,
            context.shared_heap,
            context.worker_static,
        )
    }

    fn run(
        &mut self,
        context: engine::EngineCall<'_>,
        entry: engine::EntryPoint,
        args: &[engine::Value],
    ) -> Result<Outcome, Self::Error> {
        let function_id = self.function_for_entry(entry);
        let context = context.memory;

        self.run_function_yielding(
            context.worker_static,
            context.heap,
            context.shared_heap,
            context.shared_cache,
            context.shared_gc_worker,
            function_id,
            args,
        )
    }

    fn resume(
        &mut self,
        context: engine::EngineCall<'_>,
        continuation: Continuation,
        value: engine::Value,
    ) -> Result<Outcome, Self::Error> {
        let context = context.memory;

        Machine::resume(
            self,
            context.worker_static,
            context.heap,
            context.shared_heap,
            context.shared_cache,
            context.shared_gc_worker,
            continuation,
            value,
        )
    }

    fn fork(&self, _context: engine::EngineMemory<'_>) -> Result<Self, Self::Error> {
        Machine::fork(self)
    }

    fn image(&self, _context: engine::EngineMemory<'_>) -> Result<Self::Image, Self::Error> {
        Ok(Arc::new(Machine::image(self)?))
    }

    fn restore(
        &mut self,
        _context: engine::EngineMemory<'_>,
        image: &Self::Image,
    ) -> Result<(), Self::Error> {
        Machine::restore_image(self, image)
    }

    fn visit_root_slots(
        &mut self,
        statics: &mut engine::StaticSpace,
        visit: &mut dyn FnMut(destack_heap::RootSlot<'_>) -> destack_heap::HeapResult<()>,
    ) -> Result<(), Self::Error> {
        Machine::visit_root_slots(self, statics, &mut [], visit)
    }

    fn visit_continuation_root_slots(
        &mut self,
        continuation: &mut Self::Continuation,
        visit: &mut dyn FnMut(destack_heap::RootSlot<'_>) -> destack_heap::HeapResult<()>,
    ) -> Result<(), Self::Error> {
        Machine::visit_continuation_root_slots(self, continuation, visit)
    }
}
