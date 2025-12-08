use std::collections::HashMap;

use super::value::{HeapHandle, Value};

/// A managed heap for interpreter allocations.
///
/// This is a simple implementation for the interpreter.
/// The runtime will have a more sophisticated GC-based heap.
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
    pub is_marked: bool,
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
                is_marked: false,
            },
        );
        handle
    }

    /// Allocate a cell with a given number of slots (initialized to Void).
    pub fn allocate_with_slots(&mut self, slot_count: usize) -> HeapHandle {
        let handle = HeapHandle::new(self.next_id);
        self.next_id += 1;
        self.cells.insert(
            handle,
            HeapCell {
                slots: vec![Value::Void; slot_count],
                is_marked: false,
            },
        );
        handle
    }

    /// Allocate a cell with the given slot values.
    pub fn allocate_with_values(&mut self, slots: Vec<Value>) -> HeapHandle {
        let handle = HeapHandle::new(self.next_id);
        self.next_id += 1;
        self.cells.insert(
            handle,
            HeapCell {
                slots,
                is_marked: false,
            },
        );
        handle
    }

    /// Get a cell by handle.
    #[inline]
    pub fn get(&self, handle: HeapHandle) -> Option<&HeapCell> {
        self.cells.get(&handle)
    }

    /// Get a mutable reference to a cell.
    #[inline]
    pub fn get_mut(&mut self, handle: HeapHandle) -> Option<&mut HeapCell> {
        self.cells.get_mut(&handle)
    }

    /// Get a slot of a cell.
    #[inline]
    pub fn get_slot(&self, handle: HeapHandle, index: usize) -> Option<&Value> {
        self.cells.get(&handle)?.slots.get(index)
    }

    /// Set a slot of a cell.
    #[inline]
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
    #[inline]
    pub fn free(&mut self, handle: HeapHandle) {
        self.cells.remove(&handle);
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
        self.next_id = 1;
    }

    /// Run mark-and-sweep garbage collection.
    ///
    /// Takes a list of root handles (values reachable from the call stack).
    /// Marks all reachable cells, then sweeps (removes) unmarked cells.
    pub fn collect(&mut self, roots: &[HeapHandle]) {
        // reset all marks
        for cell in self.cells.values_mut() {
            cell.is_marked = false;
        }

        // mark phase: mark all reachable cells from roots
        let mut worklist: Vec<HeapHandle> = roots.to_vec();
        while let Some(handle) = worklist.pop() {
            if let Some(cell) = self.cells.get_mut(&handle) {
                if cell.is_marked {
                    continue; // already visited
                }
                cell.is_marked = true;

                // add any managed references in slots to worklist
                for slot in &cell.slots {
                    if let Value::ManagedReference(child_handle) = slot {
                        worklist.push(*child_handle);
                    }
                    // also check inside aggregates
                    if let Value::Aggregate(fields) = slot {
                        Self::collect_handles_from_aggregate(fields, &mut worklist);
                    }
                }
            }
        }

        // sweep phase: remove all unmarked cells
        self.cells.retain(|_, cell| cell.is_marked);
    }

    /// Collect heap handles from an aggregate value.
    fn collect_handles_from_aggregate(values: &[Value], worklist: &mut Vec<HeapHandle>) {
        for value in values {
            match value {
                Value::ManagedReference(handle) => {
                    worklist.push(*handle);
                }
                Value::Aggregate(fields) => {
                    Self::collect_handles_from_aggregate(fields, worklist);
                }
                _ => {}
            }
        }
    }
}
