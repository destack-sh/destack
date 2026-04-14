use std::fmt;

use {destack_heap as heap, destack_vm as vm};

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
    /// Trace ended before the requested event.
    TraceExhausted {
        /// Sequence number of the missing event.
        sequence: u64,
    } = 107,
    /// Trace did not match the current execution.
    TraceMismatch {
        /// Binding name for the mismatch.
        name: String,
    } = 108,
    /// Trace payload policy is not supported by a binding.
    TracePayloadUnsupported {
        /// Binding name for the mismatch.
        name: String,
    } = 109,
    /// Binding call context was not available.
    BindingCallContextMissing = 110,
    /// Trace payload failed to encode.
    TraceEncodeFailed {
        /// Binding name for the failed payload.
        name: String,
    } = 111,
    /// Trace payload failed to decode.
    TraceDecodeFailed {
        /// Binding name for the failed payload.
        name: String,
    } = 112,
    /// World branch identifier was not found in lineage.
    BranchNotFound {
        /// Missing branch identifier.
        branch_id: u128,
    } = 115,
    /// World revision identifier was not found in lineage.
    RevisionNotFound {
        /// Missing revision identifier.
        revision_id: u128,
    } = 116,
    /// World checkpoint identifier was not found in lineage.
    CheckpointNotFound {
        /// Missing checkpoint identifier.
        checkpoint_id: u128,
    } = 117,
    /// World image identifier was not found in lineage.
    ImageNotFound {
        /// Missing image identifier.
        image_id: u128,
    } = 118,
    /// One world moment was not found in lineage.
    MomentNotFound {
        /// Branch identifier for the missing moment.
        branch_id: u128,
        /// Trace sequence for the missing moment.
        sequence: u64,
    } = 144,
    /// One requested moment does not belong to the active world branch.
    MomentBranchMismatch {
        /// Requested moment branch identifier.
        moment_branch_id: u128,
        /// Active world branch identifier.
        world_branch_id: u128,
    } = 145,
    /// Observation subscription identifier was not found.
    ObservationSubscriptionNotFound {
        /// Missing observation subscription identifier.
        subscription_id: u64,
    } = 141,
    /// Control handle identifier was not found.
    ControlHandleNotFound {
        /// Missing control-handle identifier.
        handle_id: u64,
        /// Expected control-handle kind.
        kind: String,
    } = 142,
    /// Control handle kind did not match the requested operation.
    ControlHandleKindMismatch {
        /// The control-handle identifier.
        handle_id: u64,
        /// The expected control-handle kind.
        expected: String,
        /// The actual control-handle kind.
        actual: String,
    } = 143,
    /// World runtime identifier was not found.
    RuntimeNotFound {
        /// Missing runtime identifier.
        runtime_id: u64,
    } = 127,
    /// Runtime agent identifier was not found.
    AgentNotFound {
        /// Missing agent identifier.
        agent_id: u64,
    } = 128,
    /// World or runtime already contains the requested runtime identifier.
    RuntimeAlreadyExists {
        /// Conflicting runtime identifier.
        runtime_id: u64,
    } = 129,
    /// Runtime already contains the requested agent identifier.
    AgentAlreadyExists {
        /// Conflicting agent identifier.
        agent_id: u64,
    } = 130,
    /// Runtime image is missing its declared primary agent.
    PrimaryAgentMissing {
        /// Runtime identifier with the invalid primary-agent reference.
        runtime_id: u64,
        /// Missing primary agent identifier.
        agent_id: u64,
    } = 131,
    /// Runtime cannot remove its last remaining agent.
    LastAgentRemoval = 132,
    /// Runtime cannot remove the primary agent until a replacement is selected.
    PrimaryAgentRemoval = 133,
    /// Runtime capability profile configuration is invalid.
    CapabilityProfileInvalid {
        /// Invalid capability profile name.
        profile: String,
        /// Human-readable validation detail.
        detail: String,
    } = 134,
    /// World topology is missing one runtime identity record.
    TopologyRuntimeMissing {
        /// Missing runtime identifier.
        runtime_id: u64,
    } = 135,
    /// World topology is missing one agent identity record.
    TopologyAgentMissing {
        /// Missing agent identifier.
        agent_id: u64,
    } = 136,
    /// World-controlled virtual time cannot advance while host time is active.
    HostTimeAdvance = 137,
    /// Engine adapter received an entry for the wrong engine kind.
    EngineEntryMismatch {
        /// Engine kind that received the entry.
        engine: String,
        /// Entry kind that was requested.
        entry: String,
    } = 138,
    /// Engine adapter received a continuation for the wrong engine kind.
    EngineContinuationMismatch {
        /// Engine kind that received the continuation.
        engine: String,
        /// Continuation kind that was requested.
        continuation: String,
    } = 139,
    /// Runtime image contains duplicate agent records.
    DuplicateAgentImage {
        /// Runtime identifier owning the duplicate agent image.
        runtime_id: u64,
        /// Duplicate agent identifier.
        agent_id: u64,
    } = 140,
    /// One revision points at one missing world image.
    RevisionImageMissing {
        /// Revision identifier with the dangling image reference.
        revision_id: u128,
        /// Missing image identifier.
        image_id: u128,
    } = 119,
    /// One revision points at one missing trace image.
    RevisionTraceImageMissing {
        /// Revision identifier with the dangling trace-image reference.
        revision_id: u128,
    } = 120,
    /// Exclusive access is already held.
    ExclusiveAccessHeld = 122,
    /// Shared activity was attempted while exclusive access is held.
    ExclusiveAccessConflict = 123,
    /// Snapshot branch metadata does not match the target world.
    SnapshotBranchMismatch {
        /// Captured snapshot branch identifier.
        snapshot_branch_id: u128,
        /// Active world branch identifier.
        world_branch_id: u128,
    } = 124,
    /// Capture failed because one subsystem cannot honestly materialize the requested mode.
    CaptureBarrier {
        /// The component that rejected capture.
        component: String,
        /// The requested capture mode.
        mode: String,
        /// Human-readable barrier detail.
        detail: String,
    } = 125,
    /// One captured image or snapshot is internally inconsistent.
    InconsistentImage {
        /// Human-readable inconsistency detail.
        detail: String,
    } = 126,
    /// One runtime heap exceeded its configured hard limit.
    HeapLimitExceeded {
        /// The limited heap scope.
        scope: String,
        /// The exact retained heap bytes currently in use.
        used_bytes: u64,
        /// The configured hard limit in bytes.
        max_bytes: u64,
    } = 146,
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
            RuntimeError::TraceExhausted { sequence } => {
                format!("trace exhausted at {sequence}")
            }
            RuntimeError::TraceMismatch { name } => {
                format!("trace mismatch for {name}")
            }
            RuntimeError::TracePayloadUnsupported { name } => {
                format!("trace payload unsupported for {name}")
            }
            RuntimeError::BindingCallContextMissing => "binding call context missing".to_string(),
            RuntimeError::TraceEncodeFailed { name } => {
                format!("failed to encode trace payload for {name}")
            }
            RuntimeError::TraceDecodeFailed { name } => {
                format!("failed to decode trace payload for {name}")
            }
            RuntimeError::BranchNotFound { branch_id } => {
                format!("branch not found: {branch_id}")
            }
            RuntimeError::RevisionNotFound { revision_id } => {
                format!("revision not found: {revision_id}")
            }
            RuntimeError::CheckpointNotFound { checkpoint_id } => {
                format!("checkpoint not found: {checkpoint_id}")
            }
            RuntimeError::ImageNotFound { image_id } => {
                format!("image not found: {image_id}")
            }
            RuntimeError::MomentNotFound {
                branch_id,
                sequence,
            } => {
                format!("moment not found: branch {branch_id} at sequence {sequence}")
            }
            RuntimeError::MomentBranchMismatch {
                moment_branch_id,
                world_branch_id,
            } => {
                format!(
                    "moment branch mismatch: moment branch {moment_branch_id} does not match world branch {world_branch_id}"
                )
            }
            RuntimeError::ObservationSubscriptionNotFound { subscription_id } => {
                format!("observation subscription not found: {subscription_id}")
            }
            RuntimeError::ControlHandleNotFound { handle_id, kind } => {
                format!("{kind} handle not found: {handle_id}")
            }
            RuntimeError::ControlHandleKindMismatch {
                handle_id,
                expected,
                actual,
            } => {
                format!("control handle {handle_id} has kind {actual}, expected {expected}")
            }
            RuntimeError::RuntimeNotFound { runtime_id } => {
                format!("runtime not found: {runtime_id}")
            }
            RuntimeError::AgentNotFound { agent_id } => {
                format!("agent not found: {agent_id}")
            }
            RuntimeError::RuntimeAlreadyExists { runtime_id } => {
                format!("runtime already exists: {runtime_id}")
            }
            RuntimeError::AgentAlreadyExists { agent_id } => {
                format!("agent already exists: {agent_id}")
            }
            RuntimeError::PrimaryAgentMissing {
                runtime_id,
                agent_id,
            } => {
                format!("runtime {runtime_id} is missing primary agent {agent_id}")
            }
            RuntimeError::LastAgentRemoval => "runtime must keep at least one agent".to_string(),
            RuntimeError::PrimaryAgentRemoval => {
                "cannot remove primary agent: set a new primary agent first".to_string()
            }
            RuntimeError::CapabilityProfileInvalid { profile, detail } => {
                format!("runtime capability profile `{profile}` is invalid: {detail}")
            }
            RuntimeError::TopologyRuntimeMissing { runtime_id } => {
                format!("runtime {runtime_id} is not registered in world topology")
            }
            RuntimeError::TopologyAgentMissing { agent_id } => {
                format!("agent {agent_id} is not registered in world topology")
            }
            RuntimeError::HostTimeAdvance => {
                "cannot advance virtual time while world uses host time".to_string()
            }
            RuntimeError::EngineEntryMismatch { engine, entry } => {
                format!("{engine} engine cannot run {entry} entry")
            }
            RuntimeError::EngineContinuationMismatch {
                engine,
                continuation,
            } => {
                format!("{engine} engine cannot handle {continuation} continuation")
            }
            RuntimeError::DuplicateAgentImage {
                runtime_id,
                agent_id,
            } => {
                format!("runtime {runtime_id} image contains duplicate agent {agent_id}")
            }
            RuntimeError::RevisionImageMissing {
                revision_id,
                image_id,
            } => {
                format!("revision {revision_id} is missing image {image_id}")
            }
            RuntimeError::RevisionTraceImageMissing { revision_id } => {
                format!("revision {revision_id} is missing trace image")
            }
            RuntimeError::ExclusiveAccessHeld => {
                "world is already under exclusive access".to_string()
            }
            RuntimeError::ExclusiveAccessConflict => {
                "world is under exclusive access for capture or restore".to_string()
            }
            RuntimeError::SnapshotBranchMismatch {
                snapshot_branch_id,
                world_branch_id,
            } => {
                format!(
                    "snapshot active branch {snapshot_branch_id} does not match world branch {world_branch_id}"
                )
            }
            RuntimeError::CaptureBarrier {
                component,
                mode,
                detail,
            } => {
                format!("{component} cannot capture for {mode}: {detail}")
            }
            RuntimeError::InconsistentImage { detail } => {
                format!("captured image is inconsistent: {detail}")
            }
            RuntimeError::HeapLimitExceeded {
                scope,
                used_bytes,
                max_bytes,
            } => {
                format!(
                    "{scope} heap limit exceeded: using {used_bytes} bytes with limit {max_bytes}"
                )
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
        match error {
            vm::Error::HeapLimitExceeded {
                scope,
                used_bytes,
                max_bytes,
            } => RuntimeError::HeapLimitExceeded {
                scope,
                used_bytes,
                max_bytes,
            },
            error => RuntimeError::Vm(Box::new(error)),
        }
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

impl From<heap::HeapLimitError> for Box<RuntimeError> {
    /// Convert one heap limit violation into one runtime error.
    fn from(error: heap::HeapLimitError) -> Self {
        RuntimeError::HeapLimitExceeded {
            scope: error.scope.name().to_string(),
            used_bytes: error.used_bytes,
            max_bytes: error.max_bytes,
        }
        .boxed()
    }
}

impl From<heap::HeapLayoutError> for Box<RuntimeError> {
    /// Convert one heap layout failure into one runtime error.
    fn from(error: heap::HeapLayoutError) -> Self {
        RuntimeError::Internal {
            message: error.to_string(),
        }
        .boxed()
    }
}

impl From<heap::ManagedCollectError> for Box<RuntimeError> {
    /// Convert one managed collection failure into one runtime error.
    fn from(error: heap::ManagedCollectError) -> Self {
        RuntimeError::Internal {
            message: error.to_string(),
        }
        .boxed()
    }
}

impl From<heap::SharedLimitError> for Box<RuntimeError> {
    /// Convert one shared-memory limit violation into one runtime error.
    fn from(error: heap::SharedLimitError) -> Self {
        RuntimeError::HeapLimitExceeded {
            scope: "shared".to_string(),
            used_bytes: error.used_bytes,
            max_bytes: error.max_bytes,
        }
        .boxed()
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
            RuntimeError::HeapLimitExceeded {
                scope,
                used_bytes,
                max_bytes,
            } => vm::Error::HeapLimitExceeded {
                scope,
                used_bytes,
                max_bytes,
            },
            other => vm::Error::Panic {
                message: other.message(),
            },
        }
    }
}

/// Result type for runtime execution.
pub type RuntimeResult<T> = Result<T, Box<RuntimeError>>;
