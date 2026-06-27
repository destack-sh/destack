use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use destack_serde::Reflect;

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
    /// Static storage.
    pub const STATIC: Self = Self(1 << 3);
    /// Device storage.
    pub const DEVICE: Self = Self(1 << 4);
    /// Workgroup storage.
    pub const WORKGROUP: Self = Self(1 << 5);
    /// All storage regions.
    pub const ANY: Self = Self(
        Self::LOCAL.0
            | Self::SHARED.0
            | Self::FRAME.0
            | Self::STATIC.0
            | Self::DEVICE.0
            | Self::WORKGROUP.0,
    );

    /// Check if the set is empty.
    pub fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Check if this set contains the other set.
    pub fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// Insert another storage set.
    pub fn insert(&mut self, other: Self) {
        self.0 |= other.0;
    }

    /// Return the union of two storage sets.
    pub fn union(self, other: Self) -> Self {
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
            "static" => Ok(StorageSet::STATIC),
            "device" => Ok(StorageSet::DEVICE),
            "workgroup" => Ok(StorageSet::WORKGROUP),
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
}

impl MemoryOrdering {
    /// Text representation for formatting and parsing.
    pub fn to_str(self) -> &'static str {
        match self {
            MemoryOrdering::Relaxed => "relaxed",
            MemoryOrdering::Acquire => "acquire",
            MemoryOrdering::Release => "release",
            MemoryOrdering::AcquireRelease => "acquireRelease",
            MemoryOrdering::SequentiallyConsistent => "sequentiallyConsistent",
        }
    }
}

impl fmt::Display for MemoryOrdering {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.to_str())
    }
}

impl FromStr for MemoryOrdering {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "relaxed" => Ok(MemoryOrdering::Relaxed),
            "acquire" => Ok(MemoryOrdering::Acquire),
            "release" => Ok(MemoryOrdering::Release),
            "acquireRelease" => Ok(MemoryOrdering::AcquireRelease),
            "sequentiallyConsistent" => Ok(MemoryOrdering::SequentiallyConsistent),
            _ => Err(()),
        }
    }
}

impl TryFrom<&str> for MemoryOrdering {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "Relaxed" => Ok(MemoryOrdering::Relaxed),
            "Acquire" => Ok(MemoryOrdering::Acquire),
            "Release" => Ok(MemoryOrdering::Release),
            "AcquireRelease" => Ok(MemoryOrdering::AcquireRelease),
            "SequentiallyConsistent" => Ok(MemoryOrdering::SequentiallyConsistent),
            _ => Err(()),
        }
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
    Sub,
    /// Bitwise and and return the old value.
    And,
    /// Bitwise or and return the old value.
    Or,
    /// Bitwise xor and return the old value.
    Xor,
    /// Signed minimum and return the old value.
    Min,
    /// Signed maximum and return the old value.
    Max,
    /// Unsigned minimum and return the old value.
    Umin,
    /// Unsigned maximum and return the old value.
    Umax,
    /// Floating add and return the old value.
    Fadd,
    /// Floating minimum and return the old value.
    Fmin,
    /// Floating maximum and return the old value.
    Fmax,
}

impl AtomicRmwOperator {
    /// Return the canonical text form.
    pub fn to_str(self) -> &'static str {
        match self {
            AtomicRmwOperator::Exchange => "xchg",
            AtomicRmwOperator::Add => "add",
            AtomicRmwOperator::Sub => "sub",
            AtomicRmwOperator::And => "and",
            AtomicRmwOperator::Or => "or",
            AtomicRmwOperator::Xor => "xor",
            AtomicRmwOperator::Min => "min",
            AtomicRmwOperator::Max => "max",
            AtomicRmwOperator::Umin => "umin",
            AtomicRmwOperator::Umax => "umax",
            AtomicRmwOperator::Fadd => "fadd",
            AtomicRmwOperator::Fmin => "fmin",
            AtomicRmwOperator::Fmax => "fmax",
        }
    }

    /// Parse one canonical text form.
    pub fn parse(text: &str) -> Option<Self> {
        Some(match text {
            "xchg" => AtomicRmwOperator::Exchange,
            "add" => AtomicRmwOperator::Add,
            "sub" => AtomicRmwOperator::Sub,
            "and" => AtomicRmwOperator::And,
            "or" => AtomicRmwOperator::Or,
            "xor" => AtomicRmwOperator::Xor,
            "min" => AtomicRmwOperator::Min,
            "max" => AtomicRmwOperator::Max,
            "umin" => AtomicRmwOperator::Umin,
            "umax" => AtomicRmwOperator::Umax,
            "fadd" => AtomicRmwOperator::Fadd,
            "fmin" => AtomicRmwOperator::Fmin,
            "fmax" => AtomicRmwOperator::Fmax,
            _ => return None,
        })
    }
}

impl fmt::Display for AtomicRmwOperator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.to_str())
    }
}

impl FromStr for AtomicRmwOperator {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s).ok_or(())
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
    /// Text representation for formatting and parsing.
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
    /// Ordering used by the failed comparison load.
    pub failure_ordering: MemoryOrdering,
}

impl CompareExchangeAccess {
    /// Create one compare exchange access.
    pub const fn new(success: AtomicAccess, failure_ordering: MemoryOrdering) -> Self {
        Self {
            success,
            failure_ordering,
        }
    }

    /// Create one compare exchange access with default failure ordering.
    pub const fn with_success(success: AtomicAccess) -> Self {
        Self::new(success, Self::default_failure_ordering(success.ordering))
    }

    /// Create one compare exchange access with default scope and failure ordering.
    pub const fn ordered(ordering: MemoryOrdering) -> Self {
        Self::with_success(AtomicAccess::ordered(ordering))
    }

    /// Return the default failure ordering for one success ordering.
    pub const fn default_failure_ordering(success: MemoryOrdering) -> MemoryOrdering {
        match success {
            MemoryOrdering::Release => MemoryOrdering::Relaxed,
            MemoryOrdering::AcquireRelease => MemoryOrdering::Acquire,
            other => other,
        }
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
