use std::sync::Arc;

use destack_heap::{HeapResult, RootSlot, TraceView};
use destack_program as program;
use destack_vm as vm;
use serde::{Deserialize, Serialize};

use super::{Continuation, Entry, Image, Outcome, ProgramActivation, ProgramStorage, native};
use crate::diagnostic::{MachineError, RuntimeError, RuntimeResult};

const NATIVE_MACHINE: &str = "native";
const VM_MACHINE: &str = "vm";

/// Stable identifier for one worker-owned machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct MachineId(pub u64);

impl MachineId {
    /// Create one machine identifier.
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Return the raw machine identifier value.
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Worker-owned runtime machine.
pub struct Machine {
    /// Stable machine identity.
    id: MachineId,
    /// Selected execution engine.
    engine: Engine,
}

/// Runtime execution strategy shared by worker machines.
#[derive(Clone)]
pub enum Execution {
    /// Execute with the VM only.
    Vm {
        /// VM machine options.
        options: vm::MachineOptions,
    },
    /// Execute with native code and VM fallback.
    Native {
        /// VM machine options.
        options: vm::MachineOptions,
        /// Process-local linked native code.
        code: Arc<native::Code>,
    },
}

/// Captured execution strategy for one runtime image.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ExecutionImage {
    /// VM-only execution.
    Vm {
        /// VM machine options.
        options: vm::MachineOptions,
    },
    /// Native-capable execution.
    Native {
        /// VM machine options.
        options: vm::MachineOptions,
    },
}

/// Selected execution engine.
pub(crate) enum Engine {
    /// VM execution machine.
    Vm(Box<vm::Machine>),
    /// Native-capable machine with VM fallback.
    Native {
        /// VM machine used for fallback and deoptimization.
        vm: Box<vm::Machine>,
        /// Linked native code.
        code: Arc<native::Code>,
    },
}

impl Execution {
    /// Create VM-only execution.
    pub const fn vm(options: vm::MachineOptions) -> Self {
        Self::Vm { options }
    }

    /// Create native-capable execution.
    pub fn native(options: vm::MachineOptions, code: native::Code) -> Self {
        Self::Native {
            options,
            code: Arc::new(code),
        }
    }

    /// Return the VM options used by worker machines.
    pub const fn options(&self) -> &vm::MachineOptions {
        match self {
            Self::Vm { options } | Self::Native { options, .. } => options,
        }
    }

    /// Capture this execution strategy without process-local native code.
    pub fn image(&self) -> ExecutionImage {
        match self {
            Self::Vm { options } => ExecutionImage::Vm {
                options: options.clone(),
            },
            Self::Native { options, .. } => ExecutionImage::Native {
                options: options.clone(),
            },
        }
    }

    /// Restore one execution strategy for one durable program.
    pub fn from_image(
        program: &program::Program,
        image: &ExecutionImage,
        native_linker: Option<&dyn native::Linker>,
    ) -> RuntimeResult<Self> {
        match image {
            ExecutionImage::Vm { options } => Ok(Self::vm(options.clone())),
            ExecutionImage::Native { options } => {
                let Some(linker) = native_linker else {
                    return Err(machine_error(
                        NATIVE_MACHINE,
                        MachineError::Unsupported {
                            feature: "image restore without native linker".to_string(),
                        },
                    ));
                };
                let code = linker.load(program).map_err(native_runtime_error)?;

                Ok(Self::native(options.clone(), code))
            }
        }
    }
}

impl Engine {
    /// Create one execution engine for one program.
    pub(crate) fn new(
        program: Arc<program::Program>,
        execution: &Execution,
    ) -> RuntimeResult<Self> {
        let machine = vm::Machine::new(program, execution.options().clone())
            .map_err(Box::<RuntimeError>::from)?;

        match execution {
            Execution::Vm { .. } => Ok(Self::Vm(Box::new(machine))),
            Execution::Native { code, .. } => Ok(Self::Native {
                vm: Box::new(machine),
                code: code.clone(),
            }),
        }
    }
}

impl Machine {
    /// Create one worker-owned machine.
    pub fn new(
        id: MachineId,
        program: Arc<program::Program>,
        execution: &Execution,
    ) -> RuntimeResult<Self> {
        let engine = Engine::new(program, execution)?;

        Ok(Self { id, engine })
    }

    /// Return the stable machine identity.
    pub const fn id(&self) -> MachineId {
        self.id
    }

    /// Return the immutable program.
    pub fn program(&self) -> &program::Program {
        match &self.engine {
            Engine::Vm(machine) => machine.program(),
            Engine::Native { vm, .. } => vm.program(),
        }
    }

    /// Return compact program trace rows.
    pub fn trace_view(&self) -> TraceView<'_> {
        match &self.engine {
            Engine::Vm(machine) => machine.trace_view(),
            Engine::Native { vm, .. } => vm.trace_view(),
        }
    }

    /// Initialize worker-owned static bytes.
    pub fn initialize(&mut self, context: ProgramStorage<'_>) -> RuntimeResult<()> {
        match &mut self.engine {
            Engine::Vm(machine) => vm::Machine::initialize(
                machine.as_mut(),
                context.heap,
                context.shared_heap,
                context.local_static,
                context.shared_static,
            )
            .map_err(Box::<RuntimeError>::from),
            Engine::Native { vm, .. } => vm::Machine::initialize(
                vm.as_mut(),
                context.heap,
                context.shared_heap,
                context.local_static,
                context.shared_static,
            )
            .map_err(Box::<RuntimeError>::from),
        }
    }

    /// Run one entrypoint.
    pub fn run(
        &mut self,
        mut context: ProgramActivation<'_>,
        entry: &Entry,
        args: &[program::Value],
        stop_points: Option<&program::StopSet>,
        watch_points: Option<&program::WatchSet>,
        mut profile: Option<&mut program::Profile>,
    ) -> RuntimeResult<Outcome<Continuation>> {
        let id = self.id;

        match &mut self.engine {
            Engine::Vm(machine) => {
                let entry = machine
                    .entry_by_name(entry.name())
                    .map_err(Box::<RuntimeError>::from)?;
                let context = context.storage;
                let function_id = entry.function();
                let outcome = machine
                    .run_function_yielding(
                        context.local_static,
                        context.shared_static,
                        context.heap,
                        context.shared_heap,
                        context.shared_cache,
                        context.shared_mark_worker,
                        stop_points,
                        watch_points,
                        profile.as_deref_mut(),
                        function_id,
                        args,
                    )
                    .map_err(Box::<RuntimeError>::from)?;

                Ok(outcome_from_vm(id, outcome))
            }
            Engine::Native { vm, code } => {
                // reject native profiling until native code emits profile hooks
                if profile.is_some() {
                    return Err(machine_error(
                        NATIVE_MACHINE,
                        MachineError::Unsupported {
                            feature: "profile recording".to_string(),
                        },
                    ));
                }

                let program = vm.program();

                // enter native code when no VM-only runtime hooks are active
                if stop_points.is_none_or(program::StopSet::is_empty)
                    && watch_points.is_none_or(program::WatchSet::is_empty)
                {
                    match code.entry_by_name(program, entry.name()) {
                        Ok(entry) => {
                            let outcome = code
                                .run(program, &mut context, entry, args)
                                .map_err(native_runtime_error)?;

                            return outcome_from_native(id, vm.as_mut(), context, outcome);
                        }
                        Err(native::Error::EntryNotFound { .. }) => {}
                        Err(error) => return Err(native_runtime_error(error)),
                    }
                }

                // fall back to VM when stop or watch hooks are active
                let entry = vm
                    .entry_by_name(entry.name())
                    .map_err(Box::<RuntimeError>::from)?;
                let context = context.storage;
                let function_id = entry.function();
                let outcome = vm
                    .run_function_yielding(
                        context.local_static,
                        context.shared_static,
                        context.heap,
                        context.shared_heap,
                        context.shared_cache,
                        context.shared_mark_worker,
                        stop_points,
                        watch_points,
                        profile.as_deref_mut(),
                        function_id,
                        args,
                    )
                    .map_err(Box::<RuntimeError>::from)?;

                Ok(outcome_from_vm(id, outcome))
            }
        }
    }

    /// Resume one continuation.
    pub fn resume(
        &mut self,
        mut context: ProgramActivation<'_>,
        continuation: Continuation,
        value: program::Value,
        stop_points: Option<&program::StopSet>,
        watch_points: Option<&program::WatchSet>,
        mut profile: Option<&mut program::Profile>,
    ) -> RuntimeResult<Outcome<Continuation>> {
        if continuation.machine() != self.id {
            return Err(machine_continuation_mismatch(
                &machine_name(self.kind(), self.id),
                &machine_name("continuation", continuation.machine()),
            ));
        }

        let id = self.id;
        let continuation = continuation.program;

        match &mut self.engine {
            Engine::Vm(machine) => {
                let context = context.storage;
                let outcome = vm::Machine::resume(
                    machine.as_mut(),
                    context.local_static,
                    context.shared_static,
                    context.heap,
                    context.shared_heap,
                    context.shared_cache,
                    context.shared_mark_worker,
                    stop_points,
                    watch_points,
                    profile.as_deref_mut(),
                    continuation,
                    value,
                )
                .map_err(Box::<RuntimeError>::from)?;

                Ok(outcome_from_vm(id, outcome))
            }
            Engine::Native { code, vm } => {
                // reject native profiling until native code emits profile hooks
                if profile.is_some() {
                    return Err(machine_error(
                        NATIVE_MACHINE,
                        MachineError::Unsupported {
                            feature: "profile recording".to_string(),
                        },
                    ));
                }

                let program = vm.program();

                // resume natively when a matching resume entry exists
                if stop_points.is_none_or(program::StopSet::is_empty)
                    && watch_points.is_none_or(program::WatchSet::is_empty)
                    && code
                        .can_resume(&continuation)
                        .map_err(native_runtime_error)?
                {
                    let outcome = code
                        .resume(program, &mut context, continuation, value)
                        .map_err(native_runtime_error)?;

                    outcome_from_native(id, vm.as_mut(), context, outcome)
                }
                // otherwise resume through the VM with the same durable continuation
                else {
                    let context = context.storage;
                    let outcome = vm::Machine::resume(
                        vm.as_mut(),
                        context.local_static,
                        context.shared_static,
                        context.heap,
                        context.shared_heap,
                        context.shared_cache,
                        context.shared_mark_worker,
                        stop_points,
                        watch_points,
                        profile.as_deref_mut(),
                        continuation,
                        value,
                    )
                    .map_err(Box::<RuntimeError>::from)?;

                    Ok(outcome_from_vm(id, outcome))
                }
            }
        }
    }

    /// Continue one stopped continuation without a resume value.
    pub fn continue_continuation(
        &mut self,
        context: ProgramActivation<'_>,
        continuation: Continuation,
        stop_points: Option<&program::StopSet>,
        watch_points: Option<&program::WatchSet>,
        mut profile: Option<&mut program::Profile>,
        resume_skip: Option<program::ResumeSkip>,
    ) -> RuntimeResult<Outcome<Continuation>> {
        if continuation.machine() != self.id {
            return Err(machine_continuation_mismatch(
                &machine_name(self.kind(), self.id),
                &machine_name("continuation", continuation.machine()),
            ));
        }

        let id = self.id;
        let continuation = continuation.program;
        let context = context.storage;

        let vm = match &mut self.engine {
            Engine::Vm(machine) => machine.as_mut(),
            Engine::Native { .. } if profile.is_some() => {
                // reject native profiling until native code emits profile hooks
                return Err(machine_error(
                    NATIVE_MACHINE,
                    MachineError::Unsupported {
                        feature: "profile recording".to_string(),
                    },
                ));
            }
            Engine::Native { vm, .. } => vm.as_mut(),
        };
        let outcome = vm::Machine::continue_continuation(
            vm,
            context.local_static,
            context.shared_static,
            context.heap,
            context.shared_heap,
            context.shared_cache,
            context.shared_mark_worker,
            stop_points,
            watch_points,
            profile.as_deref_mut(),
            resume_skip,
            continuation,
        )
        .map_err(Box::<RuntimeError>::from)?;

        Ok(outcome_from_vm(id, outcome))
    }

    /// Visit mutable heap root slots from active machine state.
    pub fn visit_root_slots(
        &mut self,
        local_static: &mut program::StaticSpace,
        visit: &mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>,
    ) -> RuntimeResult<()> {
        match &mut self.engine {
            Engine::Vm(machine) => {
                vm::Machine::visit_root_slots(machine.as_mut(), local_static, &mut [], visit)
                    .map_err(Box::<RuntimeError>::from)
            }
            Engine::Native { vm, .. } => {
                vm::Machine::visit_root_slots(vm.as_mut(), local_static, &mut [], visit)
                    .map_err(Box::<RuntimeError>::from)
            }
        }
    }

    /// Visit mutable heap root slots from one static space.
    pub fn visit_static_root_slots(
        &mut self,
        location: program::GlobalLocation,
        static_space: &mut program::StaticSpace,
        visit: &mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>,
    ) -> RuntimeResult<()> {
        match &mut self.engine {
            Engine::Vm(machine) => vm::Machine::visit_static_root_slots(
                machine.as_mut(),
                location,
                static_space,
                visit,
            )
            .map_err(Box::<RuntimeError>::from),
            Engine::Native { vm, .. } => {
                vm::Machine::visit_static_root_slots(vm.as_mut(), location, static_space, visit)
                    .map_err(Box::<RuntimeError>::from)
            }
        }
    }

    /// Visit mutable heap root slots from one live continuation.
    pub fn visit_continuation_root_slots(
        &mut self,
        continuation: &mut Continuation,
        visit: &mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>,
    ) -> RuntimeResult<()> {
        if continuation.machine() != self.id {
            return Ok(());
        }

        match &mut self.engine {
            Engine::Vm(machine) => vm::Machine::visit_continuation_root_slots(
                machine.as_mut(),
                &mut continuation.program,
                visit,
            )
            .map_err(Box::<RuntimeError>::from),
            Engine::Native { vm, .. } => vm::Machine::visit_continuation_root_slots(
                vm.as_mut(),
                &mut continuation.program,
                visit,
            )
            .map_err(Box::<RuntimeError>::from),
        }
    }

    /// Fork this machine over already-forked memory.
    pub fn fork(&self) -> RuntimeResult<Self> {
        let engine = match &self.engine {
            Engine::Vm(machine) => {
                let machine =
                    vm::Machine::fork(machine.as_ref()).map_err(Box::<RuntimeError>::from)?;

                Engine::Vm(Box::new(machine))
            }
            Engine::Native { vm, code } => {
                let vm = vm::Machine::fork(vm.as_ref()).map_err(Box::<RuntimeError>::from)?;

                Engine::Native {
                    vm: Box::new(vm),
                    code: code.clone(),
                }
            }
        };

        Ok(Self {
            id: self.id,
            engine,
        })
    }

    /// Capture one immutable machine image.
    pub fn image(&self) -> RuntimeResult<Image> {
        match &self.engine {
            Engine::Vm(machine) => {
                let image =
                    vm::Machine::image(machine.as_ref()).map_err(Box::<RuntimeError>::from)?;

                Ok(Image::Vm {
                    machine: self.id,
                    image: Arc::new(image),
                })
            }
            Engine::Native { vm, .. } => {
                let vm_image =
                    vm::Machine::image(vm.as_ref()).map_err(Box::<RuntimeError>::from)?;

                Ok(Image::Native {
                    machine: self.id,
                    vm: Arc::new(vm_image),
                })
            }
        }
    }

    /// Restore one immutable machine image.
    pub fn restore(&mut self, image: &Image) -> RuntimeResult<()> {
        if image.machine() != self.id {
            return Err(machine_image_mismatch(
                &machine_name(self.kind(), self.id),
                &machine_name(image_kind(image), image.machine()),
            ));
        }

        match (&mut self.engine, image) {
            (Engine::Vm(machine), Image::Vm { image, .. }) => {
                vm::Machine::restore_image(machine.as_mut(), image)
                    .map_err(Box::<RuntimeError>::from)
            }
            (Engine::Native { vm, .. }, Image::Native { vm: vm_image, .. }) => {
                vm::Machine::restore_image(vm.as_mut(), vm_image).map_err(Box::<RuntimeError>::from)
            }
            (Engine::Vm(_), Image::Native { .. }) => {
                Err(machine_image_mismatch(VM_MACHINE, NATIVE_MACHINE))
            }
            (Engine::Native { vm, .. }, Image::Vm { image, .. }) => {
                vm::Machine::restore_image(vm.as_mut(), image).map_err(Box::<RuntimeError>::from)
            }
        }
    }

    /// Rebuild one runtime machine from an image.
    pub fn from_image(
        id: MachineId,
        program: Arc<program::Program>,
        image: &Image,
        execution: &Execution,
    ) -> RuntimeResult<Self> {
        if image.machine() != id {
            return Err(machine_image_mismatch(
                &machine_name(image_kind(image), id),
                &machine_name(image_kind(image), image.machine()),
            ));
        }

        match image {
            Image::Vm { image, .. } => {
                let machine = vm::Machine::from_image(program, image.clone())
                    .map_err(Box::<RuntimeError>::from)?;

                match execution {
                    Execution::Vm { .. } => Ok(Self {
                        id,
                        engine: Engine::Vm(Box::new(machine)),
                    }),
                    Execution::Native { code, .. } => Ok(Self {
                        id,
                        engine: Engine::Native {
                            vm: Box::new(machine),
                            code: code.clone(),
                        },
                    }),
                }
            }
            Image::Native { vm: image, .. } => {
                let machine = vm::Machine::from_image(program, image.clone())
                    .map_err(Box::<RuntimeError>::from)?;

                match execution {
                    Execution::Vm { .. } => Err(machine_image_mismatch(VM_MACHINE, NATIVE_MACHINE)),
                    Execution::Native { code, .. } => Ok(Self {
                        id,
                        engine: Engine::Native {
                            vm: Box::new(machine),
                            code: code.clone(),
                        },
                    }),
                }
            }
        }
    }

    /// Return the concrete machine kind.
    fn kind(&self) -> &'static str {
        match &self.engine {
            Engine::Vm(_) => VM_MACHINE,
            Engine::Native { .. } => NATIVE_MACHINE,
        }
    }
}

impl std::fmt::Debug for Machine {
    /// Format the machine without exposing runtime internals.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Machine")
            .field("id", &self.id)
            .field("engine", &self.engine)
            .finish()
    }
}

impl std::fmt::Debug for Execution {
    /// Format the execution strategy without exposing runtime internals.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Vm { options } => formatter
                .debug_struct("Vm")
                .field("options", options)
                .finish(),
            Self::Native { options, code } => formatter
                .debug_struct("Native")
                .field("options", options)
                .field("code", code)
                .finish(),
        }
    }
}

impl std::fmt::Debug for Engine {
    /// Format the engine without exposing runtime internals.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Vm(machine) => formatter.debug_tuple("Vm").field(machine).finish(),
            Self::Native { vm, code } => formatter
                .debug_struct("Native")
                .field("vm", vm)
                .field("code", code)
                .finish(),
        }
    }
}

/// Convert one VM execution outcome into one runtime outcome.
fn outcome_from_vm(id: MachineId, outcome: vm::Outcome) -> Outcome<Continuation> {
    match outcome {
        vm::Outcome::Completed { value } => Outcome::Completed { value },
        vm::Outcome::Yielded {
            continuation,
            value,
        } => Outcome::Yielded {
            continuation: Continuation {
                machine: id,
                program: continuation,
            },
            value,
        },
        vm::Outcome::Stopped {
            continuation,
            reason,
        } => Outcome::Stopped {
            continuation: Continuation {
                machine: id,
                program: continuation,
            },
            reason,
        },
    }
}

/// Convert one native execution outcome into one runtime outcome.
fn outcome_from_native(
    id: MachineId,
    vm: &mut vm::Machine,
    context: ProgramActivation<'_>,
    outcome: native::Outcome,
) -> RuntimeResult<Outcome<Continuation>> {
    match outcome {
        native::Outcome::Completed { value } => Ok(Outcome::Completed { value }),
        native::Outcome::Yielded {
            continuation,
            value,
        } => Ok(Outcome::Yielded {
            continuation: Continuation {
                machine: id,
                program: continuation,
            },
            value,
        }),
        native::Outcome::Stopped {
            continuation,
            reason,
        } => Ok(Outcome::Stopped {
            continuation: Continuation {
                machine: id,
                program: continuation,
            },
            reason,
        }),
        native::Outcome::Deoptimized { continuation } => {
            let context = context.storage;
            let outcome = vm::Machine::continue_continuation(
                vm,
                context.local_static,
                context.shared_static,
                context.heap,
                context.shared_heap,
                context.shared_cache,
                context.shared_mark_worker,
                None,
                None,
                None,
                None,
                continuation,
            )
            .map_err(Box::<RuntimeError>::from)?;

            Ok(outcome_from_vm(id, outcome))
        }
    }
}

/// Return one machine image kind.
fn image_kind(image: &Image) -> &'static str {
    match image {
        Image::Vm { .. } => VM_MACHINE,
        Image::Native { .. } => NATIVE_MACHINE,
    }
}

/// Return one machine display name.
fn machine_name(kind: &str, id: MachineId) -> String {
    format!("{kind} {}", id.get())
}

/// Return one machine continuation mismatch.
fn machine_continuation_mismatch(machine: &str, continuation: &str) -> Box<RuntimeError> {
    machine_error(
        machine,
        MachineError::ContinuationMismatch {
            continuation: continuation.to_string(),
        },
    )
}

/// Return one machine image mismatch.
fn machine_image_mismatch(machine: &str, image: &str) -> Box<RuntimeError> {
    machine_error(
        machine,
        MachineError::ImageMismatch {
            image: image.to_string(),
        },
    )
}

/// Return one machine error.
fn machine_error(machine: &str, reason: MachineError) -> Box<RuntimeError> {
    RuntimeError::Machine {
        machine: machine.to_string(),
        reason,
    }
    .boxed()
}

/// Convert one native execution error into one runtime error.
fn native_runtime_error(error: native::Error) -> Box<RuntimeError> {
    match error {
        native::Error::YieldedWithoutContinuation { .. } => {
            machine_error(NATIVE_MACHINE, MachineError::YieldMissing)
        }
        native::Error::Trapped { .. } => machine_error(NATIVE_MACHINE, MachineError::Trap),
        native::Error::DeoptimizedWithoutContinuation { .. } => {
            machine_error(NATIVE_MACHINE, MachineError::DeoptMissing)
        }
        native::Error::StoppedWithoutContinuation { .. } => {
            machine_error(NATIVE_MACHINE, MachineError::StopMissing)
        }
        native::Error::StopPointMissing { safepoint } => machine_error(
            NATIVE_MACHINE,
            MachineError::Unsupported {
                feature: format!("stop safepoint {safepoint}"),
            },
        ),
        native::Error::Panicked { .. } => machine_error(NATIVE_MACHINE, MachineError::Panic),
        native::Error::InvalidExit(error) => machine_error(
            NATIVE_MACHINE,
            MachineError::Unsupported {
                feature: format!("exit {error}"),
            },
        ),
        native::Error::InvalidTrap(error) => machine_error(
            NATIVE_MACHINE,
            MachineError::Unsupported {
                feature: format!("trap {error}"),
            },
        ),
        native::Error::Value(error) => machine_error(
            NATIVE_MACHINE,
            MachineError::Unsupported {
                feature: format!("value {error}"),
            },
        ),
        native::Error::InvalidContinuation(error) => machine_error(
            NATIVE_MACHINE,
            MachineError::Unsupported {
                feature: format!("continuation {error}"),
            },
        ),
        native::Error::InvalidContinuationFrame { frame_state } => machine_error(
            NATIVE_MACHINE,
            MachineError::Unsupported {
                feature: format!("continuation frame {frame_state:?}"),
            },
        ),
        native::Error::EmptyContinuation => machine_error(
            NATIVE_MACHINE,
            MachineError::Unsupported {
                feature: "empty continuation".to_string(),
            },
        ),
        native::Error::ResumeEntryNotFound { frame_state } => machine_error(
            NATIVE_MACHINE,
            MachineError::Unsupported {
                feature: format!("resume entry {frame_state:?}"),
            },
        ),
        native::Error::NativeCodeMissing => machine_error(
            NATIVE_MACHINE,
            MachineError::Unsupported {
                feature: "native code".to_string(),
            },
        ),
        native::Error::ProgramStringMissing { string } => machine_error(
            NATIVE_MACHINE,
            MachineError::Unsupported {
                feature: format!("program string {string:?}"),
            },
        ),
        native::Error::NativeSymbolMissing { symbol } => machine_error(
            NATIVE_MACHINE,
            MachineError::Unsupported {
                feature: format!("native symbol {symbol}"),
            },
        ),
        native::Error::EntryNotFound { name } => machine_error(
            NATIVE_MACHINE,
            MachineError::EntryUnavailable { entry: name },
        ),
    }
}
