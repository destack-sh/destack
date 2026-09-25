use std::fmt;

use serde::{Deserialize, Serialize};
use tspp_heap as heap;
use tspp_memory as memory;
use tspp_program as program;
use tspp_vm as vm;

use crate::diagnostic::HostError;
use crate::machine::native;

/// Error type for runtime execution failures.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RuntimeError {
    /// VM runtime error bubbled through the runtime boundary.
    Vm(Box<vm::Error>),
    /// Native execution failure.
    Native(Box<native::Error>),
    /// Program operation failure.
    Program(Box<program::Error>),
    /// Host binding failure.
    Host(Box<HostError>),
    /// Binding boundary failure.
    Binding {
        /// Fully qualified binding name.
        name: String,
        /// Binding failure reason.
        reason: BindingError,
    },
    /// Runtime entity failure.
    Entity {
        /// Entity failure reason.
        reason: EntityError,
    },
    /// Runtime execution failure.
    Runtime {
        /// Runtime failure reason.
        reason: RuntimeFailure,
    },
    /// Trace failure.
    Trace {
        /// Trace failure reason.
        reason: TraceFailure,
    },
    /// Runtime machine operation failed.
    Machine {
        /// Machine failure reason.
        reason: MachineError,
    },
    /// Capture, restore, or image failure.
    Capture {
        /// Capture failure reason.
        reason: CaptureError,
    },
    /// Runtime configuration is invalid.
    Configuration {
        /// The configuration scope that failed validation.
        scope: String,
        /// Human-readable validation detail.
        detail: String,
    },
    /// Runtime memory failure.
    Memory {
        /// Memory failure reason.
        reason: MemoryError,
    },
    /// Internal runtime error.
    Internal { message: String },
}

/// Binding boundary failure reason.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BindingError {
    /// Binding name was not found in the binding table.
    NotFound,
    /// Binding call rejected by policy.
    PolicyViolation,
    /// Binding call rejected due to an execution-affinity mismatch.
    AffinityViolation {
        /// Required affinity for this binding.
        affinity: String,
    },
}

/// Runtime entity failure reason.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntityError {
    /// Entity was not found.
    NotFound(Entity),
    /// Entity already exists.
    AlreadyExists(Entity),
    /// Entity is missing from topology.
    MissingTopology(Entity),
    /// Runtime image is missing its declared default worker.
    DefaultWorkerMissing {
        /// Runtime identifier with the invalid default-worker reference.
        runtime_id: u64,
        /// Missing default worker identifier.
        worker_id: u64,
    },
    /// Runtime image contains duplicate worker records.
    DuplicateWorkerImage {
        /// Runtime identifier owning the duplicate worker image.
        runtime_id: u64,
        /// Duplicate worker identifier.
        worker_id: u64,
    },
    /// One revision points at one missing world image.
    RevisionImageMissing {
        /// Revision identifier with the dangling image reference.
        revision_id: u64,
        /// Missing image identifier.
        image_id: u64,
    },
    /// One revision points at one missing trace image.
    RevisionTraceImageMissing {
        /// Revision identifier with the dangling trace-image reference.
        revision_id: u64,
    },
    /// One requested moment does not belong to the active world branch.
    MomentBranchMismatch {
        /// Requested moment branch identifier.
        moment_branch_id: u64,
        /// Active world branch identifier.
        world_branch_id: u64,
    },
}

/// Runtime entity reference.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Entity {
    /// Host or runtime resource.
    Resource {
        /// Resource id or handle.
        resource_id: u64,
        /// Optional resource type or table name.
        resource_kind: Option<String>,
    },
    /// World branch.
    Branch {
        /// Branch identifier.
        branch_id: u64,
    },
    /// World revision.
    Revision {
        /// Revision identifier.
        revision_id: u64,
    },
    /// World image.
    Image {
        /// Image identifier.
        image_id: u64,
    },
    /// World moment.
    Moment {
        /// Branch identifier.
        branch_id: u64,
        /// Trace sequence.
        sequence: u64,
    },
    /// Runtime.
    Runtime {
        /// Runtime identifier.
        runtime_id: u64,
    },
    /// Worker.
    Worker {
        /// Worker identifier.
        worker_id: u64,
    },
}

/// Runtime execution failure reason.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuntimeFailure {
    /// No execution form implements one requested entry.
    EntryUnavailable { entry: String },
    /// Event loop became idle before completing a task.
    EventLoopIdle { task_id: u64 },
    /// Runtime cannot remove its last remaining worker.
    LastWorkerRemoval,
    /// Runtime cannot remove the default worker until a replacement is selected.
    DefaultWorkerRemoval,
    /// World-controlled virtual time cannot advance while host time is active.
    HostTimeAdvance,
    /// Execution stopped outside a stepping or debugging entrypoint.
    ExecutionStopped,
    /// One worker has no debugger-stopped runnable to resume.
    WorkerNotStopped {
        /// Worker that was not stopped.
        worker_id: u64,
    },
    /// Coroutine suspension escaped its language-level owner.
    SuspensionEscaped,
    /// Execution was terminated by a worker handshake.
    Terminated,
}

/// Runtime memory failure reason.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryError {
    /// One runtime memory scope exceeded its configured hard limit.
    LimitExceeded {
        /// The limited memory scope.
        scope: String,
        /// The exact retained bytes currently in use.
        used_bytes: u64,
        /// The configured hard limit in bytes.
        max_bytes: u64,
    },
}

/// Trace failure reason.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TraceFailure {
    /// Trace ended before the requested event.
    Exhausted {
        /// Sequence number of the missing event.
        sequence: u64,
    },
    /// Trace did not match the current execution.
    Mismatch {
        /// Channel or binding name for the mismatch.
        name: String,
    },
    /// Trace payload policy is not supported by a binding.
    PayloadUnsupported {
        /// Binding name for the mismatch.
        name: String,
    },
    /// Trace payload failed to encode.
    EncodeFailed {
        /// Binding name for the failed payload.
        name: String,
    },
    /// Trace payload failed to decode.
    DecodeFailed {
        /// Binding name for the failed payload.
        name: String,
    },
}

/// Capture, restore, or image failure reason.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CaptureError {
    /// Exclusive access is already held.
    ExclusiveAccessHeld,
    /// Shared activity was attempted while exclusive access is held.
    ExclusiveAccessConflict,
    /// Snapshot branch metadata does not match the target world.
    SnapshotBranchMismatch {
        /// Captured snapshot branch identifier.
        snapshot_branch_id: u64,
        /// Active world branch identifier.
        world_branch_id: u64,
    },
    /// Capture failed because one subsystem cannot honestly materialize the requested mode.
    Barrier {
        /// The component that rejected capture.
        component: String,
        /// The requested capture mode.
        mode: String,
        /// Human-readable barrier detail.
        detail: String,
    },
    /// One captured image or snapshot is internally inconsistent.
    InconsistentImage {
        /// Human-readable inconsistency detail.
        detail: String,
    },
}

impl BindingError {
    /// Return the stable runtime error code for this binding failure.
    pub const fn code(&self) -> u16 {
        match self {
            Self::NotFound => 100,
            Self::PolicyViolation => 101,
            Self::AffinityViolation { .. } => 114,
        }
    }

    /// Return a human-readable binding failure message.
    pub fn message(&self, name: &str) -> String {
        match self {
            Self::NotFound => format!("binding not found: {name}"),
            Self::PolicyViolation => format!("binding forbidden by policy: {name}"),
            Self::AffinityViolation { affinity } => {
                format!("binding affinity denied: {name} requires {affinity}")
            }
        }
    }
}

impl EntityError {
    /// Return the stable runtime error code for this entity failure.
    pub const fn code(&self) -> u16 {
        match self {
            Self::NotFound(entity) => entity.not_found_code(),
            Self::AlreadyExists(entity) => entity.already_exists_code(),
            Self::MissingTopology(Entity::Runtime { .. }) => 135,
            Self::MissingTopology(Entity::Worker { .. }) => 136,
            Self::MissingTopology(_) => 126,
            Self::DefaultWorkerMissing { .. } => 131,
            Self::DuplicateWorkerImage { .. } => 140,
            Self::RevisionImageMissing { .. } => 119,
            Self::RevisionTraceImageMissing { .. } => 120,
            Self::MomentBranchMismatch { .. } => 145,
        }
    }

    /// Return a human-readable entity failure message.
    pub fn message(&self) -> String {
        match self {
            Self::NotFound(entity) => format!("{} not found", entity.description()),
            Self::AlreadyExists(entity) => format!("{} already exists", entity.description()),
            Self::MissingTopology(entity) => {
                format!(
                    "{} is not registered in world topology",
                    entity.description()
                )
            }
            Self::DefaultWorkerMissing {
                runtime_id,
                worker_id,
            } => {
                format!("runtime {runtime_id} is missing default worker {worker_id}")
            }
            Self::DuplicateWorkerImage {
                runtime_id,
                worker_id,
            } => {
                format!("runtime {runtime_id} image contains duplicate worker {worker_id}")
            }
            Self::RevisionImageMissing {
                revision_id,
                image_id,
            } => {
                format!("revision {revision_id} is missing image {image_id}")
            }
            Self::RevisionTraceImageMissing { revision_id } => {
                format!("revision {revision_id} is missing trace image")
            }
            Self::MomentBranchMismatch {
                moment_branch_id,
                world_branch_id,
            } => {
                format!(
                    "moment branch mismatch: moment branch {moment_branch_id} does not match world branch {world_branch_id}"
                )
            }
        }
    }
}

impl Entity {
    /// Return the previous stable not-found error code.
    pub const fn not_found_code(&self) -> u16 {
        match self {
            Self::Resource { .. } => 103,
            Self::Branch { .. } => 115,
            Self::Revision { .. } => 116,
            Self::Image { .. } => 118,
            Self::Runtime { .. } => 127,
            Self::Worker { .. } => 128,
            Self::Moment { .. } => 144,
        }
    }

    /// Return the previous stable already-exists error code.
    pub const fn already_exists_code(&self) -> u16 {
        match self {
            Self::Runtime { .. } => 129,
            Self::Worker { .. } => 130,
            _ => 126,
        }
    }

    /// Return a human-readable entity reference.
    pub fn description(&self) -> String {
        match self {
            Self::Resource {
                resource_id,
                resource_kind,
            } => {
                if let Some(resource_kind) = resource_kind {
                    format!("resource {resource_kind} {resource_id}")
                } else {
                    format!("resource {resource_id}")
                }
            }
            Self::Branch { branch_id } => format!("branch {branch_id}"),
            Self::Revision { revision_id } => format!("revision {revision_id}"),
            Self::Image { image_id } => format!("image {image_id}"),
            Self::Moment {
                branch_id,
                sequence,
            } => {
                format!("moment branch {branch_id} at sequence {sequence}")
            }
            Self::Runtime { runtime_id } => format!("runtime {runtime_id}"),
            Self::Worker { worker_id } => format!("worker {worker_id}"),
        }
    }
}

impl RuntimeFailure {
    /// Return the stable runtime error code for this runtime failure.
    pub const fn code(&self) -> u16 {
        match self {
            Self::EntryUnavailable { .. } => 149,
            Self::EventLoopIdle { .. } => 104,
            Self::LastWorkerRemoval => 132,
            Self::DefaultWorkerRemoval => 133,
            Self::HostTimeAdvance => 137,
            Self::ExecutionStopped => 147,
            Self::SuspensionEscaped => 148,
            Self::Terminated => 150,
            Self::WorkerNotStopped { .. } => 151,
        }
    }

    /// Return a human-readable runtime failure message.
    pub fn message(&self) -> String {
        match self {
            Self::EntryUnavailable { entry } => {
                format!("no execution form implements {entry} entry")
            }
            Self::EventLoopIdle { task_id } => {
                format!("event loop idle before completing task {task_id}")
            }
            Self::LastWorkerRemoval => "runtime must keep at least one worker".to_string(),
            Self::DefaultWorkerRemoval => {
                "cannot remove default worker: set a new default worker first".to_string()
            }
            Self::HostTimeAdvance => {
                "cannot advance virtual time while world uses host time".to_string()
            }
            Self::ExecutionStopped => "execution stopped outside a stepping entrypoint".to_string(),
            Self::WorkerNotStopped { worker_id } => {
                format!("worker {worker_id} has no debugger stop to resume")
            }
            Self::SuspensionEscaped => {
                "coroutine suspension escaped its language-level owner".to_string()
            }
            Self::Terminated => "execution terminated by runtime request".to_string(),
        }
    }
}

impl MemoryError {
    /// Return the stable runtime error code for this memory failure.
    pub const fn code(&self) -> u16 {
        match self {
            Self::LimitExceeded { .. } => 146,
        }
    }

    /// Return a human-readable memory failure message.
    pub fn message(&self) -> String {
        match self {
            Self::LimitExceeded {
                scope,
                used_bytes,
                max_bytes,
            } => {
                format!(
                    "{scope} memory limit exceeded: using {used_bytes} bytes with limit {max_bytes}"
                )
            }
        }
    }
}

impl TraceFailure {
    /// Return the stable runtime error code for this trace failure.
    pub const fn code(&self) -> u16 {
        match self {
            Self::Exhausted { .. } => 107,
            Self::Mismatch { .. } => 108,
            Self::PayloadUnsupported { .. } => 109,
            Self::EncodeFailed { .. } => 111,
            Self::DecodeFailed { .. } => 112,
        }
    }

    /// Return a human-readable trace failure message.
    pub fn message(&self) -> String {
        match self {
            Self::Exhausted { sequence } => format!("trace exhausted at {sequence}"),
            Self::Mismatch { name } => format!("trace mismatch for {name}"),
            Self::PayloadUnsupported { name } => {
                format!("trace payload unsupported for {name}")
            }
            Self::EncodeFailed { name } => {
                format!("failed to encode trace payload for {name}")
            }
            Self::DecodeFailed { name } => {
                format!("failed to decode trace payload for {name}")
            }
        }
    }
}

impl CaptureError {
    /// Return the stable runtime error code for this capture failure.
    pub const fn code(&self) -> u16 {
        match self {
            Self::ExclusiveAccessHeld => 122,
            Self::ExclusiveAccessConflict => 123,
            Self::SnapshotBranchMismatch { .. } => 124,
            Self::Barrier { .. } => 125,
            Self::InconsistentImage { .. } => 126,
        }
    }

    /// Return a human-readable capture failure message.
    pub fn message(&self) -> String {
        match self {
            Self::ExclusiveAccessHeld => "world is already under exclusive access".to_string(),
            Self::ExclusiveAccessConflict => {
                "world is under exclusive access for capture or restore".to_string()
            }
            Self::SnapshotBranchMismatch {
                snapshot_branch_id,
                world_branch_id,
            } => {
                format!(
                    "snapshot active branch {snapshot_branch_id} does not match world branch {world_branch_id}"
                )
            }
            Self::Barrier {
                component,
                mode,
                detail,
            } => {
                format!("{component} cannot capture for {mode}: {detail}")
            }
            Self::InconsistentImage { detail } => {
                format!("captured image is inconsistent: {detail}")
            }
        }
    }
}

/// Machine failure reason.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MachineError {
    /// Machine feature is unsupported.
    Unsupported { feature: String },
    /// Destructor reached a runtime stop point.
    DropStopped,
    /// Destructor attempted to suspend.
    DropSuspended,
    /// Destructor completed through coroutine cancellation.
    DropCancelled,
}

impl RuntimeError {
    /// Create one machine operation failure.
    pub const fn machine(reason: MachineError) -> Self {
        Self::Machine { reason }
    }

    /// Return a binding-not-found error.
    pub fn binding_not_found(name: impl Into<String>) -> Self {
        Self::Binding {
            name: name.into(),
            reason: BindingError::NotFound,
        }
    }

    /// Return a binding policy violation.
    pub fn policy_violation(name: impl Into<String>) -> Self {
        Self::Binding {
            name: name.into(),
            reason: BindingError::PolicyViolation,
        }
    }

    /// Return a binding affinity violation.
    pub fn affinity_violation(name: impl Into<String>, affinity: impl Into<String>) -> Self {
        Self::Binding {
            name: name.into(),
            reason: BindingError::AffinityViolation {
                affinity: affinity.into(),
            },
        }
    }

    /// Return a resource-not-found error.
    pub fn resource_not_found(resource_id: u64, resource_kind: Option<String>) -> Self {
        Self::Entity {
            reason: EntityError::NotFound(Entity::Resource {
                resource_id,
                resource_kind,
            }),
        }
    }

    /// Return a branch-not-found error.
    pub fn branch_not_found(branch_id: u64) -> Self {
        Self::Entity {
            reason: EntityError::NotFound(Entity::Branch { branch_id }),
        }
    }

    /// Return a revision-not-found error.
    pub fn revision_not_found(revision_id: u64) -> Self {
        Self::Entity {
            reason: EntityError::NotFound(Entity::Revision { revision_id }),
        }
    }

    /// Return an image-not-found error.
    pub fn image_not_found(image_id: u64) -> Self {
        Self::Entity {
            reason: EntityError::NotFound(Entity::Image { image_id }),
        }
    }

    /// Return a moment-not-found error.
    pub fn moment_not_found(branch_id: u64, sequence: u64) -> Self {
        Self::Entity {
            reason: EntityError::NotFound(Entity::Moment {
                branch_id,
                sequence,
            }),
        }
    }

    /// Return a moment branch mismatch error.
    pub fn moment_branch_mismatch(moment_branch_id: u64, world_branch_id: u64) -> Self {
        Self::Entity {
            reason: EntityError::MomentBranchMismatch {
                moment_branch_id,
                world_branch_id,
            },
        }
    }

    /// Return a runtime-not-found error.
    pub fn runtime_not_found(runtime_id: u64) -> Self {
        Self::Entity {
            reason: EntityError::NotFound(Entity::Runtime { runtime_id }),
        }
    }

    /// Return a worker-not-found error.
    pub fn worker_not_found(worker_id: u64) -> Self {
        Self::Entity {
            reason: EntityError::NotFound(Entity::Worker { worker_id }),
        }
    }

    /// Return a runtime-already-exists error.
    pub fn runtime_already_exists(runtime_id: u64) -> Self {
        Self::Entity {
            reason: EntityError::AlreadyExists(Entity::Runtime { runtime_id }),
        }
    }

    /// Return a worker-already-exists error.
    pub fn worker_already_exists(worker_id: u64) -> Self {
        Self::Entity {
            reason: EntityError::AlreadyExists(Entity::Worker { worker_id }),
        }
    }

    /// Return a missing default worker error.
    pub fn default_worker_missing(runtime_id: u64, worker_id: u64) -> Self {
        Self::Entity {
            reason: EntityError::DefaultWorkerMissing {
                runtime_id,
                worker_id,
            },
        }
    }

    /// Return a missing topology runtime error.
    pub fn topology_runtime_missing(runtime_id: u64) -> Self {
        Self::Entity {
            reason: EntityError::MissingTopology(Entity::Runtime { runtime_id }),
        }
    }

    /// Return a missing topology worker error.
    pub fn topology_worker_missing(worker_id: u64) -> Self {
        Self::Entity {
            reason: EntityError::MissingTopology(Entity::Worker { worker_id }),
        }
    }

    /// Return a duplicate worker image error.
    pub fn duplicate_worker_image(runtime_id: u64, worker_id: u64) -> Self {
        Self::Entity {
            reason: EntityError::DuplicateWorkerImage {
                runtime_id,
                worker_id,
            },
        }
    }

    /// Return a revision image missing error.
    pub fn revision_image_missing(revision_id: u64, image_id: u64) -> Self {
        Self::Entity {
            reason: EntityError::RevisionImageMissing {
                revision_id,
                image_id,
            },
        }
    }

    /// Return a revision trace image missing error.
    pub fn revision_trace_image_missing(revision_id: u64) -> Self {
        Self::Entity {
            reason: EntityError::RevisionTraceImageMissing { revision_id },
        }
    }

    /// Return an event-loop-idle error.
    pub fn event_loop_idle(task_id: u64) -> Self {
        Self::Runtime {
            reason: RuntimeFailure::EventLoopIdle { task_id },
        }
    }

    /// Return an unavailable execution entry error.
    pub fn entry_unavailable(entry: impl Into<String>) -> Self {
        Self::Runtime {
            reason: RuntimeFailure::EntryUnavailable {
                entry: entry.into(),
            },
        }
    }

    /// Return a last-worker-removal error.
    pub fn last_worker_removal() -> Self {
        Self::Runtime {
            reason: RuntimeFailure::LastWorkerRemoval,
        }
    }

    /// Return a default-worker-removal error.
    pub fn default_worker_removal() -> Self {
        Self::Runtime {
            reason: RuntimeFailure::DefaultWorkerRemoval,
        }
    }

    /// Return a host-time-advance error.
    pub fn host_time_advance() -> Self {
        Self::Runtime {
            reason: RuntimeFailure::HostTimeAdvance,
        }
    }

    /// Return an execution-stopped error.
    pub fn execution_stopped() -> Self {
        Self::Runtime {
            reason: RuntimeFailure::ExecutionStopped,
        }
    }

    /// Return a worker-not-stopped error.
    pub fn worker_not_stopped(worker_id: u64) -> Self {
        Self::Runtime {
            reason: RuntimeFailure::WorkerNotStopped { worker_id },
        }
    }

    /// Return an escaped coroutine suspension error.
    pub fn suspension_escaped() -> Self {
        Self::Runtime {
            reason: RuntimeFailure::SuspensionEscaped,
        }
    }

    /// Return an execution-terminated error.
    pub fn execution_terminated() -> Self {
        Self::Runtime {
            reason: RuntimeFailure::Terminated,
        }
    }

    /// Return a trace-exhausted error.
    pub fn trace_exhausted(sequence: u64) -> Self {
        Self::Trace {
            reason: TraceFailure::Exhausted { sequence },
        }
    }

    /// Return a trace mismatch error.
    pub fn trace_mismatch(name: impl Into<String>) -> Self {
        Self::Trace {
            reason: TraceFailure::Mismatch { name: name.into() },
        }
    }

    /// Return a trace payload unsupported error.
    pub fn trace_payload_unsupported(name: impl Into<String>) -> Self {
        Self::Trace {
            reason: TraceFailure::PayloadUnsupported { name: name.into() },
        }
    }

    /// Return a trace encode failure.
    pub fn trace_encode_failed(name: impl Into<String>) -> Self {
        Self::Trace {
            reason: TraceFailure::EncodeFailed { name: name.into() },
        }
    }

    /// Return a trace decode failure.
    pub fn trace_decode_failed(name: impl Into<String>) -> Self {
        Self::Trace {
            reason: TraceFailure::DecodeFailed { name: name.into() },
        }
    }

    /// Return an exclusive access error.
    pub fn exclusive_access_held() -> Self {
        Self::Capture {
            reason: CaptureError::ExclusiveAccessHeld,
        }
    }

    /// Return an exclusive access conflict.
    pub fn exclusive_access_conflict() -> Self {
        Self::Capture {
            reason: CaptureError::ExclusiveAccessConflict,
        }
    }

    /// Return a snapshot branch mismatch.
    pub fn snapshot_branch_mismatch(snapshot_branch_id: u64, world_branch_id: u64) -> Self {
        Self::Capture {
            reason: CaptureError::SnapshotBranchMismatch {
                snapshot_branch_id,
                world_branch_id,
            },
        }
    }

    /// Return a capture barrier.
    pub fn capture_barrier(
        component: impl Into<String>,
        mode: impl Into<String>,
        detail: impl Into<String>,
    ) -> Self {
        Self::Capture {
            reason: CaptureError::Barrier {
                component: component.into(),
                mode: mode.into(),
                detail: detail.into(),
            },
        }
    }

    /// Return an inconsistent image error.
    pub fn inconsistent_image(detail: impl Into<String>) -> Self {
        Self::Capture {
            reason: CaptureError::InconsistentImage {
                detail: detail.into(),
            },
        }
    }

    /// Return a human-readable error message.
    pub fn message(&self) -> String {
        match self {
            RuntimeError::Vm(error) => error.to_string(),
            RuntimeError::Native(error) => error.to_string(),
            RuntimeError::Program(error) => error.to_string(),
            RuntimeError::Host(error) => error.message(),
            RuntimeError::Binding { name, reason } => reason.message(name),
            RuntimeError::Entity { reason } => reason.message(),
            RuntimeError::Runtime { reason } => reason.message(),
            RuntimeError::Trace { reason } => reason.message(),
            RuntimeError::Machine { reason } => reason.message(),
            RuntimeError::Capture { reason } => reason.message(),
            RuntimeError::Configuration { scope, detail } => {
                format!("invalid runtime configuration for {scope}: {detail}")
            }
            RuntimeError::Memory { reason } => reason.message(),
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
        match self {
            Self::Vm(_) => 1,
            Self::Native(_) => 149,
            Self::Program(_) => 148,
            Self::Host(_) => 2,
            Self::Binding { reason, .. } => reason.code(),
            Self::Entity { reason } => reason.code(),
            Self::Runtime { reason } => reason.code(),
            Self::Trace { reason } => reason.code(),
            Self::Internal { .. } => 113,
            Self::Machine { .. } => 138,
            Self::Capture { reason } => reason.code(),
            Self::Memory { reason } => reason.code(),
            Self::Configuration { .. } => 147,
        }
    }

    /// Return a sub-code for status mapping.
    pub fn sub_code(&self) -> u32 {
        match self {
            RuntimeError::Vm(_) => self.code() as u32,
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

impl MachineError {
    /// Return a human-readable machine failure message.
    pub fn message(&self) -> String {
        match self {
            Self::Unsupported { feature } => format!("machine does not support {feature}"),
            Self::DropStopped => "machine stopped while dropping a value".to_string(),
            Self::DropSuspended => "machine suspended while dropping a value".to_string(),
            Self::DropCancelled => "machine cancelled while dropping a value".to_string(),
        }
    }
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message())
    }
}

impl std::error::Error for RuntimeError {
    /// Return the underlying subsystem failure when present.
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Vm(error) => Some(error.as_ref()),
            Self::Native(error) => Some(error.as_ref()),
            Self::Program(error) => Some(error.as_ref()),
            Self::Host(error) => Some(error.as_ref()),
            _ => None,
        }
    }
}

impl From<vm::Error> for RuntimeError {
    /// Preserve one VM execution failure.
    fn from(error: vm::Error) -> Self {
        RuntimeError::Vm(Box::new(error))
    }
}

impl From<vm::Error> for Box<RuntimeError> {
    /// Preserve one VM execution failure.
    fn from(error: vm::Error) -> Self {
        Box::new(RuntimeError::from(error))
    }
}

impl From<native::Error> for RuntimeError {
    /// Preserve one native execution failure.
    fn from(error: native::Error) -> Self {
        Self::Native(Box::new(error))
    }
}

impl From<native::Error> for Box<RuntimeError> {
    /// Preserve one native execution failure.
    fn from(error: native::Error) -> Self {
        RuntimeError::from(error).boxed()
    }
}

impl From<HostError> for Box<RuntimeError> {
    /// Preserve one host operation failure.
    fn from(error: HostError) -> Self {
        Box::new(RuntimeError::from(error))
    }
}

impl From<program::Error> for Box<RuntimeError> {
    /// Preserve one Program operation failure.
    fn from(error: program::Error) -> Self {
        RuntimeError::Program(Box::new(error)).boxed()
    }
}

impl From<heap::HeapError> for Box<RuntimeError> {
    /// Convert one heap failure into one runtime error.
    fn from(error: heap::HeapError) -> Self {
        match error {
            heap::HeapError::Configuration { .. } => RuntimeError::Configuration {
                scope: "heap".into(),
                detail: error.to_string(),
            }
            .boxed(),

            heap::HeapError::LimitExceeded {
                region,
                used_bytes,
                max_bytes,
            } => RuntimeError::Memory {
                reason: MemoryError::LimitExceeded {
                    scope: region.to_string(),
                    used_bytes,
                    max_bytes,
                },
            }
            .boxed(),

            error => RuntimeError::Internal {
                message: error.to_string(),
            }
            .boxed(),
        }
    }
}

impl From<memory::MemoryError> for Box<RuntimeError> {
    /// Convert one world memory failure into a runtime failure.
    fn from(error: memory::MemoryError) -> Self {
        RuntimeError::Internal {
            message: error.to_string(),
        }
        .boxed()
    }
}

impl From<HostError> for RuntimeError {
    /// Preserve one host operation failure.
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

/// Result type for runtime execution.
pub type RuntimeResult<T> = Result<T, Box<RuntimeError>>;
