use std::fmt;

use destack_vm as vm;

use crate::platform::diagnostic::PlatformError;

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
    /// Binding call rejected due to a missing required capability.
    CapabilityViolation {
        /// Fully qualified binding name.
        name: String,
        /// Required capability that was not granted.
        capability: String,
    } = 102,
    /// Binding call rejected due to an execution-affinity mismatch.
    AffinityViolation {
        /// Fully qualified binding name.
        name: String,
        /// Required affinity for this binding.
        affinity: String,
    } = 114,
    /// Resource identifier was not found.
    ResourceNotFound {
        /// Resource id or handle.
        resource_id: u64,
        /// Optional resource type or table name.
        resource_kind: Option<String>,
    } = 103,
    /// Event loop became idle before completing a task.
    EventLoopIdle { task_id: u64 } = 104,
    /// Replay log ended before the requested event.
    ReplayLogExhausted {
        /// Sequence number of the missing event.
        sequence: u64,
    } = 107,
    /// Replay log did not match the current execution.
    ReplayMismatch {
        /// Binding name for the mismatch.
        name: String,
    } = 108,
    /// Replay payload policy is not supported by a binding.
    ReplayPayloadUnsupported {
        /// Binding name for the mismatch.
        name: String,
    } = 109,
    /// Binding call context was not available.
    BindingCallContextMissing = 110,
    /// Replay payload failed to encode.
    ReplayEncodeFailed {
        /// Binding name for the failed payload.
        name: String,
    } = 111,
    /// Replay payload failed to decode.
    ReplayDecodeFailed {
        /// Binding name for the failed payload.
        name: String,
    } = 112,
    /// Internal runtime error.
    Internal { message: String } = 113,
}

impl RuntimeError {
    /// Return a human-readable error message.
    pub fn message(&self) -> String {
        match self {
            RuntimeError::Vm(error) => error.message(),
            RuntimeError::Platform(error) => error.message(),
            RuntimeError::BindingNotFound { name } => format!("binding not found: {name}"),
            RuntimeError::PolicyViolation { name } => {
                format!("binding forbidden by policy: {name}")
            }
            RuntimeError::CapabilityViolation { name, capability } => {
                format!("binding capability denied: {name} requires {capability}")
            }
            RuntimeError::AffinityViolation { name, affinity } => {
                format!("binding affinity denied: {name} requires {affinity}")
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
            RuntimeError::EventLoopIdle { task_id } => {
                format!("event loop idle before completing task {task_id}")
            }
            RuntimeError::ReplayLogExhausted { sequence } => {
                format!("replay log exhausted at {sequence}")
            }
            RuntimeError::ReplayMismatch { name } => {
                format!("replay mismatch for {name}")
            }
            RuntimeError::ReplayPayloadUnsupported { name } => {
                format!("replay payload unsupported for {name}")
            }
            RuntimeError::BindingCallContextMissing => "binding call context missing".to_string(),
            RuntimeError::ReplayEncodeFailed { name } => {
                format!("failed to encode replay payload for {name}")
            }
            RuntimeError::ReplayDecodeFailed { name } => {
                format!("failed to decode replay payload for {name}")
            }
            RuntimeError::Internal { message } => format!("internal error: {message}"),
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
            RuntimeError::Platform(error) => error.code.number(),
            _ => self.code() as u32,
        }
    }

    /// Return the platform error if this is a platform failure.
    pub fn platform_error(&self) -> Option<&PlatformError> {
        match self {
            RuntimeError::Platform(error) => Some(error.as_ref()),
            _ => None,
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
        RuntimeError::Vm(Box::new(error))
    }
}

impl From<vm::Error> for Box<RuntimeError> {
    fn from(error: vm::Error) -> Self {
        Box::new(RuntimeError::from(error))
    }
}

impl From<PlatformError> for Box<RuntimeError> {
    fn from(error: PlatformError) -> Self {
        Box::new(RuntimeError::from(error))
    }
}

impl From<vm::RuntimeError> for Box<RuntimeError> {
    fn from(error: vm::RuntimeError) -> Self {
        Box::new(RuntimeError::from(error.error))
    }
}

impl From<PlatformError> for RuntimeError {
    fn from(error: PlatformError) -> Self {
        RuntimeError::Platform(Box::new(error))
    }
}

impl From<&RuntimeError> for PlatformError {
    fn from(error: &RuntimeError) -> Self {
        if let RuntimeError::Platform(error) = error {
            return error.as_ref().clone();
        }

        PlatformError::generic(None, error.message())
    }
}

impl From<RuntimeError> for PlatformError {
    fn from(error: RuntimeError) -> Self {
        PlatformError::from(&error)
    }
}

impl From<Box<RuntimeError>> for vm::Error {
    fn from(error: Box<RuntimeError>) -> Self {
        match *error {
            RuntimeError::Vm(error) => *error,
            RuntimeError::Platform(error) => (*error).into(),
            RuntimeError::BindingNotFound { name } => vm::Error::ExternalFunctionNotFound { name },
            RuntimeError::PolicyViolation { name } => vm::Error::ExternalCallForbidden { name },
            RuntimeError::CapabilityViolation { name, capability } => {
                vm::Error::ExternalCallForbidden {
                    name: format!("{name} ({capability})"),
                }
            }
            RuntimeError::AffinityViolation { name, affinity } => {
                vm::Error::ExternalCallForbidden {
                    name: format!("{name} ({affinity})"),
                }
            }
            other => vm::Error::Panic {
                message: other.message(),
            },
        }
    }
}

/// Result type for runtime execution.
pub type RuntimeResult<T> = Result<T, Box<RuntimeError>>;
