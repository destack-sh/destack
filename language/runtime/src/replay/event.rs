use serde::{Deserialize, Serialize};

use crate::platform::ResourceId;
use crate::platform::bindings::{BindingId, CodecId};
use crate::random::RandomStreamId;
use crate::replay::CheckpointId;
use crate::scheduler::{MicrotaskId, TaskId};

/// Event types recorded for deterministic replay.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReplayEvent {
    /// Scheduler event for task ordering.
    SchedulerEvent(SchedulerEvent),
    /// Time seed or wall clock read.
    TimeEvent(TimeEvent),
    /// Random seed or random bytes.
    RandomEvent(RandomEvent),
    /// External binding call and result.
    BindingCall(BindingCallEvent),
    /// External resource attachment mapping.
    ResourceAttach(ResourceAttachEvent),
    /// Replay checkpoint marker.
    Checkpoint(CheckpointEvent),
}

/// Scheduler event captured for replay.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerEvent {
    /// Subject scheduled by the runtime.
    pub subject: SchedulerSubject,
    /// Scheduler event kind.
    pub kind: SchedulerEventKind,
}

/// Subject identifier for scheduler events.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum SchedulerSubject {
    /// A macrotask scheduled by the event loop.
    Task(TaskId),
    /// A microtask scheduled for Promise jobs.
    Microtask(MicrotaskId),
}

/// Scheduler event kind captured for replay.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum SchedulerEventKind {
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
}

/// Time event captured for replay.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeEvent {
    /// Clock event kind.
    pub kind: TimeEventKind,
    /// Clock value in nanoseconds.
    pub time_nanos: u64,
}

/// Clock event kind captured for replay.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum TimeEventKind {
    /// Virtual clock advanced.
    VirtualTick,
    /// Monotonic clock sample.
    MonotonicSample,
    /// Wall clock read.
    WallClockRead,
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
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum RandomEventKind {
    /// Stream seed or reseed event.
    Seed,
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

/// External resource attachment event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceAttachEvent {
    /// Resource identifier.
    pub resource_id: ResourceId,
}

/// Checkpoint event for snapshots.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointEvent {
    /// Checkpoint identifier.
    pub checkpoint_id: CheckpointId,
}
