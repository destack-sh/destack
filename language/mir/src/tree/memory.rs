use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// Set of backing memory spaces that an operation may access.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SpaceSet(
    /// Bitset describing accessible memory spaces.
    u16,
);

impl SpaceSet {
    /// No memory spaces.
    pub const NONE: Self = Self(0);
    /// Local storage.
    pub const LOCAL: Self = Self(1 << 0);
    /// Shared storage.
    pub const SHARED: Self = Self(1 << 1);
    /// Frame storage.
    pub const FRAME: Self = Self(1 << 2);
    /// Static storage.
    pub const STATIC: Self = Self(1 << 3);
    /// All memory spaces.
    pub const ANY: Self = Self(Self::LOCAL.0 | Self::SHARED.0 | Self::FRAME.0 | Self::STATIC.0);

    /// Check if the set is empty.
    pub fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Check if this set contains the other set.
    pub fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// Insert another set of spaces.
    pub fn insert(&mut self, other: Self) {
        self.0 |= other.0;
    }

    /// Return the intersection of two memory space sets.
    pub fn intersection(self, other: Self) -> Self {
        Self(self.0 & other.0)
    }

    /// Check whether two memory space sets intersect.
    pub fn intersects(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }

    /// Check whether two memory space sets are disjoint.
    pub fn is_disjoint(self, other: Self) -> bool {
        self.0 & other.0 == 0
    }
}

impl Default for SpaceSet {
    fn default() -> Self {
        Self::ANY
    }
}

impl TryFrom<&str> for SpaceSet {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "none" => Ok(SpaceSet::NONE),
            "local" => Ok(SpaceSet::LOCAL),
            "shared" => Ok(SpaceSet::SHARED),
            "frame" => Ok(SpaceSet::FRAME),
            "static" => Ok(SpaceSet::STATIC),
            "any" => Ok(SpaceSet::ANY),
            _ => Err(()),
        }
    }
}

/// Memory ordering for atomic operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

/// Synchronization scope for atomic operations and fences.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SyncScope {
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

impl SyncScope {
    /// Text representation for formatting and parsing.
    pub fn to_str(self) -> &'static str {
        match self {
            SyncScope::Invocation => "invocation",
            SyncScope::Subgroup => "subgroup",
            SyncScope::Workgroup => "workgroup",
            SyncScope::Device => "device",
            SyncScope::System => "system",
        }
    }
}

impl fmt::Display for SyncScope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.to_str())
    }
}

impl FromStr for SyncScope {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "invocation" => Ok(SyncScope::Invocation),
            "subgroup" => Ok(SyncScope::Subgroup),
            "workgroup" => Ok(SyncScope::Workgroup),
            "device" => Ok(SyncScope::Device),
            "system" => Ok(SyncScope::System),
            _ => Err(()),
        }
    }
}

impl TryFrom<&str> for SyncScope {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "Invocation" => Ok(SyncScope::Invocation),
            "Subgroup" => Ok(SyncScope::Subgroup),
            "Workgroup" => Ok(SyncScope::Workgroup),
            "Device" => Ok(SyncScope::Device),
            "System" => Ok(SyncScope::System),
            _ => Err(()),
        }
    }
}

/// Memory scope for fences.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MemoryScope {
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

impl MemoryScope {
    /// Text representation for formatting and parsing.
    pub fn to_str(self) -> &'static str {
        match self {
            MemoryScope::Invocation => "invocation",
            MemoryScope::Subgroup => "subgroup",
            MemoryScope::Workgroup => "workgroup",
            MemoryScope::Device => "device",
            MemoryScope::System => "system",
        }
    }
}

impl fmt::Display for MemoryScope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.to_str())
    }
}

impl FromStr for MemoryScope {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "invocation" => Ok(MemoryScope::Invocation),
            "subgroup" => Ok(MemoryScope::Subgroup),
            "workgroup" => Ok(MemoryScope::Workgroup),
            "device" => Ok(MemoryScope::Device),
            "system" => Ok(MemoryScope::System),
            _ => Err(()),
        }
    }
}

impl TryFrom<&str> for MemoryScope {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "Invocation" => Ok(MemoryScope::Invocation),
            "Subgroup" => Ok(MemoryScope::Subgroup),
            "Workgroup" => Ok(MemoryScope::Workgroup),
            "Device" => Ok(MemoryScope::Device),
            "System" => Ok(MemoryScope::System),
            _ => Err(()),
        }
    }
}

/// Memory flags for fences.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MemoryFlags {
    /// The memory spaces affected by the fence.
    pub spaces: SpaceSet,
    /// Whether this makes writes available to other scopes.
    pub makes_available: bool,
    /// Whether this makes writes visible to other scopes.
    pub makes_visible: bool,
}

impl MemoryFlags {
    /// Default memory flags.
    pub const DEFAULT: Self = Self {
        spaces: SpaceSet::ANY,
        makes_available: false,
        makes_visible: false,
    };

    /// Create flags for the provided spaces.
    pub fn new(spaces: SpaceSet) -> Self {
        Self {
            spaces,
            makes_available: false,
            makes_visible: false,
        }
    }

    /// Create flags with explicit predicates.
    pub fn with_flags(spaces: SpaceSet, makes_available: bool, makes_visible: bool) -> Self {
        Self {
            spaces,
            makes_available,
            makes_visible,
        }
    }
}

impl Default for MemoryFlags {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Atomic ordering and scope for one memory operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AtomicAccess {
    /// The memory ordering.
    pub ordering: MemoryOrdering,
    /// The synchronization scope.
    pub scope: SyncScope,
    /// Whether the access must be preserved as a volatile operation.
    pub is_volatile: bool,
}

impl AtomicAccess {
    /// Create one atomic access.
    pub const fn new(ordering: MemoryOrdering, scope: SyncScope, is_volatile: bool) -> Self {
        Self {
            ordering,
            scope,
            is_volatile,
        }
    }

    /// Create one atomic access with system scope and nonvolatile access.
    pub const fn ordered(ordering: MemoryOrdering) -> Self {
        Self::new(ordering, SyncScope::System, false)
    }
}

impl Default for AtomicAccess {
    fn default() -> Self {
        Self::ordered(MemoryOrdering::default())
    }
}

/// Access for one compare exchange operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

/// Fence ordering, scope, and memory visibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FenceAccess {
    /// The memory ordering.
    pub ordering: MemoryOrdering,
    /// The synchronization scope.
    pub scope: SyncScope,
    /// The memory visibility scope.
    pub memory_scope: MemoryScope,
    /// The memory flags.
    pub flags: MemoryFlags,
}

impl FenceAccess {
    /// Create one fence access.
    pub const fn new(
        ordering: MemoryOrdering,
        scope: SyncScope,
        memory_scope: MemoryScope,
        flags: MemoryFlags,
    ) -> Self {
        Self {
            ordering,
            scope,
            memory_scope,
            flags,
        }
    }

    /// Create one fence access with system scope and default flags.
    pub const fn ordered(ordering: MemoryOrdering) -> Self {
        Self::new(
            ordering,
            SyncScope::System,
            MemoryScope::System,
            MemoryFlags::DEFAULT,
        )
    }
}

impl Default for FenceAccess {
    fn default() -> Self {
        Self::ordered(MemoryOrdering::default())
    }
}
