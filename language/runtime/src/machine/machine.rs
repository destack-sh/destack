use std::fmt;
use std::sync::Arc;

use destack_heap::{DropReference, GcDrop, HeapResult, RootSlot};
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
    /// World memory containing retained execution state.
    memory: Arc<MemoryMap>,
    /// Worker-local continuation storage.
    continuations: program::ContinuationTable,
    /// Engine-neutral execution retained outside an active engine.
    activation: Option<program::ActivationImage>,
    /// Worker-local bytecode execution state when bytecode is available.
    vm: Option<vm::Machine>,
}

impl Machine {
    /// Create one worker-owned machine from a shared engine.
    pub(crate) fn new(engine: Engine, memory: Arc<MemoryMap>) -> RuntimeResult<Self> {
        let vm = if engine.program().bytecode().is_some() {
            let machine =
                vm::Machine::new(engine.program().clone(), memory.clone(), engine.limits())
                    .map_err(Box::<RuntimeError>::from)?;

            Some(machine)
        } else {
            None
        };

        Ok(Self {
            engine,
            memory,
            continuations: program::ContinuationTable::default(),
            activation: None,
            vm,
        })
    }

    /// Return the immutable program.
    pub fn program(&self) -> &program::Program {
        self.engine.program()
    }

    /// Return the world memory containing retained execution state.
    pub(crate) fn memory(&self) -> Arc<MemoryMap> {
        self.memory.clone()
    }

    /// Return the innermost point of the captured activation when present.
    pub fn activation_point(&self) -> Option<program::ProgramPoint> {
        self.activation
            .as_ref()
            .and_then(|activation| activation.frames().last())
            .map(|frame| frame.point())
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
        self.execute(
            activation,
            function,
            None,
            args,
            stop_points,
            watch_points,
            profile,
        )
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
        if self.vm.is_none() {
            continuation
                .release(&self.memory)
                .map_err(Box::<RuntimeError>::from)?;

            return Err(Self::unsupported("continuation resume"));
        }

        let Self {
            activation: captured,
            continuations,
            vm,
            ..
        } = self;
        let machine = Self::require_vm(vm, "continuation resume")?;

        let outcome = machine.resume(
            continuations,
            activation,
            continuation,
            value,
            stop_points,
            watch_points,
            profile,
        )?;

        Self::capture_vm_outcome(captured, machine, outcome)
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
        if self.vm.is_none() {
            continuation
                .release(&self.memory)
                .map_err(Box::<RuntimeError>::from)?;

            return Err(Self::unsupported("continuation completion"));
        }

        let Self {
            activation: captured,
            continuations,
            vm,
            ..
        } = self;
        let machine = Self::require_vm(vm, "continuation completion")?;

        let outcome = machine.complete(
            continuations,
            activation,
            continuation,
            value,
            stop_points,
            watch_points,
            profile,
        )?;

        Self::capture_vm_outcome(captured, machine, outcome)
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
        if self.vm.is_none() {
            continuation
                .release(&self.memory)
                .map_err(Box::<RuntimeError>::from)?;

            return Err(Self::unsupported("continuation cancellation"));
        }

        let Self {
            activation: captured,
            continuations,
            vm,
            ..
        } = self;
        let machine = Self::require_vm(vm, "continuation cancellation")?;

        let outcome = machine.cancel(
            continuations,
            activation,
            continuation,
            stop_points,
            watch_points,
            profile,
        )?;

        Self::capture_vm_outcome(captured, machine, outcome)
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

        // discard invalid retained destructor state before returning its failure
        if result.is_err() {
            self.clear()?;
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
            activation: captured,
            continuations,
            vm,
            ..
        } = self;
        let machine = Self::require_vm(vm, "retained execution")?;
        let image = captured.take().ok_or_else(|| {
            RuntimeError::Internal {
                message: "retained execution has no activation image".to_string(),
            }
            .boxed()
        })?;
        machine.restore(image).map_err(Box::<RuntimeError>::from)?;
        let outcome = machine.continue_execution(
            continuations,
            activation,
            stop_points,
            watch_points,
            profile,
            resume_skip,
        )?;

        Self::capture_vm_outcome(captured, machine, outcome)
    }

    /// Capture retained engine-neutral execution state.
    pub fn image(&self) -> MachineImage {
        MachineImage::new(
            self.continuations.inherit(),
            self.activation
                .as_ref()
                .map(program::ActivationImage::inherit),
        )
    }

    /// Restore retained engine-neutral execution state.
    pub(crate) fn restore(&mut self, image: &MachineImage) -> RuntimeResult<()> {
        self.clear()?;
        self.activation = image.activation().map(program::ActivationImage::inherit);
        self.continuations = image.continuation_table().inherit();

        Ok(())
    }

    /// Clear retained execution state.
    pub(crate) fn clear(&mut self) -> RuntimeResult<()> {
        let activation = self.activation.take();
        let continuations = std::mem::take(&mut self.continuations);

        // release every owned execution range even when one release fails
        let activation = activation
            .map(|activation| activation.release(&self.memory))
            .transpose()
            .map_err(Box::<RuntimeError>::from);
        let continuations = continuations
            .release(&self.memory)
            .map_err(Box::<RuntimeError>::from);
        let vm = self
            .vm
            .as_mut()
            .map(vm::Machine::clear)
            .transpose()
            .map_err(Box::<RuntimeError>::from);

        activation?;
        continuations?;
        vm?;

        Ok(())
    }

    /// Release one continuation that cannot reach a language-level owner.
    pub(crate) fn release_continuation(
        &self,
        continuation: program::Continuation,
    ) -> RuntimeResult<()> {
        continuation
            .release(&self.memory)
            .map_err(Box::<RuntimeError>::from)
    }

    /// Visit mutable heap roots retained by canonical execution state.
    pub fn visit_root_slots(
        &mut self,
        visit: &mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>,
    ) -> RuntimeResult<()> {
        if let Some(activation) = &mut self.activation {
            self.engine
                .program()
                .visit_activation_root_slots(&self.memory, activation, visit)
                .map_err(Box::<RuntimeError>::from)?;
        }

        // visit canonical continuations shared by all execution forms
        let program = self.engine.program();

        self.continuations
            .visit_root_slots(program, &self.memory, visit)
            .map_err(Box::<RuntimeError>::from)?;

        Ok(())
    }

    /// Fork this machine over already-forked memory.
    pub fn fork(&self, memory: Arc<MemoryMap>) -> Self {
        let vm = self.vm.as_ref().map(|machine| machine.fork(memory.clone()));

        Self {
            engine: self.engine.clone(),
            memory,
            continuations: self.continuations.inherit(),
            activation: self
                .activation
                .as_ref()
                .map(program::ActivationImage::inherit),
            vm,
        }
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
        mut profile: Option<&'run mut program::Profile>,
    ) -> RuntimeResult<Outcome<Value>> {
        let mut target = self
            .engine
            .target(function)
            .ok_or_else(|| Self::entry_unavailable(format!("function {}", function.index())))?;
        let is_observed = stop_points.is_some_and(|points| !points.is_empty())
            || watch_points.is_some_and(|points| !points.is_empty());

        // execute observed native entries through their canonical bytecode form
        if target == Target::Native && is_observed {
            if !self.engine.has_bytecode(function) {
                return Err(Self::unsupported("native observation without bytecode"));
            }
            target = Target::Bytecode;
        }

        match target {
            Target::Bytecode => {
                let Self {
                    activation: captured,
                    continuations,
                    vm,
                    ..
                } = self;
                let machine = vm.as_mut().ok_or_else(|| {
                    RuntimeError::Internal {
                        message: "bytecode entry has no worker machine".to_string(),
                    }
                    .boxed()
                })?;

                let outcome = machine.run(
                    continuations,
                    activation,
                    function,
                    environment,
                    arguments,
                    stop_points,
                    watch_points,
                    profile,
                )?;

                Self::capture_vm_outcome(captured, machine, outcome)
            }
            Target::Native => {
                let engine = self.engine.clone();
                let code = engine.loaded_native().ok_or_else(|| {
                    RuntimeError::Internal {
                        message: "native entry has no loaded code".to_string(),
                    }
                    .boxed()
                })?;

                let outcome = code.run(
                    engine.program(),
                    &mut activation,
                    engine.functions(),
                    engine.virtuals(),
                    engine.dynamics(),
                    &self.memory,
                    program::EntryPoint::from(function),
                    environment,
                    arguments,
                    profile.as_deref_mut(),
                    &mut self.activation,
                )?;

                // continue canonical deoptimized state without exposing a host stop
                match outcome {
                    super::native::Outcome::Program(outcome) => Ok(outcome),
                    super::native::Outcome::Deoptimized => {
                        let image = self.activation.take().ok_or_else(|| {
                            RuntimeError::Internal {
                                message: "native deoptimization retained no activation".to_string(),
                            }
                            .boxed()
                        })?;
                        let Self {
                            activation: captured,
                            continuations,
                            vm,
                            ..
                        } = self;
                        let machine = Self::require_vm(vm, "native deoptimization")?;
                        machine.restore(image).map_err(Box::<RuntimeError>::from)?;
                        let outcome = machine.continue_execution(
                            continuations,
                            activation,
                            stop_points,
                            watch_points,
                            profile,
                            None,
                        )?;

                        Self::capture_vm_outcome(captured, machine, outcome)
                    }
                }
            }
        }
    }

    /// Return the worker-local bytecode machine required by one operation.
    fn require_vm<'machine>(
        vm: &'machine mut Option<vm::Machine>,
        feature: &str,
    ) -> RuntimeResult<&'machine mut vm::Machine> {
        vm.as_mut().ok_or_else(|| Self::unsupported(feature))
    }

    /// Capture canonical stopped execution from a VM into runtime ownership.
    fn capture_vm_outcome(
        captured: &mut Option<program::ActivationImage>,
        machine: &mut vm::Machine,
        outcome: Outcome<Value>,
    ) -> RuntimeResult<Outcome<Value>> {
        if matches!(outcome, Outcome::Stopped { .. }) {
            let image = machine.take_activation().ok_or_else(|| {
                RuntimeError::Internal {
                    message: "retained VM outcome has no activation image".to_string(),
                }
                .boxed()
            })?;
            *captured = Some(image);
        }

        Ok(outcome)
    }

    /// Return one unavailable entry error.
    fn entry_unavailable(entry: impl Into<String>) -> Box<RuntimeError> {
        RuntimeError::entry_unavailable(entry).boxed()
    }

    /// Return one unsupported machine feature error.
    fn unsupported(feature: impl Into<String>) -> Box<RuntimeError> {
        RuntimeError::machine(MachineError::Unsupported {
            feature: feature.into(),
        })
        .boxed()
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

impl Drop for Machine {
    fn drop(&mut self) {
        // abort because dropping owned execution ranges must not corrupt world memory
        if self.clear().is_err() {
            std::process::abort();
        }
    }
}
