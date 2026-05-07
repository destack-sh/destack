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

    fn initialize(&mut self, context: engine::Context<'_>) -> Result<(), Self::Error> {
        Isolate::initialize(
            self,
            context.heap,
            context.shared_heap,
            context.worker_static,
        )
    }

    fn run(
        &mut self,
        context: engine::Context<'_>,
        entry: engine::Entry,
        args: &[engine::Value],
    ) -> Result<Outcome, Self::Error> {
        let function_id = self.function_for_entry(entry);

        self.run_function_yielding(
            context.worker_static,
            context.heap,
            context.shared_heap,
            context.shared_allocator,
            context.shared_gc,
            function_id,
            args,
        )
    }

    fn resume(
        &mut self,
        context: engine::Context<'_>,
        continuation: Continuation,
        value: engine::Value,
    ) -> Result<Outcome, Self::Error> {
        Isolate::resume(
            self,
            context.worker_static,
            context.heap,
            context.shared_heap,
            context.shared_allocator,
            context.shared_gc,
            continuation,
            value,
        )
    }

    fn fork(&self, _context: engine::Context<'_>) -> Result<Self, Self::Error> {
        Isolate::fork(self)
    }

    fn image(&self, _context: engine::Context<'_>) -> Result<Self::Image, Self::Error> {
        Ok(Arc::new(Isolate::image(self)?))
    }

    fn restore(
        &mut self,
        _context: engine::Context<'_>,
        image: &Self::Image,
    ) -> Result<(), Self::Error> {
        Isolate::restore_image(self, image)
    }
}
