use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::ResourceKind;
use crate::world::topology::Topology;
use crate::world::{EdgeKind, EntityKind};

use super::{EdgeSelector, EntitySelector, Rule, RuleAction};

/// Base fault verbs supported by all entity kinds.
const BASE_ENTITY_FAULTS: &[&str] = &[
    "runtime.fault.call.error",
    "runtime.fault.call.timeout",
    "runtime.fault.timing.delay",
    "runtime.fault.call.block",
    "runtime.fault.scheduler.starve",
    "runtime.fault.resource.exhaust",
    "runtime.fault.resource.quota",
];

/// Base fault verbs supported by all edge kinds.
const BASE_EDGE_FAULTS: &[&str] = BASE_ENTITY_FAULTS;

/// Transport fault verbs for stream and link kinds.
const TRANSPORT_FAULTS: &[&str] = &[
    "runtime.fault.transport.drop",
    "runtime.fault.transport.duplicate",
    "runtime.fault.transport.reorder",
    "runtime.fault.transport.corrupt",
    "runtime.fault.transport.truncate",
    "runtime.fault.transport.partial",
    "runtime.fault.transport.disconnect",
    "runtime.fault.transport.reset",
    "runtime.fault.transport.partition",
    "runtime.fault.transport.blackhole",
    "runtime.fault.transport.throttle",
    "runtime.fault.transport.limit",
];

/// Lifecycle fault verbs for process kinds.
const LIFECYCLE_FAULTS: &[&str] = &[
    "runtime.fault.process.crash",
    "runtime.fault.process.restart",
    "runtime.fault.process.reboot",
];

/// Clock fault verbs for time-bearing kinds.
const CLOCK_FAULTS: &[&str] = &[
    "runtime.fault.clock.jump",
    "runtime.fault.clock.drift",
    "runtime.fault.clock.freeze",
];

/// Durability fault verbs for persistent-state kinds.
const DURABILITY_FAULTS: &[&str] = &["runtime.fault.durability.violate"];

/// Jitter distribution for runtime delay faults.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RuntimeJitterDistribution {
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
pub enum FaultCorruptionMode {
    /// Flip one random bit in the payload.
    BitFlip,
    /// Replace all bytes with zero.
    ZeroFill,
    /// Replace bytes with random data.
    RandomBytes,
}

/// Durability-violation mode for storage and write-path faults.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FaultDurabilityMode {
    /// Acknowledge writes before durability barriers complete.
    AckWithoutSync,
    /// Lose one write that was already acknowledged.
    LoseAcknowledgedWrite,
    /// Persist one write only partially.
    TornWrite,
    /// Persist writes in one order different from issued order.
    ReorderPersist,
    /// Persist file payload without required metadata durability.
    MetadataNotDurable,
    /// Report sync success without enforcing durability.
    SyncNoop,
    /// Acknowledge one distributed write before quorum durability.
    AckBeforeQuorumPersist,
    /// Diverge committed state across replicas.
    ReplicaDivergence,
}

/// Resource class for capacity and quota faults.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FaultResourceKind {
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
        | "host.random.stream"
        | "host.tty.device"
        | "runtime.error.channel"
        | "runtime.debug.channel" => TRANSPORT_FAULTS,
        "host.fs.inode" | "host.fs.dentry" | "host.fs.mount" => DURABILITY_FAULTS,
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
        ResourceKind::File | ResourceKind::Directory | ResourceKind::CryptoStore => {
            DURABILITY_FAULTS
        }

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

        ResourceKind::Timer | ResourceKind::TimerFd => CLOCK_FAULTS,

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
#[serde(rename_all = "camelCase")]
pub enum FaultType {
    /// Inject one deterministic error outcome.
    Error {
        /// Error code name for this injected failure.
        code: String,
    },
    /// Inject one timeout outcome.
    Timeout {
        /// Timeout duration in nanoseconds.
        timeout_ns: u64,
        /// Optional timeout error code override.
        code: Option<String>,
    },
    /// Inject deterministic delay.
    Delay {
        /// Base delay in nanoseconds.
        base_ns: u64,
        /// Jitter delay in nanoseconds.
        jitter_ns: Option<u64>,
        /// Delay distribution mode.
        distribution: Option<RuntimeJitterDistribution>,
    },
    /// Inject hard blocking.
    Block {},
    /// Inject starvation.
    Starve {
        /// Optional starvation duration in nanoseconds.
        duration_ns: Option<u64>,
    },
    /// Inject drops.
    Drop {},
    /// Inject duplicates.
    Duplicate {
        /// Number of duplicates emitted when the fault triggers.
        copies: Option<u32>,
    },
    /// Inject reordering.
    Reorder {
        /// Reordering window size.
        window: Option<u32>,
    },
    /// Inject payload corruption.
    Corrupt {
        /// Optional corruption mode selector.
        mode: Option<FaultCorruptionMode>,
    },
    /// Inject truncation.
    Truncate {
        /// Maximum bytes preserved after truncation.
        max_bytes: u64,
    },
    /// Inject partial completion.
    Partial {
        /// Maximum bytes completed before returning.
        max_bytes: u64,
    },
    /// Inject connection disconnects.
    Disconnect {},
    /// Inject connection resets.
    Reset {},
    /// Inject partitions.
    Partition {
        /// Direction selector for one-way or two-way partition.
        direction: Option<FaultDirection>,
    },
    /// Inject blackholes.
    Blackhole {
        /// Direction selector for one-way or two-way blackhole.
        direction: Option<FaultDirection>,
    },
    /// Inject throughput throttling.
    Throttle {
        /// Maximum throughput in bytes per second.
        bytes_per_second: u64,
    },
    /// Inject operation-rate limiting.
    Limit {
        /// Maximum operations per second.
        ops_per_second: u64,
    },
    /// Inject resource exhaustion.
    Exhaust {
        /// Resource class that is exhausted.
        resource: FaultResourceKind,
    },
    /// Inject quota enforcement.
    Quota {
        /// Resource class with quota enforcement.
        resource: FaultResourceKind,
        /// Resource quota limit.
        limit: u64,
    },
    /// Inject process crashes.
    Crash {
        /// Optional crash signal number.
        signal: Option<i64>,
    },
    /// Inject process restarts.
    Restart {
        /// Optional restart delay in nanoseconds.
        delay_ns: Option<u64>,
    },
    /// Inject reboots.
    Reboot {},
    /// Inject one wall or monotonic clock jump.
    ClockJump {
        /// Signed jump delta in nanoseconds.
        delta_ns: i64,
    },
    /// Inject persistent wall or monotonic drift.
    ClockDrift {
        /// Signed rate offset in parts-per-million.
        rate_ppm: i64,
    },
    /// Inject clock freezes.
    ClockFreeze {},
    /// Inject durability violations.
    DurabilityViolation {
        /// Durability violation mode selector.
        mode: FaultDurabilityMode,
    },
}

impl FaultType {
    /// Create one error fault type.
    pub fn error(code: impl Into<String>) -> Self {
        Self::Error { code: code.into() }
    }

    /// Create one timeout fault type.
    pub fn timeout_ns(timeout_ns: u64) -> Self {
        Self::Timeout {
            timeout_ns,
            code: None,
        }
    }

    /// Create one timeout fault type with one explicit code.
    pub fn timeout_with_code(timeout_ns: u64, code: impl Into<String>) -> Self {
        Self::Timeout {
            timeout_ns,
            code: Some(code.into()),
        }
    }

    /// Create one delay fault type.
    pub fn delay_ns(base_ns: u64) -> Self {
        Self::Delay {
            base_ns,
            jitter_ns: None,
            distribution: None,
        }
    }

    /// Create one delay fault type with jitter and distribution.
    pub fn delay_with_jitter(
        base_ns: u64,
        jitter_ns: u64,
        distribution: RuntimeJitterDistribution,
    ) -> Self {
        Self::Delay {
            base_ns,
            jitter_ns: Some(jitter_ns),
            distribution: Some(distribution),
        }
    }

    /// Create one block fault type.
    pub fn block() -> Self {
        Self::Block {}
    }

    /// Create one starvation fault type.
    pub fn starve(duration_ns: Option<u64>) -> Self {
        Self::Starve { duration_ns }
    }

    /// Create one drop fault type.
    pub fn drop() -> Self {
        Self::Drop {}
    }

    /// Create one duplicate fault type.
    pub fn duplicate(copies: Option<u32>) -> Self {
        Self::Duplicate { copies }
    }

    /// Create one reorder fault type.
    pub fn reorder(window: Option<u32>) -> Self {
        Self::Reorder { window }
    }

    /// Create one corruption fault type.
    pub fn corrupt(mode: Option<FaultCorruptionMode>) -> Self {
        Self::Corrupt { mode }
    }

    /// Create one truncation fault type.
    pub fn truncate(max_bytes: u64) -> Self {
        Self::Truncate { max_bytes }
    }

    /// Create one partial-completion fault type.
    pub fn partial(max_bytes: u64) -> Self {
        Self::Partial { max_bytes }
    }

    /// Create one disconnect fault type.
    pub fn disconnect() -> Self {
        Self::Disconnect {}
    }

    /// Create one reset fault type.
    pub fn reset() -> Self {
        Self::Reset {}
    }

    /// Create one partition fault type.
    pub fn partition(direction: Option<FaultDirection>) -> Self {
        Self::Partition { direction }
    }

    /// Create one bidirectional partition fault type.
    pub fn partition_both() -> Self {
        Self::Partition {
            direction: Some(FaultDirection::Both),
        }
    }

    /// Create one blackhole fault type.
    pub fn blackhole(direction: Option<FaultDirection>) -> Self {
        Self::Blackhole { direction }
    }

    /// Create one throughput throttle fault type.
    pub fn throttle(bytes_per_second: u64) -> Self {
        Self::Throttle { bytes_per_second }
    }

    /// Create one operation-rate limit fault type.
    pub fn limit(ops_per_second: u64) -> Self {
        Self::Limit { ops_per_second }
    }

    /// Create one resource exhaustion fault type.
    pub fn exhaust(resource: FaultResourceKind) -> Self {
        Self::Exhaust { resource }
    }

    /// Create one resource quota fault type.
    pub fn quota(resource: FaultResourceKind, limit: u64) -> Self {
        Self::Quota { resource, limit }
    }

    /// Create one process crash fault type.
    pub fn crash(signal: Option<i64>) -> Self {
        Self::Crash { signal }
    }

    /// Create one process restart fault type.
    pub fn restart(delay_ns: Option<u64>) -> Self {
        Self::Restart { delay_ns }
    }

    /// Create one reboot fault type.
    pub fn reboot() -> Self {
        Self::Reboot {}
    }

    /// Create one clock-jump fault type.
    pub fn clock_jump(delta_ns: i64) -> Self {
        Self::ClockJump { delta_ns }
    }

    /// Create one clock-drift fault type.
    pub fn clock_drift(rate_ppm: i64) -> Self {
        Self::ClockDrift { rate_ppm }
    }

    /// Create one clock-freeze fault type.
    pub fn clock_freeze() -> Self {
        Self::ClockFreeze {}
    }

    /// Create one durability-violation fault type.
    pub fn durability_violation(mode: FaultDurabilityMode) -> Self {
        Self::DurabilityViolation { mode }
    }

    /// Create one ack-without-sync durability fault type.
    pub fn ack_without_sync() -> Self {
        Self::DurabilityViolation {
            mode: FaultDurabilityMode::AckWithoutSync,
        }
    }

    /// Create one lose-acknowledged-write durability fault type.
    pub fn lose_acknowledged_write() -> Self {
        Self::DurabilityViolation {
            mode: FaultDurabilityMode::LoseAcknowledgedWrite,
        }
    }

    /// Create one torn-write durability fault type.
    pub fn torn_write() -> Self {
        Self::DurabilityViolation {
            mode: FaultDurabilityMode::TornWrite,
        }
    }

    /// Create one reorder-persist durability fault type.
    pub fn reorder_persist() -> Self {
        Self::DurabilityViolation {
            mode: FaultDurabilityMode::ReorderPersist,
        }
    }

    /// Create one metadata-not-durable fault type.
    pub fn metadata_not_durable() -> Self {
        Self::DurabilityViolation {
            mode: FaultDurabilityMode::MetadataNotDurable,
        }
    }

    /// Create one sync-noop durability fault type.
    pub fn sync_noop() -> Self {
        Self::DurabilityViolation {
            mode: FaultDurabilityMode::SyncNoop,
        }
    }

    /// Create one ack-before-quorum-persist durability fault type.
    pub fn ack_before_quorum_persist() -> Self {
        Self::DurabilityViolation {
            mode: FaultDurabilityMode::AckBeforeQuorumPersist,
        }
    }

    /// Create one replica-divergence durability fault type.
    pub fn replica_divergence() -> Self {
        Self::DurabilityViolation {
            mode: FaultDurabilityMode::ReplicaDivergence,
        }
    }

    /// Return the stable hierarchical identifier for this fault verb.
    pub fn verb_id(&self) -> &'static str {
        match self {
            FaultType::Error { .. } => "runtime.fault.call.error",
            FaultType::Timeout { .. } => "runtime.fault.call.timeout",
            FaultType::Delay { .. } => "runtime.fault.timing.delay",
            FaultType::Block {} => "runtime.fault.call.block",
            FaultType::Starve { .. } => "runtime.fault.scheduler.starve",
            FaultType::Drop {} => "runtime.fault.transport.drop",
            FaultType::Duplicate { .. } => "runtime.fault.transport.duplicate",
            FaultType::Reorder { .. } => "runtime.fault.transport.reorder",
            FaultType::Corrupt { .. } => "runtime.fault.transport.corrupt",
            FaultType::Truncate { .. } => "runtime.fault.transport.truncate",
            FaultType::Partial { .. } => "runtime.fault.transport.partial",
            FaultType::Disconnect {} => "runtime.fault.transport.disconnect",
            FaultType::Reset {} => "runtime.fault.transport.reset",
            FaultType::Partition { .. } => "runtime.fault.transport.partition",
            FaultType::Blackhole { .. } => "runtime.fault.transport.blackhole",
            FaultType::Throttle { .. } => "runtime.fault.transport.throttle",
            FaultType::Limit { .. } => "runtime.fault.transport.limit",
            FaultType::Exhaust { .. } => "runtime.fault.resource.exhaust",
            FaultType::Quota { .. } => "runtime.fault.resource.quota",
            FaultType::Crash { .. } => "runtime.fault.process.crash",
            FaultType::Restart { .. } => "runtime.fault.process.restart",
            FaultType::Reboot {} => "runtime.fault.process.reboot",
            FaultType::ClockJump { .. } => "runtime.fault.clock.jump",
            FaultType::ClockDrift { .. } => "runtime.fault.clock.drift",
            FaultType::ClockFreeze {} => "runtime.fault.clock.freeze",
            FaultType::DurabilityViolation { .. } => "runtime.fault.durability.violate",
        }
    }

    /// Validate that one fault verb supports one target kind.
    fn validate_target(&self, target_kind: FaultTargetKind) -> RuntimeResult<()> {
        let is_valid_target = match (self, target_kind) {
            // call-site interception currently supports direct call outcomes only
            (
                FaultType::Error { .. }
                | FaultType::Timeout { .. }
                | FaultType::Delay { .. }
                | FaultType::Block {},
                FaultTargetKind::Call,
            ) => true,

            // all other verbs require world entity or edge targets
            (_, FaultTargetKind::Call) => false,

            // process, clock, and durability verbs target entities, not relations
            (
                FaultType::Crash { .. }
                | FaultType::Restart { .. }
                | FaultType::Reboot {}
                | FaultType::ClockJump { .. }
                | FaultType::ClockDrift { .. }
                | FaultType::ClockFreeze {}
                | FaultType::DurabilityViolation { .. },
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
            FaultType::Timeout { timeout_ns, .. } if *timeout_ns == 0 => {
                Err(RuntimeError::Internal {
                    message: "fault timeout requires timeout_ns > 0".to_string(),
                }
                .boxed())
            }
            FaultType::Duplicate {
                copies: Some(copies),
            } if *copies == 0 => Err(RuntimeError::Internal {
                message: "fault duplicate requires copies > 0".to_string(),
            }
            .boxed()),
            FaultType::Reorder {
                window: Some(window),
            } if *window == 0 => Err(RuntimeError::Internal {
                message: "fault reorder requires window > 0".to_string(),
            }
            .boxed()),
            FaultType::Throttle { bytes_per_second } if *bytes_per_second == 0 => {
                Err(RuntimeError::Internal {
                    message: "fault throttle requires bytes_per_second > 0".to_string(),
                }
                .boxed())
            }
            FaultType::Limit { ops_per_second } if *ops_per_second == 0 => {
                Err(RuntimeError::Internal {
                    message: "fault rate_limit requires ops_per_second > 0".to_string(),
                }
                .boxed())
            }
            FaultType::Quota { limit, .. } if *limit == 0 => Err(RuntimeError::Internal {
                message: "fault quota requires limit > 0".to_string(),
            }
            .boxed()),
            _ => Ok(()),
        }
    }
}

/// Runtime fault payload for one runtime rule action.
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

/// Topology fault-support lookup used for fault compatibility checks.
pub(crate) trait FaultKindCatalog {
    /// Return one entity kind supported fault set by kind id.
    fn entity_kind_supported_faults(&self, kind: &str) -> Option<&BTreeSet<String>>;
    /// Return one edge kind supported fault set by kind id.
    fn edge_kind_supported_faults(&self, kind: &str) -> Option<&BTreeSet<String>>;
}

impl FaultKindCatalog for Topology {
    fn entity_kind_supported_faults(&self, kind: &str) -> Option<&BTreeSet<String>> {
        self.entity_kind_supported_faults(kind)
    }

    fn edge_kind_supported_faults(&self, kind: &str) -> Option<&BTreeSet<String>> {
        self.edge_kind_supported_faults(kind)
    }
}

/// Validate one fault rule target and fault pair.
pub(crate) fn validate_rule_fault_compatibility(
    rule: &Rule,
    kind_catalog: &(impl FaultKindCatalog + ?Sized),
) -> RuntimeResult<()> {
    let RuleAction::Fault { fault } = &rule.action else {
        return Ok(());
    };

    validate_fault_target_compatible(&fault.target, &fault.fault_type, kind_catalog).map_err(
        |error| {
            RuntimeError::Internal {
                message: format!(
                    "runtime fault is incompatible with target in rule {}: {}",
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
    kind_catalog: &(impl FaultKindCatalog + ?Sized),
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
            let Some(supported_faults) = kind_catalog.entity_kind_supported_faults(kind.as_str())
            else {
                return Err(RuntimeError::Internal {
                    message: format!("unknown topology entity kind: {}", kind.as_str()),
                }
                .boxed());
            };

            ("entity", kind.as_str(), Some(supported_faults))
        }
        FaultTarget::Edge { kind, .. } => {
            let Some(supported_faults) = kind_catalog.edge_kind_supported_faults(kind.as_str())
            else {
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
