use destack_mir as mir;

use super::WordLayout;

/// Compiled projection from a base address to one value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Projection {
    /// The projected value type.
    pub value_type: mir::LocalNodeId<mir::Type>,
    /// The fixed byte offset from the base address.
    pub byte_offset: usize,
    /// The byte stride for indexed projections.
    pub byte_stride: usize,
    /// The number of addressable elements when statically known.
    pub length: u64,
    /// The byte width of the projected value.
    pub byte_len: usize,
    /// The word representation for scalar projections.
    pub word_layout: Option<WordLayout>,
}

impl Projection {
    /// Return a fixed byte projection.
    pub(crate) fn fixed(
        value_type: mir::LocalNodeId<mir::Type>,
        byte_offset: usize,
        byte_len: usize,
        word_layout: Option<WordLayout>,
    ) -> Self {
        Self {
            value_type,
            byte_offset,
            byte_stride: 0,
            length: 0,
            byte_len,
            word_layout,
        }
    }

    /// Return an indexed projection.
    pub(crate) fn indexed(
        value_type: mir::LocalNodeId<mir::Type>,
        length: u64,
        byte_stride: usize,
        byte_len: usize,
        word_layout: Option<WordLayout>,
    ) -> Self {
        Self {
            value_type,
            byte_offset: 0,
            byte_stride,
            length,
            byte_len,
            word_layout,
        }
    }

    /// Return whether this projection fits in one VM word.
    #[inline(always)]
    pub(crate) fn is_word(self) -> bool {
        self.word_layout.is_some()
    }

    /// Return this projection at a fixed byte offset.
    #[inline(always)]
    pub(crate) fn at_offset(self, byte_offset: usize) -> Self {
        Self {
            byte_offset,
            ..self
        }
    }

    /// Return this projection with a known element count.
    #[inline(always)]
    pub(crate) fn with_length(self, length: u64) -> Self {
        Self { length, ..self }
    }
}

/// Compiled projection from a base address to one physical word slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SlotProjection {
    /// The fixed byte offset from the base address.
    pub byte_offset: usize,
    /// The byte width of the slot payload.
    pub byte_len: usize,
    /// The word representation for this slot.
    pub word_layout: WordLayout,
}

impl SlotProjection {
    /// Return a fixed slot projection.
    pub(crate) fn fixed(byte_offset: usize, byte_len: usize, word_layout: WordLayout) -> Self {
        Self {
            byte_offset,
            byte_len,
            word_layout,
        }
    }
}

/// Compiled projection data for one slice descriptor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SliceProjection {
    /// The slice data projection.
    pub data: SlotProjection,
    /// The slice length projection.
    pub length: SlotProjection,
    /// The backing element projection.
    pub element: Projection,
}
