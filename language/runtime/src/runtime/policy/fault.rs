use serde::{Deserialize, Serialize};

use crate::diagnostic::{RuntimeError, RuntimeResult};

use super::{Effect, Rule};

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

/// Data corruption mode for data-plane faults.
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

/// Entity class for simulation-world targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RuntimeEntityKind {
    /// Process entity in the core module.
    CoreProcess,
    /// Runtime instance entity in the core module.
    CoreRuntime,
    /// Timer entity in the core module.
    CoreTimer,
    /// Resource entity in the core module.
    CoreResource,
    /// Thread entity in the core module.
    CoreThread,
    /// Stream entity in the core module.
    CoreStream,
    /// Inode entity in the fs module.
    FsInode,
    /// Directory entry entity in the fs module.
    FsDentry,
    /// Open file description entity in the fs module.
    FsOpenFile,
    /// Mount entity in the fs module.
    FsMount,
    /// Filesystem watch entity in the fs module.
    FsWatch,
    /// Network namespace entity in the net module.
    NetNamespace,
    /// Interface entity in the net module.
    NetInterface,
    /// Socket entity in the net module.
    NetSocket,
    /// Listener entity in the net module.
    NetListener,
    /// Connection entity in the net module.
    NetConnection,
    /// Resolver entity in the net module.
    NetResolver,
}

/// Edge class for simulation-world link targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RuntimeEdgeKind {
    /// Parent to child namespace edge in the fs module.
    FsParentChild,
    /// Process file-descriptor binding edge in the fs module.
    FsFdBinding,
    /// Mount attachment edge in the fs module.
    FsMountAttachment,
    /// Network link edge in the net module.
    NetNetworkLink,
    /// Stream transport link edge in the net module.
    NetStreamLink,
    /// Route edge in the net module.
    NetRoute,
    /// IPC channel edge in the ipc module.
    IpcChannel,
    /// Process pipe edge in the process module.
    ProcessPipe,
}

/// Stable identifier for one simulation-world entity.
pub type RuntimeEntityId = String;

/// Stable identifier for one simulation-world edge.
pub type RuntimeEdgeId = String;

/// Label selector for simulation-world entities and edges.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeLabelSelector {
    /// Labels that must all be present on the candidate.
    pub all: Vec<String>,
    /// Labels where at least one must be present on the candidate.
    pub any: Vec<String>,
    /// Labels that must not be present on the candidate.
    pub none: Vec<String>,
}

impl RuntimeLabelSelector {
    /// Return true when no label constraints are configured.
    pub fn is_empty(&self) -> bool {
        self.all.is_empty() && self.any.is_empty() && self.none.is_empty()
    }
}

/// Selector for one simulation-world entity target.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum RuntimeEntitySelector {
    /// Match all entities of this class.
    Any,
    /// Match one entity by stable id.
    Id {
        /// Stable simulation-world entity identifier.
        entity_id: RuntimeEntityId,
    },
    /// Match a fixed set of entities by stable id.
    Ids {
        /// Stable simulation-world entity identifiers.
        entity_ids: Vec<RuntimeEntityId>,
    },
    /// Match entities by label constraints.
    Labels {
        /// Label selector expression.
        labels: RuntimeLabelSelector,
    },
}

/// Selector for one simulation-world edge target.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum RuntimeEdgeSelector {
    /// Match all edges of this class.
    Any,
    /// Match one edge by stable id.
    Id {
        /// Stable simulation-world edge identifier.
        edge_id: RuntimeEdgeId,
    },
    /// Match a fixed set of edges by stable id.
    Ids {
        /// Stable simulation-world edge identifiers.
        edge_ids: Vec<RuntimeEdgeId>,
    },
    /// Match edges incident to one entity selector.
    Incident {
        /// Incident entity selector expression.
        entity: RuntimeEntitySelector,
    },
    /// Match edges by source and destination entity selectors.
    Between {
        /// Source entity selector expression.
        from: RuntimeEntitySelector,
        /// Destination entity selector expression.
        to: RuntimeEntitySelector,
    },
    /// Match edges by label constraints.
    Labels {
        /// Label selector expression.
        labels: RuntimeLabelSelector,
    },
}

/// Fault target selector.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "target", rename_all = "camelCase")]
pub enum FaultTarget {
    /// Target one binding call site.
    Call {},
    /// Target one simulation-world entity.
    Entity {
        /// Entity class selector.
        kind: RuntimeEntityKind,
        /// Entity selector.
        selector: RuntimeEntitySelector,
    },
    /// Target one simulation-world edge.
    Edge {
        /// Edge class selector.
        kind: RuntimeEdgeKind,
        /// Edge selector.
        selector: RuntimeEdgeSelector,
        /// Optional edge direction selector.
        direction: Option<FaultDirection>,
    },
}

/// Fault type payload.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
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
    /// Inject hard blocking behavior.
    Block {},
    /// Inject starvation behavior.
    Starve {
        /// Optional starvation duration in nanoseconds.
        duration_ns: Option<u64>,
    },
    /// Inject drop behavior.
    Drop {},
    /// Inject duplicate behavior.
    Duplicate {
        /// Number of duplicates emitted when the fault triggers.
        copies: Option<u32>,
    },
    /// Inject reorder behavior.
    Reorder {
        /// Reordering window size.
        window: Option<u32>,
    },
    /// Inject payload corruption behavior.
    Corrupt {
        /// Optional corruption mode selector.
        mode: Option<FaultCorruptionMode>,
    },
    /// Inject truncation behavior.
    Truncate {
        /// Maximum bytes preserved after truncation.
        max_bytes: u64,
    },
    /// Inject partial completion behavior.
    Partial {
        /// Maximum bytes completed before returning.
        max_bytes: u64,
    },
    /// Inject connection disconnect behavior.
    Disconnect {},
    /// Inject connection reset behavior.
    Reset {},
    /// Inject partition behavior.
    Partition {
        /// Direction selector for one-way or two-way partition.
        direction: Option<FaultDirection>,
    },
    /// Inject blackhole behavior.
    Blackhole {
        /// Direction selector for one-way or two-way blackhole.
        direction: Option<FaultDirection>,
    },
    /// Inject throughput throttling behavior.
    Throttle {
        /// Maximum throughput in bytes per second.
        bytes_per_second: u64,
    },
    /// Inject operation-rate limiting behavior.
    RateLimit {
        /// Maximum operations per second.
        ops_per_second: u64,
    },
    /// Inject resource exhaustion behavior.
    Exhaust {
        /// Resource class that is exhausted.
        resource: FaultResourceKind,
    },
    /// Inject quota enforcement behavior.
    Quota {
        /// Resource class with quota enforcement.
        resource: FaultResourceKind,
        /// Resource quota limit.
        limit: u64,
    },
    /// Inject process crash behavior.
    Crash {
        /// Optional crash signal number.
        signal: Option<i64>,
    },
    /// Inject process restart behavior.
    Restart {
        /// Optional restart delay in nanoseconds.
        delay_ns: Option<u64>,
    },
    /// Inject reboot behavior.
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
    /// Inject clock freeze behavior.
    ClockFreeze {},
    /// Inject successful acknowledgement without durability.
    AckWithoutDurability {},
}

/// Runtime fault payload for one runtime rule action.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Fault {
    /// Fault target selector.
    pub target: FaultTarget,
    /// Fault type payload.
    pub fault_type: FaultType,
}

/// Validate one fault rule target and fault pair.
pub(crate) fn validate_rule_fault_compatibility(rule: &Rule) -> RuntimeResult<()> {
    let Effect::Fault { fault } = &rule.action else {
        return Ok(());
    };

    let is_compatible = is_fault_target_compatible(&fault.target, &fault.fault_type);
    if is_compatible {
        return Ok(());
    }

    Err(RuntimeError::Internal {
        message: format!(
            "runtime fault is incompatible with target in rule {}",
            rule.id.0
        ),
    }
    .boxed())
}

/// Fault class used by the target-fault compatibility matrix.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum FaultTypeClass {
    /// Generic behavior that applies broadly.
    Generic,
    /// Transport and data-plane behavior.
    Transport,
    /// Process lifecycle behavior.
    ProcessLifecycle,
    /// Virtual clock behavior.
    Clock,
    /// Durability consistency behavior.
    Durability,
}

/// Target class used by the target-fault compatibility matrix.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum FaultTargetClass {
    /// Binding call target.
    Call,
    /// Process entity in the core module.
    EntityCoreProcess,
    /// Time-bearing core entity.
    EntityCoreTime,
    /// Filesystem entity.
    EntityFs,
    /// Network entity.
    EntityNet,
    /// Any other entity.
    EntityOther,
    /// Any edge target.
    Edge,
}

/// Return true when one target can apply one fault type.
fn is_fault_target_compatible(target: &FaultTarget, fault_type: &FaultType) -> bool {
    let target_class = fault_target_class(target);
    let fault_class = fault_type_class(fault_type);
    fault_matrix_allows(target_class, fault_class)
}

/// Return one target class for fault matrix evaluation.
fn fault_target_class(target: &FaultTarget) -> FaultTargetClass {
    match target {
        FaultTarget::Call {} => FaultTargetClass::Call,
        FaultTarget::Entity { kind, .. } => match kind {
            RuntimeEntityKind::CoreProcess => FaultTargetClass::EntityCoreProcess,
            RuntimeEntityKind::CoreRuntime | RuntimeEntityKind::CoreTimer => {
                FaultTargetClass::EntityCoreTime
            }
            RuntimeEntityKind::FsInode
            | RuntimeEntityKind::FsDentry
            | RuntimeEntityKind::FsOpenFile
            | RuntimeEntityKind::FsMount
            | RuntimeEntityKind::FsWatch => FaultTargetClass::EntityFs,
            RuntimeEntityKind::NetNamespace
            | RuntimeEntityKind::NetInterface
            | RuntimeEntityKind::NetSocket
            | RuntimeEntityKind::NetListener
            | RuntimeEntityKind::NetConnection
            | RuntimeEntityKind::NetResolver => FaultTargetClass::EntityNet,
            RuntimeEntityKind::CoreResource
            | RuntimeEntityKind::CoreThread
            | RuntimeEntityKind::CoreStream => FaultTargetClass::EntityOther,
        },
        FaultTarget::Edge { .. } => FaultTargetClass::Edge,
    }
}

/// Return one fault class for fault matrix evaluation.
fn fault_type_class(fault_type: &FaultType) -> FaultTypeClass {
    match fault_type {
        FaultType::Drop {}
        | FaultType::Duplicate { .. }
        | FaultType::Reorder { .. }
        | FaultType::Corrupt { .. }
        | FaultType::Truncate { .. }
        | FaultType::Partial { .. }
        | FaultType::Disconnect {}
        | FaultType::Reset {}
        | FaultType::Partition { .. }
        | FaultType::Blackhole { .. }
        | FaultType::Throttle { .. }
        | FaultType::RateLimit { .. } => FaultTypeClass::Transport,
        FaultType::Crash { .. } | FaultType::Restart { .. } | FaultType::Reboot {} => {
            FaultTypeClass::ProcessLifecycle
        }
        FaultType::ClockJump { .. } | FaultType::ClockDrift { .. } | FaultType::ClockFreeze {} => {
            FaultTypeClass::Clock
        }
        FaultType::AckWithoutDurability {} => FaultTypeClass::Durability,
        FaultType::Error { .. }
        | FaultType::Timeout { .. }
        | FaultType::Delay { .. }
        | FaultType::Block {}
        | FaultType::Starve { .. }
        | FaultType::Exhaust { .. }
        | FaultType::Quota { .. } => FaultTypeClass::Generic,
    }
}

/// Return true when one target class permits one fault class.
fn fault_matrix_allows(target_class: FaultTargetClass, fault_class: FaultTypeClass) -> bool {
    match (target_class, fault_class) {
        // calls can express all faults
        (FaultTargetClass::Call, _) => true,

        // process entities are the only process-lifecycle targets
        (FaultTargetClass::EntityCoreProcess, FaultTypeClass::ProcessLifecycle) => true,

        // time entities are the only clock-fault targets
        (FaultTargetClass::EntityCoreTime, FaultTypeClass::Clock) => true,

        // fs entities own durability faults
        (FaultTargetClass::EntityFs, FaultTypeClass::Durability) => true,

        // net entities own transport faults
        (FaultTargetClass::EntityNet, FaultTypeClass::Transport) => true,

        // generic faults apply broadly across entities
        (
            FaultTargetClass::EntityCoreProcess
            | FaultTargetClass::EntityCoreTime
            | FaultTargetClass::EntityFs
            | FaultTargetClass::EntityNet
            | FaultTargetClass::EntityOther,
            FaultTypeClass::Generic,
        ) => true,

        // edges can express generic and transport semantics
        (FaultTargetClass::Edge, FaultTypeClass::Generic | FaultTypeClass::Transport) => true,

        // all other pairs are invalid
        _ => false,
    }
}
