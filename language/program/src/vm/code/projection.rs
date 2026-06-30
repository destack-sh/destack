use destack_core::{Optional, SectionEntry};
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::{CellLayout, TypeId};

/// Compiled projection from a base address to one value.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct Projection {
    /// The projected value type.
    pub value_type: TypeId,
    /// The fixed byte offset from the base address.
    pub byte_offset: u64,
    /// The byte stride for indexed projections.
    pub byte_stride: u64,
    /// The number of addressable elements when statically known.
    pub length: u64,
    /// The byte width of the projected value.
    pub byte_len: u64,
    /// The cell representation for scalar projections.
    pub cell_layout: Optional<CellLayout>,
}

impl Projection {
    /// Return a fixed byte projection.
    pub fn fixed(
        value_type: TypeId,
        byte_offset: usize,
        byte_len: usize,
        cell_layout: Option<CellLayout>,
    ) -> Self {
        Self {
            value_type,
            byte_offset: byte_offset as u64,
            byte_stride: 0,
            length: 0,
            byte_len: byte_len as u64,
            cell_layout: cell_layout.into(),
        }
    }

    /// Return an indexed projection.
    pub fn indexed(
        value_type: TypeId,
        length: u64,
        byte_stride: usize,
        byte_len: usize,
        cell_layout: Option<CellLayout>,
    ) -> Self {
        Self {
            value_type,
            byte_offset: 0,
            byte_stride: byte_stride as u64,
            length,
            byte_len: byte_len as u64,
            cell_layout: cell_layout.into(),
        }
    }

    /// Return the fixed byte offset from the base address.
    #[inline(always)]
    pub const fn byte_offset(self) -> usize {
        self.byte_offset as usize
    }

    /// Return the byte stride for indexed projections.
    #[inline(always)]
    pub const fn byte_stride(self) -> usize {
        self.byte_stride as usize
    }

    /// Return the byte width of the projected value.
    #[inline(always)]
    pub const fn byte_len(self) -> usize {
        self.byte_len as usize
    }

    /// Return the cell representation for scalar projections.
    #[inline(always)]
    pub fn cell_layout(self) -> Option<CellLayout> {
        self.cell_layout.get()
    }

    /// Return whether this projection fits in one VM cell.
    #[inline(always)]
    pub fn is_cell(self) -> bool {
        self.cell_layout().is_some()
    }

    /// Return this projection at a fixed byte offset.
    #[inline(always)]
    pub fn at_offset(self, byte_offset: usize) -> Self {
        Self {
            byte_offset: byte_offset as u64,
            ..self
        }
    }

    /// Return this projection with a known element count.
    #[inline(always)]
    pub fn with_length(self, length: u64) -> Self {
        Self { length, ..self }
    }
}

/// Compiled projection from a base address to one physical cell slot.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct SlotProjection {
    /// The fixed byte offset from the base address.
    pub byte_offset: u64,
    /// The byte width of the slot payload.
    pub byte_len: u64,
    /// The cell representation for this slot.
    pub cell_layout: CellLayout,
}

impl SlotProjection {
    /// Return a fixed slot projection.
    pub fn fixed(byte_offset: usize, byte_len: usize, cell_layout: CellLayout) -> Self {
        Self {
            byte_offset: byte_offset as u64,
            byte_len: byte_len as u64,
            cell_layout,
        }
    }

    /// Return the fixed byte offset from the base address.
    #[inline(always)]
    pub const fn byte_offset(self) -> usize {
        self.byte_offset as usize
    }

    /// Return the byte width of the slot payload.
    #[inline(always)]
    pub const fn byte_len(self) -> usize {
        self.byte_len as usize
    }

    /// Return the cell representation for this slot.
    #[inline(always)]
    pub const fn cell_layout(self) -> CellLayout {
        self.cell_layout
    }
}

/// Compiled projections for one slice descriptor.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct SliceProjection {
    /// The slice pointer projection.
    pub pointer: SlotProjection,
    /// The slice length projection.
    pub length: SlotProjection,
    /// The backing element projection.
    pub element: Projection,
}

// SAFETY: projection entries contain only fixed-width VM entry values.
unsafe impl SectionEntry for Projection {}
unsafe impl SectionEntry for SlotProjection {}
unsafe impl SectionEntry for SliceProjection {}
