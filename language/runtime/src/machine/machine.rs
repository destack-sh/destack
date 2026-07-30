use std::fmt;
use std::sync::Arc;

use destack_heap::{DropReference, GcDrop};
use destack_memory::MemoryMap;
use destack_program as program;
use destack_vm as vm;
use program::{Outcome, Value};

use super::{Engine, Entry, MachineImage, Target};
use crate::diagnostic::{MachineError, RuntimeError, RuntimeResult};
use crate::worker::Activation;

/// Worker-owned runtime machine.
pub struct Machine {
    /// Shared process-local execution engine.
    engine: Engine,
    /// Worker-local continuation storage.
    continuations: program::ContinuationTable,
    /// Worker-local bytecode execution state when bytecode is available.
    vm: Option<vm::Machine>,
}

impl Machine {
    /// Create one worker-owned machine from a shared engine.
    pub(crate) fn new(engine: Engine, memory: Arc<MemoryMap>) -> RuntimeResult<Self> {
        let vm = if engine.program().bytecode().is_some() {
            let machine = vm::Machine::new(engine.program().clone(), memory, engine.limits())
                .map_err(Box::<RuntimeError>::from)?;

            Some(machine)
        } else {
            None
        };

        Ok(Self {
            engine,
            continuations: program::ContinuationTable::default(),
            vm,
        })
    }

    /// Return the immutable program.
    pub fn program(&self) -> &program::Program {
        self.engine.program()
    }

    /// Run one entrypoint.
    pub fn run<'run>(
        &mut self,
        activation: program::Activation<'run, 'run, Activation<'_>>,
        entry: &Entry,
        args: &[program::Value],
        stop_points: Option<&'run program::StopSet>,
        watch_points: Option<&'run program::WatchSet>,
        profile: Option<&'run mut program::Profile>,
    ) -> RuntimeResult<Outcome<Value>> {
        let function = self
            .program()
            .function_id_by_name(entry.name())
            .ok_or_else(|| Self::entry_unavailable(entry.name()))?;
        let outcome = self.execute(
            activation,
            function,
            None,
            args,
            stop_points,
            watch_points,
            profile,
        )?;

        Ok(outcome)
    }

    /// Run one linked function with an optional closure environment.
    pub(crate) fn run_function<'run>(
        &mut self,
        activation: program::Activation<'run, 'run, Activation<'_>>,
        function: program::FunctionId,
        environment: Option<&program::Value>,
        arguments: &[program::Value],
        stop_points: Option<&'run program::StopSet>,
        watch_points: Option<&'run program::WatchSet>,
        profile: Option<&'run mut program::Profile>,
    ) -> RuntimeResult<Outcome<Value>> {
        self.execute(
            activation,
            function,
            environment,
            arguments,
            stop_points,
            watch_points,
            profile,
        )
    }

    /// Resume one canonical coroutine continuation.
    pub fn resume<'run>(
        &mut self,
        activation: program::Activation<'run, 'run, Activation<'_>>,
        continuation: program::Continuation,
        value: &program::Value,
        stop_points: Option<&'run program::StopSet>,
        watch_points: Option<&'run program::WatchSet>,
        profile: Option<&'run mut program::Profile>,
    ) -> RuntimeResult<Outcome<Value>> {
        let Self {
            continuations, vm, ..
        } = self;
        let machine = Self::require_vm(vm, "continuation resume")?;

        machine.resume(
            continuations,
            activation,
            continuation,
            value,
            stop_points,
            watch_points,
            profile,
        )
    }

    /// Complete one canonical generator continuation.
    pub fn complete<'run>(
        &mut self,
        activation: program::Activation<'run, 'run, Activation<'_>>,
        continuation: program::Continuation,
        value: &program::Value,
        stop_points: Option<&'run program::StopSet>,
        watch_points: Option<&'run program::WatchSet>,
        profile: Option<&'run mut program::Profile>,
    ) -> RuntimeResult<Outcome<Value>> {
        let Self {
            continuations, vm, ..
        } = self;
        let machine = Self::require_vm(vm, "continuation completion")?;

        machine.complete(
            continuations,
            activation,
            continuation,
            value,
            stop_points,
            watch_points,
            profile,
        )
    }

    /// Cancel one canonical asynchronous continuation.
    pub fn cancel<'run>(
        &mut self,
        activation: program::Activation<'run, 'run, Activation<'_>>,
        continuation: program::Continuation,
        stop_points: Option<&'run program::StopSet>,
        watch_points: Option<&'run program::WatchSet>,
        profile: Option<&'run mut program::Profile>,
    ) -> RuntimeResult<Outcome<Value>> {
        let Self {
            continuations, vm, ..
        } = self;
        let machine = Self::require_vm(vm, "continuation cancellation")?;

        machine.cancel(
            continuations,
            activation,
            continuation,
            stop_points,
            watch_points,
            profile,
        )
    }

    /// Destroy one unreachable value to completion.
    pub fn drop_value(
        &mut self,
        mut activation: program::Activation<'_, '_, Activation<'_>>,
        drop: GcDrop,
    ) -> RuntimeResult<()> {
        // resolve the executable destructor
        let program = self.program();
        let Some(entry) = program.drop_entry(drop.drop) else {
            return Err(Self::entry_unavailable(format!(
                "drop {}",
                drop.drop.index()
            )));
        };
        let (function, reference, storage) = match drop.reference {
            DropReference::Local(reference) => (entry.local.get(), reference.bits(), "local"),
            DropReference::Shared(reference) => (entry.shared.get(), reference.bits(), "shared"),
        };
        let function = function.ok_or_else(|| {
            RuntimeError::Internal {
                message: format!("drop {} has no {} destructor", drop.drop.index(), storage),
            }
            .boxed()
        })?;
        let parameter = program
            .function_parameters(function)
            .and_then(|parameters| parameters.first())
            .copied()
            .ok_or_else(|| {
                RuntimeError::Internal {
                    message: format!("drop {} has no reference parameter", drop.drop.index()),
                }
                .boxed()
            })?;
        let value = program
            .value(parameter, [program::Word::from_bits(reference as u64)])
            .map_err(Box::<RuntimeError>::from)?;

        self.run_destructor(&mut activation, function, value)
    }

    /// Destroy one scheduler-owned runtime value to completion.
    pub fn destroy_value<'run>(
        &mut self,
        activation: program::Activation<'run, 'run, Activation<'_>>,
        value: program::Value,
    ) -> RuntimeResult<()> {
        let Self {
            continuations, vm, ..
        } = self;
        let machine = Self::require_vm(vm, "scheduler value destruction")?;

        machine.destroy_value(continuations, activation, value)
    }

    /// Run one value destructor to completion.
    fn run_destructor(
        &mut self,
        activation: &mut program::Activation<'_, '_, Activation<'_>>,
        function: program::FunctionId,
        value: program::Value,
    ) -> RuntimeResult<()> {
        let args = [value];

        // execute without debugger or profiler hooks
        let outcome = self.execute(
            activation.reborrow(),
            function,
            None,
            &args,
            None,
            None,
            None,
        )?;
        let result = match outcome {
            Outcome::Completed { .. } => Ok(()),
            Outcome::Cancelled => Err(RuntimeError::machine(MachineError::DropCancelled).boxed()),
            Outcome::Stopped { .. } => {
                Err(RuntimeError::machine(MachineError::DropStopped).boxed())
            }
            Outcome::Awaited { .. } | Outcome::Yielded { .. } => {
                Err(RuntimeError::machine(MachineError::DropSuspended).boxed())
            }
        };
        if result.is_err() {
            self.clear();
        }

        result
    }

    /// Continue execution retained at one debugger stop.
    pub fn continue_execution<'run>(
        &mut self,
        activation: program::Activation<'run, 'run, Activation<'_>>,
        stop_points: Option<&'run program::StopSet>,
        watch_points: Option<&'run program::WatchSet>,
        profile: Option<&'run mut program::Profile>,
        resume_skip: Option<program::ResumeSkip>,
    ) -> RuntimeResult<Outcome<Value>> {
        let Self {
            continuations, vm, ..
        } = self;
        let machine = Self::require_vm(vm, "retained execution")?;
        let outcome = machine.continue_execution(
            continuations,
            activation,
            stop_points,
            watch_points,
            profile,
            resume_skip,
        )?;

        Ok(outcome)
    }

    /// Capture retained physical execution state.
    pub fn image(&self) -> RuntimeResult<MachineImage> {
        let vm = self
            .vm
            .as_ref()
            .map(vm::Machine::capture)
            .transpose()
            .map_err(Box::<RuntimeError>::from)?;

        Ok(MachineImage::new(self.continuations.fork(), vm))
    }

    /// Restore retained physical execution state.
    pub fn restore(&mut self, image: &MachineImage) -> RuntimeResult<()> {
        match (&mut self.vm, image.vm()) {
            (Some(machine), Some(image)) => {
                machine.restore(image).map_err(Box::<RuntimeError>::from)?;
            }
            (None, None) => {}
            (Some(machine), None) => machine.clear(),
            (None, Some(_)) => {
                return Err(RuntimeError::Internal {
                    message: "machine image requires unavailable bytecode state".to_string(),
                }
                .boxed());
            }
        };

        // restore canonical continuations after physical machine state
        self.continuations = image.continuation_table().fork();

        Ok(())
    }

    /// Clear retained physical execution state.
    pub(crate) fn clear(&mut self) {
        if let Some(machine) = &mut self.vm {
            machine.clear();
        }
    }

    /// Visit mutable heap roots retained by physical execution state.
    pub fn visit_root_slots(
        &mut self,
        visit: &mut dyn FnMut(destack_heap::RootSlot<'_>) -> destack_heap::HeapResult<()>,
    ) -> RuntimeResult<()> {
        if let Some(machine) = &mut self.vm {
            machine
                .visit_root_slots(visit)
                .map_err(Box::<RuntimeError>::from)?;
        }

        // visit canonical continuations shared by all execution forms
        let program = self.engine.program();

        self.continuations
            .visit_root_slots(program, visit)
            .map_err(Box::<RuntimeError>::from)?;

        Ok(())
    }

    /// Fork this machine over already-forked memory.
    pub fn fork(&self, memory: Arc<MemoryMap>) -> RuntimeResult<Self> {
        let vm = self.vm.as_ref().map(|machine| machine.fork(memory));

        Ok(Self {
            engine: self.engine.clone(),
            continuations: self.continuations.fork(),
            vm,
        })
    }

    /// Execute one linked function through its current engine entry.
    #[allow(clippy::too_many_arguments)]
    fn execute<'run>(
        &mut self,
        mut activation: program::Activation<'run, 'run, Activation<'_>>,
        function: program::FunctionId,
        environment: Option<&program::Value>,
        arguments: &[program::Value],
        stop_points: Option<&'run program::StopSet>,
        watch_points: Option<&'run program::WatchSet>,
        profile: Option<&'run mut program::Profile>,
    ) -> RuntimeResult<Outcome<Value>> {
        let target = self
            .engine
            .target(function)
            .ok_or_else(|| Self::entry_unavailable(format!("function {}", function.index())))?;

        match target {
            Target::Bytecode => {
                let machine = self.vm.as_mut().ok_or_else(|| {
                    RuntimeError::Internal {
                        message: "bytecode entry has no worker machine".to_string(),
                    }
                    .boxed()
                })?;

                machine.run(
                    &mut self.continuations,
                    activation,
                    function,
                    environment,
                    arguments,
                    stop_points,
                    watch_points,
                    profile,
                )
            }
            Target::Native => {
                Self::reject_native_hooks(stop_points, watch_points, profile.as_deref())?;
                let code = self.engine.loaded_native().ok_or_else(|| {
                    RuntimeError::Internal {
                        message: "native entry has no loaded code".to_string(),
                    }
                    .boxed()
                })?;

                code.run(
                    self.engine.program(),
                    &mut activation,
                    program::EntryPoint::from(function),
                    environment,
                    arguments,
                )
            }
        }
    }

    /// Return the worker-local bytecode machine required by one operation.
    fn require_vm<'machine>(
        vm: &'machine mut Option<vm::Machine>,
        feature: &str,
    ) -> RuntimeResult<&'machine mut vm::Machine> {
        vm.as_mut().ok_or_else(|| {
            RuntimeError::machine(MachineError::Unsupported {
                feature: feature.to_string(),
            })
            .boxed()
        })
    }

    /// Reject runtime hooks that native code does not implement.
    fn reject_native_hooks(
        stop_points: Option<&program::StopSet>,
        watch_points: Option<&program::WatchSet>,
        profile: Option<&program::Profile>,
    ) -> RuntimeResult<()> {
        let feature = if profile.is_some() {
            Some("profile recording")
        } else if stop_points.is_some_and(|points| !points.is_empty()) {
            Some("stop points")
        } else if watch_points.is_some_and(|points| !points.is_empty()) {
            Some("watch points")
        } else {
            None
        };
        let Some(feature) = feature else {
            return Ok(());
        };

        Err(RuntimeError::machine(MachineError::Unsupported {
            feature: feature.to_string(),
        })
        .boxed())
    }

    /// Return one unavailable entry error.
    fn entry_unavailable(entry: impl Into<String>) -> Box<RuntimeError> {
        RuntimeError::entry_unavailable(entry).boxed()
    }
}

impl fmt::Debug for Machine {
    /// Format the machine without exposing runtime internals.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Machine")
            .field("engine", &self.engine)
            .field("continuations", &self.continuations)
            .field("vm", &self.vm)
            .finish()
    }
}
