use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// Set of memory regions that an operation may access.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EffectRegionSet(
    /// Bitset describing accessible memory regions.
    u16,
);

impl EffectRegionSet {
    /// No memory regions.
    pub const NONE: Self = Self(0);
    /// Heap allocated memory.
    pub const HEAP: Self = Self(1 << 0);
    /// Raw manually managed heap memory.
    pub const RAW_HEAP: Self = Self(1 << 1);
    /// Stack memory.
    pub const STACK: Self = Self(1 << 2);
    /// Global or static memory.
    pub const GLOBAL: Self = Self(1 << 3);
    /// Shared or workgroup memory.
    pub const SHARED: Self = Self(1 << 4);
    /// Target local or thread local memory.
    pub const LOCAL: Self = Self(1 << 5);
    /// Target constant or read only memory.
    pub const CONSTANT: Self = Self(1 << 6);
    /// Memory mapped IO or other side channel memory.
    pub const IO: Self = Self(1 << 7);
    /// All memory regions.
    pub const ANY: Self = Self(
        Self::HEAP.0
            | Self::RAW_HEAP.0
            | Self::STACK.0
            | Self::GLOBAL.0
            | Self::SHARED.0
            | Self::LOCAL.0
            | Self::CONSTANT.0
            | Self::IO.0,
    );

    /// Check if the set is empty.
    pub fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Check if this set contains the other set.
    pub fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// Insert another set of regions.
    pub fn insert(&mut self, other: Self) {
        self.0 |= other.0;
    }

    /// Return the intersection of two region sets.
    pub fn intersection(self, other: Self) -> Self {
        Self(self.0 & other.0)
    }

    /// Check whether two region sets intersect.
    pub fn intersects(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }

    /// Check whether two region sets are disjoint.
    pub fn is_disjoint(self, other: Self) -> bool {
        self.0 & other.0 == 0
    }
}

impl Default for EffectRegionSet {
    fn default() -> Self {
        Self::ANY
    }
}

impl TryFrom<&str> for EffectRegionSet {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "None" => Ok(EffectRegionSet::NONE),
            "Heap" => Ok(EffectRegionSet::HEAP),
            "RawHeap" => Ok(EffectRegionSet::RAW_HEAP),
            "Stack" => Ok(EffectRegionSet::STACK),
            "Global" => Ok(EffectRegionSet::GLOBAL),
            "Shared" => Ok(EffectRegionSet::SHARED),
            "Local" => Ok(EffectRegionSet::LOCAL),
            "Constant" => Ok(EffectRegionSet::CONSTANT),
            "Io" => Ok(EffectRegionSet::IO),
            "Any" => Ok(EffectRegionSet::ANY),
            _ => Err(()),
        }
    }
}

/// Memory ordering for atomic operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum MemoryOrdering {
    /// No ordering constraints (weakest).
    Relaxed,
    /// Acquire semantics (reads can't be reordered before this).
    Acquire,
    /// Release semantics (writes can't be reordered after this).
    Release,
    /// Both acquire and release semantics.
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

/// Read modify write operator for atomic memory operations.
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

/// Execution scope for atomic operations and barriers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AtomicScope {
    /// Single invocation or thread scope.
    Invocation,
    /// Subgroup or warp scope.
    Subgroup,
    /// Workgroup or threadgroup scope.
    Workgroup,
    /// Device scope.
    Device,
    /// Cross device scope.
    CrossDevice,
    /// Queue family scope.
    QueueFamily,
    /// Shader call group scope.
    ShaderCallGroup,
    /// System scope.
    System,
}

impl AtomicScope {
    /// Text representation for formatting and parsing.
    pub fn to_str(self) -> &'static str {
        match self {
            AtomicScope::Invocation => "invocation",
            AtomicScope::Subgroup => "subgroup",
            AtomicScope::Workgroup => "workgroup",
            AtomicScope::Device => "device",
            AtomicScope::CrossDevice => "crossDevice",
            AtomicScope::QueueFamily => "queueFamily",
            AtomicScope::ShaderCallGroup => "shaderCallGroup",
            AtomicScope::System => "system",
        }
    }
}

impl fmt::Display for AtomicScope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.to_str())
    }
}

impl FromStr for AtomicScope {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "invocation" => Ok(AtomicScope::Invocation),
            "subgroup" => Ok(AtomicScope::Subgroup),
            "workgroup" => Ok(AtomicScope::Workgroup),
            "device" => Ok(AtomicScope::Device),
            "crossDevice" => Ok(AtomicScope::CrossDevice),
            "queueFamily" => Ok(AtomicScope::QueueFamily),
            "shaderCallGroup" => Ok(AtomicScope::ShaderCallGroup),
            "system" => Ok(AtomicScope::System),
            _ => Err(()),
        }
    }
}

impl TryFrom<&str> for AtomicScope {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "Invocation" => Ok(AtomicScope::Invocation),
            "Subgroup" => Ok(AtomicScope::Subgroup),
            "Workgroup" => Ok(AtomicScope::Workgroup),
            "Device" => Ok(AtomicScope::Device),
            "CrossDevice" => Ok(AtomicScope::CrossDevice),
            "QueueFamily" => Ok(AtomicScope::QueueFamily),
            "ShaderCallGroup" => Ok(AtomicScope::ShaderCallGroup),
            "System" => Ok(AtomicScope::System),
            _ => Err(()),
        }
    }
}

/// Memory scope for atomic operations and barriers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MemoryScope {
    /// Single invocation or thread scope.
    Invocation,
    /// Subgroup or warp scope.
    Subgroup,
    /// Workgroup or threadgroup scope.
    Workgroup,
    /// Device scope.
    Device,
    /// Cross device scope.
    CrossDevice,
    /// Queue family scope.
    QueueFamily,
    /// Shader call group scope.
    ShaderCallGroup,
    /// System scope.
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
            MemoryScope::CrossDevice => "crossDevice",
            MemoryScope::QueueFamily => "queueFamily",
            MemoryScope::ShaderCallGroup => "shaderCallGroup",
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
            "crossDevice" => Ok(MemoryScope::CrossDevice),
            "queueFamily" => Ok(MemoryScope::QueueFamily),
            "shaderCallGroup" => Ok(MemoryScope::ShaderCallGroup),
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
            "CrossDevice" => Ok(MemoryScope::CrossDevice),
            "QueueFamily" => Ok(MemoryScope::QueueFamily),
            "ShaderCallGroup" => Ok(MemoryScope::ShaderCallGroup),
            "System" => Ok(MemoryScope::System),
            _ => Err(()),
        }
    }
}

/// Memory semantics for atomics and barriers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MemorySemantics {
    /// Effect regions participating in the synchronization.
    pub regions: EffectRegionSet,
    /// Whether the access is volatile.
    pub is_volatile: bool,
    /// Whether this makes writes available to other scopes.
    pub is_make_available: bool,
    /// Whether this makes writes visible to other scopes.
    pub is_make_visible: bool,
}

impl MemorySemantics {
    /// Create semantics for the provided regions.
    pub fn new(regions: EffectRegionSet) -> Self {
        Self {
            regions,
            is_volatile: false,
            is_make_available: false,
            is_make_visible: false,
        }
    }

    /// Create semantics with explicit flags.
    pub fn with_flags(
        regions: EffectRegionSet,
        is_volatile: bool,
        is_make_available: bool,
        is_make_visible: bool,
    ) -> Self {
        Self {
            regions,
            is_volatile,
            is_make_available,
            is_make_visible,
        }
    }
}

impl Default for MemorySemantics {
    fn default() -> Self {
        Self {
            regions: EffectRegionSet::ANY,
            is_volatile: false,
            is_make_available: false,
            is_make_visible: false,
        }
    }
}
