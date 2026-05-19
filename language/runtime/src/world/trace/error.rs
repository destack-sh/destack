use crate::diagnostic::RuntimeError;
use crate::host::HostError;
use destack_vm as vm;
use serde::{Deserialize, Serialize};

/// Encoded trace error payload for deterministic trace.
// NOTE #Performance #Cleanup: keep trace errors simple until the payload model settles
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TraceError {
    /// VM runtime error payload.
    Vm(vm::Error),
    /// Host runtime error payload.
    Host(HostError),
    /// Binding-not-found runtime error payload.
    BindingNotFound {
        /// Fully qualified binding name.
        name: String,
    },
    /// Policy-violation runtime error payload.
    PolicyViolation {
        /// Fully qualified binding name.
        name: String,
    },
    /// Action-violation runtime error payload.
    ActionDenied {
        /// Fully qualified binding name.
        name: String,
        /// Missing required action.
        action: String,
    },
    /// Affinity-violation runtime error payload.
    AffinityViolation {
        /// Fully qualified binding name.
        name: String,
        /// Required affinity classification.
        affinity: String,
    },
    /// Resource-not-found runtime error payload.
    ResourceNotFound {
        /// Resource id or handle.
        resource_id: u64,
        /// Optional resource kind.
        resource_kind: Option<String>,
    },
    /// Event-loop-idle runtime error payload.
    EventLoopIdle {
        /// Task identifier for the idle event.
        task_id: u64,
    },
    /// Trace-exhausted runtime error payload.
    TraceExhausted {
        /// Sequence number of the missing replay event.
        sequence: u64,
    },
    /// Trace-mismatch runtime error payload.
    TraceMismatch {
        /// Channel name that mismatched.
        name: String,
    },
    /// Trace-payload-unsupported runtime error payload.
    TracePayloadUnsupported {
        /// Binding name for this payload mismatch.
        name: String,
    },
    /// Trace-encode-failed runtime error payload.
    TraceEncodeFailed {
        /// Binding name for this payload encode failure.
        name: String,
    },
    /// Trace-decode-failed runtime error payload.
    TraceDecodeFailed {
        /// Binding name for this payload decode failure.
        name: String,
    },
    /// Internal runtime error payload.
    Internal {
        /// Runtime error message.
        message: String,
    },
}

impl From<&RuntimeError> for TraceError {
    fn from(error: &RuntimeError) -> Self {
        match error {
            RuntimeError::Vm(error) => Self::Vm(error.as_ref().clone()),
            RuntimeError::Host(error) => Self::Host(error.as_ref().clone()),
            RuntimeError::BindingNotFound { name } => Self::BindingNotFound { name: name.clone() },
            RuntimeError::PolicyViolation { name } => Self::PolicyViolation { name: name.clone() },
            RuntimeError::ActionDenied { name, action } => Self::ActionDenied {
                name: name.clone(),
                action: action.clone(),
            },
            RuntimeError::AffinityViolation { name, affinity } => Self::AffinityViolation {
                name: name.clone(),
                affinity: affinity.clone(),
            },
            RuntimeError::ResourceNotFound {
                resource_id,
                resource_kind,
            } => Self::ResourceNotFound {
                resource_id: *resource_id,
                resource_kind: resource_kind.clone(),
            },
            RuntimeError::EventLoopIdle { task_id } => Self::EventLoopIdle { task_id: *task_id },
            RuntimeError::TraceExhausted { sequence } => Self::TraceExhausted {
                sequence: *sequence,
            },
            RuntimeError::TraceMismatch { name } => Self::TraceMismatch { name: name.clone() },
            RuntimeError::TracePayloadUnsupported { name } => {
                Self::TracePayloadUnsupported { name: name.clone() }
            }
            RuntimeError::TraceEncodeFailed { name } => {
                Self::TraceEncodeFailed { name: name.clone() }
            }
            RuntimeError::TraceDecodeFailed { name } => {
                Self::TraceDecodeFailed { name: name.clone() }
            }
            RuntimeError::Internal { message } => Self::Internal {
                message: message.clone(),
            },
            other => Self::Internal {
                message: other.message(),
            },
        }
    }
}

impl From<TraceError> for RuntimeError {
    fn from(error: TraceError) -> Self {
        match error {
            TraceError::Vm(error) => Self::Vm(Box::new(error)),
            TraceError::Host(error) => Self::Host(error.boxed()),
            TraceError::BindingNotFound { name } => Self::BindingNotFound { name },
            TraceError::PolicyViolation { name } => Self::PolicyViolation { name },
            TraceError::ActionDenied { name, action } => Self::ActionDenied { name, action },
            TraceError::AffinityViolation { name, affinity } => {
                Self::AffinityViolation { name, affinity }
            }
            TraceError::ResourceNotFound {
                resource_id,
                resource_kind,
            } => Self::ResourceNotFound {
                resource_id,
                resource_kind,
            },
            TraceError::EventLoopIdle { task_id } => Self::EventLoopIdle { task_id },
            TraceError::TraceExhausted { sequence } => Self::TraceExhausted { sequence },
            TraceError::TraceMismatch { name } => Self::TraceMismatch { name },
            TraceError::TracePayloadUnsupported { name } => Self::TracePayloadUnsupported { name },
            TraceError::TraceEncodeFailed { name } => Self::TraceEncodeFailed { name },
            TraceError::TraceDecodeFailed { name } => Self::TraceDecodeFailed { name },
            TraceError::Internal { message } => Self::Internal { message },
        }
    }
}

impl From<TraceError> for Box<RuntimeError> {
    fn from(error: TraceError) -> Self {
        RuntimeError::from(error).boxed()
    }
}
