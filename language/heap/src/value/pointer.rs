use destack_mir as mir;
use serde::{Deserialize, Serialize};

pub(crate) const POINTER_BASE_MASK: u64 = 0xFFFF_FFFF;
pub(crate) const POINTER_SLOT_SHIFT: u64 = 32;
pub(crate) const STACK_INDEX_MASK: u64 = 0xFFFF;
pub(crate) const STACK_SLOT_SHIFT: u64 = 16;
pub(crate) const REF_META_SHIFT: u64 = 16;
pub(crate) const REF_META_MASK: u64 = 0xFF << REF_META_SHIFT;

/// Reference to a managed (GC-tracked) heap object.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ManagedReference(pub(crate) u64);

impl ManagedReference {
    /// The null reference.
    pub const NULL: Self = ManagedReference(0);

    /// Create a new managed reference from a raw id.
    #[inline]
    pub fn new(id: u32) -> Self {
        ManagedReference::with_slot_offset(id, 0)
    }

    /// Check if this reference is null.
    #[inline]
    pub fn is_null(&self) -> bool {
        self.id() == 0
    }

    /// Return the raw id of this reference.
    #[inline]
    pub fn id(&self) -> u32 {
        (self.0 & POINTER_BASE_MASK) as u32
    }

    /// Return the raw bits for this reference.
    #[inline]
    pub const fn bits(self) -> u64 {
        self.0
    }

    /// Restore a managed reference from raw bits.
    #[inline]
    pub const fn from_bits(bits: u64) -> Self {
        Self(bits)
    }

    /// Return the byte offset stored in this reference.
    #[inline]
    pub fn byte_offset(&self) -> usize {
        self.slot_offset()
    }

    /// Return the aggregate slot offset stored in this reference.
    #[inline]
    pub fn slot_offset(&self) -> usize {
        (self.0 >> POINTER_SLOT_SHIFT) as usize
    }

    /// Create a new reference with an aggregate slot offset.
    #[inline]
    pub fn with_slot_offset(id: u32, slot_offset: u32) -> Self {
        let base = id as u64;
        let slot = (slot_offset as u64) << POINTER_SLOT_SHIFT;
        ManagedReference(base | slot)
    }

    /// Create a new reference with a byte offset.
    #[inline]
    pub fn with_byte_offset(id: u32, byte_offset: u32) -> Self {
        Self::with_slot_offset(id, byte_offset)
    }
}

/// Pointer to a raw (manually managed) heap object.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct RawPointer(pub(crate) u64);

impl RawPointer {
    /// The null pointer.
    pub const NULL: Self = RawPointer(0);

    /// Create a new raw pointer from an id.
    #[inline]
    pub fn new(id: u32) -> Self {
        RawPointer::with_slot_offset(id, 0)
    }

    /// Check if this pointer is null.
    #[inline]
    pub fn is_null(&self) -> bool {
        self.id() == 0
    }

    /// Return the raw id of this pointer.
    #[inline]
    pub fn id(&self) -> u32 {
        (self.0 & POINTER_BASE_MASK) as u32
    }

    /// Return the raw bits for this pointer.
    #[inline]
    pub const fn bits(self) -> u64 {
        self.0
    }

    /// Restore a raw pointer from raw bits.
    #[inline]
    pub const fn from_bits(bits: u64) -> Self {
        Self(bits)
    }

    /// Return the byte offset stored in this pointer.
    #[inline]
    pub fn byte_offset(&self) -> usize {
        self.slot_offset()
    }

    /// Return the aggregate slot offset stored in this pointer.
    #[inline]
    pub fn slot_offset(&self) -> usize {
        (self.0 >> POINTER_SLOT_SHIFT) as usize
    }

    /// Create a new raw pointer with an aggregate slot offset.
    #[inline]
    pub fn with_slot_offset(id: u32, slot_offset: u32) -> Self {
        let base = id as u64;
        let slot = (slot_offset as u64) << POINTER_SLOT_SHIFT;
        RawPointer(base | slot)
    }

    /// Create a new raw pointer with a byte offset.
    #[inline]
    pub fn with_byte_offset(id: u32, byte_offset: u32) -> Self {
        Self::with_slot_offset(id, byte_offset)
    }
}

/// Pointer to a world-shared space allocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SharedPointer(pub(crate) u64);

impl SharedPointer {
    /// The null pointer.
    pub const NULL: Self = SharedPointer(0);

    /// Create a new shared pointer from an id.
    #[inline]
    pub fn new(id: u32) -> Self {
        SharedPointer::with_byte_offset(id, 0)
    }

    /// Check if this pointer is null.
    #[inline]
    pub fn is_null(&self) -> bool {
        self.id() == 0
    }

    /// Return the raw id of this pointer.
    #[inline]
    pub fn id(&self) -> u32 {
        (self.0 & POINTER_BASE_MASK) as u32
    }

    /// Return the raw bits for this pointer.
    #[inline]
    pub const fn bits(self) -> u64 {
        self.0
    }

    /// Restore a shared pointer from raw bits.
    #[inline]
    pub const fn from_bits(bits: u64) -> Self {
        Self(bits)
    }

    /// Return the byte offset stored in this pointer.
    #[inline]
    pub fn byte_offset(&self) -> usize {
        (self.0 >> POINTER_SLOT_SHIFT) as usize
    }

    /// Create a new shared pointer with a byte offset.
    #[inline]
    pub fn with_byte_offset(id: u32, byte_offset: u32) -> Self {
        let base = id as u64;
        let offset = (byte_offset as u64) << POINTER_SLOT_SHIFT;
        SharedPointer(base | offset)
    }
}

/// Pointer to a stack-allocated object (frame-scoped).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GlobalPointer {
    /// The global identifier.
    pub id: mir::LocalNodeId<mir::Global>,
    /// The slot offset within the global value.
    pub slot_offset: usize,
}
