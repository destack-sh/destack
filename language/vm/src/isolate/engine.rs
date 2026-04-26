use std::sync::Arc;

use {destack_engine as engine, destack_heap as heap};

use super::Isolate;
use crate::Word;
use crate::diagnostic::RuntimeError;
use crate::interpreter::{Continuation, Outcome};
use crate::snapshot::IsolateImage;

impl engine::Engine for Isolate {
    type Continuation = Continuation;
    type Image = Arc<IsolateImage>;
    type Error = RuntimeError;

    fn initialize(&mut self, context: engine::Context<'_>) -> Result<(), Self::Error> {
        Isolate::initialize_statics(self, context.worker_static)
    }

    fn run(
        &mut self,
        context: engine::Context<'_>,
        entry: &engine::Entry,
        args: &[engine::Value],
    ) -> Result<Outcome, Self::Error> {
        let args = args.iter().map(Word::from).collect::<Vec<_>>();

        self.run_function_by_name_yielding(
            context.worker_static,
            context.heap,
            context.shared,
            entry.name(),
            &args,
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
            context.shared,
            continuation,
            value,
        )
    }

    fn fork(&mut self, _heap: &mut heap::Heap) -> Result<Self, Self::Error> {
        Isolate::fork(self)
    }

    fn image(&mut self) -> Result<Self::Image, Self::Error> {
        Ok(Arc::new(Isolate::image(self)?))
    }

    fn restore(&mut self, heap: &mut heap::Heap, image: &Self::Image) -> Result<(), Self::Error> {
        Isolate::restore_image(self, heap, image)
    }
}
