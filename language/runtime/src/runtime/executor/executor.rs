use std::sync::Arc;

use destack_heap as heap;
use destack_mir as mir;
use destack_native as native;
use destack_program as program;
use destack_vm as vm;

use super::{
    Continuation, ContinuationImage, Entry, ExecutionCall, ExecutionMemory, ExecutorId, Image,
    Outcome,
};
use crate::diagnostic::{ExecutorError, RuntimeError, RuntimeResult};

const NATIVE_EXECUTOR: &str = "native";
const VM_EXECUTOR: &str = "vm";

/// Worker-owned runtime executor.
pub struct Executor {
    /// Stable executor identity.
    id: ExecutorId,
    /// Concrete execution backend.
    backend: Backend,
}

/// Concrete execution backend before worker ownership is attached.
pub enum Backend {
    /// VM machine backend.
    Vm(Box<vm::Machine>),
    /// Native compiled backend.
    Native(native::Executor),
}

impl Backend {
    /// Return the program trace table used by heap metadata.
    pub fn trace_table(&self) -> RuntimeResult<Arc<mir::TraceTable>> {
        match self {
            Self::Vm(executor) => Ok(executor.trace_table()),
            Self::Native(executor) => Ok(executor.program().trace_table()),
        }
    }

    /// Return immutable program constants.
    pub fn constants(&self) -> program::StaticSpace {
        match self {
            Self::Vm(executor) => executor.constants().clone(),
            Self::Native(executor) => executor.program().constants().clone(),
        }
    }

    /// Return initial shared static storage.
    pub fn shared_statics(&self) -> program::StaticSpace {
        match self {
            Self::Vm(executor) => executor.shared_statics().clone(),
            Self::Native(executor) => executor.program().shared_statics().clone(),
        }
    }
}

impl Executor {
    /// Create one worker-owned executor.
    pub const fn new(id: ExecutorId, backend: Backend) -> Self {
        Self { id, backend }
    }

    /// Return the stable executor identity.
    pub const fn id(&self) -> ExecutorId {
        self.id
    }

    /// Return the program trace table used by heap metadata.
    pub fn trace_table(&self) -> RuntimeResult<Arc<mir::TraceTable>> {
        self.backend.trace_table()
    }

    /// Return immutable program constants.
    pub fn constants(&self) -> program::StaticSpace {
        self.backend.constants()
    }

    /// Return initial shared static storage.
    pub fn shared_statics(&self) -> program::StaticSpace {
        self.backend.shared_statics()
    }

    /// Initialize worker-owned static bytes.
    pub fn initialize(&mut self, context: ExecutionMemory<'_>) -> RuntimeResult<()> {
        match &mut self.backend {
            Backend::Vm(executor) => vm::Machine::initialize(
                executor.as_mut(),
                context.heap,
                context.shared_heap,
                context.local_static,
                context.shared_static,
            )
            .map_err(Box::<RuntimeError>::from),
            Backend::Native(executor) => executor.initialize(context).map_err(native_runtime_error),
        }
    }

    /// Run one entrypoint.
    pub fn run(
        &mut self,
        context: ExecutionCall<'_>,
        entry: &Entry,
        args: &[program::Value],
    ) -> RuntimeResult<Outcome<Continuation>> {
        let id = self.id;

        match &mut self.backend {
            Backend::Vm(executor) => {
                let entry = executor
                    .entry_by_name(entry.name())
                    .map_err(Box::<RuntimeError>::from)?;
                let context = context.memory;
                let function_id = executor.function_for_entry(entry);
                let outcome = executor
                    .run_function_yielding(
                        context.local_static,
                        context.shared_static,
                        context.heap,
                        context.shared_heap,
                        context.shared_cache,
                        context.shared_gc_worker,
                        function_id,
                        args,
                    )
                    .map_err(Box::<RuntimeError>::from)?;

                Ok(outcome_from_vm(id, outcome))
            }
            Backend::Native(executor) => {
                let entry = executor
                    .entry_by_name(entry.name())
                    .map_err(native_runtime_error)?;
                let outcome = executor
                    .run(context, entry, args)
                    .map_err(native_runtime_error)?;

                Ok(outcome_from_native(id, outcome))
            }
        }
    }

    /// Resume one continuation.
    pub fn resume(
        &mut self,
        context: ExecutionCall<'_>,
        continuation: Continuation,
        value: program::Value,
    ) -> RuntimeResult<Outcome<Continuation>> {
        if continuation.executor() != self.id {
            return Err(executor_continuation_mismatch(
                &executor_name(self.kind(), self.id),
                &executor_name(continuation_kind(&continuation), continuation.executor()),
            ));
        }

        let id = self.id;
        match (&mut self.backend, continuation) {
            (Backend::Vm(executor), Continuation::Vm { continuation, .. }) => {
                let context = context.memory;
                let outcome = vm::Machine::resume(
                    executor.as_mut(),
                    context.local_static,
                    context.shared_static,
                    context.heap,
                    context.shared_heap,
                    context.shared_cache,
                    context.shared_gc_worker,
                    continuation,
                    value,
                )
                .map_err(Box::<RuntimeError>::from)?;

                Ok(outcome_from_vm(id, outcome))
            }
            (Backend::Native(executor), Continuation::Native { continuation, .. }) => {
                let outcome = executor
                    .resume(context, continuation, value)
                    .map_err(native_runtime_error)?;

                Ok(outcome_from_native(id, outcome))
            }
            (Backend::Vm(_), Continuation::Native { .. }) => {
                Err(executor_continuation_mismatch(VM_EXECUTOR, NATIVE_EXECUTOR))
            }
            (Backend::Native(_), Continuation::Vm { .. }) => {
                Err(executor_continuation_mismatch(NATIVE_EXECUTOR, VM_EXECUTOR))
            }
        }
    }

    /// Visit mutable heap root slots from active backend state.
    pub fn visit_root_slots(
        &mut self,
        local_static: &mut program::StaticSpace,
        visit: &mut dyn FnMut(heap::RootSlot<'_>) -> heap::HeapResult<()>,
    ) -> RuntimeResult<()> {
        match &mut self.backend {
            Backend::Vm(executor) => {
                vm::Machine::visit_root_slots(executor.as_mut(), local_static, &mut [], visit)
                    .map_err(Box::<RuntimeError>::from)
            }
            Backend::Native(executor) => executor
                .visit_root_slots(local_static, visit)
                .map_err(native_runtime_error),
        }
    }

    /// Visit mutable heap root slots from one static space.
    pub fn visit_static_root_slots(
        &mut self,
        static_space: &mut program::StaticSpace,
        visit: &mut dyn FnMut(heap::RootSlot<'_>) -> heap::HeapResult<()>,
    ) -> RuntimeResult<()> {
        match &mut self.backend {
            Backend::Vm(executor) => {
                vm::Machine::visit_static_root_slots(executor.as_mut(), static_space, visit)
                    .map_err(Box::<RuntimeError>::from)
            }
            Backend::Native(_) => Ok(()),
        }
    }

    /// Visit mutable heap root slots from one live continuation.
    pub fn visit_continuation_root_slots(
        &mut self,
        continuation: &mut Continuation,
        visit: &mut dyn FnMut(heap::RootSlot<'_>) -> heap::HeapResult<()>,
    ) -> RuntimeResult<()> {
        if continuation.executor() != self.id {
            return Ok(());
        }

        match (&mut self.backend, continuation) {
            (Backend::Vm(executor), Continuation::Vm { continuation, .. }) => {
                vm::Machine::visit_continuation_root_slots(executor.as_mut(), continuation, visit)
                    .map_err(Box::<RuntimeError>::from)
            }
            (Backend::Native(executor), Continuation::Native { continuation, .. }) => executor
                .visit_continuation_root_slots(continuation, visit)
                .map_err(native_runtime_error),
            (Backend::Vm(_), Continuation::Native { .. })
            | (Backend::Native(_), Continuation::Vm { .. }) => Ok(()),
        }
    }

    /// Visit mutable heap root slots from one captured continuation image.
    pub fn visit_continuation_image_root_slots(
        &mut self,
        continuation: &mut ContinuationImage,
        visit: &mut dyn FnMut(heap::RootSlot<'_>) -> heap::HeapResult<()>,
    ) -> RuntimeResult<()> {
        if continuation.executor() != self.id {
            return Ok(());
        }

        match (&mut self.backend, continuation) {
            (Backend::Vm(executor), ContinuationImage::Vm { image, .. }) => {
                vm::Machine::visit_image_root_slots(executor.as_mut(), image, visit)
                    .map_err(Box::<RuntimeError>::from)
            }
            (Backend::Native(_), ContinuationImage::Native { .. }) => Ok(()),
            (Backend::Vm(_), ContinuationImage::Native { .. })
            | (Backend::Native(_), ContinuationImage::Vm { .. }) => Ok(()),
        }
    }

    /// Fork this executor over already-forked memory.
    pub fn fork(&self, context: ExecutionMemory<'_>) -> RuntimeResult<Self> {
        let backend = match &self.backend {
            Backend::Vm(executor) => {
                let _context = context;
                let executor =
                    vm::Machine::fork(executor.as_ref()).map_err(Box::<RuntimeError>::from)?;

                Backend::Vm(Box::new(executor))
            }
            Backend::Native(executor) => {
                let executor = executor.fork(context).map_err(native_runtime_error)?;

                Backend::Native(executor)
            }
        };

        Ok(Self::new(self.id, backend))
    }

    /// Capture one immutable executor image.
    pub fn image(&self, context: ExecutionMemory<'_>) -> RuntimeResult<Image> {
        match &self.backend {
            Backend::Vm(executor) => {
                let _context = context;
                let image =
                    vm::Machine::image(executor.as_ref()).map_err(Box::<RuntimeError>::from)?;

                Ok(Image::Vm {
                    executor: self.id,
                    image: Arc::new(image),
                })
            }
            Backend::Native(executor) => {
                let image = executor
                    .image_with_memory(context)
                    .map_err(native_runtime_error)?;

                Ok(Image::Native {
                    executor: self.id,
                    image,
                })
            }
        }
    }

    /// Restore one immutable executor image.
    pub fn restore(&mut self, context: ExecutionMemory<'_>, image: &Image) -> RuntimeResult<()> {
        if image.executor() != self.id {
            return Err(executor_image_mismatch(
                &executor_name(self.kind(), self.id),
                &executor_name(image_kind(image), image.executor()),
            ));
        }

        match (&mut self.backend, image) {
            (Backend::Vm(executor), Image::Vm { image, .. }) => {
                let _context = context;
                vm::Machine::restore_image(executor.as_mut(), image)
                    .map_err(Box::<RuntimeError>::from)
            }
            (Backend::Native(executor), Image::Native { image, .. }) => executor
                .restore(context, image)
                .map_err(native_runtime_error),
            (Backend::Vm(_), Image::Native { .. }) => {
                Err(executor_image_mismatch(VM_EXECUTOR, NATIVE_EXECUTOR))
            }
            (Backend::Native(_), Image::Vm { .. }) => {
                Err(executor_image_mismatch(NATIVE_EXECUTOR, VM_EXECUTOR))
            }
        }
    }

    /// Capture one continuation as one immutable continuation image.
    pub fn continuation_image(
        &mut self,
        continuation: &Continuation,
    ) -> RuntimeResult<ContinuationImage> {
        if continuation.executor() != self.id {
            return Err(executor_continuation_mismatch(
                &executor_name(self.kind(), self.id),
                &executor_name(continuation_kind(continuation), continuation.executor()),
            ));
        }

        match (&mut self.backend, continuation) {
            (Backend::Vm(executor), Continuation::Vm { continuation, .. }) => {
                let image = vm::Machine::continuation_image(executor.as_ref(), continuation)
                    .map_err(Box::<RuntimeError>::from)?;

                Ok(ContinuationImage::Vm {
                    executor: self.id,
                    image,
                })
            }
            (Backend::Native(_), Continuation::Native { continuation, .. }) => {
                Ok(ContinuationImage::Native {
                    executor: self.id,
                    image: continuation.continuation.clone(),
                })
            }
            (Backend::Vm(_), Continuation::Native { .. }) => {
                Err(executor_continuation_mismatch(VM_EXECUTOR, NATIVE_EXECUTOR))
            }
            (Backend::Native(_), Continuation::Vm { .. }) => {
                Err(executor_continuation_mismatch(NATIVE_EXECUTOR, VM_EXECUTOR))
            }
        }
    }

    /// Restore one continuation from one immutable continuation image.
    pub fn restore_continuation_image(
        &mut self,
        image: &ContinuationImage,
    ) -> RuntimeResult<Continuation> {
        if image.executor() != self.id {
            return Err(executor_continuation_mismatch(
                &executor_name(self.kind(), self.id),
                &executor_name(continuation_image_kind(image), image.executor()),
            ));
        }

        match (&mut self.backend, image) {
            (Backend::Vm(executor), ContinuationImage::Vm { image, .. }) => {
                let continuation =
                    vm::Machine::restore_continuation_image(executor.as_ref(), image)
                        .map_err(Box::<RuntimeError>::from)?;

                Ok(Continuation::Vm {
                    executor: self.id,
                    continuation,
                })
            }
            (Backend::Native(_), ContinuationImage::Native { image, .. }) => {
                Ok(Continuation::Native {
                    executor: self.id,
                    continuation: native::Continuation::new(image.clone()),
                })
            }
            (Backend::Vm(_), ContinuationImage::Native { .. }) => {
                Err(executor_continuation_mismatch(VM_EXECUTOR, NATIVE_EXECUTOR))
            }
            (Backend::Native(_), ContinuationImage::Vm { .. }) => {
                Err(executor_continuation_mismatch(NATIVE_EXECUTOR, VM_EXECUTOR))
            }
        }
    }

    /// Rebuild one runtime executor from an image.
    pub fn from_image(id: ExecutorId, image: &Image) -> RuntimeResult<Self> {
        if image.executor() != id {
            return Err(executor_image_mismatch(
                &executor_name(image_kind(image), id),
                &executor_name(image_kind(image), image.executor()),
            ));
        }

        match image {
            Image::Vm { image, .. } => {
                let executor =
                    vm::Machine::from_image(image.clone()).map_err(Box::<RuntimeError>::from)?;

                Ok(Self::new(id, Backend::Vm(Box::new(executor))))
            }
            Image::Native { .. } => Err(executor_error(
                NATIVE_EXECUTOR,
                ExecutorError::Unsupported {
                    feature: "image restore".to_string(),
                },
            )),
        }
    }

    /// Return the concrete executor kind.
    fn kind(&self) -> &'static str {
        match &self.backend {
            Backend::Vm(_) => VM_EXECUTOR,
            Backend::Native(_) => NATIVE_EXECUTOR,
        }
    }
}

impl std::fmt::Debug for Executor {
    /// Format the executor without exposing runtime internals.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Executor")
            .field("id", &self.id)
            .field("backend", &self.backend)
            .finish()
    }
}

impl std::fmt::Debug for Backend {
    /// Format the backend without exposing runtime internals.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Vm(executor) => formatter.debug_tuple("Vm").field(executor).finish(),
            Self::Native(executor) => formatter.debug_tuple("Native").field(executor).finish(),
        }
    }
}

impl From<vm::Machine> for Backend {
    /// Wrap one VM machine as an unbound runtime backend.
    fn from(machine: vm::Machine) -> Self {
        Self::Vm(Box::new(machine))
    }
}

impl From<native::Executor> for Backend {
    /// Wrap one native executor as an unbound runtime backend.
    fn from(executor: native::Executor) -> Self {
        Self::Native(executor)
    }
}

/// Convert one VM execution outcome into one runtime outcome.
fn outcome_from_vm(id: ExecutorId, outcome: vm::Outcome) -> Outcome<Continuation> {
    match outcome {
        vm::Outcome::Completed { value } => Outcome::Completed { value },
        vm::Outcome::Yielded {
            continuation,
            value,
        } => Outcome::Yielded {
            continuation: Continuation::Vm {
                executor: id,
                continuation,
            },
            value,
        },
    }
}

/// Convert one native execution outcome into one runtime outcome.
fn outcome_from_native(
    id: ExecutorId,
    outcome: program::Outcome<native::Continuation, program::Value>,
) -> Outcome<Continuation> {
    match outcome {
        program::Outcome::Completed { value } => Outcome::Completed { value },
        program::Outcome::Yielded {
            continuation,
            value,
        } => Outcome::Yielded {
            continuation: Continuation::Native {
                executor: id,
                continuation,
            },
            value,
        },
    }
}

/// Return one continuation backend kind.
fn continuation_kind(continuation: &Continuation) -> &'static str {
    match continuation {
        Continuation::Vm { .. } => VM_EXECUTOR,
        Continuation::Native { .. } => NATIVE_EXECUTOR,
    }
}

/// Return one continuation image backend kind.
fn continuation_image_kind(image: &ContinuationImage) -> &'static str {
    match image {
        ContinuationImage::Vm { .. } => VM_EXECUTOR,
        ContinuationImage::Native { .. } => NATIVE_EXECUTOR,
    }
}

/// Return one execution image backend kind.
fn image_kind(image: &Image) -> &'static str {
    match image {
        Image::Vm { .. } => VM_EXECUTOR,
        Image::Native { .. } => NATIVE_EXECUTOR,
    }
}

/// Return one executor display name.
fn executor_name(kind: &str, id: ExecutorId) -> String {
    format!("{kind} {}", id.get())
}

/// Return one executor continuation mismatch.
fn executor_continuation_mismatch(executor: &str, continuation: &str) -> Box<RuntimeError> {
    executor_error(
        executor,
        ExecutorError::ContinuationMismatch {
            continuation: continuation.to_string(),
        },
    )
}

/// Return one executor image mismatch.
fn executor_image_mismatch(executor: &str, image: &str) -> Box<RuntimeError> {
    executor_error(
        executor,
        ExecutorError::ImageMismatch {
            image: image.to_string(),
        },
    )
}

/// Return one executor error.
fn executor_error(executor: &str, reason: ExecutorError) -> Box<RuntimeError> {
    RuntimeError::Executor {
        executor: executor.to_string(),
        reason,
    }
    .boxed()
}

/// Convert one native backend error into one runtime error.
fn native_runtime_error(error: native::Error) -> Box<RuntimeError> {
    match error {
        native::Error::YieldedWithoutContinuation { .. } => {
            executor_error(NATIVE_EXECUTOR, ExecutorError::YieldMissing)
        }
        native::Error::Trapped { .. } => executor_error(NATIVE_EXECUTOR, ExecutorError::Trap),
        native::Error::DeoptimizedWithoutMaterialization { .. } => {
            executor_error(NATIVE_EXECUTOR, ExecutorError::DeoptMissing)
        }
        native::Error::Panicked { .. } => executor_error(NATIVE_EXECUTOR, ExecutorError::Panic),
        native::Error::InvalidStatus(error) => executor_error(
            NATIVE_EXECUTOR,
            ExecutorError::Unsupported {
                feature: format!("status {error}"),
            },
        ),
        native::Error::InvalidTrap(error) => executor_error(
            NATIVE_EXECUTOR,
            ExecutorError::Unsupported {
                feature: format!("trap {error}"),
            },
        ),
        native::Error::Value(error) => executor_error(
            NATIVE_EXECUTOR,
            ExecutorError::Unsupported {
                feature: format!("value {error}"),
            },
        ),
        native::Error::ContinuationUnavailable => executor_error(
            NATIVE_EXECUTOR,
            ExecutorError::Unsupported {
                feature: "continuation".to_string(),
            },
        ),
        native::Error::RootMapUnavailable => executor_error(
            NATIVE_EXECUTOR,
            ExecutorError::Unsupported {
                feature: "root map".to_string(),
            },
        ),
        native::Error::EntryNotFound { name } => executor_error(
            NATIVE_EXECUTOR,
            ExecutorError::EntryUnavailable { entry: name },
        ),
    }
}
