use crate::diagnostic::{
    BindingError, Entity, EntityError, RuntimeError, RuntimeFailure, TraceFailure,
};
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
            RuntimeError::Binding { name, reason } => match reason {
                BindingError::NotFound => Self::BindingNotFound { name: name.clone() },
                BindingError::PolicyViolation => Self::PolicyViolation { name: name.clone() },
                BindingError::ActionDenied { action } => Self::ActionDenied {
                    name: name.clone(),
                    action: action.clone(),
                },
                BindingError::AffinityViolation { affinity } => Self::AffinityViolation {
                    name: name.clone(),
                    affinity: affinity.clone(),
                },
            },
            RuntimeError::Entity {
                reason:
                    EntityError::NotFound(Entity::Resource {
                        resource_id,
                        resource_kind,
                    }),
            } => Self::ResourceNotFound {
                resource_id: *resource_id,
                resource_kind: resource_kind.clone(),
            },
            RuntimeError::Runtime {
                reason: RuntimeFailure::EventLoopIdle { task_id },
            } => Self::EventLoopIdle { task_id: *task_id },
            RuntimeError::Trace { reason } => match reason {
                TraceFailure::Exhausted { sequence } => Self::TraceExhausted {
                    sequence: *sequence,
                },
                TraceFailure::Mismatch { name } => Self::TraceMismatch { name: name.clone() },
                TraceFailure::PayloadUnsupported { name } => {
                    Self::TracePayloadUnsupported { name: name.clone() }
                }
                TraceFailure::EncodeFailed { name } => {
                    Self::TraceEncodeFailed { name: name.clone() }
                }
                TraceFailure::DecodeFailed { name } => {
                    Self::TraceDecodeFailed { name: name.clone() }
                }
            },
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
            TraceError::BindingNotFound { name } => Self::binding_not_found(name),
            TraceError::PolicyViolation { name } => Self::policy_violation(name),
            TraceError::ActionDenied { name, action } => Self::action_denied(name, action),
            TraceError::AffinityViolation { name, affinity } => {
                Self::affinity_violation(name, affinity)
            }
            TraceError::ResourceNotFound {
                resource_id,
                resource_kind,
            } => Self::resource_not_found(resource_id, resource_kind),
            TraceError::EventLoopIdle { task_id } => Self::event_loop_idle(task_id),
            TraceError::TraceExhausted { sequence } => Self::trace_exhausted(sequence),
            TraceError::TraceMismatch { name } => Self::trace_mismatch(name),
            TraceError::TracePayloadUnsupported { name } => Self::trace_payload_unsupported(name),
            TraceError::TraceEncodeFailed { name } => Self::trace_encode_failed(name),
            TraceError::TraceDecodeFailed { name } => Self::trace_decode_failed(name),
            TraceError::Internal { message } => Self::Internal { message },
        }
    }
}

impl From<TraceError> for Box<RuntimeError> {
    fn from(error: TraceError) -> Self {
        RuntimeError::from(error).boxed()
    }
}
