use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::num::NonZeroU32;

use destack_core::CowBuffer;
use destack_mir::LayoutTrace;
use serde::{Deserialize, Serialize};

use crate::{HeapError, HeapResult, LayoutId};

/// The sentinel for one empty reverse-lookup slot.
const EMPTY_SHAPE_SLOT: u32 = u32::MAX;

/// Heap scan metadata for one managed payload.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HeapScan {
    /// Payload contains no managed references.
    None,
    /// Payload stores direct managed-reference words at fixed byte offsets.
    Reference {
        /// Byte offsets of encoded local managed references.
        local_offsets: Box<[u32]>,
        /// Byte offsets of encoded shared managed references.
        shared_offsets: Box<[u32]>,
    },
    /// Payload stores full packed VM values at fixed byte offsets.
    PackedValue {
        /// Byte offsets of encoded packed values.
        offsets: Box<[u32]>,
    },
    /// Payload stores repeated elements with managed-reference words at fixed element offsets.
    RepeatedReference {
        /// The number of elements in the payload.
        count: u32,
        /// The element byte stride.
        stride: u32,
        /// Local managed-reference byte offsets within each element.
        local_offsets: Box<[u32]>,
        /// Shared managed-reference byte offsets within each element.
        shared_offsets: Box<[u32]>,
    },
}

impl HeapScan {
    /// Return the empty heap scan.
    pub const fn empty() -> Self {
        Self::None
    }

    /// Report whether this payload may contain managed references.
    pub fn has_reference(&self) -> bool {
        self.has_local_reference() || self.has_shared_reference()
    }

    /// Report whether this payload may contain local managed references.
    pub fn has_local_reference(&self) -> bool {
        match self {
            Self::None => false,
            Self::Reference { local_offsets, .. } => !local_offsets.is_empty(),
            Self::PackedValue { offsets } => !offsets.is_empty(),
            Self::RepeatedReference {
                count,
                local_offsets,
                ..
            } => *count > 0 && !local_offsets.is_empty(),
        }
    }

    /// Report whether this payload may contain shared managed references.
    pub fn has_shared_reference(&self) -> bool {
        match self {
            Self::None => false,
            Self::Reference { shared_offsets, .. } => !shared_offsets.is_empty(),
            Self::PackedValue { offsets } => !offsets.is_empty(),
            Self::RepeatedReference {
                count,
                shared_offsets,
                ..
            } => *count > 0 && !shared_offsets.is_empty(),
        }
    }
}

impl From<LayoutTrace> for HeapScan {
    fn from(trace: LayoutTrace) -> Self {
        match trace {
            LayoutTrace::None => Self::None,
            LayoutTrace::Reference {
                local_offsets,
                shared_offsets,
            } => Self::Reference {
                local_offsets,
                shared_offsets,
            },
            LayoutTrace::RepeatedReference {
                count,
                stride,
                local_offsets,
                shared_offsets,
            } => Self::RepeatedReference {
                count,
                stride,
                local_offsets,
                shared_offsets,
            },
        }
    }
}

impl PartialEq<LayoutTrace> for HeapScan {
    fn eq(&self, other: &LayoutTrace) -> bool {
        match (self, other) {
            (Self::None, LayoutTrace::None) => true,
            (
                Self::Reference {
                    local_offsets: left_local,
                    shared_offsets: left_shared,
                },
                LayoutTrace::Reference {
                    local_offsets: right_local,
                    shared_offsets: right_shared,
                },
            ) => left_local == right_local && left_shared == right_shared,
            (
                Self::RepeatedReference {
                    count: left_count,
                    stride: left_stride,
                    local_offsets: left_local,
                    shared_offsets: left_shared,
                },
                LayoutTrace::RepeatedReference {
                    count: right_count,
                    stride: right_stride,
                    local_offsets: right_local,
                    shared_offsets: right_shared,
                },
            ) => {
                left_count == right_count
                    && left_stride == right_stride
                    && left_local == right_local
                    && left_shared == right_shared
            }
            _ => false,
        }
    }
}

impl PartialEq<HeapScan> for LayoutTrace {
    fn eq(&self, other: &HeapScan) -> bool {
        other == self
    }
}

/// One shape table entry.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct Shape {
    /// The exact heap scan metadata for this payload.
    pub(crate) scan: HeapScan,
    /// The canonical MIR layout identity for this payload, if any.
    pub(crate) layout_id: Option<LayoutId>,
}

/// One interned shape identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct ShapeId(NonZeroU32);

impl ShapeId {
    /// Create one shape identifier from one packed raw value.
    pub(crate) fn from_raw(raw: u32) -> HeapResult<Self> {
        let raw = NonZeroU32::new(raw).ok_or(HeapError::InvalidShapeId { index: 0 })?;

        if raw.get() == EMPTY_SHAPE_SLOT {
            return Err(HeapError::InvalidShapeId {
                index: EMPTY_SHAPE_SLOT as usize,
            });
        }

        Ok(Self(raw))
    }

    /// Create one shape identifier from a table index.
    pub(crate) fn from_index(index: usize) -> HeapResult<Self> {
        let Some(raw) = index.checked_add(1) else {
            return Err(HeapError::InvalidShapeId { index });
        };

        if raw >= EMPTY_SHAPE_SLOT as usize {
            return Err(HeapError::InvalidShapeId { index });
        }

        let raw = NonZeroU32::new(raw as u32).ok_or(HeapError::InvalidShapeId { index })?;

        Ok(Self(raw))
    }

    /// Return the shape-table index for this identifier.
    pub(crate) const fn index(self) -> usize {
        self.0.get() as usize - 1
    }

    /// Return the packed identifier used in reverse-lookup slots.
    pub(crate) const fn raw(self) -> u32 {
        self.0.get()
    }
}

/// Interned heap shapes for one heap.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ShapeTable {
    /// Interned shape entries.
    shapes: CowBuffer<Shape>,
    /// Open-addressed reverse lookup keyed by shape id.
    shape_slots: Vec<u32>,
}

impl ShapeTable {
    /// Create one empty shape table.
    pub(crate) fn new() -> Self {
        Self {
            shapes: CowBuffer::new(),
            shape_slots: vec![EMPTY_SHAPE_SLOT; 8],
        }
    }

    /// Restore one shape table from one flattened shape list.
    pub(crate) fn from_shapes(shapes: Vec<Shape>) -> HeapResult<Self> {
        let mut table = Self {
            shapes: CowBuffer::from_vec(shapes),
            shape_slots: Vec::new(),
        };

        table.rebuild_lookup_slots()?;

        Ok(table)
    }

    /// Intern one shape entry and return its stable identifier.
    pub(crate) fn intern(&mut self, shape: Shape) -> HeapResult<ShapeId> {
        if self.shape_slots.is_empty() {
            self.rebuild_lookup_slots()?;
        }

        if let Ok(shape_id) = self.lookup(&shape) {
            return Ok(shape_id);
        }

        if self.needs_rehash() {
            self.rebuild_lookup_slots_for_count(self.shapes.len().saturating_add(1))?;
        }

        let slot_index = match self.lookup(&shape) {
            Ok(shape_id) => return Ok(shape_id),
            Err(slot_index) => slot_index,
        };
        let shape_id = ShapeId::from_index(self.shapes.len())?;
        self.shapes.make_mut().push(shape);
        self.shape_slots[slot_index] = shape_id.raw();

        Ok(shape_id)
    }

    /// Return the flattened shape entries.
    pub(crate) fn shapes(&self) -> &[Shape] {
        self.shapes.as_slice()
    }

    /// Return one interned shape entry by id.
    pub(crate) fn shape(&self, shape_id: ShapeId) -> Option<&Shape> {
        self.shapes.as_slice().get(shape_id.index())
    }

    /// Return whether the reverse lookup should grow before one more insert.
    fn needs_rehash(&self) -> bool {
        self.shape_slots.is_empty()
            || self.shapes.len().saturating_add(1) * 4 > self.shape_slots.len() * 3
    }

    /// Rebuild the reverse lookup for the current shape set.
    fn rebuild_lookup_slots(&mut self) -> HeapResult<()> {
        self.rebuild_lookup_slots_for_count(self.shapes.len())
    }

    /// Rebuild the reverse lookup for the given target shape count.
    fn rebuild_lookup_slots_for_count(&mut self, count: usize) -> HeapResult<()> {
        let slot_count = count
            .checked_mul(2)
            .and_then(|count| count.max(8).checked_next_power_of_two())
            .ok_or(HeapError::InvalidShapeTableLen { len: count })?;
        self.shape_slots = vec![EMPTY_SHAPE_SLOT; slot_count];

        for (index, shape) in self.shapes.as_slice().iter().enumerate() {
            let shape_id = ShapeId::from_index(index)?;
            let slot_index = match self.lookup_in_slots(shape, &self.shape_slots) {
                Ok(_) => return Err(HeapError::DuplicateShape { index }),
                Err(slot_index) => slot_index,
            };
            self.shape_slots[slot_index] = shape_id.raw();
        }

        Ok(())
    }

    /// Return one interned identifier or one insertion slot for this shape.
    fn lookup(&self, shape: &Shape) -> Result<ShapeId, usize> {
        match self.lookup_in_slots(shape, &self.shape_slots) {
            Ok(index) => ShapeId::from_index(index).map_err(|_| 0),
            Err(slot_index) => Err(slot_index),
        }
    }

    /// Probe one reverse-lookup table for this shape.
    fn lookup_in_slots(&self, shape: &Shape, slots: &[u32]) -> Result<usize, usize> {
        if slots.is_empty() {
            return Err(0);
        }

        let mask = slots.len() - 1;
        let mut slot_index = (Self::hash_shape(shape) as usize) & mask;

        loop {
            let shape_raw = slots[slot_index];
            if shape_raw == EMPTY_SHAPE_SLOT {
                return Err(slot_index);
            }

            let shape_index = shape_raw as usize - 1;

            if self.shapes.as_slice()[shape_index] == *shape {
                return Ok(shape_index);
            }

            slot_index = (slot_index + 1) & mask;
        }
    }

    /// Hash one shape entry for the reverse lookup.
    fn hash_shape(shape: &Shape) -> u64 {
        let mut hasher = DefaultHasher::new();
        shape.hash(&mut hasher);
        hasher.finish()
    }
}
