use std::fmt;

use {destack_heap as heap, destack_vm as vm};

use crate::diagnostic::HostError;

/// Error type for runtime execution and host binding failures.
#[derive(Debug, Clone)]
#[repr(u16)]
pub enum RuntimeError {
    /// VM runtime error bubbled through the runtime boundary.
    Vm(Box<vm::Error>) = 1,
    /// Host binding failure.
    Host(Box<HostError>) = 2,
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
    /// Binding call rejected due to a missing required action.
    ActionDenied {
        /// Fully qualified binding name.
        name: String,
        /// Required action that was not granted.
        action: String,
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
    /// World runtime identifier was not found.
    RuntimeNotFound {
        /// Missing runtime identifier.
        runtime_id: u64,
    } = 127,
    /// Runtime worker identifier was not found.
    WorkerNotFound {
        /// Missing worker identifier.
        worker_id: u64,
    } = 128,
    /// World or runtime already contains the requested runtime identifier.
    RuntimeAlreadyExists {
        /// Conflicting runtime identifier.
        runtime_id: u64,
    } = 129,
    /// Runtime already contains the requested worker identifier.
    WorkerAlreadyExists {
        /// Conflicting worker identifier.
        worker_id: u64,
    } = 130,
    /// Runtime image is missing its declared default worker.
    DefaultWorkerMissing {
        /// Runtime identifier with the invalid default-worker reference.
        runtime_id: u64,
        /// Missing default worker identifier.
        worker_id: u64,
    } = 131,
    /// Runtime cannot remove its last remaining worker.
    LastWorkerRemoval = 132,
    /// Runtime cannot remove the default worker until a replacement is selected.
    DefaultWorkerRemoval = 133,
    /// Runtime action profile configuration is invalid.
    ActionProfileInvalid {
        /// Invalid action profile name.
        profile: String,
        /// Human-readable validation detail.
        detail: String,
    } = 134,
    /// World topology is missing one runtime identity record.
    TopologyRuntimeMissing {
        /// Missing runtime identifier.
        runtime_id: u64,
    } = 135,
    /// World topology is missing one worker identity record.
    TopologyWorkerMissing {
        /// Missing worker identifier.
        worker_id: u64,
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
    /// Engine adapter received an image for the wrong engine kind.
    EngineImageMismatch {
        /// Engine kind that received the image.
        engine: String,
        /// Image kind that was requested.
        image: String,
    } = 149,
    /// Engine backend is not implemented yet.
    EngineUnsupported {
        /// Requested engine kind.
        engine: String,
    } = 148,
    /// Runtime image contains duplicate worker records.
    DuplicateWorkerImage {
        /// Runtime identifier owning the duplicate worker image.
        runtime_id: u64,
        /// Duplicate worker identifier.
        worker_id: u64,
    } = 140,
    /// Runtime configuration is invalid.
    ConfigurationInvalid {
        /// The configuration scope that failed validation.
        scope: String,
        /// Human-readable validation detail.
        detail: String,
    } = 147,
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
            RuntimeError::Host(error) => error.message(),
            RuntimeError::BindingNotFound { name } => format!("binding not found: {name}"),
            RuntimeError::PolicyViolation { name } => {
                format!("binding forbidden by policy: {name}")
            }
            RuntimeError::ActionDenied { name, action } => {
                format!("binding action denied: {name} requires {action}")
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
            RuntimeError::RuntimeNotFound { runtime_id } => {
                format!("runtime not found: {runtime_id}")
            }
            RuntimeError::WorkerNotFound { worker_id } => {
                format!("worker not found: {worker_id}")
            }
            RuntimeError::RuntimeAlreadyExists { runtime_id } => {
                format!("runtime already exists: {runtime_id}")
            }
            RuntimeError::WorkerAlreadyExists { worker_id } => {
                format!("worker already exists: {worker_id}")
            }
            RuntimeError::DefaultWorkerMissing {
                runtime_id,
                worker_id,
            } => {
                format!("runtime {runtime_id} is missing default worker {worker_id}")
            }
            RuntimeError::LastWorkerRemoval => "runtime must keep at least one worker".to_string(),
            RuntimeError::DefaultWorkerRemoval => {
                "cannot remove default worker: set a new default worker first".to_string()
            }
            RuntimeError::ActionProfileInvalid { profile, detail } => {
                format!("runtime action profile `{profile}` is invalid: {detail}")
            }
            RuntimeError::TopologyRuntimeMissing { runtime_id } => {
                format!("runtime {runtime_id} is not registered in world topology")
            }
            RuntimeError::TopologyWorkerMissing { worker_id } => {
                format!("worker {worker_id} is not registered in world topology")
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
            RuntimeError::EngineImageMismatch { engine, image } => {
                format!("{engine} engine cannot restore {image} image")
            }
            RuntimeError::EngineUnsupported { engine } => {
                format!("{engine} engine is not implemented")
            }
            RuntimeError::DuplicateWorkerImage {
                runtime_id,
                worker_id,
            } => {
                format!("runtime {runtime_id} image contains duplicate worker {worker_id}")
            }
            RuntimeError::ConfigurationInvalid { scope, detail } => {
                format!("invalid runtime configuration for {scope}: {detail}")
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
            RuntimeError::Host(error) => error.code.number(),
            _ => self.code() as u32,
        }
    }

    /// Return the host error if this is a host failure.
    pub fn host_error(&self) -> Option<&HostError> {
        match self {
            RuntimeError::Host(error) => Some(error.as_ref()),
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

impl From<HostError> for Box<RuntimeError> {
    fn from(error: HostError) -> Self {
        Box::new(RuntimeError::from(error))
    }
}

impl From<vm::RuntimeError> for Box<RuntimeError> {
    fn from(error: vm::RuntimeError) -> Self {
        Box::new(RuntimeError::from(error.error))
    }
}

impl From<heap::HeapError> for Box<RuntimeError> {
    /// Convert one heap failure into one runtime error.
    fn from(error: heap::HeapError) -> Self {
        match error {
            heap::HeapError::InvalidPageBytes { .. }
            | heap::HeapError::InvalidAllocatorChunkBytes { .. }
            | heap::HeapError::InvalidSpaceBytes { .. }
            | heap::HeapError::MisalignedAllocatorChunkBytes { .. }
            | heap::HeapError::MisalignedSpaceBytes { .. }
            | heap::HeapError::AllocatorPageBytesMismatch { .. }
            | heap::HeapError::AllocatorChunkBytesMismatch { .. }
            | heap::HeapError::InvalidGcTriggerPercent { .. }
            | heap::HeapError::InvalidGcMinimumWorkBytes { .. }
            | heap::HeapError::HeapYoungThresholdExceedsCapacity { .. }
            | heap::HeapError::HeapYoungCapacityTooLarge { .. }
            | heap::HeapError::InvalidSmallAllocationAlignmentBytes { .. }
            | heap::HeapError::EmptySizeClassTable
            | heap::HeapError::ZeroSizeClass
            | heap::HeapError::NonMonotonicSizeClass { .. }
            | heap::HeapError::InvalidSizeClassPolicyRange { .. }
            | heap::HeapError::InvalidSizeClassPolicyAlignment { .. }
            | heap::HeapError::InvalidSizeClassPolicyWaste { .. }
            | heap::HeapError::InvalidSizeClass { .. }
            | heap::HeapError::MisalignedSizeClass { .. }
            | heap::HeapError::SmallSpanTooSmall { .. } => RuntimeError::ConfigurationInvalid {
                scope: "heap".into(),
                detail: error.to_string(),
            }
            .boxed(),

            heap::HeapError::LimitExceeded {
                region,
                used_bytes,
                max_bytes,
            } => RuntimeError::HeapLimitExceeded {
                scope: region.to_string().into(),
                used_bytes,
                max_bytes,
            }
            .boxed(),

            heap::HeapError::TotalLimitExceeded {
                used_bytes,
                max_bytes,
            } => RuntimeError::HeapLimitExceeded {
                scope: "total".into(),
                used_bytes,
                max_bytes,
            }
            .boxed(),

            error => RuntimeError::Internal {
                message: error.to_string(),
            }
            .boxed(),
        }
    }
}

impl From<HostError> for RuntimeError {
    fn from(error: HostError) -> Self {
        RuntimeError::Host(Box::new(error))
    }
}

impl From<&RuntimeError> for HostError {
    fn from(error: &RuntimeError) -> Self {
        if let RuntimeError::Host(error) = error {
            return error.as_ref().clone();
        }

        HostError::generic(None, error.message())
    }
}

impl From<RuntimeError> for HostError {
    fn from(error: RuntimeError) -> Self {
        HostError::from(&error)
    }
}

impl From<Box<RuntimeError>> for vm::Error {
    fn from(error: Box<RuntimeError>) -> Self {
        match *error {
            RuntimeError::Vm(error) => *error,
            RuntimeError::Host(error) => (*error).into(),
            RuntimeError::BindingNotFound { name } => vm::Error::BindingFunctionNotFound { name },
            RuntimeError::PolicyViolation { name } => vm::Error::BindingCallForbidden { name },
            RuntimeError::ActionDenied { name, action } => vm::Error::BindingCallForbidden {
                name: format!("{name} ({action})"),
            },
            RuntimeError::AffinityViolation { name, affinity } => vm::Error::BindingCallForbidden {
                name: format!("{name} ({affinity})"),
            },
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
