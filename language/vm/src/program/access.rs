use destack_mir as mir;

use crate::ReferenceMeta;

use super::{PointerClass, WordLayout};

/// One compiled frame access.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FrameAccess {
    /// The accessed value type.
    pub value_type: mir::LocalNodeId<mir::Type>,
    /// The reference contract for address and store checks.
    pub reference: ReferenceMeta,
    /// The fixed byte offset from the frame value base.
    pub byte_offset: usize,
    /// The byte stride for indexed access.
    pub byte_stride: usize,
    /// The number of indexed elements.
    pub length: u64,
    /// The byte width of the accessed value.
    pub byte_len: usize,
    /// The word representation for scalar access.
    pub word_layout: Option<WordLayout>,
}

/// One compiled field access.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FieldAccess {
    /// The runtime pointer class.
    pub pointer_class: PointerClass,
    /// The field value type.
    pub value_type: mir::LocalNodeId<mir::Type>,
    /// The field index inside its aggregate.
    pub index: u32,
    /// The number of fields in the aggregate.
    pub field_count: u32,
    /// The byte offset of the field value.
    pub byte_offset: usize,
    /// The byte width of the field value.
    pub byte_len: usize,
    /// The word representation for scalar field access.
    pub word_layout: Option<WordLayout>,
}

/// One compiled element access.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ElementAccess {
    /// The runtime pointer class.
    pub pointer_class: PointerClass,
    /// The reference contract for element addresses and stores.
    pub reference: ReferenceMeta,
    /// The element value type.
    pub value_type: mir::LocalNodeId<mir::Type>,
    /// The number of elements when the access has fixed bounds.
    pub length: u64,
    /// The byte stride between adjacent elements.
    pub byte_stride: usize,
    /// The byte width of the element value.
    pub byte_len: usize,
    /// The word representation for scalar element access.
    pub word_layout: Option<WordLayout>,
}

impl ElementAccess {
    /// Return whether this access fits in one VM word.
    #[inline(always)]
    pub(crate) fn is_word(self) -> bool {
        self.word_layout.is_some()
    }
}

/// One compiled slice element address.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SliceElementAccess {
    /// The reference contract for the element address.
    pub reference: ReferenceMeta,
    /// The slice data.
    pub data: FieldAccess,
    /// The slice length.
    pub length: FieldAccess,
    /// The backing element access.
    pub element: ElementAccess,
}

/// One compiled pointer pointee access.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PointeeAccess {
    /// The runtime pointer class.
    pub pointer_class: PointerClass,
    /// The pointee value type.
    pub value_type: mir::LocalNodeId<mir::Type>,
    /// The byte offset from the pointer base.
    pub byte_offset: usize,
    /// The byte width of the pointee value.
    pub byte_len: usize,
    /// The word representation for scalar pointee access.
    pub word_layout: Option<WordLayout>,
}

impl PointeeAccess {
    /// Return whether this access fits in one VM word.
    #[inline(always)]
    pub(crate) fn is_word(self) -> bool {
        self.word_layout.is_some()
    }
}

impl From<FieldAccess> for PointeeAccess {
    fn from(field: FieldAccess) -> Self {
        Self {
            pointer_class: field.pointer_class,
            value_type: field.value_type,
            byte_offset: field.byte_offset,
            byte_len: field.byte_len,
            word_layout: field.word_layout,
        }
    }
}

impl From<FieldAccess> for FrameAccess {
    fn from(field: FieldAccess) -> Self {
        Self {
            value_type: field.value_type,
            reference: ReferenceMeta::NONE,
            byte_offset: field.byte_offset,
            byte_stride: 0,
            length: 0,
            byte_len: field.byte_len,
            word_layout: field.word_layout,
        }
    }
}

impl ElementAccess {
    /// Return this element access as one indexed frame access.
    pub(crate) fn into_frame_access(self, byte_offset: usize, length: u64) -> FrameAccess {
        FrameAccess {
            value_type: self.value_type,
            reference: self.reference,
            byte_offset,
            byte_stride: self.byte_stride,
            length,
            byte_len: self.byte_len,
            word_layout: self.word_layout,
        }
    }
}

impl From<FrameAccess> for PointeeAccess {
    fn from(access: FrameAccess) -> Self {
        Self {
            pointer_class: PointerClass::Frame,
            value_type: access.value_type,
            byte_offset: access.byte_offset,
            byte_len: access.byte_len,
            word_layout: access.word_layout,
        }
    }
}

impl From<PointeeAccess> for FrameAccess {
    fn from(access: PointeeAccess) -> Self {
        Self {
            value_type: access.value_type,
            reference: ReferenceMeta::NONE,
            byte_offset: access.byte_offset,
            byte_stride: 0,
            length: 0,
            byte_len: access.byte_len,
            word_layout: access.word_layout,
        }
    }
}

impl From<ElementAccess> for PointeeAccess {
    fn from(element: ElementAccess) -> Self {
        Self {
            pointer_class: element.pointer_class,
            value_type: element.value_type,
            byte_offset: 0,
            byte_len: element.byte_len,
            word_layout: element.word_layout,
        }
    }
}
