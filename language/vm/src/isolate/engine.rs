use std::sync::Arc;

use destack_engine as engine;

use super::Isolate;
use crate::diagnostic::RuntimeError;
use crate::interpreter::{Continuation, Outcome};
use crate::isolate::IsolateImage;

impl engine::Engine for Isolate {
    type Continuation = Continuation;
    type Image = Arc<IsolateImage>;
    type Error = RuntimeError;

    fn initialize(&mut self, context: engine::MemoryContext<'_>) -> Result<(), Self::Error> {
        Isolate::initialize(
            self,
            context.heap,
            context.shared_heap,
            context.worker_static,
        )
    }

    fn run(
        &mut self,
        context: engine::CallContext<'_>,
        entry: engine::EntryPoint,
        args: &[engine::Value],
    ) -> Result<Outcome, Self::Error> {
        let function_id = self.function_for_entry(entry);
        let context = context.memory;

        self.run_function_yielding(
            context.worker_static,
            context.heap,
            context.shared_heap,
            context.shared_allocator,
            context.shared_gc_worker,
            function_id,
            args,
        )
    }

    fn resume(
        &mut self,
        context: engine::CallContext<'_>,
        continuation: Continuation,
        value: engine::Value,
    ) -> Result<Outcome, Self::Error> {
        let context = context.memory;

        Isolate::resume(
            self,
            context.worker_static,
            context.heap,
            context.shared_heap,
            context.shared_allocator,
            context.shared_gc_worker,
            continuation,
            value,
        )
    }

    fn fork(&self, _context: engine::MemoryContext<'_>) -> Result<Self, Self::Error> {
        Isolate::fork(self)
    }

    fn image(&self, _context: engine::MemoryContext<'_>) -> Result<Self::Image, Self::Error> {
        Ok(Arc::new(Isolate::image(self)?))
    }

    fn restore(
        &mut self,
        _context: engine::MemoryContext<'_>,
        image: &Self::Image,
    ) -> Result<(), Self::Error> {
        Isolate::restore_image(self, image)
    }

    fn visit_root_slots(
        &mut self,
        statics: &mut engine::StaticSpace,
        visit: &mut dyn FnMut(destack_heap::RootSlot<'_>) -> destack_heap::HeapResult<()>,
    ) -> Result<(), Self::Error> {
        Isolate::visit_root_slots(self, statics, &mut [], visit)
    }

    fn visit_continuation_root_slots(
        &mut self,
        continuation: &mut Self::Continuation,
        visit: &mut dyn FnMut(destack_heap::RootSlot<'_>) -> destack_heap::HeapResult<()>,
    ) -> Result<(), Self::Error> {
        Isolate::visit_continuation_root_slots(self, continuation, visit)
    }
}
