use serde::{Deserialize, Serialize};

use destack_core::SectionEntry;
use destack_serde::Reflect;

use crate::{Discriminant, VariantEncoding};

/// Reference trace map for one value layout.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum TraceMap {
    /// The payload contains no references.
    Empty,
    /// The payload stores reference words at fixed byte offsets.
    Fixed {
        /// Byte offsets of encoded local heap references.
        local_offsets: Box<[u32]>,
        /// Byte offsets of encoded shared heap references.
        shared_offsets: Box<[u32]>,
        /// Byte offsets of encoded frame references.
        frame_offsets: Box<[u32]>,
    },
    /// The payload stores one nested map at a byte offset.
    Nested {
        /// The byte offset of the nested payload.
        byte_offset: u32,
        /// The nested trace map.
        map: Box<TraceMap>,
    },
    /// The payload stores multiple nested maps.
    Composite {
        /// The nested trace maps.
        maps: Box<[TraceMap]>,
    },
    /// The payload stores repeated elements with one nested trace map.
    Repeated {
        /// The number of elements in the payload.
        count: u32,
        /// The element byte stride.
        stride: u32,
        /// The per-element trace map.
        element: Box<TraceMap>,
    },
    /// The payload stores a variant with case-specific trace maps.
    Variant {
        /// The physical discriminant encoding.
        encoding: VariantEncoding,
        /// Variant trace maps in case order.
        cases: Box<[VariantTrace]>,
    },
}

impl TraceMap {
    /// Return the empty trace map.
    pub const fn empty() -> Self {
        Self::Empty
    }

    /// Return whether this map can reach any reference.
    pub fn has_reference(&self) -> bool {
        self.has_heap_reference() || self.has_frame_reference()
    }

    /// Return whether this map can reach any heap reference.
    pub fn has_heap_reference(&self) -> bool {
        self.has_local_reference() || self.has_shared_reference()
    }

    /// Return whether this map can reach local heap references.
    pub fn has_local_reference(&self) -> bool {
        match self {
            Self::Empty => false,
            Self::Fixed { local_offsets, .. } => !local_offsets.is_empty(),
            Self::Nested { map, .. } => map.has_local_reference(),
            Self::Composite { maps } => maps.iter().any(Self::has_local_reference),
            Self::Repeated { count, element, .. } => *count > 0 && element.has_local_reference(),
            Self::Variant { cases, .. } => cases.iter().any(|case| case.map.has_local_reference()),
        }
    }

    /// Return whether this map can reach shared heap references.
    pub fn has_shared_reference(&self) -> bool {
        match self {
            Self::Empty => false,
            Self::Fixed { shared_offsets, .. } => !shared_offsets.is_empty(),
            Self::Nested { map, .. } => map.has_shared_reference(),
            Self::Composite { maps } => maps.iter().any(Self::has_shared_reference),
            Self::Repeated { count, element, .. } => *count > 0 && element.has_shared_reference(),
            Self::Variant { cases, .. } => cases.iter().any(|case| case.map.has_shared_reference()),
        }
    }

    /// Return whether this map can reach frame references.
    pub fn has_frame_reference(&self) -> bool {
        match self {
            Self::Empty => false,
            Self::Fixed { frame_offsets, .. } => !frame_offsets.is_empty(),
            Self::Nested { map, .. } => map.has_frame_reference(),
            Self::Composite { maps } => maps.iter().any(Self::has_frame_reference),
            Self::Repeated { count, element, .. } => *count > 0 && element.has_frame_reference(),
            Self::Variant { cases, .. } => cases.iter().any(|case| case.map.has_frame_reference()),
        }
    }

    /// Return whether scanning requires selecting an active variant case.
    pub fn has_variant_reference(&self) -> bool {
        match self {
            Self::Empty | Self::Fixed { .. } => false,
            Self::Nested { map, .. } => map.has_variant_reference(),
            Self::Composite { maps } => maps.iter().any(Self::has_variant_reference),
            Self::Repeated { element, .. } => element.has_variant_reference(),
            Self::Variant { cases, .. } => cases.iter().any(|case| case.map.has_reference()),
        }
    }

    /// Select the trace map for one physical variant discriminant scalar.
    pub fn variant(&self, scalar: u128) -> Option<&VariantTrace> {
        let Self::Variant { encoding, cases } = self else {
            return None;
        };

        match *encoding {
            VariantEncoding::Direct { field } => {
                let discriminant = field.extract(scalar);

                cases
                    .iter()
                    .find(|case| case.discriminant.bits() == discriminant)
            }
            encoding @ VariantEncoding::Niche { .. } => {
                let index = encoding.decode_niche(scalar)? as usize;

                cases.get(index)
            }
        }
    }
}

/// One discriminant-selected trace variant.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct VariantTrace {
    /// The logical discriminant bits selecting this variant.
    pub discriminant: Discriminant,
    /// The byte offset of the variant payload.
    pub payload_offset: u32,
    /// The payload trace map for this variant.
    pub map: TraceMap,
}

/// Stable non-zero identifier for one trace map.
#[repr(transparent)]
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    Reflect,
    SectionEntry,
)]
pub struct TraceId(u32);

impl TraceId {
    /// Create a trace identifier when the raw id is non-zero.
    pub const fn from_raw(raw: u32) -> Option<Self> {
        if raw == 0 { None } else { Some(Self(raw)) }
    }

    /// Create a trace identifier from one raw table id.
    pub const fn new(raw: u32) -> Self {
        match Self::from_raw(raw) {
            Some(id) => id,
            None => unreachable!(),
        }
    }

    /// Return the raw non-zero trace identifier value.
    #[inline]
    pub const fn raw(self) -> u32 {
        self.0
    }

    /// Return the zero-based trace table index.
    #[inline]
    pub const fn index(self) -> usize {
        self.raw() as usize - 1
    }
}

/// Shared trace map table for one MIR module or lowered program.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
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
