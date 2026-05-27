use std::num::NonZeroU32;

use serde::{Deserialize, Serialize};

/// Heap trace metadata for one runtime payload.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TraceMap {
    /// Payload contains no heap references.
    Empty,
    /// Payload stores heap-reference words at fixed byte offsets.
    Fixed {
        /// Byte offsets of encoded local heap references.
        local_offsets: Box<[u32]>,
        /// Byte offsets of encoded shared heap references.
        shared_offsets: Box<[u32]>,
    },
    /// Payload stores one nested map at a byte offset.
    Nested {
        /// The byte offset of the nested payload.
        byte_offset: u32,
        /// The nested trace map.
        map: Box<TraceMap>,
    },
    /// Payload stores multiple nested maps.
    Composite {
        /// The nested trace maps.
        maps: Box<[TraceMap]>,
    },
    /// Payload stores repeated elements with one nested trace map.
    Repeated {
        /// The number of elements in the payload.
        count: u32,
        /// The element byte stride.
        stride: u32,
        /// The per-element trace map.
        element: Box<TraceMap>,
    },
    /// Payload stores a tagged variant with variant-specific trace maps.
    Tagged {
        /// The byte offset of the variant tag.
        tag_offset: u32,
        /// The byte width of the variant tag.
        tag_bytes: u8,
        /// Variant trace maps keyed by normalized tag value.
        variants: Box<[TraceVariant]>,
    },
}

impl TraceMap {
    /// Return the empty trace map.
    pub const fn empty() -> Self {
        Self::Empty
    }

    /// Report whether this map can reach heap references.
    pub fn has_reference(&self) -> bool {
        self.has_local_reference() || self.has_shared_reference()
    }

    /// Report whether this map can reach local heap references.
    pub fn has_local_reference(&self) -> bool {
        match self {
            Self::Empty => false,
            Self::Fixed { local_offsets, .. } => !local_offsets.is_empty(),
            Self::Nested { map, .. } => map.has_local_reference(),
            Self::Composite { maps } => maps.iter().any(Self::has_local_reference),
            Self::Repeated { count, element, .. } => *count > 0 && element.has_local_reference(),
            Self::Tagged { variants, .. } => variants
                .iter()
                .any(|variant| variant.map.has_local_reference()),
        }
    }

    /// Report whether this map can reach shared heap references.
    pub fn has_shared_reference(&self) -> bool {
        match self {
            Self::Empty => false,
            Self::Fixed { shared_offsets, .. } => !shared_offsets.is_empty(),
            Self::Nested { map, .. } => map.has_shared_reference(),
            Self::Composite { maps } => maps.iter().any(Self::has_shared_reference),
            Self::Repeated { count, element, .. } => *count > 0 && element.has_shared_reference(),
            Self::Tagged { variants, .. } => variants
                .iter()
                .any(|variant| variant.map.has_shared_reference()),
        }
    }

    /// Report whether this map requires reading payload tags while scanning.
    pub fn has_tagged_reference(&self) -> bool {
        match self {
            Self::Empty | Self::Fixed { .. } => false,
            Self::Nested { map, .. } => map.has_tagged_reference(),
            Self::Composite { maps } => maps.iter().any(Self::has_tagged_reference),
            Self::Repeated { element, .. } => element.has_tagged_reference(),
            Self::Tagged { variants, .. } => {
                variants.iter().any(|variant| variant.map.has_reference())
            }
        }
    }
}

/// One tag-selected trace variant.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TraceVariant {
    /// The normalized numeric tag value selecting this variant.
    pub tag: u64,
    /// The byte offset of the variant storage.
    pub storage_offset: u32,
    /// The storage trace map for this variant.
    pub map: TraceMap,
}

/// Stable non-zero identifier for one heap trace map.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct TraceId(NonZeroU32);

impl TraceId {
    /// Create a trace identifier from one raw table id.
    pub const fn new(raw: u32) -> Self {
        match NonZeroU32::new(raw) {
            Some(raw) => Self(raw),
            None => panic!("trace identifiers must be non-zero"),
        }
    }

    /// Return the raw non-zero trace identifier value.
    #[inline]
    pub const fn raw(self) -> u32 {
        self.0.get()
    }

    /// Return the zero-based trace table index.
    #[inline]
    pub const fn index(self) -> usize {
        self.raw() as usize - 1
    }
}

/// Shared table of heap trace maps for one lowered program.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraceTable {
    /// Trace maps indexed by TraceId.
    traces: Vec<TraceMap>,
}

impl Default for TraceTable {
    fn default() -> Self {
        Self::new()
    }
}

impl TraceTable {
    /// Create one empty trace table.
    pub fn new() -> Self {
        Self { traces: Vec::new() }
    }

    /// Insert one trace map and return its stable id.
    pub fn insert(&mut self, trace: TraceMap) -> TraceId {
        if let Some(id) = self.id(&trace) {
            return id;
        }

        let index = self.traces.len();
        self.traces.push(trace);

        TraceId::new(index as u32 + 1)
    }

    /// Return the stable id for one already interned trace map.
    pub fn id(&self, trace: &TraceMap) -> Option<TraceId> {
        self.traces
            .iter()
            .position(|existing| existing == trace)
            .map(|index| TraceId::new(index as u32 + 1))
    }

    /// Borrow one trace map when the id is present.
    pub fn trace(&self, id: TraceId) -> Option<&TraceMap> {
        self.traces.get(id.index())
    }

    /// Borrow all trace maps.
    pub fn traces(&self) -> &[TraceMap] {
        &self.traces
    }
}
