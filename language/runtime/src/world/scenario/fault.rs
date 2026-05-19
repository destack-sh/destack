use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::ResourceKind;
use crate::world::policy::{EdgeSelector, EntitySelector};
use crate::world::{EdgeKind, EntityKind, Topology};

use super::FaultRule;

/// Base fault verbs supported by all entity kinds.
const BASE_ENTITY_FAULTS: &[&str] = &[
    "operation.error",
    "operation.timeout",
    "operation.delay",
    "resource.pressure",
    "resource.limit",
];

/// Base fault verbs supported by all edge kinds.
const BASE_EDGE_FAULTS: &[&str] = BASE_ENTITY_FAULTS;

/// Transport fault verbs for stream and link kinds.
const TRANSPORT_FAULTS: &[&str] = &[
    "transport.delay",
    "transport.drop",
    "transport.duplicate",
    "transport.reorder",
    "transport.corrupt",
    "transport.partial",
    "transport.fragment",
    "transport.disconnect",
    "transport.reset",
    "transport.partition",
    "transport.blackhole",
    "transport.limit",
];

/// Lifecycle fault verbs for process kinds.
const LIFECYCLE_FAULTS: &[&str] = &[
    "lifecycle.crash",
    "lifecycle.pause",
    "lifecycle.restart",
    "lifecycle.reboot",
];

/// Clock fault verbs for time-bearing kinds.
const CLOCK_FAULTS: &[&str] = &[
    "clock.jump",
    "clock.drift",
    "clock.freeze",
    "clock.resolution",
    "clock.jitter",
];

/// Randomness fault verbs for entropy-bearing kinds.
const RANDOM_FAULTS: &[&str] = &[
    "random.error",
    "random.repeat",
    "random.zero",
    "random.bias",
];

/// Storage fault verbs for persistent-state kinds.
const STORAGE_FAULTS: &[&str] = &[
    "storage.corrupt",
    "storage.order",
    "storage.durability",
    "storage.consistency",
];

/// Runtime fault payload for one scenario rule.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Fault {
    /// Fault target selector.
    pub target: FaultTarget,
    /// Fault type payload.
    pub fault_type: FaultType,
}

impl Fault {
    /// Create one fault from one target and one fault type.
    pub fn new(target: FaultTarget, fault_type: FaultType) -> Self {
        Self { target, fault_type }
    }
}

/// Stable error code used by scenario faults.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ErrorCode(pub String);

impl ErrorCode {
    /// Create one fault error code.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}

/// Error payload for operation and randomness faults.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FaultError {
    /// Stable error code.
    pub code: ErrorCode,
    /// Optional module-specific error class.
    pub class: Option<String>,
    /// Optional diagnostic message.
    pub message: Option<String>,
}

impl FaultError {
    /// Create one fault error with a stable code.
    pub fn new(code: impl Into<String>) -> Self {
        Self {
            code: ErrorCode::new(code),
            class: None,
            message: None,
        }
    }

    /// Attach an error class.
    pub fn class(mut self, class: impl Into<String>) -> Self {
        self.class = Some(class.into());
        self
    }

    /// Attach a diagnostic message.
    pub fn message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }
}

/// Jitter distribution for runtime delay faults.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum JitterDistribution {
    /// Use a uniform random distribution.
    Uniform,
    /// Use a normal random distribution.
    Normal,
    /// Use an exponential random distribution.
    Exponential,
}

/// Direction selector for directional faults.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FaultDirection {
    /// Affect ingress traffic.
    Ingress,
    /// Affect egress traffic.
    Egress,
    /// Affect both directions.
    Both,
}

/// Data corruption mode for payload faults.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CorruptionMode {
    /// Flip one random bit in the payload.
    BitFlip,
    /// Replace all bytes with zero.
    ZeroFill,
    /// Replace bytes with random data.
    RandomBytes,
    /// Return only a prefix of the payload.
    Truncate,
}

/// Transport limit mode for delivery-shaped faults.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum TransportLimit {
    /// Limit the total transported bytes.
    Data {
        /// Total byte budget.
        bytes: u64,
    },
    /// Limit sustained transport throughput.
    Throughput {
        /// Maximum throughput in bytes per second.
        bytes_per_second: u64,
    },
    /// Limit transport operation rate.
    Rate {
        /// Maximum operations per second.
        ops_per_second: u64,
    },
}

/// Storage corruption target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StorageCorruptionTarget {
    /// Stored payload bytes.
    Data,
    /// Storage metadata.
    Metadata,
    /// Write-ahead journal or log.
    Journal,
    /// Secondary index or derived lookup state.
    Index,
}

/// Storage ordering fault.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StorageOrderFault {
    /// Persist writes in an order different from issue order.
    ReorderPersist,
    /// Expose writes in an order different from issue order.
    ReorderVisibility,
    /// Expose only part of an ordered write sequence.
    TornSequence,
}

/// Storage durability fault.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StorageDurabilityFault {
    /// Acknowledge writes before durability barriers complete.
    AckWithoutSync,
    /// Lose one write that was already acknowledged.
    LoseAcknowledgedWrite,
    /// Report sync success without enforcing durability.
    SyncNoop,
    /// Persist file payload without required metadata durability.
    MetadataNotDurable,
    /// Acknowledge one distributed write before quorum durability.
    AckBeforeQuorumPersist,
}

/// Storage consistency fault.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StorageConsistencyFault {
    /// Return an old value from storage.
    StaleRead,
    /// Hide one entry that should be visible.
    MissingEntry,
    /// Expose one entry that should not be visible.
    ExtraEntry,
    /// Return stale metadata for a visible entry.
    StaleMetadata,
    /// Expose rename as multiple non-atomic steps.
    NonAtomicRename,
    /// Diverge committed state across replicas.
    ReplicaDivergence,
}

/// Resource limit value with explicit units.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ResourceLimit {
    /// Limit a countable resource.
    Count {
        /// Count limit.
        count: u64,
    },
    /// Limit a byte-sized resource.
    Bytes {
        /// Byte limit.
        bytes: u64,
    },
    /// Limit a per-second resource rate.
    Rate {
        /// Maximum units per second.
        per_second: u64,
    },
    /// Limit a proportional resource budget.
    Percent {
        /// Budget in parts per million.
        ppm: u32,
    },
}

/// Resource class for capacity and quota faults.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FaultResource {
    /// Memory budget.
    Memory,
    /// File descriptor budget.
    FileDescriptor,
    /// File handle budget.
    FileHandle,
    /// Socket budget.
    Socket,
    /// Thread budget.
    Thread,
    /// Process budget.
    Process,
    /// Disk space budget.
    DiskSpace,
    /// Inode budget.
    Inode,
    /// Network bandwidth budget.
    Bandwidth,
    /// CPU budget.
    Cpu,
    /// GPU budget.
    Gpu,
    /// Queue depth budget.
    Queue,
    /// Timer budget.
    Timer,
}

/// Fault target selector.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FaultTarget {
    /// Target one binding call site.
    Call {},
    /// Target one simulation-world entity.
    Entity {
        /// Entity kind selector.
        kind: EntityKind,
        /// Entity selector.
        selector: EntitySelector,
    },
    /// Target one simulation-world edge.
    Edge {
        /// Edge kind selector.
        kind: EdgeKind,
        /// Edge selector.
        selector: EdgeSelector,
        /// Optional edge direction selector.
        direction: Option<FaultDirection>,
    },
}

/// Return base fault verbs supported by all entity kinds.
pub(crate) fn base_entity_faults() -> BTreeSet<String> {
    collect_faults(BASE_ENTITY_FAULTS, &[])
}

/// Return base fault verbs supported by all edge kinds.
pub(crate) fn base_edge_faults() -> BTreeSet<String> {
    collect_faults(BASE_EDGE_FAULTS, &[])
}

/// Return fault verbs supported by one builtin entity kind.
pub(crate) fn entity_kind_faults(kind: &str) -> BTreeSet<String> {
    let faults = match kind {
        "runtime.instance" | "host.time.clock" => CLOCK_FAULTS,
        "host.process.instance" => LIFECYCLE_FAULTS,
        "host.io.stream"
        | "host.net.connection"
        | "host.net.resolver"
        | "host.ipc.channel"
        | "host.tty.device"
        | "runtime.error.channel"
        | "runtime.debug.channel" => TRANSPORT_FAULTS,
        "host.random.stream" => RANDOM_FAULTS,
        "host.fs.inode" | "host.fs.dentry" | "host.fs.mount" => STORAGE_FAULTS,
        _ => &[],
    };

    collect_faults(BASE_ENTITY_FAULTS, faults)
}

/// Return fault verbs supported by one builtin edge kind.
pub(crate) fn edge_kind_faults(kind: &str) -> BTreeSet<String> {
    let faults = match kind {
        "host.net.network_link"
        | "host.net.stream_link"
        | "host.net.route"
        | "host.ipc.channel"
        | "host.process.pipe" => TRANSPORT_FAULTS,
        _ => &[],
    };

    collect_faults(BASE_EDGE_FAULTS, faults)
}

/// Return fault verbs supported by one builtin resource kind.
pub(crate) fn resource_kind_faults(resource_kind: ResourceKind) -> BTreeSet<String> {
    let faults = match resource_kind {
        ResourceKind::File | ResourceKind::Directory | ResourceKind::CryptoStore => STORAGE_FAULTS,

        ResourceKind::Socket
        | ResourceKind::Listener
        | ResourceKind::Pipe
        | ResourceKind::Pty
        | ResourceKind::Tty
        | ResourceKind::AudioStream
        | ResourceKind::CameraStream
        | ResourceKind::TlsSession
        | ResourceKind::Signal
        | ResourceKind::SignalFd
        | ResourceKind::MessageQueue
        | ResourceKind::AudioEvent
        | ResourceKind::AccessibilityAction
        | ResourceKind::AccessibilityDocument
        | ResourceKind::MidiEvent
        | ResourceKind::InputMonitor
        | ResourceKind::DisplayBeginFrame
        | ResourceKind::BluetoothAdapterWatch
        | ResourceKind::BluetoothSubscription
        | ResourceKind::CameraWatch
        | ResourceKind::NetworkWatch
        | ResourceKind::SerialWatch
        | ResourceKind::UsbWatch
        | ResourceKind::WindowEvent
        | ResourceKind::DisplayEvent
        | ResourceKind::Watch => TRANSPORT_FAULTS,

        ResourceKind::Process => LIFECYCLE_FAULTS,

        ResourceKind::Timer => CLOCK_FAULTS,

        _ => &[],
    };

    collect_faults(BASE_ENTITY_FAULTS, faults)
}

impl FaultTarget {
    /// Target one binding call site.
    pub fn call() -> Self {
        Self::Call {}
    }

    /// Target one entity selector for one entity kind.
    pub fn entity(kind: impl Into<EntityKind>, selector: EntitySelector) -> Self {
        Self::Entity {
            kind: kind.into(),
            selector,
        }
    }

    /// Target one edge selector for one edge kind.
    pub fn edge(
        kind: impl Into<EdgeKind>,
        selector: EdgeSelector,
        direction: Option<FaultDirection>,
    ) -> Self {
        Self::Edge {
            kind: kind.into(),
            selector,
            direction,
        }
    }
}

/// Return one merged fault set.
fn collect_faults(base_faults: &[&str], extra_faults: &[&str]) -> BTreeSet<String> {
    strings(base_faults).chain(strings(extra_faults)).collect()
}

/// Return one iterator over owned fault strings.
fn strings<'a>(faults: &'a [&'a str]) -> impl Iterator<Item = String> + 'a {
    faults.iter().map(|fault| (*fault).to_string())
}

/// Fault type payload.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all_fields = "camelCase")]
pub enum FaultType {
    /// Inject one deterministic error outcome.
    #[serde(rename = "operation.error")]
    OperationError {
        /// Error payload for the injected failure.
        error: FaultError,
    },
    /// Inject one timeout outcome.
    #[serde(rename = "operation.timeout")]
    OperationTimeout {
        /// Optional timeout duration in nanoseconds.
        after_nanos: Option<u64>,
        /// Optional timeout error override.
        error: Option<FaultError>,
    },
    /// Delay the matched operation.
    #[serde(rename = "operation.delay")]
    OperationDelay {
        /// Base delay in nanoseconds.
        base_nanos: u64,
        /// Jitter delay in nanoseconds.
        jitter_nanos: Option<u64>,
        /// Delay distribution mode.
        distribution: Option<JitterDistribution>,
    },
    /// Delay the matched transport.
    #[serde(rename = "transport.delay")]
    TransportDelay {
        /// Base delay in nanoseconds.
        base_nanos: u64,
        /// Jitter delay in nanoseconds.
        jitter_nanos: Option<u64>,
        /// Delay distribution mode.
        distribution: Option<JitterDistribution>,
    },
    /// Inject drops.
    #[serde(rename = "transport.drop")]
    TransportDrop {},
    /// Inject duplicates.
    #[serde(rename = "transport.duplicate")]
    TransportDuplicate {
        /// Number of duplicates emitted when the fault triggers.
        copies: Option<u32>,
    },
    /// Inject reordering.
    #[serde(rename = "transport.reorder")]
    TransportReorder {
        /// Reordering window size.
        window: Option<u32>,
    },
    /// Inject payload corruption.
    #[serde(rename = "transport.corrupt")]
    TransportCorrupt {
        /// Optional corruption mode selector.
        mode: Option<CorruptionMode>,
    },
    /// Inject partial completion.
    #[serde(rename = "transport.partial")]
    TransportPartial {
        /// Maximum bytes completed before returning.
        max_bytes: u64,
    },
    /// Fragment the matched stream or message payload.
    #[serde(rename = "transport.fragment")]
    TransportFragment {
        /// Average fragment byte count.
        average_bytes: u64,
        /// Optional fragment size variation in bytes.
        variation_bytes: Option<u64>,
        /// Optional delay between fragments in nanoseconds.
        delay_nanos: Option<u64>,
    },
    /// Inject connection disconnects.
    #[serde(rename = "transport.disconnect")]
    TransportDisconnect {},
    /// Inject connection resets.
    #[serde(rename = "transport.reset")]
    TransportReset {
        /// Optional reset delay in nanoseconds.
        delay_nanos: Option<u64>,
    },
    /// Inject partitions.
    #[serde(rename = "transport.partition")]
    TransportPartition {
        /// Direction selector for one-way or two-way partition.
        direction: Option<FaultDirection>,
    },
    /// Inject blackholes.
    #[serde(rename = "transport.blackhole")]
    TransportBlackhole {
        /// Direction selector for one-way or two-way blackhole.
        direction: Option<FaultDirection>,
    },
    /// Inject one transport limit.
    #[serde(rename = "transport.limit")]
    TransportLimit {
        /// Transport limit payload.
        limit: TransportLimit,
    },
    /// Inject resource pressure.
    #[serde(rename = "resource.pressure")]
    ResourcePressure {
        /// Resource class under pressure.
        resource: FaultResource,
        /// Optional pressure level in parts per million.
        level_ppm: Option<u32>,
    },
    /// Inject one resource limit.
    #[serde(rename = "resource.limit")]
    ResourceLimit {
        /// Resource class with limit enforcement.
        resource: FaultResource,
        /// Resource limit.
        limit: ResourceLimit,
    },
    /// Inject lifecycle crashes.
    #[serde(rename = "lifecycle.crash")]
    LifecycleCrash {
        /// Optional crash signal number.
        signal: Option<i64>,
    },
    /// Inject lifecycle pauses.
    #[serde(rename = "lifecycle.pause")]
    LifecyclePause {
        /// Optional pause duration in nanoseconds.
        duration_nanos: Option<u64>,
    },
    /// Inject lifecycle restarts.
    #[serde(rename = "lifecycle.restart")]
    LifecycleRestart {
        /// Optional restart delay in nanoseconds.
        delay_nanos: Option<u64>,
    },
    /// Inject lifecycle reboots.
    #[serde(rename = "lifecycle.reboot")]
    LifecycleReboot {
        /// Optional reboot delay in nanoseconds.
        delay_nanos: Option<u64>,
    },
    /// Inject one wall or monotonic clock jump.
    #[serde(rename = "clock.jump")]
    ClockJump {
        /// Signed jump delta in nanoseconds.
        delta_nanos: i64,
    },
    /// Inject persistent wall or monotonic drift.
    #[serde(rename = "clock.drift")]
    ClockDrift {
        /// Signed rate offset in parts per million.
        rate_ppm: i64,
    },
    /// Inject clock freezes.
    #[serde(rename = "clock.freeze")]
    ClockFreeze {
        /// Optional freeze duration in nanoseconds.
        duration_nanos: Option<u64>,
    },
    /// Adjust clock resolution.
    #[serde(rename = "clock.resolution")]
    ClockResolution {
        /// Clock resolution in nanoseconds.
        resolution_nanos: u64,
    },
    /// Inject clock jitter.
    #[serde(rename = "clock.jitter")]
    ClockJitter {
        /// Jitter in nanoseconds.
        jitter_nanos: u64,
        /// Optional jitter distribution.
        distribution: Option<JitterDistribution>,
    },
    /// Inject random-source errors.
    #[serde(rename = "random.error")]
    RandomError {
        /// Error payload for the injected failure.
        error: FaultError,
    },
    /// Repeat bytes from the random stream.
    #[serde(rename = "random.repeat")]
    RandomRepeat {
        /// Optional repeated byte window.
        window_bytes: Option<u64>,
    },
    /// Replace random bytes with zero.
    #[serde(rename = "random.zero")]
    RandomZero {},
    /// Bias random bits.
    #[serde(rename = "random.bias")]
    RandomBias {
        /// Optional bit index to bias.
        bit: Option<u8>,
        /// Bias probability in parts per million.
        probability_ppm: u32,
    },
    /// Inject storage corruption faults.
    #[serde(rename = "storage.corrupt")]
    StorageCorrupt {
        /// Corruption target.
        target: StorageCorruptionTarget,
        /// Optional corruption mode selector.
        mode: Option<CorruptionMode>,
    },
    /// Inject storage order faults.
    #[serde(rename = "storage.order")]
    StorageOrder {
        /// Storage ordering fault selector.
        fault: StorageOrderFault,
    },
    /// Inject storage durability faults.
    #[serde(rename = "storage.durability")]
    StorageDurability {
        /// Durability fault selector.
        fault: StorageDurabilityFault,
    },
    /// Inject storage consistency faults.
    #[serde(rename = "storage.consistency")]
    StorageConsistency {
        /// Storage consistency fault selector.
        fault: StorageConsistencyFault,
        /// Optional maximum staleness in nanoseconds.
        max_age_nanos: Option<u64>,
    },
}

impl FaultType {
    /// Create one error fault type.
    pub fn error(code: impl Into<String>) -> Self {
        Self::OperationError {
            error: FaultError::new(code),
        }
    }

    /// Create one timeout fault type.
    pub fn timeout_ns(after_nanos: u64) -> Self {
        Self::OperationTimeout {
            after_nanos: Some(after_nanos),
            error: None,
        }
    }

    /// Create one operation delay fault type.
    pub fn delay_ns(base_nanos: u64) -> Self {
        Self::OperationDelay {
            base_nanos,
            jitter_nanos: None,
            distribution: None,
        }
    }

    /// Return the stable hierarchical identifier for this fault verb.
    pub fn verb_id(&self) -> &'static str {
        match self {
            FaultType::OperationError { .. } => "operation.error",
            FaultType::OperationTimeout { .. } => "operation.timeout",
            FaultType::OperationDelay { .. } => "operation.delay",
            FaultType::TransportDelay { .. } => "transport.delay",
            FaultType::TransportDrop {} => "transport.drop",
            FaultType::TransportDuplicate { .. } => "transport.duplicate",
            FaultType::TransportReorder { .. } => "transport.reorder",
            FaultType::TransportCorrupt { .. } => "transport.corrupt",
            FaultType::TransportPartial { .. } => "transport.partial",
            FaultType::TransportFragment { .. } => "transport.fragment",
            FaultType::TransportDisconnect {} => "transport.disconnect",
            FaultType::TransportReset { .. } => "transport.reset",
            FaultType::TransportPartition { .. } => "transport.partition",
            FaultType::TransportBlackhole { .. } => "transport.blackhole",
            FaultType::TransportLimit { .. } => "transport.limit",
            FaultType::ResourcePressure { .. } => "resource.pressure",
            FaultType::ResourceLimit { .. } => "resource.limit",
            FaultType::LifecycleCrash { .. } => "lifecycle.crash",
            FaultType::LifecyclePause { .. } => "lifecycle.pause",
            FaultType::LifecycleRestart { .. } => "lifecycle.restart",
            FaultType::LifecycleReboot { .. } => "lifecycle.reboot",
            FaultType::ClockJump { .. } => "clock.jump",
            FaultType::ClockDrift { .. } => "clock.drift",
            FaultType::ClockFreeze { .. } => "clock.freeze",
            FaultType::ClockResolution { .. } => "clock.resolution",
            FaultType::ClockJitter { .. } => "clock.jitter",
            FaultType::RandomError { .. } => "random.error",
            FaultType::RandomRepeat { .. } => "random.repeat",
            FaultType::RandomZero {} => "random.zero",
            FaultType::RandomBias { .. } => "random.bias",
            FaultType::StorageCorrupt { .. } => "storage.corrupt",
            FaultType::StorageOrder { .. } => "storage.order",
            FaultType::StorageDurability { .. } => "storage.durability",
            FaultType::StorageConsistency { .. } => "storage.consistency",
        }
    }

    /// Validate that one fault verb supports one target kind.
    fn validate_target(&self, target_kind: FaultTargetKind) -> RuntimeResult<()> {
        let is_valid_target = match (self, target_kind) {
            // call-site interception currently supports direct call outcomes only
            (
                FaultType::OperationError { .. }
                | FaultType::OperationTimeout { .. }
                | FaultType::OperationDelay { .. },
                FaultTargetKind::Call,
            ) => true,

            // all other verbs require world entity or edge targets
            (_, FaultTargetKind::Call) => false,

            // process, clock, and durability verbs target entities, not relations
            (
                FaultType::LifecycleCrash { .. }
                | FaultType::LifecyclePause { .. }
                | FaultType::LifecycleRestart { .. }
                | FaultType::LifecycleReboot { .. }
                | FaultType::ClockJump { .. }
                | FaultType::ClockDrift { .. }
                | FaultType::ClockFreeze { .. }
                | FaultType::ClockResolution { .. }
                | FaultType::ClockJitter { .. }
                | FaultType::RandomError { .. }
                | FaultType::RandomRepeat { .. }
                | FaultType::RandomZero {}
                | FaultType::RandomBias { .. }
                | FaultType::StorageCorrupt { .. }
                | FaultType::StorageOrder { .. }
                | FaultType::StorageDurability { .. }
                | FaultType::StorageConsistency { .. },
                FaultTargetKind::Edge,
            ) => false,

            // all remaining verbs can target entities and edges
            (_, FaultTargetKind::Entity | FaultTargetKind::Edge) => true,
        };

        if is_valid_target {
            return Ok(());
        }

        Err(RuntimeError::Internal {
            message: format!(
                "fault verb {} does not support target kind {:?}",
                self.verb_id(),
                target_kind
            ),
        }
        .boxed())
    }

    /// Validate payload invariants for this fault verb.
    fn validate_payload(&self) -> RuntimeResult<()> {
        match self {
            FaultType::OperationTimeout {
                after_nanos: Some(after_nanos),
                ..
            } if *after_nanos == 0 => Err(RuntimeError::Internal {
                message: "fault timeout requires after_nanos > 0".to_string(),
            }
            .boxed()),
            FaultType::OperationDelay { base_nanos, .. }
            | FaultType::TransportDelay { base_nanos, .. }
                if *base_nanos == 0 =>
            {
                Err(RuntimeError::Internal {
                    message: "fault delay requires base_nanos > 0".to_string(),
                }
                .boxed())
            }
            FaultType::TransportDuplicate {
                copies: Some(copies),
            } if *copies == 0 => Err(RuntimeError::Internal {
                message: "fault duplicate requires copies > 0".to_string(),
            }
            .boxed()),
            FaultType::TransportReorder {
                window: Some(window),
            } if *window == 0 => Err(RuntimeError::Internal {
                message: "fault reorder requires window > 0".to_string(),
            }
            .boxed()),
            FaultType::TransportPartial { max_bytes } if *max_bytes == 0 => {
                Err(RuntimeError::Internal {
                    message: "fault byte limit requires max_bytes > 0".to_string(),
                }
                .boxed())
            }
            FaultType::TransportFragment { average_bytes, .. } if *average_bytes == 0 => {
                Err(RuntimeError::Internal {
                    message: "fault fragment requires average_bytes > 0".to_string(),
                }
                .boxed())
            }
            FaultType::TransportLimit {
                limit: TransportLimit::Data { bytes },
            } if *bytes == 0 => Err(RuntimeError::Internal {
                message: "fault transport data limit requires bytes > 0".to_string(),
            }
            .boxed()),
            FaultType::TransportLimit {
                limit: TransportLimit::Throughput { bytes_per_second },
            } if *bytes_per_second == 0 => Err(RuntimeError::Internal {
                message: "fault transport throughput limit requires bytes_per_second > 0"
                    .to_string(),
            }
            .boxed()),
            FaultType::TransportLimit {
                limit: TransportLimit::Rate { ops_per_second },
            } if *ops_per_second == 0 => Err(RuntimeError::Internal {
                message: "fault transport rate limit requires ops_per_second > 0".to_string(),
            }
            .boxed()),
            FaultType::ResourceLimit {
                limit: ResourceLimit::Count { count },
                ..
            } if *count == 0 => Err(RuntimeError::Internal {
                message: "fault resource count limit requires count > 0".to_string(),
            }
            .boxed()),
            FaultType::ResourceLimit {
                limit: ResourceLimit::Bytes { bytes },
                ..
            } if *bytes == 0 => Err(RuntimeError::Internal {
                message: "fault resource byte limit requires bytes > 0".to_string(),
            }
            .boxed()),
            FaultType::ResourceLimit {
                limit: ResourceLimit::Rate { per_second },
                ..
            } if *per_second == 0 => Err(RuntimeError::Internal {
                message: "fault resource rate limit requires per_second > 0".to_string(),
            }
            .boxed()),
            FaultType::ResourceLimit {
                limit: ResourceLimit::Percent { ppm },
                ..
            } if *ppm == 0 || *ppm > 1_000_000 => Err(RuntimeError::Internal {
                message: "fault resource percent limit requires 0 < ppm <= 1000000".to_string(),
            }
            .boxed()),
            FaultType::ClockResolution { resolution_nanos } if *resolution_nanos == 0 => {
                Err(RuntimeError::Internal {
                    message: "fault clock resolution requires resolution_nanos > 0".to_string(),
                }
                .boxed())
            }
            FaultType::ClockJitter { jitter_nanos, .. } if *jitter_nanos == 0 => {
                Err(RuntimeError::Internal {
                    message: "fault clock jitter requires jitter_nanos > 0".to_string(),
                }
                .boxed())
            }
            FaultType::RandomBias {
                probability_ppm, ..
            } if *probability_ppm == 0 || *probability_ppm > 1_000_000 => {
                Err(RuntimeError::Internal {
                    message: "fault random bias requires 0 < probability_ppm <= 1000000"
                        .to_string(),
                }
                .boxed())
            }
            _ => Ok(()),
        }
    }
}

/// Validate one fault rule target and fault pair.
pub(crate) fn validate_fault_rule_compatibility(
    rule: &FaultRule,
    topology: &Topology,
) -> RuntimeResult<()> {
    validate_fault_target_compatible(&rule.fault.target, &rule.fault.fault_type, topology).map_err(
        |error| {
            RuntimeError::Internal {
                message: format!(
                    "runtime fault is incompatible with target in fault rule {}: {}",
                    rule.id.0,
                    error.message()
                ),
            }
            .boxed()
        },
    )
}

/// Target kind for fault compatibility validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum FaultTargetKind {
    /// Call-target fault.
    Call,
    /// Entity-target fault.
    Entity,
    /// Edge-target fault.
    Edge,
}

/// Validate target and fault compatibility against the topology kind catalog.
fn validate_fault_target_compatible(
    target: &FaultTarget,
    fault_type: &FaultType,
    topology: &Topology,
) -> RuntimeResult<()> {
    // validate payload invariants first
    fault_type.validate_payload()?;

    // validate target-kind support before checking kind labels
    let target_kind = match target {
        FaultTarget::Call {} => FaultTargetKind::Call,
        FaultTarget::Entity { .. } => FaultTargetKind::Entity,
        FaultTarget::Edge { .. } => FaultTargetKind::Edge,
    };
    fault_type.validate_target(target_kind)?;

    // call targets can express all fault classes
    if matches!(target, FaultTarget::Call {}) {
        return Ok(());
    }

    let (target_kind, kind, supported_faults) = match target {
        FaultTarget::Call {} => ("call", "call", None),
        FaultTarget::Entity { kind, .. } => {
            let Some(supported_faults) = topology.entity_kind_supported_faults(kind.as_str())
            else {
                return Err(RuntimeError::Internal {
                    message: format!("unknown topology entity kind: {}", kind.as_str()),
                }
                .boxed());
            };

            ("entity", kind.as_str(), Some(supported_faults))
        }
        FaultTarget::Edge { kind, .. } => {
            let Some(supported_faults) = topology.edge_kind_supported_faults(kind.as_str()) else {
                return Err(RuntimeError::Internal {
                    message: format!("unknown topology edge kind: {}", kind.as_str()),
                }
                .boxed());
            };

            ("edge", kind.as_str(), Some(supported_faults))
        }
    };

    let Some(supported_faults) = supported_faults else {
        return Ok(());
    };

    let verb_id = fault_type.verb_id();
    if !supported_faults.contains(verb_id) {
        return Err(RuntimeError::Internal {
            message: format!(
                "runtime fault verb {verb_id} is not supported on {target_kind} kind {kind}",
            ),
        }
        .boxed());
    }

    Ok(())
}
