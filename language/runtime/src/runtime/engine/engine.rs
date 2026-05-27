use destack_engine as engine;
use destack_heap as heap;
use destack_native as native;
use destack_vm as vm;

use super::{CallContext, Continuation, ContinuationImage, Entry, Image, MemoryContext, Outcome};
use crate::diagnostic::{RuntimeError, RuntimeResult};

/// Execution backend owned by one worker.
pub enum Engine {
    /// VM interpreter backend.
    Vm(vm::Isolate),
    /// Native compiled backend.
    Native(native::Engine),
}

impl Engine {
    /// Initialize worker-owned static bytes.
    pub fn initialize(&mut self, context: MemoryContext<'_>) -> RuntimeResult<()> {
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
        context: CallContext<'_>,
        entry: &Entry,
        args: &[engine::Value],
    ) -> RuntimeResult<Outcome<Continuation>> {
        match self {
            Self::Vm(engine) => {
                let entry = engine
                    .entry_by_name(entry.name())
                    .map_err(Box::<RuntimeError>::from)?;
                let outcome = engine::Engine::run(engine, context, entry, args)
                    .map_err(Box::<RuntimeError>::from)?;

                Ok(outcome_from_vm(outcome))
            }
            Self::Native(engine) => {
                let entry = engine
                    .entry_by_name(entry.name())
                    .map_err(native_runtime_error)?;
                let outcome = engine::Engine::run(engine, context, entry, args)
                    .map_err(native_runtime_error)?;

                Ok(outcome_from_native(outcome))
            }
        }
    }

    /// Resume one continuation.
    pub fn resume(
        &mut self,
        context: CallContext<'_>,
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

    /// Visit mutable heap root slots from active backend state.
    pub fn visit_root_slots(
        &mut self,
        worker_static: &mut engine::StaticSpace,
        visit: &mut dyn FnMut(heap::RootSlot<'_>) -> heap::HeapResult<()>,
    ) -> RuntimeResult<()> {
        match self {
            Self::Vm(engine) => engine::Engine::visit_root_slots(engine, worker_static, visit)
                .map_err(Box::<RuntimeError>::from),
            Self::Native(engine) => engine::Engine::visit_root_slots(engine, worker_static, visit)
                .map_err(native_runtime_error),
        }
    }

    /// Visit mutable heap root slots from one live continuation.
    pub fn visit_continuation_root_slots(
        &mut self,
        continuation: &mut Continuation,
        visit: &mut dyn FnMut(heap::RootSlot<'_>) -> heap::HeapResult<()>,
    ) -> RuntimeResult<()> {
        match (self, continuation) {
            (Self::Vm(engine), Continuation::Vm(continuation)) => {
                engine::Engine::visit_continuation_root_slots(engine, continuation, visit)
                    .map_err(Box::<RuntimeError>::from)
            }
            (Self::Native(engine), Continuation::Native(continuation)) => {
                engine::Engine::visit_continuation_root_slots(engine, continuation, visit)
                    .map_err(native_runtime_error)
            }
            (Self::Vm(_), Continuation::Native(_)) | (Self::Native(_), Continuation::Vm(_)) => {
                Ok(())
            }
        }
    }

    /// Visit mutable heap root slots from one captured continuation image.
    pub fn visit_continuation_image_root_slots(
        &mut self,
        continuation: &mut ContinuationImage,
        visit: &mut dyn FnMut(heap::RootSlot<'_>) -> heap::HeapResult<()>,
    ) -> RuntimeResult<()> {
        match (self, continuation) {
            (Self::Vm(engine), ContinuationImage::Vm(continuation)) => {
                vm::Isolate::visit_image_root_slots(engine, continuation, visit)
                    .map_err(Box::<RuntimeError>::from)
            }
            (Self::Native(_), ContinuationImage::Native(_)) => Ok(()),
            (Self::Vm(_), ContinuationImage::Native(_))
            | (Self::Native(_), ContinuationImage::Vm(_)) => Ok(()),
        }
    }

    /// Fork this engine over already-forked memory.
    pub fn fork(&self, context: MemoryContext<'_>) -> RuntimeResult<Self> {
        match self {
            Self::Vm(engine) => {
                let engine =
                    engine::Engine::fork(engine, context).map_err(Box::<RuntimeError>::from)?;

                Ok(Self::Vm(engine))
            }
            Self::Native(engine) => {
                let engine = engine::Engine::fork(engine, context).map_err(native_runtime_error)?;

                Ok(Self::Native(engine))
            }
        }
    }

    /// Capture one immutable engine image.
    pub fn image(&self, context: MemoryContext<'_>) -> RuntimeResult<Image> {
        match self {
            Self::Vm(engine) => {
                let image =
                    engine::Engine::image(engine, context).map_err(Box::<RuntimeError>::from)?;

                Ok(Image::Vm(image))
            }
            Self::Native(engine) => {
                let image = engine::Engine::image(engine, context).map_err(native_runtime_error)?;

                Ok(Image::Native(image))
            }
        }
    }

    /// Restore one immutable engine image.
    pub fn restore(&mut self, context: MemoryContext<'_>, image: &Image) -> RuntimeResult<()> {
        match (self, image) {
            (Self::Vm(engine), Image::Vm(image)) => {
                engine::Engine::restore(engine, context, image).map_err(Box::<RuntimeError>::from)
            }
            (Self::Native(engine), Image::Native(image)) => {
                engine::Engine::restore(engine, context, image).map_err(native_runtime_error)
            }
            (Self::Vm(_), Image::Native(_)) => Err(engine_image_mismatch("vm", "native")),
            (Self::Native(_), Image::Vm(_)) => Err(engine_image_mismatch("native", "vm")),
        }
    }

    /// Capture one continuation as one immutable continuation image.
    pub fn continuation_image(
        &mut self,
        continuation: &Continuation,
    ) -> RuntimeResult<ContinuationImage> {
        match (self, continuation) {
            (Self::Vm(engine), Continuation::Vm(continuation)) => {
                let image = vm::Isolate::continuation_image(engine, continuation)
                    .map_err(Box::<RuntimeError>::from)?;

                Ok(ContinuationImage::Vm(image))
            }
            (Self::Native(_), Continuation::Native(continuation)) => {
                Ok(ContinuationImage::Native(continuation.continuation.clone()))
            }
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
        match (self, image) {
            (Self::Vm(engine), ContinuationImage::Vm(image)) => {
                let continuation = vm::Isolate::restore_continuation_image(engine, image)
                    .map_err(Box::<RuntimeError>::from)?;

                Ok(Continuation::Vm(continuation))
            }
            (Self::Native(_), ContinuationImage::Native(image)) => Ok(Continuation::Native(
                native::Continuation::new(image.clone()),
            )),
            (Self::Vm(_), ContinuationImage::Native(_)) => {
                Err(engine_continuation_mismatch("vm", "native"))
            }
            (Self::Native(_), ContinuationImage::Vm(_)) => {
                Err(engine_continuation_mismatch("native", "vm"))
            }
        }
    }

    /// Rebuild one runtime engine from an image.
    pub fn from_image(image: &Image) -> RuntimeResult<Self> {
        match image {
            Image::Vm(image) => {
                let engine =
                    vm::Isolate::from_image(image.clone()).map_err(Box::<RuntimeError>::from)?;

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
        vm::Outcome::Completed { value } => Outcome::Completed { value },
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
        engine::Outcome::Completed { value } => Outcome::Completed { value },
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
    RuntimeError::EngineImageMismatch {
        engine: engine.to_string(),
        image: image.to_string(),
    }
    .boxed()
}

/// Convert one native backend error into one runtime error.
fn native_runtime_error(error: native::Error) -> Box<RuntimeError> {
    match error {
        native::Error::YieldedWithoutContinuation { .. } => RuntimeError::EngineYieldMissing {
            engine: "native".to_string(),
        }
        .boxed(),
        native::Error::Trapped { .. } => RuntimeError::EngineTrap {
            engine: "native".to_string(),
        }
        .boxed(),
        native::Error::DeoptimizedWithoutMaterialization { .. } => {
            RuntimeError::EngineDeoptMissing {
                engine: "native".to_string(),
            }
            .boxed()
        }
        native::Error::Panicked { .. } => RuntimeError::EnginePanic {
            engine: "native".to_string(),
        }
        .boxed(),
        native::Error::InvalidStatus(error) => RuntimeError::EngineUnsupported {
            engine: format!("native status {error}"),
        }
        .boxed(),
        native::Error::InvalidTrap(error) => RuntimeError::EngineUnsupported {
            engine: format!("native trap {error}"),
        }
        .boxed(),
        native::Error::Value(error) => RuntimeError::EngineUnsupported {
            engine: format!("native value {error}"),
        }
        .boxed(),
        native::Error::ContinuationUnavailable => RuntimeError::EngineUnsupported {
            engine: "native continuation".to_string(),
        }
        .boxed(),
        native::Error::ImageEngineMismatch {
            engine_id,
            image_engine_id,
        } => RuntimeError::EngineImageMismatch {
            engine: format!("native {}", engine_id.get()),
            image: format!("native {}", image_engine_id.get()),
        }
        .boxed(),
        native::Error::RootMapUnavailable => RuntimeError::EngineUnsupported {
            engine: "native root map".to_string(),
        }
        .boxed(),
        native::Error::EntryNotFound { name } => RuntimeError::EngineEntryMismatch {
            engine: "native".to_string(),
            entry: name,
        }
        .boxed(),
    }
}
