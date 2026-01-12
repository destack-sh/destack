use super::slot::{HeapCell, SlotStorage};
use super::value::{HeapHandle, Value};

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
    /// Count of live heap cells (excluding the null slot).
    allocated_cells: usize,
}

impl ManagedHeap {
    /// Create a new empty heap.
    pub fn new() -> Self {
        Self {
            // reserve slot 0 for null handle
            cells: vec![None],
            free_list: Vec::new(),
            allocated_cells: 0,
        }
    }

    /// Allocate a new cell and return its handle.
    pub fn allocate(&mut self) -> HeapHandle {
        self.allocate_cell(HeapCell::new())
    }

    /// Allocate a cell with a given number of slots (initialized to Void).
    pub fn allocate_with_slots(&mut self, slot_count: usize) -> HeapHandle {
        self.allocate_cell(HeapCell::with_slots(slot_count))
    }

    /// Allocate a cell with the given slot values.
    pub fn allocate_with_values(&mut self, slots: Vec<Value>) -> HeapHandle {
        self.allocate_cell(HeapCell {
            slots: SlotStorage::from_values(slots),
            marked: false,
        })
    }

    /// Allocate a cell with exactly 2 slot values (avoids Vec allocation).
    #[inline]
    pub fn allocate_pair(&mut self, first: Value, second: Value) -> HeapHandle {
        self.allocate_cell(HeapCell {
            slots: SlotStorage::from_pair(first, second),
            marked: false,
        })
    }

    /// Allocate a cell with exactly 1 slot value (avoids Vec allocation).
    #[inline]
    pub fn allocate_single(&mut self, value: Value) -> HeapHandle {
        self.allocate_cell(HeapCell {
            slots: SlotStorage::from_single(value),
            marked: false,
        })
    }

    /// Internal: allocate a cell, reusing free slots if available.
    #[inline]
    fn allocate_cell(&mut self, cell: HeapCell) -> HeapHandle {
        if let Some(index) = self.free_list.pop() {
            self.cells[index] = Some(cell);
            self.allocated_cells += 1;
            HeapHandle::new(index as u64)
        } else {
            let index = self.cells.len();
            self.cells.push(Some(cell));
            self.allocated_cells += 1;
            HeapHandle::new(index as u64)
        }
    }

    /// Get a cell by handle.
    #[inline(always)]
    pub fn get(&self, handle: HeapHandle) -> Option<&HeapCell> {
        self.cells.get(handle.id() as usize)?.as_ref()
    }

    /// Get a mutable reference to a cell.
    #[inline(always)]
    pub fn get_mut(&mut self, handle: HeapHandle) -> Option<&mut HeapCell> {
        self.cells.get_mut(handle.id() as usize)?.as_mut()
    }

    /// Get a cell by handle without bounds checks.
    ///
    /// # Safety
    /// Caller must ensure the handle points to a valid allocated cell.
    #[inline(always)]
    pub unsafe fn get_unchecked(&self, handle: HeapHandle) -> &HeapCell {
        let index = handle.id() as usize;
        debug_assert!(index < self.cells.len(), "heap handle out of bounds");
        debug_assert!(
            unsafe { self.cells.get_unchecked(index).is_some() },
            "heap handle points to freed cell"
        );
        unsafe { self.cells.get_unchecked(index).as_ref().unwrap_unchecked() }
    }

    /// Get a mutable reference to a cell without bounds checks.
    ///
    /// # Safety
    /// Caller must ensure the handle points to a valid allocated cell.
    #[inline(always)]
    pub unsafe fn get_unchecked_mut(&mut self, handle: HeapHandle) -> &mut HeapCell {
        let index = handle.id() as usize;
        debug_assert!(index < self.cells.len(), "heap handle out of bounds");
        debug_assert!(
            unsafe { self.cells.get_unchecked(index).is_some() },
            "heap handle points to freed cell"
        );
        unsafe {
            self.cells
                .get_unchecked_mut(index)
                .as_mut()
                .unwrap_unchecked()
        }
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
            && let Some(slot) = cell.slots.get_mut(index)
        {
            *slot = value;
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
            debug_assert!(self.allocated_cells > 0, "heap allocation count underflow");
            self.allocated_cells -= 1;
        }
    }

    /// Get the number of allocated cells.
    #[inline]
    pub fn cell_count(&self) -> usize {
        self.allocated_cells
    }

    /// Check whether a handle points to a live heap cell.
    #[inline]
    pub fn is_allocated(&self, handle: HeapHandle) -> bool {
        self.cells
            .get(handle.id() as usize)
            .and_then(|cell| cell.as_ref())
            .is_some()
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
        self.cells.push(None); // reserve slot 0 again
        self.free_list.clear();
        self.allocated_cells = 0;
    }

    /// Run basic mark-and-sweep garbage collection.
    ///
    /// Takes a list of root handles (values reachable from the call stack).
    /// Marks all reachable cells, then sweeps (frees) unmarked cells.
    pub fn collect(&mut self, roots: &[HeapHandle]) -> usize {
        let before = self.allocated_cells;

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

                // add any managed references in slots to worklist
                for slot in &cell.slots {
                    Self::collect_handles_from_value(slot, &mut worklist);
                }
            }
        }

        // sweep phase: free all unmarked cells
        for index in 1..self.cells.len() {
            let should_free = matches!(self.cells[index].as_ref(), Some(cell) if !cell.marked);
            if should_free {
                self.cells[index] = None;
                self.free_list.push(index);
                debug_assert!(self.allocated_cells > 0, "heap allocation count underflow");
                self.allocated_cells -= 1;
            }
        }

        // return freed cell count
        before.saturating_sub(self.allocated_cells)
    }

    /// Collect heap handles from a value (recursively for aggregates).
    fn collect_handles_from_value(value: &Value, worklist: &mut Vec<HeapHandle>) {
        if let Some(handle) = value.as_heap_handle() {
            worklist.push(handle);
        }
    }
}
