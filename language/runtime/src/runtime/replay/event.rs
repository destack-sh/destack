use serde::{Deserialize, Serialize};

use crate::runtime::bindings::{BindingId, CodecId};
use crate::runtime::random::RandomStreamId;
use crate::runtime::scheduler::{MicrotaskId, TaskId};

/// Event types recorded for deterministic replay.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReplayEvent {
    /// Task queue event for task ordering.
    TaskQueueEvent(TaskQueueEvent),
    /// Time seed or wall clock read.
    TimeEvent(TimeEvent),
    /// Random seed or random bytes.
    RandomEvent(RandomEvent),
    /// External binding call and result.
    BindingCall(BindingCallEvent),
    /// Application of one world command.
    WorldCommand {
        /// Encoded world command payload.
        payload: Vec<u8>,
    },
    /// Checkpoint marker for snapshot references.
    Checkpoint(CheckpointEvent),
    /// Branch marker for replaying from checkpoints.
    Branch(BranchEvent),
    /// Debug or profiling marker.
    Debug(DebugEvent),
}

/// Sequence number for events within a replay log.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LogSequence(u64);

impl LogSequence {
    /// Create a new log sequence number.
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Return the raw sequence number.
    pub const fn get(self) -> u64 {
        self.0
    }

    /// Return the next sequence number.
    pub const fn next(self) -> Self {
        Self(self.0 + 1)
    }
}

/// Identifier for a replay branch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BranchId(u128);

impl BranchId {
    /// Create a new branch identifier.
    pub const fn new(value: u128) -> Self {
        Self(value)
    }

    /// Return the raw branch identifier value.
    pub const fn get(self) -> u128 {
        self.0
    }
}

/// Identifier for a replay checkpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CheckpointId(u128);

impl CheckpointId {
    /// Create a new checkpoint identifier.
    pub const fn new(value: u128) -> Self {
        Self(value)
    }

    /// Return the raw checkpoint identifier value.
    pub const fn get(self) -> u128 {
        self.0
    }
}

/// Task queue event captured for replay.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskQueueEvent {
    /// Subject scheduled by the runtime.
    pub subject: TaskSubject,
    /// Task queue for the event.
    pub queue: TaskQueue,
    /// Task queue event kind.
    pub kind: QueueEventKind,
    /// Monotonic sequence counter for ordering.
    pub sequence: u64,
}

/// Subject identifier for task queue events.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum TaskSubject {
    /// A macrotask scheduled by the event loop.
    Task(TaskId),
    /// A microtask scheduled for Promise jobs.
    Microtask(MicrotaskId),
}

/// Task queues for runtime execution.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum TaskQueue {
    /// Microtask queue drained between macrotasks.
    Microtask,
    /// Primary macrotask queue.
    Macrotask,
    /// Timer queue for delayed callbacks.
    Timer,
    /// I/O readiness queue.
    Io,
    /// Immediate or next-tick queue.
    Immediate,
    /// Idle queue for low-priority work.
    Idle,
}

/// Task queue event kinds captured for replay.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum QueueEventKind {
    /// Enqueued for execution.
    Enqueue,
    /// Dequeued for execution.
    Dequeue,
    /// Yielded while waiting.
    Yield,
    /// Resumed after a wait.
    Resume,
    /// Completed execution.
    Complete,
    /// Drained microtask queue.
    DrainMicrotasks,
    /// Woken by an external event.
    ExternalWake(ExternalWakeSource),
}

/// External wakeup sources for task queue events.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ExternalWakeSource {
    /// Timer fired.
    Timer,
    /// I/O readiness.
    Io,
    /// Signal delivery.
    Signal,
    /// Process exit or status change.
    Process,
}

/// Time event captured for replay.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeEvent {
    /// Clock event kind.
    pub kind: TimeEventKind,
    /// Clock value in nanoseconds.
    pub time_nanos: u64,
    /// Optional interval or period in nanoseconds.
    pub interval_nanos: Option<u64>,
    /// Optional timer identifier.
    pub timer_id: Option<u64>,
}

/// Clock event kind captured for replay.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimeEventKind {
    /// Virtual clock seed or reset.
    Seed,
    /// Monotonic clock sample.
    MonotonicSample,
    /// Wall clock read.
    WallClockRead,
    /// Timer scheduled.
    TimerScheduled,
    /// Timer fired.
    TimerFired,
    /// Timer canceled.
    TimerCanceled,
    /// Sleep scheduled by the runtime.
    SleepScheduled,
    /// Sleep completed by the runtime.
    SleepWake,
}

/// Random event captured for replay.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RandomEvent {
    /// Random stream identifier.
    pub stream_id: RandomStreamId,
    /// Random event kind.
    pub kind: RandomEventKind,
    /// Random payload bytes.
    pub bytes: Vec<u8>,
}

/// Random event kind captured for replay.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RandomEventKind {
    /// Stream seed or reseed event.
    Seed,
    /// Stream allocation event.
    Stream,
    /// Random bytes produced by the stream.
    Bytes,
    /// Random u64 produced by the stream.
    NextU64,
}

/// External binding call event captured for replay.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BindingCallEvent {
    /// Binding identifier from the registry.
    pub binding_id: BindingId,
    /// Codec identifier for the payload.
    pub codec: CodecId,
    /// Encoded payload for replay.
    pub payload: Vec<u8>,
}

/// Checkpoint event for snapshot references.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointEvent {
    /// Checkpoint identifier.
    pub checkpoint_id: CheckpointId,
    /// Branch identifier associated with the snapshot.
    pub branch_id: BranchId,
}

/// Branch event for replay fork points.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchEvent {
    /// Branch identifier.
    pub branch_id: BranchId,
    /// Parent branch identifier.
    pub parent_id: BranchId,
    /// Source checkpoint identifier.
    pub checkpoint_id: CheckpointId,
}

/// Debug event payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebugEvent {
    /// Debug message or marker.
    pub message: String,
}
