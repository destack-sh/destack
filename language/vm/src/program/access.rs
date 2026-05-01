use destack_mir as mir;

use super::{PointerClass, WordLayout};

/// One compiled field access.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FieldAccess {
    /// The runtime pointer class.
    pub pointer_class: PointerClass,
    /// The field value type.
    pub value_type: mir::LocalNodeId<mir::Type>,
    /// The byte offset of the field value.
    pub byte_offset: usize,
    /// The byte width of the field value.
    pub byte_len: usize,
    /// The word representation for scalar field access.
    pub word_layout: Option<WordLayout>,
}

impl FieldAccess {
    /// Return whether this access fits in one VM word.
    #[inline(always)]
    pub(crate) fn is_word(self) -> bool {
        self.word_layout.is_some()
    }
}

/// One compiled element access.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ElementAccess {
    /// The runtime pointer class.
    pub pointer_class: PointerClass,
    /// The element value type.
    pub value_type: mir::LocalNodeId<mir::Type>,
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
