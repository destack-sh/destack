use serde::{Deserialize, Serialize};
use tspp_heap as heap;
use tspp_program as program;
use tspp_serde::Reflect;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::worker::RunnableScope;

/// One identified event-loop execution.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Runnable {
    /// Runnable identifier used for ordering and logging.
    pub id: RunnableId,
    /// Function invocation to execute.
    pub invocation: Invocation,
}

/// One consumable program invocation.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Invocation {
    /// Invoke one function.
    Function(Call),
    /// Wake one parked fiber with a delivered value.
    Wake {
        /// Fiber identity to resume.
        fiber_id: program::FiberId,
        /// Value delivered to the parked call.
        value: program::Value,
    },
}

/// One function call with its environment, arguments, and captured context.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Call {
    /// Function to invoke.
    pub function: program::FunctionId,
    /// Hidden closure environment when one exists.
    pub environment: Option<program::Value>,
    /// Source-level arguments in parameter order.
    pub arguments: Vec<program::Value>,
    /// Dynamically scoped context captured when the call was made.
    pub context: program::Context,
}

/// Runnable retained across one runtime handshake or debugger stop.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetainedRunnable {
    /// Runnable identifier used for ordering and logging.
    pub id: RunnableId,
    /// Runnable scope active when execution stopped.
    pub scope: RunnableScope,
    /// Fiber identity whose execution the worker machine retains.
    pub fiber_id: program::FiberId,
    /// Debugger stop reason when execution is externally paused.
    pub reason: Option<program::StopReason>,
}

/// Opaque runnable identifier used by the event loop.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Reflect,
)]
pub struct RunnableId(u64);

impl Runnable {
    /// Create one identified event-loop execution.
    pub(super) const fn new(id: RunnableId, invocation: Invocation) -> Self {
        Self { id, invocation }
    }

    /// Fork this runnable for one forked World.
    pub(super) fn inherit(&self) -> Self {
        Self {
            id: self.id,
            invocation: self.invocation.inherit(),
        }
    }

    /// Visit mutable heap root slots retained by this runnable.
    pub(crate) fn visit_root_slots(
        &mut self,
        program: &program::Program,
        visit: &mut dyn FnMut(heap::RootSlot<'_>) -> heap::HeapResult<()>,
    ) -> RuntimeResult<()> {
        self.invocation.visit_root_slots(program, visit)
    }

    /// Release the values owned by this runnable.
    pub(crate) fn release(self) -> Option<program::Value> {
        match self.invocation {
            Invocation::Wake { value, .. } => Some(value),
            Invocation::Function(_) => None,
        }
    }
}

impl Invocation {
    /// Create one direct function invocation.
    pub fn call(
        function: program::FunctionId,
        arguments: impl IntoIterator<Item = program::Value>,
        context: program::Context,
    ) -> Self {
        Self::Function(Call::new(function, arguments, context))
    }

    /// Create one fiber wake delivery.
    pub fn wake(fiber_id: program::FiberId, value: program::Value) -> Self {
        Self::Wake { fiber_id, value }
    }

    /// Fork this invocation for one forked World.
    pub fn inherit(&self) -> Self {
        match self {
            Self::Function(call) => Self::Function(call.fork()),
            Self::Wake { fiber_id, value } => Self::Wake {
                fiber_id: *fiber_id,
                value: value.fork(),
            },
        }
    }

    /// Return the dynamically scoped context carried by fresh invocations.
    pub const fn context(&self) -> Option<program::Context> {
        match self {
            Self::Function(call) => Some(call.context),
            Self::Wake { .. } => None,
        }
    }

    /// Visit mutable heap root slots retained by this runnable.
    pub(crate) fn visit_root_slots(
        &mut self,
        program: &program::Program,
        visit: &mut dyn FnMut(heap::RootSlot<'_>) -> heap::HeapResult<()>,
    ) -> RuntimeResult<()> {
        match self {
            // function calls
            Self::Function(call) => call.visit_root_slots(program, visit)?,
            // wake value delivered to one parked fiber
            Self::Wake { value, .. } => {
                program
                    .visit_value_root_slots(value, visit)
                    .map_err(Box::<RuntimeError>::from)?;
            }
        }

        Ok(())
    }
}

impl Call {
    /// Create one direct function call without an environment.
    pub fn new(
        function: program::FunctionId,
        arguments: impl IntoIterator<Item = program::Value>,
        context: program::Context,
    ) -> Self {
        Self {
            function,
            environment: None,
            arguments: arguments.into_iter().collect(),
            context,
        }
    }

    /// Create one invocation of this call while retaining it.
    pub fn invoke(&self) -> Invocation {
        Invocation::Function(self.fork())
    }

    /// Fork this call for one forked World.
    pub fn fork(&self) -> Self {
        Self {
            function: self.function,
            environment: self.environment.as_ref().map(program::Value::fork),
            arguments: self.arguments.iter().map(program::Value::fork).collect(),
            context: self.context,
        }
    }

    /// Visit mutable heap root slots retained by this call.
    pub(crate) fn visit_root_slots(
        &mut self,
        program: &program::Program,
        visit: &mut dyn FnMut(heap::RootSlot<'_>) -> heap::HeapResult<()>,
    ) -> RuntimeResult<()> {
        visit(heap::RootSlot::HeapReference(self.context.reference_mut()))?;

        if let Some(environment) = &mut self.environment {
            program
                .visit_value_root_slots(environment, visit)
                .map_err(Box::<RuntimeError>::from)?;
        }

        for argument in &mut self.arguments {
            program
                .visit_value_root_slots(argument, visit)
                .map_err(Box::<RuntimeError>::from)?;
        }

        Ok(())
    }
}

impl RetainedRunnable {
    /// Create one retained runnable.
    pub const fn new(
        id: RunnableId,
        scope: RunnableScope,
        fiber_id: program::FiberId,
        reason: Option<program::StopReason>,
    ) -> Self {
        Self {
            id,
            scope,
            fiber_id,
            reason,
        }
    }
}

impl RunnableId {
    /// Create a new runnable identifier.
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Return the raw runnable identifier value.
    pub const fn get(self) -> u64 {
        self.0
    }
}
