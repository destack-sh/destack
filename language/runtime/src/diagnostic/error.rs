use std::fmt;

use destack_vm as vm;

use crate::platform::diagnostic::{PlatformError, PlatformErrorCode};

/// Error type for runtime execution and platform binding failures.
#[derive(Debug, Clone)]
#[repr(u16)]
pub enum RuntimeError {
    /// VM runtime error bubbled through the runtime boundary.
    Vm(Box<vm::Error>) = 1,
    /// Platform binding failure.
    Platform(Box<PlatformError>) = 2,
    /// Binding name was not found in the registry.
    BindingNotFound {
        /// Fully qualified binding name.
        name: String,
    } = 100,
    /// Binding call rejected by policy.
    PolicyViolation {
        /// Fully qualified binding name.
        name: String,
    } = 101,
    /// Resource identifier was not found.
    ResourceNotFound {
        /// Resource id or handle.
        resource_id: u64,
        /// Optional resource type or table name.
        resource_kind: Option<String>,
    } = 103,
    /// Scheduler became idle before completing a task.
    SchedulerIdle { task_id: u64 } = 104,
    /// Replay log ended before the requested event.
    ReplayLogExhausted {
        /// Sequence number of the missing event.
        sequence: u64,
    } = 107,
    /// Runtime call context was not available.
    RuntimeContextMissing = 109,
    /// Internal runtime error.
    Internal { message: String } = 110,
}

impl RuntimeError {
    /// Wrap a VM error.
    pub fn vm(error: vm::Error) -> Self {
        Self::Vm(Box::new(error))
    }

    /// Wrap a platform error.
    pub fn platform(error: PlatformError) -> Self {
        Self::Platform(Box::new(error))
    }

    /// Build a binding-not-found error.
    pub fn binding_not_found(name: impl Into<String>) -> Self {
        Self::BindingNotFound { name: name.into() }
    }

    /// Build a policy violation error.
    pub fn policy_violation(name: impl Into<String>) -> Self {
        Self::PolicyViolation { name: name.into() }
    }

    /// Build a resource not found error.
    pub fn resource_not_found(resource_id: u64, resource_kind: Option<String>) -> Self {
        Self::ResourceNotFound {
            resource_id,
            resource_kind,
        }
    }

    /// Build a scheduler idle error.
    pub fn scheduler_idle(task_id: u64) -> Self {
        Self::SchedulerIdle { task_id }
    }

    /// Build a replay log exhausted error.
    pub fn replay_log_exhausted(sequence: u64) -> Self {
        Self::ReplayLogExhausted { sequence }
    }

    /// Build a runtime context missing error.
    pub fn runtime_context_missing() -> Self {
        Self::RuntimeContextMissing
    }

    /// Build an internal error.
    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal {
            message: message.into(),
        }
    }

    /// Return a human-readable error message.
    pub fn message(&self) -> String {
        match self {
            RuntimeError::Vm(error) => error.message(),
            RuntimeError::Platform(error) => error.message(),
            RuntimeError::BindingNotFound { name } => format!("binding not found: {name}"),
            RuntimeError::PolicyViolation { name } => {
                format!("binding forbidden by policy: {name}")
            }
            RuntimeError::ResourceNotFound {
                resource_id,
                resource_kind,
            } => {
                if let Some(resource_kind) = resource_kind {
                    format!("resource not found: {resource_kind} {resource_id}")
                } else {
                    format!("resource not found: {resource_id}")
                }
            }
            RuntimeError::SchedulerIdle { task_id } => {
                format!("scheduler idle before completing task {task_id}")
            }
            RuntimeError::ReplayLogExhausted { sequence } => {
                format!("replay log exhausted at {sequence}")
            }
            RuntimeError::RuntimeContextMissing => "runtime call context missing".to_string(),
            RuntimeError::Internal { message } => format!("internal error: {message}"),
        }
    }

    /// Convert the runtime error into a platform error.
    pub fn into_platform_error(self) -> PlatformError {
        match self {
            RuntimeError::Platform(error) => *error,
            RuntimeError::Vm(error) => {
                PlatformError::generic(None, format!("vm error: {}", error.message()))
            }
            RuntimeError::BindingNotFound { name } => {
                PlatformError::not_supported(format!("binding not found: {name}"))
            }
            RuntimeError::PolicyViolation { name } => {
                PlatformError::not_supported(format!("binding forbidden by policy: {name}"))
            }
            RuntimeError::ResourceNotFound {
                resource_id,
                resource_kind,
            } => {
                let label = resource_kind.unwrap_or_else(|| "resource".to_string());
                PlatformError::invalid_argument(format!("{label} not found: {resource_id}"))
            }
            RuntimeError::SchedulerIdle { task_id } => {
                PlatformError::generic(None, format!("scheduler idle before task {task_id}"))
            }
            RuntimeError::ReplayLogExhausted { sequence } => {
                PlatformError::generic(None, format!("replay log exhausted at {sequence}"))
            }
            RuntimeError::RuntimeContextMissing => {
                PlatformError::generic(None, "runtime call context missing")
            }
            RuntimeError::Internal { message } => {
                PlatformError::generic(None, format!("internal error: {message}"))
            }
        }
    }

    /// Boxed runtime error for result propagation.
    pub fn boxed(self) -> Box<Self> {
        Box::new(self)
    }

    /// Return the numeric error code.
    #[inline]
    pub fn code(&self) -> u16 {
        // Safety: repr(u16) ensures the discriminant is valid.
        unsafe { *(self as *const Self as *const u16) }
    }

    /// Return a sub-code for status mapping.
    pub fn sub_code(&self) -> u32 {
        match self {
            RuntimeError::Vm(error) => error.sub_code() as u32,
            RuntimeError::Platform(error) => {
                error.code.unwrap_or(PlatformErrorCode::Generic).number()
            }
            _ => self.code() as u32,
        }
    }
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message())
    }
}

impl std::error::Error for RuntimeError {}

impl From<vm::Error> for RuntimeError {
    fn from(error: vm::Error) -> Self {
        RuntimeError::vm(error)
    }
}

impl From<vm::Error> for Box<RuntimeError> {
    fn from(error: vm::Error) -> Self {
        Box::new(RuntimeError::from(error))
    }
}

impl From<PlatformError> for Box<RuntimeError> {
    fn from(error: PlatformError) -> Self {
        Box::new(RuntimeError::platform(error))
    }
}

impl From<vm::RuntimeError> for Box<RuntimeError> {
    fn from(error: vm::RuntimeError) -> Self {
        Box::new(RuntimeError::vm(error.error))
    }
}

impl From<PlatformError> for RuntimeError {
    fn from(error: PlatformError) -> Self {
        RuntimeError::platform(error)
    }
}

impl From<RuntimeError> for vm::Error {
    fn from(error: RuntimeError) -> Self {
        match error {
            RuntimeError::Vm(error) => *error,
            RuntimeError::Platform(error) => (*error).into(),
            RuntimeError::BindingNotFound { name } => vm::Error::ExternalFunctionNotFound { name },
            RuntimeError::PolicyViolation { name } => vm::Error::ExternalCallForbidden { name },
            RuntimeError::ResourceNotFound {
                resource_id,
                resource_kind,
            } => {
                let kind = resource_kind.unwrap_or_else(|| "resource".to_string());
                vm::Error::Panic {
                    message: format!("{kind} not found: {resource_id}"),
                }
            }
            RuntimeError::SchedulerIdle { task_id } => vm::Error::Panic {
                message: format!("scheduler idle before completing task {task_id}"),
            },
            RuntimeError::ReplayLogExhausted { sequence } => vm::Error::Panic {
                message: format!("replay log exhausted at {sequence}"),
            },
            RuntimeError::RuntimeContextMissing => vm::Error::Panic {
                message: "runtime call context missing".to_string(),
            },
            RuntimeError::Internal { message } => vm::Error::Panic { message },
        }
    }
}

impl From<Box<RuntimeError>> for vm::Error {
    fn from(error: Box<RuntimeError>) -> Self {
        (*error).into()
    }
}

/// Result type for runtime execution.
pub type RuntimeResult<T> = Result<T, Box<RuntimeError>>;
