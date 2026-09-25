use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

use crate::{GlobalId, GlobalLocation};

use super::{MemoryAccess, MemorySite, ProgramPoint, TypeId};

/// Runtime watchpoint identifier.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct WatchpointId(u64);

/// Memory target selected by one watchpoint or probe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum MemoryTarget {
    /// Any memory target.
    Any,
    /// Program memory operation point.
    Point(ProgramPoint),
    /// Program type.
    Type(TypeId),
    /// Executed memory byte range.
    Range(MemoryRange),
}

/// Executed memory byte range.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum MemoryRange {
    /// Local heap storage range.
    LocalHeap(ByteRange),
    /// Shared heap storage range.
    SharedHeap(ByteRange),
    /// Native address range.
    Address(ByteRange),
    /// Frame value range.
    Frame(ByteRange),
    /// Program global range.
    Global(GlobalRange),
}

/// Contiguous byte range inside one memory owner.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct ByteRange {
    /// Owner-relative byte start.
    pub start: u64,
    /// Byte length.
    pub byte_len: u64,
}

/// Contiguous byte range inside one program global.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct GlobalRange {
    /// Addressed program global.
    pub global: GlobalId,
    /// Static storage location containing the global.
    pub location: GlobalLocation,
    /// Byte range inside the global.
    pub range: ByteRange,
}

/// One program memory stop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryStop {
    /// Watchpoint that owns this stop.
    pub watchpoint_id: WatchpointId,
    /// Memory operation selected by this stop.
    pub access: MemoryAccess,
    /// Memory target selected by this stop.
    pub target: MemoryTarget,
}

/// Active program watchpoints.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WatchSet {
    /// Active memory stops.
    memory: Vec<MemoryStop>,
    /// Whether any memory stop needs executed byte ranges.
    requires_memory_range: bool,
}

impl WatchpointId {
    /// Create one watchpoint identifier.
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Return the raw watchpoint identifier value.
    pub const fn get(self) -> u64 {
        self.0
    }
}

impl ByteRange {
    /// Create one byte range.
    pub const fn new(start: u64, byte_len: u64) -> Self {
        Self { start, byte_len }
    }

    /// Return the exclusive byte end.
    pub const fn end(self) -> u64 {
        self.start + self.byte_len
    }

    /// Return whether this range overlaps another range.
    pub fn overlaps(self, other: Self) -> bool {
        self.start < other.end() && other.start < self.end()
    }
}

impl GlobalRange {
    /// Create one global byte range.
    pub const fn new(
        global: GlobalId,
        location: GlobalLocation,
        start: u64,
        byte_len: u64,
    ) -> Self {
        Self {
            global,
            location,
            range: ByteRange::new(start, byte_len),
        }
    }

    /// Return whether this range overlaps another global range.
    pub fn overlaps(self, other: Self) -> bool {
        self.global == other.global
            && self.location == other.location
            && self.range.overlaps(other.range)
    }
}

impl MemoryRange {
    /// Create one local heap range.
    pub const fn local_heap(start: u64, byte_len: u64) -> Self {
        Self::LocalHeap(ByteRange::new(start, byte_len))
    }

    /// Create one shared heap range.
    pub const fn shared_heap(start: u64, byte_len: u64) -> Self {
        Self::SharedHeap(ByteRange::new(start, byte_len))
    }

    /// Create one native address range.
    pub const fn address(start: u64, byte_len: u64) -> Self {
        Self::Address(ByteRange::new(start, byte_len))
    }

    /// Create one frame value range.
    pub const fn frame(start: u64, byte_len: u64) -> Self {
        Self::Frame(ByteRange::new(start, byte_len))
    }

    /// Create one program global range.
    pub const fn global(
        global: GlobalId,
        location: GlobalLocation,
        start: u64,
        byte_len: u64,
    ) -> Self {
        Self::Global(GlobalRange::new(global, location, start, byte_len))
    }

    /// Return whether this range overlaps another range.
    pub fn overlaps(self, other: Self) -> bool {
        match (self, other) {
            (Self::LocalHeap(left), Self::LocalHeap(right)) => left.overlaps(right),
            (Self::SharedHeap(left), Self::SharedHeap(right)) => left.overlaps(right),
            (Self::Address(left), Self::Address(right)) => left.overlaps(right),
            (Self::Frame(left), Self::Frame(right)) => left.overlaps(right),
            (Self::Global(left), Self::Global(right)) => left.overlaps(right),
            _ => false,
        }
    }
}

impl MemoryStop {
    /// Create one memory stop.
    pub const fn new(
        watchpoint_id: WatchpointId,
        access: MemoryAccess,
        target: MemoryTarget,
    ) -> Self {
        Self {
            watchpoint_id,
            access,
            target,
        }
    }

    /// Return whether this stop selects one executed memory access.
    pub fn selects(
        self,
        site: MemorySite,
        access: MemoryAccess,
        range: Option<MemoryRange>,
    ) -> bool {
        self.access.selects(access) && self.target.selects(site, range)
    }
}

impl WatchSet {
    /// Create one watch set.
    pub fn new(mut memory: Vec<MemoryStop>) -> Self {
        memory.sort_by_key(|stop| stop.watchpoint_id.get());
        let requires_memory_range = memory
            .iter()
            .any(|stop| stop.target.requires_memory_range());

        Self {
            memory,
            requires_memory_range,
        }
    }

    /// Return whether the set has no active watchpoints.
    pub fn is_empty(&self) -> bool {
        self.memory.is_empty()
    }

    /// Return whether matching this set needs executed byte ranges.
    pub fn requires_memory_range(&self) -> bool {
        self.requires_memory_range
    }

    /// Return the first watchpoint selected by one executed memory access.
    pub fn watchpoint_at(
        &self,
        site: MemorySite,
        access: MemoryAccess,
        range: Option<MemoryRange>,
    ) -> Option<WatchpointId> {
        self.memory
            .iter()
            .copied()
            .find(|stop| stop.selects(site, access, range))
            .map(|stop| stop.watchpoint_id)
    }
}

impl MemoryTarget {
    /// Return whether matching this target needs an executed byte range.
    pub const fn requires_memory_range(self) -> bool {
        matches!(self, Self::Range(_))
    }

    /// Return whether this target selects one memory site.
    pub fn selects(self, site: MemorySite, range: Option<MemoryRange>) -> bool {
        match self {
            Self::Any => true,
            Self::Point(point) => point == site.point,
            Self::Type(ty) => ty == site.value_type,
            Self::Range(target) => range.is_some_and(|range| target.overlaps(range)),
        }
    }
}
