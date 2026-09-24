use std::fmt;
use std::sync::Arc;

use destack_heap::{DropReference, GcDrop, HeapResult, RootSlot};
use destack_memory::MemoryMap;
use destack_program as program;
use destack_program::Runtime;
use destack_vm as vm;
use program::{Outcome, Value};

use super::{Engine, Entry, MachineImage, Target};
use crate::diagnostic::{MachineError, RuntimeError, RuntimeResult};
use crate::worker::Activation;

/// The usable byte length of each fiber's native stack, backed by the OS only as it is touched.
const NATIVE_STACK_BYTES: usize = 8 * 1024 * 1024;

/// Worker-owned runtime machine executing fibers over world memory.
pub struct Machine {
    /// Shared process-local execution engine.
    engine: Engine,
    /// World memory containing fiber stacks.
    memory: Arc<MemoryMap>,
    /// Fiber retained at one handshake or debugger stop.
    stopped: Option<vm::Fiber>,
    /// Idle fiber reused by synchronous destructor execution.
    scratch: Option<vm::Fiber>,
    /// Worker-local bytecode execution state when bytecode is available.
    vm: vm::Machine,
}

impl Machine {
    /// Create one worker-owned machine from a shared engine.
    pub(crate) fn new(engine: Engine, memory: Arc<MemoryMap>) -> RuntimeResult<Self> {
        let vm = vm::Machine::new(engine.program().clone(), engine.limits())
            .map_err(Box::<RuntimeError>::from)?;

        Ok(Self {
            engine,
            memory,
            stopped: None,
            scratch: None,
            vm,
        })
    }

    /// Return the immutable program.
    pub fn program(&self) -> &program::Program {
        self.engine.program()
    }

    /// Reserve one idle fiber for a fresh invocation.
    pub(crate) fn reserve_fiber(&self) -> RuntimeResult<vm::Fiber> {
        self.vm
            .reserve_fiber(self.memory.clone())
            .map_err(Box::<RuntimeError>::from)
    }

    /// Return the innermost point of the stopped fiber when present.
    pub fn activation_point(&self) -> Option<program::ProgramPoint> {
        let fiber = self.stopped.as_ref()?;

        self.vm.fiber_point(fiber)
    }

    /// Retain one stopped fiber across a handshake or debugger stop.
    pub(crate) fn retain_stopped(&mut self, fiber: vm::Fiber) {
        self.stopped = Some(fiber);
    }

    /// Take the retained stopped fiber for continuation.
    pub(crate) fn take_stopped(&mut self) -> Option<vm::Fiber> {
        self.stopped.take()
    }

    /// Run one entrypoint on one idle fiber.
    pub fn run<'run>(
        &mut self,
        fiber: &mut vm::Fiber,
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
            fiber,
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
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn run_function<'run>(
        &mut self,
        fiber: &mut vm::Fiber,
        activation: program::Activation<'run, 'run, Activation<'_>>,
        function: program::FunctionId,
        environment: Option<&program::Value>,
        arguments: &[program::Value],
        stop_points: Option<&'run program::StopSet>,
        watch_points: Option<&'run program::WatchSet>,
        profile: Option<&'run mut program::Profile>,
    ) -> RuntimeResult<Outcome<Value>> {
        self.execute(
            fiber,
            activation,
            function,
            environment,
            arguments,
            stop_points,
            watch_points,
            profile,
        )
    }

    /// Resume one parked fiber with its delivered wake value.
    pub fn resume<'run>(
        &mut self,
        fiber: &mut vm::Fiber,
        activation: program::Activation<'run, 'run, Activation<'_>>,
        value: &program::Value,
        stop_points: Option<&'run program::StopSet>,
        watch_points: Option<&'run program::WatchSet>,
        profile: Option<&'run mut program::Profile>,
    ) -> RuntimeResult<Outcome<Value>> {
        let machine = &mut self.vm;

        machine.resume(fiber, activation, value, stop_points, watch_points, profile)
    }

    /// Continue one fiber retained at a debugger stop.
    pub fn continue_execution<'run>(
        &mut self,
        fiber: &mut vm::Fiber,
        activation: program::Activation<'run, 'run, Activation<'_>>,
        stop_points: Option<&'run program::StopSet>,
        watch_points: Option<&'run program::WatchSet>,
        profile: Option<&'run mut program::Profile>,
        resume_skip: Option<program::ResumeSkip>,
    ) -> RuntimeResult<Outcome<Value>> {
        let machine = &mut self.vm;

        machine.continue_execution(
            fiber,
            activation,
            stop_points,
            watch_points,
            profile,
            resume_skip,
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
        let mut fiber = self.take_scratch()?;
        let machine = &mut self.vm;
        let result = machine.destroy_value(&mut fiber, activation, value);
        self.stash_scratch(fiber);

        result
    }

    /// Run one value destructor to completion.
    fn run_destructor(
        &mut self,
        activation: &mut program::Activation<'_, '_, Activation<'_>>,
        function: program::FunctionId,
        value: program::Value,
    ) -> RuntimeResult<()> {
        let args = [value];
        let mut fiber = self.take_scratch()?;

        // execute without debugger or profiler hooks
        let outcome = self.execute(
            &mut fiber,
            activation.reborrow(),
            function,
            None,
            &args,
            None,
            None,
            None,
        );
        self.stash_scratch(fiber);

        match outcome? {
            Outcome::Completed { .. } => Ok(()),
            Outcome::Cancelled => Err(RuntimeError::machine(MachineError::DropCancelled).boxed()),
            Outcome::Stopped { .. } => {
                Err(RuntimeError::machine(MachineError::DropStopped).boxed())
            }
            Outcome::Parked => Err(RuntimeError::machine(MachineError::DropSuspended).boxed()),
        }
    }

    /// Take the reusable synchronous destructor fiber.
    fn take_scratch(&mut self) -> RuntimeResult<vm::Fiber> {
        match self.scratch.take() {
            Some(fiber) => Ok(fiber),
            None => self.reserve_fiber(),
        }
    }

    /// Return the destructor fiber, discarding any retained execution.
    fn stash_scratch(&mut self, mut fiber: vm::Fiber) {
        fiber.clear();
        self.scratch = Some(fiber);
    }

    /// Capture retained engine-neutral execution state.
    pub fn image(&self) -> MachineImage {
        MachineImage::new(self.stopped.as_ref().map(vm::Fiber::image))
    }

    /// Restore retained engine-neutral execution state.
    pub(crate) fn restore(&mut self, image: &MachineImage) -> RuntimeResult<()> {
        self.stopped = image
            .stopped()
            .map(|image| vm::Fiber::from_image(self.memory.clone(), image))
            .transpose()
            .map_err(Box::<RuntimeError>::from)?;

        Ok(())
    }

    /// Discard retained execution state.
    pub(crate) fn clear(&mut self) {
        self.stopped = None;
    }

    /// Visit mutable heap roots retained by the stopped fiber.
    pub fn visit_root_slots(
        &mut self,
        visit: &mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>,
    ) -> RuntimeResult<()> {
        let Self { stopped, vm, .. } = self;
        let Some(fiber) = stopped.as_mut() else {
            return Ok(());
        };

        vm.visit_root_slots(fiber, visit)
            .map_err(Box::<RuntimeError>::from)
    }

    /// Visit mutable heap roots retained by one scheduler-owned fiber.
    pub(crate) fn visit_fiber_root_slots(
        &self,
        fiber: &mut vm::Fiber,
        visit: &mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>,
    ) -> RuntimeResult<()> {
        self.vm
            .visit_root_slots(fiber, visit)
            .map_err(Box::<RuntimeError>::from)
    }

    /// Fork this machine over already-forked memory.
    pub fn fork(&self, memory: Arc<MemoryMap>) -> Self {
        Self {
            engine: self.engine.clone(),
            stopped: self
                .stopped
                .as_ref()
                .map(|fiber| fiber.fork(memory.clone())),
            scratch: None,
            vm: self.vm.fork(),
            memory,
        }
    }

    /// Execute one linked function through its current engine entry.
    #[allow(clippy::too_many_arguments)]
    fn execute<'run>(
        &mut self,
        fiber: &mut vm::Fiber,
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
            || watch_points.is_some_and(|points| !points.is_empty())
            || !activation.runtime.events().is_empty();

        // execute observed native entries through their canonical bytecode form
        if target == Target::Native && is_observed {
            target = Target::Bytecode;
        }

        match target {
            Target::Bytecode => {
                let machine = &mut self.vm;

                machine.run(
                    fiber,
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
                let engine = self.engine.clone();
                let code = engine.loaded_native().ok_or_else(|| {
                    RuntimeError::Internal {
                        message: "native entry has no loaded code".to_string(),
                    }
                    .boxed()
                })?;

                let native_stack = fiber
                    .native_stack(NATIVE_STACK_BYTES)
                    .map_err(Box::<RuntimeError>::from)?;
                let mut captured = None;
                let outcome = code.run(
                    engine.program(),
                    &mut activation,
                    engine.functions(),
                    engine.virtuals(),
                    engine.dynamics(),
                    &self.memory,
                    native_stack,
                    program::EntryPoint::from(function),
                    environment,
                    arguments,
                    profile.as_deref_mut(),
                    &mut captured,
                )?;

                match outcome {
                    super::native::Outcome::Program(outcome) => {
                        // land retained native frames on the executing fiber
                        if let Some(image) = captured {
                            self.adopt_capture(fiber, image)?;
                        }

                        Ok(outcome)
                    }
                    // continue deoptimized execution through canonical bytecode
                    super::native::Outcome::Deoptimized => {
                        let image = captured.ok_or_else(|| {
                            RuntimeError::Internal {
                                message: "native deoptimization retained no activation".to_string(),
                            }
                            .boxed()
                        })?;
                        self.adopt_capture(fiber, image)?;
                        let machine = &mut self.vm;

                        machine.continue_execution(
                            fiber,
                            activation,
                            stop_points,
                            watch_points,
                            profile,
                            None,
                        )
                    }
                }
            }
        }
    }

    /// Materialize one captured native activation onto the executing fiber.
    fn adopt_capture(
        &mut self,
        fiber: &mut vm::Fiber,
        image: program::ActivationImage,
    ) -> RuntimeResult<()> {
        let result = self
            .vm
            .materialize(fiber, &image)
            .map_err(Box::<RuntimeError>::from);
        let release = image
            .release(&self.memory)
            .map_err(Box::<RuntimeError>::from);

        result?;
        release
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
            .field("vm", &self.vm)
            .finish()
    }
}
