use std::collections::HashMap;

use super::value::{HeapHandle, RawPointer, Value};

/// A managed heap for interpreter allocations (GC-tracked).
///
/// Uses a slab allocator for O(1) access.
/// Freed slots are tracked in a free list for reuse.
#[derive(Debug, Default)]
pub struct ManagedHeap {
    /// Allocated cells. Index 0 is reserved (null handle).
    cells: Vec<Option<HeapCell>>,
    /// Free slot indices available for reuse.
    free_list: Vec<usize>,
}

/// A raw heap for manual memory management (not GC-tracked).
#[derive(Debug, Default)]
pub struct RawHeap {
    /// Allocated cells, indexed by pointer ID.
    cells: HashMap<u64, HeapCell>,
    /// Next pointer ID to allocate.
    next_id: u64,
}

/// A cell on the heap (unit of allocation).
#[derive(Debug, Default)]
pub struct HeapCell {
    /// The cell's slots (for structs/tuples) or elements (for arrays).
    pub slots: Vec<Value>,
    /// Whether this cell has been marked (for GC).
    pub marked: bool,
}

impl HeapCell {
    /// Create a new empty cell.
    pub fn new() -> Self {
        Self {
            slots: Vec::new(),
            marked: false,
        }
    }

    /// Create a cell with the given number of slots (initialized to Void).
    pub fn with_slots(count: usize) -> Self {
        Self {
            slots: vec![Value::Void; count],
            marked: false,
        }
    }
}

impl ManagedHeap {
    /// Create a new empty heap.
    pub fn new() -> Self {
        Self {
            // reserve slot 0 for null handle
            cells: vec![None],
            free_list: Vec::new(),
        }
    }

    /// Allocate a new cell and return its handle.
    pub fn allocate(&mut self) -> HeapHandle {
        self.allocate_cell(HeapCell {
            slots: Vec::new(),
            marked: false,
        })
    }

    /// Allocate a cell with a given number of slots (initialized to Void).
    pub fn allocate_with_slots(&mut self, slot_count: usize) -> HeapHandle {
        self.allocate_cell(HeapCell {
            slots: vec![Value::Void; slot_count],
            marked: false,
        })
    }

    /// Allocate a cell with the given slot values.
    pub fn allocate_with_values(&mut self, slots: Vec<Value>) -> HeapHandle {
        self.allocate_cell(HeapCell {
            slots,
            marked: false,
        })
    }

    /// Internal: allocate a cell, reusing free slots if available.
    fn allocate_cell(&mut self, cell: HeapCell) -> HeapHandle {
        if let Some(index) = self.free_list.pop() {
            self.cells[index] = Some(cell);
            HeapHandle::new(index as u64)
        } else {
            let index = self.cells.len();
            self.cells.push(Some(cell));
            HeapHandle::new(index as u64)
        }
    }

    /// Get a cell by handle.
    #[inline]
    pub fn get(&self, handle: HeapHandle) -> Option<&HeapCell> {
        self.cells.get(handle.id() as usize)?.as_ref()
    }

    /// Get a mutable reference to a cell.
    #[inline]
    pub fn get_mut(&mut self, handle: HeapHandle) -> Option<&mut HeapCell> {
        self.cells.get_mut(handle.id() as usize)?.as_mut()
    }

    /// Get a slot of a cell.
    #[inline]
    pub fn get_slot(&self, handle: HeapHandle, index: usize) -> Option<&Value> {
        self.get(handle)?.slots.get(index)
    }

    /// Set a slot of a cell.
    #[inline]
    pub fn set_slot(&mut self, handle: HeapHandle, index: usize, value: Value) -> bool {
        if let Some(cell) = self.get_mut(handle)
            && index < cell.slots.len()
        {
            cell.slots[index] = value;
            return true;
        }
        false
    }

    /// Free a cell (add to free list for reuse).
    #[inline]
    pub fn free(&mut self, handle: HeapHandle) {
        let index = handle.id() as usize;
        if index < self.cells.len() && self.cells[index].is_some() {
            self.cells[index] = None;
            self.free_list.push(index);
        }
    }

    /// Get the number of allocated cells.
    #[inline]
    pub fn cell_count(&self) -> usize {
        // Count non-None cells (excluding reserved slot 0)
        self.cells.iter().skip(1).filter(|c| c.is_some()).count()
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
        self.cells.push(None); // re-reserve slot 0
        self.free_list.clear();
    }

    /// Run basic mark-and-sweep garbage collection.
    ///
    /// Takes a list of root handles (values reachable from the call stack).
    /// Marks all reachable cells, then sweeps (frees) unmarked cells.
    pub fn collect(&mut self, roots: &[HeapHandle]) {
        // reset all marks
        for cell in self.cells.iter_mut().flatten() {
            cell.marked = false;
        }

        // mark phase: mark all reachable cells from roots
        let mut worklist: Vec<HeapHandle> = roots.to_vec();
        while let Some(handle) = worklist.pop() {
            let index = handle.id() as usize;
            if let Some(Some(cell)) = self.cells.get_mut(index) {
                if cell.marked {
                    continue; // already visited
                }
                cell.marked = true;

                // Add any managed references in slots to worklist
                for slot in &cell.slots {
                    Self::collect_handles_from_value(slot, &mut worklist);
                }
            }
        }

        // sweep phase: free all unmarked cells
        for index in 1..self.cells.len() {
            if let Some(cell) = &self.cells[index]
                && !cell.marked
            {
                self.cells[index] = None;
                self.free_list.push(index);
            }
        }
    }

    /// Collect heap handles from a value (recursively for aggregates).
    fn collect_handles_from_value(value: &Value, worklist: &mut Vec<HeapHandle>) {
        match value {
            Value::ManagedReference(handle) => {
                worklist.push(*handle);
            }
            Value::Aggregate(fields) => {
                for field in fields {
                    Self::collect_handles_from_value(field, worklist);
                }
            }
            _ => {}
        }
    }
}

impl RawHeap {
    /// Create a new empty raw heap.
    pub fn new() -> Self {
        Self {
            cells: HashMap::new(),
            next_id: 1, // (start at 1 so 0 can be null)
        }
    }

    /// Allocate a new cell and return its pointer.
    pub fn allocate(&mut self) -> RawPointer {
        self.allocate_cell(HeapCell {
            slots: Vec::new(),
            marked: false,
        })
    }

    /// Allocate a cell with a given number of slots (initialized to Void).
    pub fn allocate_with_slots(&mut self, slot_count: usize) -> RawPointer {
        self.allocate_cell(HeapCell {
            slots: vec![Value::Void; slot_count],
            marked: false,
        })
    }

    /// Internal: allocate a cell with a new ID.
    fn allocate_cell(&mut self, cell: HeapCell) -> RawPointer {
        let id = self.next_id;
        self.next_id += 1;
        self.cells.insert(id, cell);
        RawPointer::new(id)
    }

    /// Get a cell by pointer.
    #[inline]
    pub fn get(&self, pointer: RawPointer) -> Option<&HeapCell> {
        self.cells.get(&pointer.id())
    }

    /// Get a mutable reference to a cell.
    #[inline]
    pub fn get_mut(&mut self, pointer: RawPointer) -> Option<&mut HeapCell> {
        self.cells.get_mut(&pointer.id())
    }

    /// Free a cell by pointer. Returns true if the cell existed.
    #[inline]
    pub fn free(&mut self, pointer: RawPointer) -> bool {
        self.cells.remove(&pointer.id()).is_some()
    }

    /// Get the number of allocated cells.
    #[inline]
    pub fn cell_count(&self) -> usize {
        self.cells.len()
    }

    /// Check if the heap is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }

    /// Clear all allocations.
    #[inline]
    pub fn clear(&mut self) {
        self.cells.clear();
    }
}
