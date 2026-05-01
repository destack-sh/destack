use destack_core::CaptureMode;
use {destack_engine as engine, destack_heap as heap, destack_native as native, destack_vm as vm};

use super::{Context, Continuation, ContinuationImage, Entry, Image, Outcome};
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::runtime::memory::RootSink;

/// VM root visitor bridged into runtime root collection.
struct VmRootSink<'a, 'b> {
    /// The runtime root visitor.
    roots: &'a mut RootSink<'b>,
}

impl vm::RootSink for VmRootSink<'_, '_> {
    fn push_heap(&mut self, reference: heap::HeapReference) {
        self.roots.push_heap(reference);
    }

    fn push_shared_heap(&mut self, reference: heap::SharedHeapReference) {
        self.roots.push_shared_heap(reference);
    }
}

/// Execution backend owned by one worker.
pub enum Engine {
    /// VM interpreter backend.
    Vm(vm::Isolate),
    /// Native compiled backend.
    Native(native::Engine),
}

impl Engine {
    /// Initialize worker-owned static bytes.
    pub fn initialize(&mut self, context: Context<'_>) -> RuntimeResult<()> {
        match self {
            Self::Vm(engine) => {
                engine::Engine::initialize(engine, context).map_err(Box::<RuntimeError>::from)
            }
            Self::Native(engine) => {
                engine::Engine::initialize(engine, context).map_err(native_runtime_error)
            }
        }
    }

    /// Run one entrypoint.
    pub fn run(
        &mut self,
        context: Context<'_>,
        entry: &Entry,
        args: &[engine::Value],
    ) -> RuntimeResult<Outcome<Continuation>> {
        match self {
            Self::Vm(engine) => {
                let outcome = engine::Engine::run(engine, context, entry, args)
                    .map_err(Box::<RuntimeError>::from)?;

                Ok(outcome_from_vm(outcome))
            }
            Self::Native(engine) => {
                let outcome = engine::Engine::run(engine, context, entry, args)
                    .map_err(native_runtime_error)?;

                Ok(outcome_from_native(outcome))
            }
        }
    }

    /// Resume one continuation.
    pub fn resume(
        &mut self,
        context: Context<'_>,
        continuation: Continuation,
        value: engine::Value,
    ) -> RuntimeResult<Outcome<Continuation>> {
        match (self, continuation) {
            (Self::Vm(engine), Continuation::Vm(continuation)) => {
                let outcome = engine::Engine::resume(engine, context, continuation, value)
                    .map_err(Box::<RuntimeError>::from)?;

                Ok(outcome_from_vm(outcome))
            }
            (Self::Native(engine), Continuation::Native(continuation)) => {
                let outcome = engine::Engine::resume(engine, context, continuation, value)
                    .map_err(native_runtime_error)?;

                Ok(outcome_from_native(outcome))
            }
            (Self::Vm(_), Continuation::Native(_)) => {
                Err(engine_continuation_mismatch("vm", "native"))
            }
            (Self::Native(_), Continuation::Vm(_)) => {
                Err(engine_continuation_mismatch("native", "vm"))
            }
        }
    }

    /// Visit roots from active backend state.
    pub fn visit_roots(
        &mut self,
        worker_static: &engine::StaticSpace,
        roots: &mut RootSink<'_>,
    ) -> RuntimeResult<()> {
        match self {
            Self::Vm(engine) => {
                let mut roots = VmRootSink { roots };

                engine
                    .visit_state_roots(worker_static, &[], &mut roots)
                    .map_err(Box::<RuntimeError>::from)
            }
            Self::Native(_) => Ok(()),
        }
    }

    /// Publish allocator-local shared heap buffers before global heap work.
    pub fn flush_shared_allocator(&mut self, shared: &heap::SharedHeap) {
        match self {
            Self::Vm(engine) => engine.flush_shared_allocator(shared),
            Self::Native(_) => {}
        }
    }

    /// Visit mutable local root slots from active backend state.
    pub fn visit_root_slots(
        &mut self,
        worker_static: &mut engine::StaticSpace,
        visit: &mut dyn FnMut(heap::RootSlot<'_>) -> heap::HeapResult<()>,
    ) -> RuntimeResult<()> {
        match self {
            Self::Vm(engine) => {
                vm::Isolate::visit_root_slots(engine, worker_static, &mut [], visit)
                    .map_err(Box::<RuntimeError>::from)
            }
            Self::Native(_) => Ok(()),
        }
    }

    /// Visit roots from one live continuation.
    pub fn visit_continuation_roots(
        &mut self,
        continuation: &Continuation,
        roots: &mut RootSink<'_>,
    ) -> RuntimeResult<()> {
        match (self, continuation) {
            (Self::Vm(engine), Continuation::Vm(continuation)) => {
                let mut roots = VmRootSink { roots };

                vm::Isolate::visit_continuation_roots(engine, continuation, &mut roots)
                    .map_err(Box::<RuntimeError>::from)
            }
            (Self::Native(_), Continuation::Native(_)) => Ok(()),
            (Self::Vm(_), Continuation::Native(_)) => {
                Err(engine_continuation_mismatch("vm", "native"))
            }
            (Self::Native(_), Continuation::Vm(_)) => {
                Err(engine_continuation_mismatch("native", "vm"))
            }
        }
    }

    /// Visit mutable local root slots from one live continuation.
    pub fn visit_continuation_root_slots(
        &mut self,
        continuation: &mut Continuation,
        visit: &mut dyn FnMut(heap::RootSlot<'_>) -> heap::HeapResult<()>,
    ) -> RuntimeResult<()> {
        match (self, continuation) {
            (Self::Vm(engine), Continuation::Vm(continuation)) => {
                vm::Isolate::visit_continuation_root_slots(engine, continuation, visit)
                    .map_err(Box::<RuntimeError>::from)
            }
            (Self::Native(_), Continuation::Native(_)) => Ok(()),
            (Self::Vm(_), Continuation::Native(_)) | (Self::Native(_), Continuation::Vm(_)) => {
                Ok(())
            }
        }
    }

    /// Visit roots from one captured continuation image.
    pub fn visit_continuation_image_roots(
        &mut self,
        continuation: &ContinuationImage,
        roots: &mut RootSink<'_>,
    ) -> RuntimeResult<()> {
        match self {
            Self::Vm(engine) => {
                let mut roots = VmRootSink { roots };

                engine
                    .visit_image_roots(continuation, &mut roots)
                    .map_err(Box::<RuntimeError>::from)
            }
            Self::Native(_) => Ok(()),
        }
    }

    /// Visit mutable local root slots from one captured continuation image.
    pub fn visit_continuation_image_root_slots(
        &mut self,
        continuation: &mut ContinuationImage,
        visit: &mut dyn FnMut(heap::RootSlot<'_>) -> heap::HeapResult<()>,
    ) -> RuntimeResult<()> {
        match self {
            Self::Vm(engine) => vm::Isolate::visit_image_root_slots(engine, continuation, visit)
                .map_err(Box::<RuntimeError>::from),
            Self::Native(_) => Ok(()),
        }
    }

    /// Fork this engine over one already-forked heap.
    pub fn fork(&mut self, heap: &mut heap::Heap) -> RuntimeResult<Self> {
        match self {
            Self::Vm(engine) => {
                let _ = heap;
                let engine = vm::Isolate::fork(engine).map_err(Box::<RuntimeError>::from)?;

                Ok(Self::Vm(engine))
            }
            Self::Native(engine) => {
                let engine = engine::Engine::fork(engine, heap).map_err(native_runtime_error)?;

                Ok(Self::Native(engine))
            }
        }
    }

    /// Capture one immutable engine image.
    pub fn image(&mut self) -> RuntimeResult<Image> {
        match self {
            Self::Vm(engine) => {
                let image = engine::Engine::image(engine).map_err(Box::<RuntimeError>::from)?;

                Ok(Image::Vm(image))
            }
            Self::Native(engine) => {
                let image = engine::Engine::image(engine).map_err(native_runtime_error)?;

                Ok(Image::Native(image))
            }
        }
    }

    /// Restore one immutable engine image.
    pub fn restore(&mut self, heap: &mut heap::Heap, image: &Image) -> RuntimeResult<()> {
        match (self, image) {
            (Self::Vm(engine), Image::Vm(image)) => {
                engine::Engine::restore(engine, heap, image).map_err(Box::<RuntimeError>::from)
            }
            (Self::Native(engine), Image::Native(image)) => {
                engine::Engine::restore(engine, heap, image).map_err(native_runtime_error)
            }
            (Self::Vm(_), Image::Native(_)) => Err(engine_image_mismatch("vm", "native")),
            (Self::Native(_), Image::Vm(_)) => Err(engine_image_mismatch("native", "vm")),
        }
    }

    /// Capture one continuation as one immutable continuation image.
    pub fn continuation_image(
        &mut self,
        continuation: &Continuation,
        mode: CaptureMode,
    ) -> RuntimeResult<ContinuationImage> {
        let _ = mode;

        match (self, continuation) {
            (Self::Vm(engine), Continuation::Vm(continuation)) => {
                vm::Isolate::continuation_image(engine, continuation)
                    .map_err(Box::<RuntimeError>::from)
            }
            (Self::Native(_), Continuation::Native(continuation)) => Ok(continuation.image.clone()),
            (Self::Vm(_), Continuation::Native(_)) => {
                Err(engine_continuation_mismatch("vm", "native"))
            }
            (Self::Native(_), Continuation::Vm(_)) => {
                Err(engine_continuation_mismatch("native", "vm"))
            }
        }
    }

    /// Restore one continuation from one immutable continuation image.
    pub fn restore_continuation_image(
        &mut self,
        image: &ContinuationImage,
    ) -> RuntimeResult<Continuation> {
        match self {
            Self::Vm(engine) => {
                let continuation = vm::Isolate::restore_continuation_image(engine, image)
                    .map_err(Box::<RuntimeError>::from)?;

                Ok(Continuation::Vm(continuation))
            }
            Self::Native(_) => Ok(Continuation::Native(native::Continuation::new(
                image.clone(),
            ))),
        }
    }

    /// Rebuild one runtime engine from an image.
    pub fn from_image(image: &Image) -> RuntimeResult<Self> {
        match image {
            Image::Vm(image) => {
                let engine = vm::Isolate::new(image.clone()).map_err(Box::<RuntimeError>::from)?;

                Ok(Self::Vm(engine))
            }
            Image::Native(_) => Err(RuntimeError::EngineUnsupported {
                engine: "native from_image".to_string(),
            }
            .boxed()),
        }
    }
}

impl std::fmt::Debug for Engine {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Vm(engine) => formatter.debug_tuple("Vm").field(engine).finish(),
            Self::Native(engine) => formatter.debug_tuple("Native").field(engine).finish(),
        }
    }
}

impl From<vm::Isolate> for Engine {
    fn from(engine: vm::Isolate) -> Self {
        Self::Vm(engine)
    }
}

impl From<native::Engine> for Engine {
    fn from(engine: native::Engine) -> Self {
        Self::Native(engine)
    }
}

/// Convert one VM execution outcome into one runtime outcome.
fn outcome_from_vm(outcome: vm::Outcome) -> Outcome<Continuation> {
    match outcome {
        vm::Outcome::Completed { output } => Outcome::Completed { output },
        vm::Outcome::Yielded {
            continuation,
            value,
        } => Outcome::Yielded {
            continuation: Continuation::Vm(continuation),
            value,
        },
    }
}

/// Convert one native execution outcome into one runtime outcome.
fn outcome_from_native(
    outcome: engine::Outcome<native::Continuation, engine::Value>,
) -> Outcome<Continuation> {
    match outcome {
        engine::Outcome::Completed { output } => Outcome::Completed { output },
        engine::Outcome::Yielded {
            continuation,
            value,
        } => Outcome::Yielded {
            continuation: Continuation::Native(continuation),
            value,
        },
    }
}

/// Return one engine continuation mismatch.
fn engine_continuation_mismatch(engine: &str, continuation: &str) -> Box<RuntimeError> {
    RuntimeError::EngineContinuationMismatch {
        engine: engine.to_string(),
        continuation: continuation.to_string(),
    }
    .boxed()
}

/// Return one engine image mismatch.
fn engine_image_mismatch(engine: &str, image: &str) -> Box<RuntimeError> {
    RuntimeError::EngineEntryMismatch {
        engine: engine.to_string(),
        entry: image.to_string(),
    }
    .boxed()
}

/// Convert one native backend error into one runtime error.
fn native_runtime_error(error: native::Error) -> Box<RuntimeError> {
    match error {
        native::Error::Unsupported { operation } => RuntimeError::EngineUnsupported {
            engine: format!("native {operation}"),
        }
        .boxed(),
        native::Error::EntryNotFound { name } => RuntimeError::EngineEntryMismatch {
            engine: "native".to_string(),
            entry: name,
        }
        .boxed(),
    }
}
