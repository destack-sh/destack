use destack_mir as mir;

pub(crate) const POINTER_BASE_MASK: u64 = 0xFFFF_FFFF;
pub(crate) const POINTER_SLOT_SHIFT: u64 = 32;
pub(crate) const STACK_INDEX_MASK: u64 = 0xFFFF;
pub(crate) const STACK_SLOT_SHIFT: u64 = 16;
pub(crate) const REF_META_SHIFT: u64 = 16;
pub(crate) const REF_META_MASK: u64 = 0xFF << REF_META_SHIFT;

/// Handle to a managed (GC-tracked) heap object.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HeapHandle(pub(crate) u64);

impl HeapHandle {
    /// The null handle (represents null reference).
    pub const NULL: Self = HeapHandle(0);

    /// Create a new heap handle from a raw id.
    #[inline]
    pub fn new(id: u64) -> Self {
        HeapHandle::with_slot(id, 0)
    }

    /// Check if this handle is null.
    #[inline]
    pub fn is_null(&self) -> bool {
        self.id() == 0
    }

    /// Get the raw id of this handle.
    #[inline]
    pub fn id(&self) -> u64 {
        self.0 & POINTER_BASE_MASK
    }

    /// Get the slot offset stored in this handle.
    #[inline]
    pub fn slot_index(&self) -> usize {
        (self.0 >> POINTER_SLOT_SHIFT) as usize
    }

    /// Create a new handle with a slot offset.
    #[inline]
    pub fn with_slot(id: u64, slot_index: u32) -> Self {
        let base = id & POINTER_BASE_MASK;
        let slot = (slot_index as u64) << POINTER_SLOT_SHIFT;
        HeapHandle(base | slot)
    }
}

/// Pointer to a raw (manually managed) heap object.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RawPointer(pub(crate) u64);

impl RawPointer {
    /// The null pointer.
    pub const NULL: Self = RawPointer(0);

    /// Create a new raw pointer from an id.
    #[inline]
    pub fn new(id: u64) -> Self {
        RawPointer::with_slot(id, 0)
    }

    /// Check if this pointer is null.
    #[inline]
    pub fn is_null(&self) -> bool {
        self.id() == 0
    }

    /// Get the base id of this pointer.
    #[inline]
    pub fn id(&self) -> u64 {
        self.0 & POINTER_BASE_MASK
    }

    /// Get the slot offset stored in this pointer.
    #[inline]
    pub fn slot_index(&self) -> usize {
        (self.0 >> POINTER_SLOT_SHIFT) as usize
    }

    /// Create a new raw pointer with a slot offset.
    #[inline]
    pub fn with_slot(id: u64, slot_index: u32) -> Self {
        let base = id & POINTER_BASE_MASK;
        let slot = (slot_index as u64) << POINTER_SLOT_SHIFT;
        RawPointer(base | slot)
    }
}

/// Pointer to a stack-allocated object (frame-scoped).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StackPointer {
    /// The frame depth (index into call stack).
    pub frame_idx: usize,
    /// The slot index within the frame's stack allocations.
    pub slot: usize,
    /// The slot offset within the stack allocation.
    pub slot_offset: usize,
}

impl StackPointer {
    /// Create a new stack pointer.
    #[inline]
    pub fn new(frame_idx: usize, slot: usize) -> Self {
        Self {
            frame_idx,
            slot,
            slot_offset: 0,
        }
    }

    /// Create a stack pointer with an offset into the slot.
    #[inline]
    pub fn with_offset(frame_idx: usize, slot: usize, slot_offset: usize) -> Self {
        Self {
            frame_idx,
            slot,
            slot_offset,
        }
    }
}

/// Pointer to a frame-local slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LocalPointer {
    /// The frame depth (index into call stack).
    pub frame_idx: usize,
    /// The local index within the frame.
    pub local: usize,
    /// The slot offset within the local value.
    pub slot_offset: usize,
}

impl LocalPointer {
    /// Create a new local pointer.
    #[inline]
    pub fn new(frame_idx: usize, local: usize) -> Self {
        Self {
            frame_idx,
            local,
            slot_offset: 0,
        }
    }

    /// Create a local pointer with an offset into the slot.
    #[inline]
    pub fn with_offset(frame_idx: usize, local: usize, slot_offset: usize) -> Self {
        Self {
            frame_idx,
            local,
            slot_offset,
        }
    }
}

/// Pointer to a global value (with optional slot offset).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GlobalPointer {
    /// The global identifier.
    pub id: mir::LocalNodeId<mir::Global>,
    /// The slot offset within the global value.
    pub slot_offset: usize,
}
