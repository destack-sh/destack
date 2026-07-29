use std::fmt;
use std::sync::Arc;

use destack_heap::{DropReference, GcDrop};
use destack_memory::MemoryMap;
use destack_program as program;
use destack_vm as vm;
use program::{Outcome, Value};

use super::{Entry, MachineImage, MachineStateImage, native};
use crate::diagnostic::{MachineError, MachineKind, RuntimeError, RuntimeResult};
use crate::worker::Activation;

/// Worker-owned runtime machine.
pub struct Machine {
    /// Worker-local continuation storage.
    continuations: program::ContinuationTable,
    /// Concrete execution state.
    state: MachineState,
}

/// Concrete worker machine state.
pub(crate) enum MachineState {
    /// Bytecode machine.
    Vm(Box<vm::Machine>),
    /// Native machine.
    Native(native::Machine),
}

impl MachineState {
    /// Return the immutable program.
    fn program(&self) -> &program::Program {
        match self {
            Self::Vm(machine) => machine.program(),
            Self::Native(machine) => machine.program(),
        }
    }

    /// Run one linked function.
    fn run<'run>(
        &mut self,
        continuations: &mut program::ContinuationTable,
        mut activation: program::Activation<'run, 'run, Activation<'_>>,
        function: program::FunctionId,
        environment: Option<&program::Value>,
        arguments: &[program::Value],
        stop_points: Option<&'run program::StopSet>,
        watch_points: Option<&'run program::WatchSet>,
        profile: Option<&'run mut program::Profile>,
    ) -> RuntimeResult<Outcome<Value>> {
        match self {
            Self::Vm(machine) => machine.run(
                continuations,
                activation,
                function,
                environment,
                arguments,
                stop_points,
                watch_points,
                profile,
            ),
            Self::Native(machine) => {
                Self::reject_native_hooks(stop_points, watch_points, profile.as_deref())?;

                // enter native code after hook support is established
                machine.run(&mut activation, function, environment, arguments)
            }
        }
    }

    /// Resume one canonical coroutine continuation.
    fn resume<'run>(
        &mut self,
        continuations: &mut program::ContinuationTable,
        activation: program::Activation<'run, 'run, Activation<'_>>,
        continuation: program::Continuation,
        value: &program::Value,
        stop_points: Option<&'run program::StopSet>,
        watch_points: Option<&'run program::WatchSet>,
        profile: Option<&'run mut program::Profile>,
    ) -> RuntimeResult<Outcome<Value>> {
        let machine = match self {
            Self::Vm(machine) => machine.as_mut(),
            Self::Native(_) => {
                return Err(Self::unsupported_native("continuation resume"));
            }
        };
        let outcome = machine.resume(
            continuations,
            activation,
            continuation,
            value,
            stop_points,
            watch_points,
            profile,
        )?;

        Ok(outcome)
    }

    /// Complete one canonical generator continuation.
    fn complete<'run>(
        &mut self,
        continuations: &mut program::ContinuationTable,
        activation: program::Activation<'run, 'run, Activation<'_>>,
        continuation: program::Continuation,
        value: &program::Value,
        stop_points: Option<&'run program::StopSet>,
        watch_points: Option<&'run program::WatchSet>,
        profile: Option<&'run mut program::Profile>,
    ) -> RuntimeResult<Outcome<Value>> {
        let machine = match self {
            Self::Vm(machine) => machine.as_mut(),
            Self::Native(_) => {
                return Err(Self::unsupported_native("continuation completion"));
            }
        };
        let outcome = machine.complete(
            continuations,
            activation,
            continuation,
            value,
            stop_points,
            watch_points,
            profile,
        )?;

        Ok(outcome)
    }

    /// Cancel one canonical asynchronous continuation.
    fn cancel<'run>(
        &mut self,
        continuations: &mut program::ContinuationTable,
        activation: program::Activation<'run, 'run, Activation<'_>>,
        continuation: program::Continuation,
        stop_points: Option<&'run program::StopSet>,
        watch_points: Option<&'run program::WatchSet>,
        profile: Option<&'run mut program::Profile>,
    ) -> RuntimeResult<Outcome<Value>> {
        let machine = match self {
            Self::Vm(machine) => machine.as_mut(),
            Self::Native(_) => {
                return Err(Self::unsupported_native("continuation cancellation"));
            }
        };
        let outcome = machine.cancel(
            continuations,
            activation,
            continuation,
            stop_points,
            watch_points,
            profile,
        )?;

        Ok(outcome)
    }

    /// Continue the execution retained by this machine.
    fn continue_execution<'run>(
        &mut self,
        continuations: &mut program::ContinuationTable,
        activation: program::Activation<'run, 'run, Activation<'_>>,
        stop_points: Option<&'run program::StopSet>,
        watch_points: Option<&'run program::WatchSet>,
        profile: Option<&'run mut program::Profile>,
        resume_skip: Option<program::ResumeSkip>,
    ) -> RuntimeResult<Outcome<Value>> {
        let machine = match self {
            Self::Vm(machine) => machine.as_mut(),
            Self::Native(_) => {
                return Err(Self::unsupported_native("retained execution"));
            }
        };
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

    /// Reject runtime hooks that native code does not implement.
    fn reject_native_hooks(
        stop_points: Option<&program::StopSet>,
        watch_points: Option<&program::WatchSet>,
        profile: Option<&program::Profile>,
    ) -> RuntimeResult<()> {
        if profile.is_some() {
            return Err(RuntimeError::machine(
                MachineKind::Native,
                MachineError::Unsupported {
                    feature: "profile recording".to_string(),
                },
            )
            .boxed());
        }
        if stop_points.is_some_and(|points| !points.is_empty()) {
            return Err(RuntimeError::machine(
                MachineKind::Native,
                MachineError::Unsupported {
                    feature: "stop points".to_string(),
                },
            )
            .boxed());
        }
        if watch_points.is_some_and(|points| !points.is_empty()) {
            return Err(RuntimeError::machine(
                MachineKind::Native,
                MachineError::Unsupported {
                    feature: "watch points".to_string(),
                },
            )
            .boxed());
        }

        Ok(())
    }

    /// Return one unsupported native execution error.
    fn unsupported_native(feature: &str) -> Box<RuntimeError> {
        RuntimeError::machine(
            MachineKind::Native,
            MachineError::Unsupported {
                feature: feature.to_string(),
            },
        )
        .boxed()
    }
}

impl Machine {
    /// Create one worker-owned machine from concrete state.
    pub(crate) fn from_state(state: MachineState) -> Self {
        Self {
            continuations: program::ContinuationTable::default(),
            state,
        }
    }

    /// Return the immutable program.
    pub fn program(&self) -> &program::Program {
        self.state.program()
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
            .ok_or_else(|| {
                RuntimeError::machine(
                    self.kind(),
                    MachineError::EntryUnavailable {
                        entry: entry.name().to_string(),
                    },
                )
                .boxed()
            })?;
        let outcome = self.state.run(
            &mut self.continuations,
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
        self.state.run(
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
        self.state.resume(
            &mut self.continuations,
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
        self.state.complete(
            &mut self.continuations,
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
        self.state.cancel(
            &mut self.continuations,
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
            return Err(RuntimeError::machine(
                self.kind(),
                MachineError::EntryUnavailable {
                    entry: format!("drop {}", drop.drop.index()),
                },
            )
            .boxed());
        };
        let parameter = program
            .function_parameters(entry.function)
            .and_then(|parameters| parameters.first())
            .copied()
            .ok_or_else(|| {
                RuntimeError::Internal {
                    message: format!("drop {} has no reference parameter", drop.drop.index()),
                }
                .boxed()
            })?;
        let reference = match drop.reference {
            DropReference::Local(reference) => reference.bits(),
            DropReference::Shared(reference) => reference.bits(),
        };
        let value = program
            .value(parameter, [program::Word::from_bits(reference as u64)])
            .map_err(Box::<RuntimeError>::from)?;

        self.run_destructor(&mut activation, entry.function, value)
    }

    /// Destroy one scheduler-owned runtime value to completion.
    pub fn destroy_value<'run>(
        &mut self,
        activation: program::Activation<'run, 'run, Activation<'_>>,
        value: program::Value,
    ) -> RuntimeResult<()> {
        match &mut self.state {
            MachineState::Vm(machine) => {
                machine.destroy_value(&mut self.continuations, activation, value)
            }
            MachineState::Native(_) => Err(MachineState::unsupported_native(
                "scheduler value destruction",
            )),
        }
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
        let outcome = self.state.run(
            &mut self.continuations,
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
            Outcome::Cancelled => {
                Err(RuntimeError::machine(self.kind(), MachineError::DropCancelled).boxed())
            }
            Outcome::Stopped { .. } => {
                Err(RuntimeError::machine(self.kind(), MachineError::DropStopped).boxed())
            }
            Outcome::Awaited { .. } | Outcome::Yielded { .. } => {
                Err(RuntimeError::machine(self.kind(), MachineError::DropSuspended).boxed())
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
        let outcome = self.state.continue_execution(
            &mut self.continuations,
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
        let state = match &self.state {
            MachineState::Vm(machine) => {
                MachineStateImage::Vm(machine.capture().map_err(Box::<RuntimeError>::from)?)
            }
            MachineState::Native(machine) => MachineStateImage::Native(machine.image()),
        };

        Ok(MachineImage::new(self.continuations.fork(), state))
    }

    /// Restore retained physical execution state.
    pub fn restore(&mut self, image: &MachineImage) -> RuntimeResult<()> {
        match (&mut self.state, image.state()) {
            (MachineState::Vm(machine), MachineStateImage::Vm(image)) => {
                machine.restore(image).map_err(Box::<RuntimeError>::from)?;
            }
            (MachineState::Native(machine), MachineStateImage::Native(image)) => {
                machine.restore(image);
            }
            _ => {
                return Err(RuntimeError::Internal {
                    message: "machine image kind does not match machine state".to_string(),
                }
                .boxed());
            }
        };
        self.continuations = image.continuation_table().fork();

        Ok(())
    }

    /// Clear retained physical execution state.
    pub(crate) fn clear(&mut self) {
        match &mut self.state {
            MachineState::Vm(machine) => machine.clear(),
            MachineState::Native(machine) => machine.clear(),
        }
    }

    /// Visit mutable heap roots retained by physical execution state.
    pub fn visit_root_slots(
        &mut self,
        visit: &mut dyn FnMut(destack_heap::RootSlot<'_>) -> destack_heap::HeapResult<()>,
    ) -> RuntimeResult<()> {
        let program = match &mut self.state {
            MachineState::Vm(machine) => {
                machine
                    .visit_root_slots(visit)
                    .map_err(Box::<RuntimeError>::from)?;

                machine.program()
            }
            MachineState::Native(machine) => machine.program(),
        };
        self.continuations
            .visit_root_slots(program, visit)
            .map_err(Box::<RuntimeError>::from)?;

        Ok(())
    }

    /// Fork this machine over already-forked memory.
    pub fn fork(&self, memory: Arc<MemoryMap>) -> RuntimeResult<Self> {
        let state = match &self.state {
            MachineState::Vm(machine) => {
                let machine = machine.fork(memory.clone());

                MachineState::Vm(Box::new(machine))
            }
            MachineState::Native(machine) => MachineState::Native(machine.fork()),
        };

        Ok(Self {
            continuations: self.continuations.fork(),
            state,
        })
    }

    /// Return the concrete machine kind.
    pub(crate) const fn kind(&self) -> MachineKind {
        match &self.state {
            MachineState::Vm(_) => MachineKind::Vm,
            MachineState::Native(_) => MachineKind::Native,
        }
    }
}

impl fmt::Debug for Machine {
    /// Format the machine without exposing runtime internals.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Machine")
            .field("continuations", &self.continuations)
            .field("state", &self.state)
            .finish()
    }
}

impl fmt::Debug for MachineState {
    /// Format the machine state without exposing runtime internals.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Vm(machine) => formatter.debug_tuple("Vm").field(machine).finish(),
            Self::Native(machine) => formatter.debug_tuple("Native").field(machine).finish(),
        }
    }
}
