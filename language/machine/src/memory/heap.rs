//! Managed heap for interpreter allocations.

use std::collections::HashMap;

use super::value::{HeapHandle, Value};

/// A managed heap for interpreter allocations.
///
/// This is a simple implementation for the interpreter. The runtime
/// will have a more sophisticated GC-based heap.
#[derive(Debug, Default)]
pub struct Heap {
    /// Next handle id to allocate.
    next_id: u64 = 1, // 0 is reserved for null
    /// Allocated cells.
    cells: HashMap<HeapHandle, HeapCell>,
}

/// A cell on the heap (unit of allocation).
#[derive(Debug)]
pub struct HeapCell {
    /// The cell's slots (for structs/tuples) or elements (for arrays).
    pub slots: Vec<Value>,
    /// Whether this cell has been marked (for GC).
    pub marked: bool,
}

impl Heap {
    /// Create a new empty heap.
    pub fn new() -> Self {
        Self::default()
    }

    /// Allocate a new cell and return its handle.
    pub fn allocate(&mut self) -> HeapHandle {
        let handle = HeapHandle::new(self.next_id);
        self.next_id += 1;
        self.cells.insert(
            handle,
            HeapCell {
                slots: Vec::new(),
                marked: false,
            },
        );
        handle
    }

    /// Allocate a cell with the given slots.
    pub fn allocate_with_slots(&mut self, slots: Vec<Value>) -> HeapHandle {
        let handle = HeapHandle::new(self.next_id);
        self.next_id += 1;
        self.cells.insert(
            handle,
            HeapCell {
                slots,
                marked: false,
            },
        );
        handle
    }

    /// Get a cell by handle.
    pub fn get(&self, handle: HeapHandle) -> Option<&HeapCell> {
        self.cells.get(&handle)
    }

    /// Get a mutable reference to a cell.
    pub fn get_mut(&mut self, handle: HeapHandle) -> Option<&mut HeapCell> {
        self.cells.get_mut(&handle)
    }

    /// Get a slot of a cell.
    pub fn get_slot(&self, handle: HeapHandle, index: usize) -> Option<&Value> {
        self.cells.get(&handle)?.slots.get(index)
    }

    /// Set a slot of a cell.
    pub fn set_slot(&mut self, handle: HeapHandle, index: usize, value: Value) -> bool {
        if let Some(cell) = self.cells.get_mut(&handle)
            && index < cell.slots.len()
        {
            cell.slots[index] = value;
            return true;
        }
        false
    }

    /// Free a cell.
    pub fn free(&mut self, handle: HeapHandle) {
        self.cells.remove(&handle);
    }

    /// Get the number of allocated cells.
    pub fn len(&self) -> usize {
        self.cells.len()
    }

    /// Check if the heap is empty.
    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }

    /// Clear all allocations.
    pub fn clear(&mut self) {
        self.cells.clear();
        self.next_id = 1;
    }
}
