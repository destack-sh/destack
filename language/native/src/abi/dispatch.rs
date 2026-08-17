use std::mem::{offset_of, size_of};

/// Process-local virtual dispatch row.
#[repr(C)]
#[derive(Debug)]
pub struct VirtualTable {
    /// Program function ids in virtual slot order.
    pub entries: [u32; 0],
}

impl VirtualTable {
    /// Return the byte offset of one virtual slot.
    pub const fn entry_offset(slot: u32) -> usize {
        offset_of!(Self, entries) + slot as usize * size_of::<u32>()
    }
}

/// Process-local dynamic dispatch row.
#[repr(C)]
#[derive(Debug)]
pub struct DynamicTable {
    /// Concrete Program type id.
    pub concrete: u32,
    /// Field offsets or function ids in dynamic slot order.
    pub entries: [u32; 0],
}

impl DynamicTable {
    /// Return the byte offset of one dynamic slot.
    pub const fn entry_offset(slot: u32) -> usize {
        offset_of!(Self, entries) + slot as usize * size_of::<u32>()
    }
}
