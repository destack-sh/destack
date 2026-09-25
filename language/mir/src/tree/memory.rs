use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use tspp_serde::Reflect;

/// Set of backing storage regions that an operation may access.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct StorageSet(
    /// Bitset describing accessible storage regions.
    u16,
);

impl StorageSet {
    /// No storage regions.
    pub const NONE: Self = Self(0);
    /// Local storage.
    pub const LOCAL: Self = Self(1 << 0);
    /// Shared storage.
    pub const SHARED: Self = Self(1 << 1);
    /// Frame storage.
    pub const FRAME: Self = Self(1 << 2);
    /// Global storage.
    pub const GLOBAL: Self = Self(1 << 3);
    /// All storage regions.
    pub const ANY: Self = Self(Self::LOCAL.0 | Self::SHARED.0 | Self::FRAME.0 | Self::GLOBAL.0);

    /// Return whether the set is empty.
    pub fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Return whether this set contains another set.
    pub fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// Insert another storage set.
    pub fn insert(&mut self, other: Self) {
        self.0 |= other.0;
    }

    /// Return the union of two storage sets.
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Return the intersection of two storage sets.
    pub fn intersection(self, other: Self) -> Self {
        Self(self.0 & other.0)
    }

    /// Return whether two storage sets intersect.
    pub fn intersects(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }

    /// Return whether two storage sets are disjoint.
    pub fn is_disjoint(self, other: Self) -> bool {
        self.0 & other.0 == 0
    }

    /// Return whether two storage sets may alias.
    pub fn may_alias(self, other: Self) -> bool {
        !self.is_disjoint(other)
    }
}

impl Default for StorageSet {
    fn default() -> Self {
        Self::ANY
    }
}

impl TryFrom<&str> for StorageSet {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "none" => Ok(StorageSet::NONE),
            "local" => Ok(StorageSet::LOCAL),
            "shared" => Ok(StorageSet::SHARED),
            "frame" => Ok(StorageSet::FRAME),
            "global" => Ok(StorageSet::GLOBAL),
            "any" => Ok(StorageSet::ANY),
            _ => Err(()),
        }
    }
}

/// Memory ordering for atomic operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Reflect)]
pub enum MemoryOrdering {
    /// No ordering constraints (weakest).
    Relaxed,
    /// Acquire ordering prevents reads from moving before this.
    Acquire,
    /// Release ordering prevents writes from moving after this.
    Release,
    /// Acquire and release ordering.
    AcquireRelease,
    /// Sequentially consistent (strongest, default).
    #[default]
    SequentiallyConsistent,
    /// The ordering one template parameter names.
    Parameter(u32),
}

impl MemoryOrdering {
    /// Parse a canonical ordering name.
    pub fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "relaxed" => MemoryOrdering::Relaxed,
            "acquire" => MemoryOrdering::Acquire,
            "release" => MemoryOrdering::Release,
            "acquireRelease" => MemoryOrdering::AcquireRelease,
            "sequentiallyConsistent" => MemoryOrdering::SequentiallyConsistent,
            _ => return None,
        })
    }

    /// Return the ordering at one position of the language enum's declaration order.
    pub fn from_ordinal(ordinal: u32) -> Option<Self> {
        Some(match ordinal {
            0 => MemoryOrdering::Relaxed,
            1 => MemoryOrdering::Acquire,
            2 => MemoryOrdering::Release,
            3 => MemoryOrdering::AcquireRelease,
            4 => MemoryOrdering::SequentiallyConsistent,
            _ => return None,
        })
    }

    /// Return this ordering when closed, or the index of the template parameter it names.
    pub const fn closed(self) -> Result<Self, u32> {
        match self {
            MemoryOrdering::Parameter(parameter) => Err(parameter),
            ordering => Ok(ordering),
        }
    }

    /// Return the canonical MIR text for a closed ordering.
    pub const fn label(self) -> Option<&'static str> {
        Some(match self {
            MemoryOrdering::Relaxed => "relaxed",
            MemoryOrdering::Acquire => "acquire",
            MemoryOrdering::Release => "release",
            MemoryOrdering::AcquireRelease => "acquireRelease",
            MemoryOrdering::SequentiallyConsistent => "sequentiallyConsistent",
            MemoryOrdering::Parameter(_) => return None,
        })
    }
}

/// Read-modify-write operator for atomic memory operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum AtomicRmwOperator {
    /// Swap the memory value with the new value.
    Exchange,
    /// Add and return the old value.
    Add,
    /// Subtract and return the old value.
    Subtract,
    /// Bitwise and and return the old value.
    And,
    /// Bitwise or and return the old value.
    Or,
    /// Bitwise xor and return the old value.
    Xor,
    /// Minimum and return the old value.
    Min,
    /// Maximum and return the old value.
    Max,
}

impl AtomicRmwOperator {
    /// Return the canonical operation name.
    pub const fn name(self) -> &'static str {
        match self {
            AtomicRmwOperator::Exchange => "xchg",
            AtomicRmwOperator::Add => "add",
            AtomicRmwOperator::Subtract => "sub",
            AtomicRmwOperator::And => "and",
            AtomicRmwOperator::Or => "or",
            AtomicRmwOperator::Xor => "xor",
            AtomicRmwOperator::Min => "min",
            AtomicRmwOperator::Max => "max",
        }
    }
}

impl fmt::Display for AtomicRmwOperator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl FromStr for AtomicRmwOperator {
    type Err = ();

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        match text {
            "xchg" => Ok(Self::Exchange),
            "add" => Ok(Self::Add),
            "sub" => Ok(Self::Subtract),
            "and" => Ok(Self::And),
            "or" => Ok(Self::Or),
            "xor" => Ok(Self::Xor),
            "min" => Ok(Self::Min),
            "max" => Ok(Self::Max),
            _ => Err(()),
        }
    }
}

/// Execution scope for atomic operations and fences.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum ExecutionScope {
    /// One invocation or thread.
    Invocation,
    /// One SIMD subgroup or warp.
    Subgroup,
    /// One workgroup or threadgroup.
    Workgroup,
    /// One device.
    Device,
    /// The whole host system.
    #[default]
    System,
}

impl ExecutionScope {
    /// Return the scope at one position of the language enum's declaration order.
    pub fn from_ordinal(ordinal: u32) -> Option<Self> {
        Some(match ordinal {
            0 => ExecutionScope::Invocation,
            1 => ExecutionScope::Subgroup,
            2 => ExecutionScope::Workgroup,
            3 => ExecutionScope::Device,
            7 => ExecutionScope::System,
            _ => return None,
        })
    }

    /// Return the canonical text name.
    pub fn to_str(self) -> &'static str {
        match self {
            ExecutionScope::Invocation => "invocation",
            ExecutionScope::Subgroup => "subgroup",
            ExecutionScope::Workgroup => "workgroup",
            ExecutionScope::Device => "device",
            ExecutionScope::System => "system",
        }
    }
}

impl fmt::Display for ExecutionScope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.to_str())
    }
}

impl FromStr for ExecutionScope {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "invocation" => Ok(ExecutionScope::Invocation),
            "subgroup" => Ok(ExecutionScope::Subgroup),
            "workgroup" => Ok(ExecutionScope::Workgroup),
            "device" => Ok(ExecutionScope::Device),
            "system" => Ok(ExecutionScope::System),
            _ => Err(()),
        }
    }
}

impl TryFrom<&str> for ExecutionScope {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "Invocation" => Ok(ExecutionScope::Invocation),
            "Subgroup" => Ok(ExecutionScope::Subgroup),
            "Workgroup" => Ok(ExecutionScope::Workgroup),
            "Device" => Ok(ExecutionScope::Device),
            "System" => Ok(ExecutionScope::System),
            _ => Err(()),
        }
    }
}

/// Atomic ordering and scope for one memory operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct AtomicAccess {
    /// The memory ordering.
    pub ordering: MemoryOrdering,
    /// The execution scope.
    pub scope: ExecutionScope,
}

impl AtomicAccess {
    /// Create one atomic access.
    pub const fn new(ordering: MemoryOrdering, scope: ExecutionScope) -> Self {
        Self { ordering, scope }
    }

    /// Create one atomic access with system scope.
    pub const fn ordered(ordering: MemoryOrdering) -> Self {
        Self::new(ordering, ExecutionScope::System)
    }
}

impl Default for AtomicAccess {
    fn default() -> Self {
        Self::ordered(MemoryOrdering::default())
    }
}

/// Access for one compare exchange operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct CompareExchangeAccess {
    /// Access used when the comparison succeeds.
    pub success: AtomicAccess,
    /// Explicit failure ordering, or derive from the success ordering.
    pub failure_ordering: Option<MemoryOrdering>,
}

impl CompareExchangeAccess {
    /// Create one compare exchange access.
    pub const fn new(success: AtomicAccess, failure_ordering: Option<MemoryOrdering>) -> Self {
        Self {
            success,
            failure_ordering,
        }
    }

    /// Create one compare exchange access with default failure ordering.
    pub const fn with_success(success: AtomicAccess) -> Self {
        Self::new(success, None)
    }

    /// Create one compare exchange access with default scope and failure ordering.
    pub const fn ordered(ordering: MemoryOrdering) -> Self {
        Self::with_success(AtomicAccess::ordered(ordering))
    }

    /// Return the closed failure ordering, or the index of its unresolved parameter.
    pub fn failure_ordering(self) -> Result<MemoryOrdering, u32> {
        let ordering = match self.failure_ordering {
            Some(ordering) => ordering,
            None => match self.success.ordering {
                MemoryOrdering::Release => MemoryOrdering::Relaxed,
                MemoryOrdering::AcquireRelease => MemoryOrdering::Acquire,
                ordering => ordering,
            },
        };

        ordering.closed()
    }
}

impl Default for CompareExchangeAccess {
    fn default() -> Self {
        Self::ordered(MemoryOrdering::default())
    }
}

/// Fence ordering, scope, and storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct FenceAccess {
    /// The memory ordering.
    pub ordering: MemoryOrdering,
    /// The execution scope.
    pub scope: ExecutionScope,
    /// The ordered storage regions.
    pub storage: StorageSet,
}

impl FenceAccess {
    /// Create one fence access.
    pub const fn new(ordering: MemoryOrdering, scope: ExecutionScope, storage: StorageSet) -> Self {
        Self {
            ordering,
            scope,
            storage,
        }
    }

    /// Create one fence access with system scope and all storage.
    pub const fn ordered(ordering: MemoryOrdering) -> Self {
        Self::new(ordering, ExecutionScope::System, StorageSet::ANY)
    }
}

impl Default for FenceAccess {
    fn default() -> Self {
        Self::ordered(MemoryOrdering::default())
    }
}
