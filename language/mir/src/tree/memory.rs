use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// Set of memory locations that an operation may access.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MemoryLocationSet(
    /// Bitset describing accessible memory locations.
    u16,
);

impl MemoryLocationSet {
    /// No memory locations.
    pub const NONE: Self = Self(0);
    /// Memory reachable from pointer arguments.
    pub const ARGUMENTS: Self = Self(1 << 0);
    /// Heap allocated memory.
    pub const HEAP: Self = Self(1 << 1);
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
    /// Inaccessible memory that cannot be aliased.
    pub const INACCESSIBLE: Self = Self(1 << 7);
    /// Memory mapped IO or other side channel memory.
    pub const IO: Self = Self(1 << 8);
    /// All memory locations.
    pub const ANY: Self = Self(
        Self::ARGUMENTS.0
            | Self::HEAP.0
            | Self::STACK.0
            | Self::GLOBAL.0
            | Self::SHARED.0
            | Self::LOCAL.0
            | Self::CONSTANT.0
            | Self::INACCESSIBLE.0
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

    /// Insert another set of locations.
    pub fn insert(&mut self, other: Self) {
        self.0 |= other.0;
    }

    /// Return the intersection of two location sets.
    pub fn intersection(self, other: Self) -> Self {
        Self(self.0 & other.0)
    }

    /// Check whether two location sets intersect.
    pub fn intersects(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }

    /// Check whether two location sets are disjoint.
    pub fn is_disjoint(self, other: Self) -> bool {
        self.0 & other.0 == 0
    }
}

impl Default for MemoryLocationSet {
    fn default() -> Self {
        Self::ANY
    }
}

impl TryFrom<&str> for MemoryLocationSet {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "None" => Ok(MemoryLocationSet::NONE),
            "Arguments" => Ok(MemoryLocationSet::ARGUMENTS),
            "Heap" => Ok(MemoryLocationSet::HEAP),
            "Stack" => Ok(MemoryLocationSet::STACK),
            "Global" => Ok(MemoryLocationSet::GLOBAL),
            "Shared" => Ok(MemoryLocationSet::SHARED),
            "Local" => Ok(MemoryLocationSet::LOCAL),
            "Constant" => Ok(MemoryLocationSet::CONSTANT),
            "Inaccessible" => Ok(MemoryLocationSet::INACCESSIBLE),
            "Io" => Ok(MemoryLocationSet::IO),
            "Any" => Ok(MemoryLocationSet::ANY),
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
    AcqRel,
    /// Sequentially consistent (strongest, default).
    #[default]
    SeqCst,
}

impl MemoryOrdering {
    /// Text representation for formatting and parsing.
    pub fn to_str(self) -> &'static str {
        match self {
            MemoryOrdering::Relaxed => "relaxed",
            MemoryOrdering::Acquire => "acquire",
            MemoryOrdering::Release => "release",
            MemoryOrdering::AcqRel => "acq_rel",
            MemoryOrdering::SeqCst => "seq_cst",
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
            "acq_rel" => Ok(MemoryOrdering::AcqRel),
            "seq_cst" => Ok(MemoryOrdering::SeqCst),
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
            "AcqRel" => Ok(MemoryOrdering::AcqRel),
            "SeqCst" => Ok(MemoryOrdering::SeqCst),
            _ => Err(()),
        }
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
            AtomicScope::CrossDevice => "cross_device",
            AtomicScope::QueueFamily => "queue_family",
            AtomicScope::ShaderCallGroup => "shader_call_group",
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
            "cross_device" => Ok(AtomicScope::CrossDevice),
            "queue_family" => Ok(AtomicScope::QueueFamily),
            "shader_call_group" => Ok(AtomicScope::ShaderCallGroup),
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
            MemoryScope::CrossDevice => "cross_device",
            MemoryScope::QueueFamily => "queue_family",
            MemoryScope::ShaderCallGroup => "shader_call_group",
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
            "cross_device" => Ok(MemoryScope::CrossDevice),
            "queue_family" => Ok(MemoryScope::QueueFamily),
            "shader_call_group" => Ok(MemoryScope::ShaderCallGroup),
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
    /// Memory locations participating in the synchronization.
    pub locations: MemoryLocationSet,
    /// Whether the access is volatile.
    pub is_volatile: bool,
    /// Whether this makes writes available to other scopes.
    pub is_make_available: bool,
    /// Whether this makes writes visible to other scopes.
    pub is_make_visible: bool,
}

impl MemorySemantics {
    /// Create semantics for the provided locations.
    pub fn new(locations: MemoryLocationSet) -> Self {
        Self {
            locations,
            is_volatile: false,
            is_make_available: false,
            is_make_visible: false,
        }
    }

    /// Create semantics with explicit flags.
    pub fn with_flags(
        locations: MemoryLocationSet,
        is_volatile: bool,
        is_make_available: bool,
        is_make_visible: bool,
    ) -> Self {
        Self {
            locations,
            is_volatile,
            is_make_available,
            is_make_visible,
        }
    }
}

impl Default for MemorySemantics {
    fn default() -> Self {
        Self {
            locations: MemoryLocationSet::ANY,
            is_volatile: false,
            is_make_available: false,
            is_make_visible: false,
        }
    }
}
