use super::slot::{HeapCell, SlotStorage};
use super::value::{RawPointer, Value};

/// A raw heap for manual memory management (not GC-tracked).
#[derive(Debug, Default)]
pub struct RawHeap {
    /// Allocated cells. Index 0 is reserved (null pointer).
    cells: Vec<Option<HeapCell>>,
    /// Free slot indices available for reuse.
    free_list: Vec<usize>,
    /// Count of live heap cells (excluding the null slot).
    allocated_cells: usize,
}

impl RawHeap {
    /// Create a new empty raw heap.
    pub fn new() -> Self {
        Self {
            // reserve slot 0 for null pointer
            cells: vec![None],
            free_list: Vec::new(),
            allocated_cells: 0,
        }
    }

    /// Allocate a new cell and return its pointer.
    pub fn allocate(&mut self) -> RawPointer {
        self.allocate_cell(HeapCell::new())
    }

    /// Allocate a cell with a given number of slots (initialized to Void).
    pub fn allocate_with_slots(&mut self, slot_count: usize) -> RawPointer {
        self.allocate_cell(HeapCell::with_slots(slot_count))
    }

    /// Allocate a cell with the given slot values.
    pub fn allocate_with_values(&mut self, slots: Vec<Value>) -> RawPointer {
        self.allocate_cell(HeapCell {
            slots: SlotStorage::from_values(slots),
            marked: false,
        })
    }

    /// Internal: allocate a cell, reusing free slots if available.
    fn allocate_cell(&mut self, cell: HeapCell) -> RawPointer {
        if let Some(index) = self.free_list.pop() {
            self.cells[index] = Some(cell);
            self.allocated_cells += 1;
            RawPointer::new(index as u64)
        } else {
            let index = self.cells.len();
            self.cells.push(Some(cell));
            self.allocated_cells += 1;
            RawPointer::new(index as u64)
        }
    }

    /// Get a cell by pointer.
    #[inline]
    pub fn get(&self, pointer: RawPointer) -> Option<&HeapCell> {
        self.cells.get(pointer.id() as usize)?.as_ref()
    }

    /// Get a mutable reference to a cell.
    #[inline]
    pub fn get_mut(&mut self, pointer: RawPointer) -> Option<&mut HeapCell> {
        self.cells.get_mut(pointer.id() as usize)?.as_mut()
    }

    /// Free a cell by pointer. Returns true if the cell existed.
    #[inline]
    pub fn free(&mut self, pointer: RawPointer) -> bool {
        let index = pointer.id() as usize;
        if index < self.cells.len() && self.cells[index].is_some() {
            self.cells[index] = None;
            self.free_list.push(index);
            debug_assert!(self.allocated_cells > 0, "heap allocation count underflow");
            self.allocated_cells -= 1;
            return true;
        }

        false
    }

    /// Get the number of allocated cells.
    #[inline]
    pub fn cell_count(&self) -> usize {
        self.allocated_cells
    }

    /// Check if the heap is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.cell_count() == 0
    }

    /// Clear all allocations.
    #[inline]
    pub fn clear(&mut self) {
        self.cells.clear();
        self.cells.push(None);
        self.free_list.clear();
        self.allocated_cells = 0;
    }
}
