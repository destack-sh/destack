use destack_heap as heap;
use destack_memory::MemoryMap;
use destack_program as program;
use serde::{Deserialize, Serialize};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::worker::RunnableScope;

/// One repeatable program callback.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Callback {
    /// Function to invoke.
    function: program::FunctionId,
    /// Hidden closure environment when one exists.
    environment: Option<program::Value>,
    /// Copyable source-level arguments in parameter order.
    arguments: Vec<program::Value>,
}

/// One consumable program invocation.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Invocation {
    /// Invoke one function.
    Function {
        /// Function to invoke.
        function: program::FunctionId,
        /// Hidden closure environment when one exists.
        environment: Option<program::Value>,
        /// Source-level arguments in parameter order.
        arguments: Vec<program::Value>,
    },
    /// Resume one suspended continuation.
    Resume {
        /// Task resumed by this invocation when present.
        task: Option<program::Task>,
        /// Canonical continuation to resume.
        continuation: program::Continuation,
        /// Value delivered to the suspended expression.
        value: program::Value,
    },
    /// Complete one suspended generator continuation.
    Complete {
        /// Canonical continuation to complete.
        continuation: program::Continuation,
        /// Value delivered to the suspended completion expression.
        value: program::Value,
    },
    /// Cancel one suspended asynchronous continuation.
    Cancel {
        /// Task cancelled by this invocation when present.
        task: Option<program::Task>,
        /// Canonical continuation to cancel.
        continuation: program::Continuation,
    },
}

/// One identified event-loop execution.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Runnable {
    /// Runnable identifier used for ordering and logging.
    pub id: RunnableId,
    /// Function invocation to execute.
    pub invocation: Invocation,
}

/// Runnable retained across one runtime handshake or debugger stop.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetainedRunnable {
    /// Runnable identifier used for ordering and logging.
    pub id: RunnableId,
    /// Runnable scope active when execution stopped.
    pub scope: RunnableScope,
    /// Task settled by this execution when present.
    pub task: Option<program::Task>,
    /// Debugger stop reason when execution is externally paused.
    pub reason: Option<program::StopReason>,
}

/// Opaque runnable identifier used by the event loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct RunnableId(u64);

impl Invocation {
    /// Create one direct function invocation.
    pub fn call(
        function: program::FunctionId,
        arguments: impl IntoIterator<Item = program::Value>,
    ) -> Self {
        Self::Function {
            function,
            environment: None,
            arguments: arguments.into_iter().collect(),
        }
    }

    /// Create one continuation resumption.
    pub fn resume(
        task: Option<program::Task>,
        continuation: program::Continuation,
        value: program::Value,
    ) -> Self {
        Self::Resume {
            task,
            continuation,
            value,
        }
    }

    /// Create one continuation completion.
    pub fn complete(continuation: program::Continuation, value: program::Value) -> Self {
        Self::Complete {
            continuation,
            value,
        }
    }

    /// Create one continuation cancellation.
    pub fn cancel(task: Option<program::Task>, continuation: program::Continuation) -> Self {
        Self::Cancel { task, continuation }
    }

    /// Fork this invocation for one forked World.
    pub fn inherit(&self) -> Self {
        match self {
            Self::Function {
                function,
                environment,
                arguments,
            } => Self::Function {
                function: *function,
                environment: environment.as_ref().map(program::Value::fork),
                arguments: arguments.iter().map(program::Value::fork).collect(),
            },
            Self::Resume {
                task,
                continuation,
                value,
            } => Self::Resume {
                task: *task,
                continuation: continuation.inherit(),
                value: value.fork(),
            },
            Self::Complete {
                continuation,
                value,
            } => Self::Complete {
                continuation: continuation.inherit(),
                value: value.fork(),
            },
            Self::Cancel { task, continuation } => Self::Cancel {
                task: *task,
                continuation: continuation.inherit(),
            },
        }
    }

    /// Return the task resumed or cancelled by this invocation.
    pub const fn task(&self) -> Option<program::Task> {
        match self {
            Self::Resume { task, .. } | Self::Cancel { task, .. } => *task,
            Self::Function { .. } | Self::Complete { .. } => None,
        }
    }

    /// Return the suspended continuation carried by this invocation when present.
    pub(crate) const fn continuation(&self) -> Option<&program::Continuation> {
        match self {
            Self::Resume { continuation, .. }
            | Self::Complete { continuation, .. }
            | Self::Cancel { continuation, .. } => Some(continuation),
            Self::Function { .. } => None,
        }
    }

    /// Release the suspended continuation owned by this invocation when present.
    pub(crate) fn release(self, memory: &MemoryMap) -> RuntimeResult<()> {
        let continuation = match self {
            Self::Resume { continuation, .. }
            | Self::Complete { continuation, .. }
            | Self::Cancel { continuation, .. } => Some(continuation),
            Self::Function { .. } => None,
        };

        continuation
            .map(|continuation| continuation.release(memory))
            .transpose()
            .map_err(Box::<RuntimeError>::from)?;

        Ok(())
    }

    /// Visit mutable heap root slots retained by this runnable.
    pub(crate) fn visit_root_slots(
        &mut self,
        program: &program::Program,
        memory: &MemoryMap,
        visit: &mut dyn FnMut(heap::RootSlot<'_>) -> heap::HeapResult<()>,
    ) -> RuntimeResult<()> {
        match self {
            // function values
            Self::Function {
                environment,
                arguments,
                ..
            } => {
                if let Some(environment) = environment {
                    program
                        .visit_value_root_slots(environment, visit)
                        .map_err(Box::<RuntimeError>::from)?;
                }

                for argument in arguments {
                    program
                        .visit_value_root_slots(argument, visit)
                        .map_err(Box::<RuntimeError>::from)?;
                }
            }
            // suspended call chain and resume value
            Self::Resume {
                continuation,
                value,
                ..
            }
            | Self::Complete {
                continuation,
                value,
            } => {
                program
                    .visit_continuation_root_slots(memory, continuation, visit)
                    .map_err(Box::<RuntimeError>::from)?;
                program
                    .visit_value_root_slots(value, visit)
                    .map_err(Box::<RuntimeError>::from)?;
            }
            // suspended call chain
            Self::Cancel { continuation, .. } => {
                program
                    .visit_continuation_root_slots(memory, continuation, visit)
                    .map_err(Box::<RuntimeError>::from)?;
            }
        }

        Ok(())
    }
}

impl Callback {
    /// Create one direct function callback.
    pub fn call(
        function: program::FunctionId,
        arguments: impl IntoIterator<Item = program::Value>,
    ) -> Self {
        Self {
            function,
            environment: None,
            arguments: arguments.into_iter().collect(),
        }
    }

    /// Create one callback invocation while retaining this registration.
    pub fn invoke(&self) -> Invocation {
        Invocation::Function {
            function: self.function,
            environment: self.environment.as_ref().map(program::Value::fork),
            arguments: self.arguments.iter().map(program::Value::fork).collect(),
        }
    }

    /// Fork this callback for one forked World.
    pub fn fork(&self) -> Self {
        Self {
            function: self.function,
            environment: self.environment.as_ref().map(program::Value::fork),
            arguments: self.arguments.iter().map(program::Value::fork).collect(),
        }
    }

    /// Visit mutable heap root slots retained by this callback.
    pub(crate) fn visit_root_slots(
        &mut self,
        program: &program::Program,
        visit: &mut dyn FnMut(heap::RootSlot<'_>) -> heap::HeapResult<()>,
    ) -> RuntimeResult<()> {
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
        memory: &MemoryMap,
        visit: &mut dyn FnMut(heap::RootSlot<'_>) -> heap::HeapResult<()>,
    ) -> RuntimeResult<()> {
        self.invocation.visit_root_slots(program, memory, visit)
    }

    /// Release the suspended continuation owned by this runnable when present.
    pub(crate) fn release(self, memory: &MemoryMap) -> RuntimeResult<()> {
        self.invocation.release(memory)
    }
}

impl RetainedRunnable {
    /// Create one retained runnable.
    pub const fn new(
        id: RunnableId,
        scope: RunnableScope,
        task: Option<program::Task>,
        reason: Option<program::StopReason>,
    ) -> Self {
        Self {
            id,
            scope,
            task,
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
