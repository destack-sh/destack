use super::slot::{HeapCell, SlotStorage};
use crate::gc::{GcPhase, GcState, GcStats};
use crate::value::{HeapHandle, Value};

const CELL_HEADER_BYTES: u64 = 24;
const VALUE_BYTES: u64 = std::mem::size_of::<Value>() as u64;

/// A managed heap for interpreter allocations (GC-tracked).
#[derive(Debug)]
pub struct ManagedHeap {
    /// Allocated cells. Index 0 is reserved (null handle).
    cells: Vec<Option<HeapCell>>,
    /// Free slot indices available for reuse.
    free_list: Vec<usize>,
    /// Count of live heap cells (excluding the null slot).
    allocated_cells: usize,
    /// Approximate heap bytes in use.
    allocated_bytes: u64,
    /// GC state and pacing targets.
    gc_state: GcState,
    /// Pending mark worklist.
    mark_queue: Vec<HeapHandle>,
}

impl Default for ManagedHeap {
    fn default() -> Self {
        Self::new()
    }
}

impl ManagedHeap {
    /// Create a new empty heap.
    pub fn new() -> Self {
        Self {
            // reserve slot 0 for null handle
            cells: vec![None],
            free_list: Vec::new(),
            allocated_cells: 0,
            allocated_bytes: 0,
            gc_state: GcState::default(),
            mark_queue: Vec::new(),
        }
    }

    /// Return the GC state.
    pub fn gc_state(&self) -> &GcState {
        &self.gc_state
    }

    /// Return a mutable reference to the GC state.
    pub fn gc_state_mut(&mut self) -> &mut GcState {
        &mut self.gc_state
    }

    /// Report the current heap byte estimate.
    pub fn heap_bytes(&self) -> u64 {
        self.allocated_bytes
    }

    /// Report the current heap cell count.
    pub fn cell_count(&self) -> usize {
        self.allocated_cells
    }

    /// Check whether a handle points to a live heap cell.
    pub fn is_allocated(&self, handle: HeapHandle) -> bool {
        self.cells
            .get(handle.id() as usize)
            .and_then(|cell| cell.as_ref())
            .is_some()
    }

    /// Check if the heap is empty.
    pub fn is_empty(&self) -> bool {
        self.cell_count() == 0
    }

    /// Determine whether a GC cycle should start.
    pub fn should_collect(&mut self) -> bool {
        // refresh pacing targets based on the current heap size
        self.gc_state
            .pacer
            .update(self.gc_state.options, self.allocated_bytes);

        // decide based on the current trigger threshold
        self.gc_state.pacer.should_start(self.allocated_bytes)
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
        // reserve a slot for the cell
        let handle = if let Some(index) = self.free_list.pop() {
            self.cells[index] = Some(cell);
            HeapHandle::new(index as u64)
        } else {
            let index = self.cells.len();
            self.cells.push(Some(cell));
            HeapHandle::new(index as u64)
        };

        // update allocation counters
        let cell_bytes = self.cell_size_bytes(handle);
        self.allocated_cells += 1;
        self.allocated_bytes = self.allocated_bytes.saturating_add(cell_bytes);

        // ensure new objects are visible during marking
        if self.gc_state.phase.is_marking() {
            self.mark_handle(handle);
        }

        handle
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
        // apply write barrier before mutating the cell
        let is_marked = self
            .cells
            .get(handle.id() as usize)
            .and_then(|cell| cell.as_ref())
            .map(|cell| cell.marked)
            .unwrap_or(false);
        self.write_barrier(is_marked, value);

        // write the value into the slot
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
        if let Some(cell) = self.cells.get_mut(index).and_then(|cell| cell.take()) {
            // update allocation counters
            let cell_bytes = Self::cell_size_for(&cell);
            self.allocated_bytes = self.allocated_bytes.saturating_sub(cell_bytes);
            self.allocated_cells = self.allocated_cells.saturating_sub(1);

            // keep the slot for reuse
            self.free_list.push(index);
        }
    }

    /// Clear all allocations.
    #[inline]
    pub fn clear(&mut self) {
        // reset the heap storage
        self.cells.clear();
        self.cells.push(None);
        self.free_list.clear();

        // reset counters
        self.allocated_cells = 0;
        self.allocated_bytes = 0;
        self.mark_queue.clear();
    }

    /// Run Go-style tri-color mark-and-sweep garbage collection.
    ///
    /// Takes a list of root handles (values reachable from the call stack).
    pub fn collect(&mut self, roots: &[HeapHandle]) -> GcStats {
        let before_cells = self.allocated_cells;
        let before_bytes = self.allocated_bytes;

        // begin the GC cycle and update pacing targets
        self.gc_state.begin_cycle(before_bytes);

        // reset marks and seed the worklist
        self.prepare_marking(roots);

        // drain marking work
        self.drain_mark_queue();
        self.gc_state.phase = GcPhase::MarkTermination;
        self.drain_mark_queue();

        // sweep unreachable cells
        self.gc_state.phase = GcPhase::Sweep;
        let (freed_cells, freed_bytes) = self.sweep();

        // record stats and finish the cycle
        let live_cells = self.allocated_cells;
        let live_bytes = self.allocated_bytes;
        let stats = GcStats {
            freed_cells,
            live_cells,
            freed_bytes,
            live_bytes,
            heap_bytes: live_bytes,
        };

        self.gc_state.finish_cycle(stats);

        debug_assert!(
            live_cells <= before_cells && live_bytes <= before_bytes,
            "gc increased heap usage"
        );

        stats
    }

    /// Prepare mark bits and seed the mark queue with roots.
    fn prepare_marking(&mut self, roots: &[HeapHandle]) {
        // clear all marks
        for cell in self.cells.iter_mut().flatten() {
            cell.marked = false;
        }

        // reset the worklist
        self.mark_queue.clear();

        // seed roots into the marking queue
        for &handle in roots {
            self.mark_handle(handle);
        }
    }

    /// Mark a handle and enqueue it for scanning.
    fn mark_handle(&mut self, handle: HeapHandle) {
        let index = handle.id() as usize;
        if let Some(Some(cell)) = self.cells.get_mut(index) {
            if cell.marked {
                return;
            }

            cell.marked = true;
            self.mark_queue.push(handle);
        }
    }

    /// Drain the mark queue until it is empty.
    fn drain_mark_queue(&mut self) {
        while let Some(handle) = self.mark_queue.pop() {
            // snapshot slot data without holding a mutable borrow
            let (slots_ptr, slots_len) = match self.cells.get(handle.id() as usize) {
                Some(Some(cell)) => (cell.slots.as_slice().as_ptr(), cell.slots.len()),
                _ => continue,
            };

            // scan all slots for managed references
            for index in 0..slots_len {
                let value = unsafe { *slots_ptr.add(index) };
                if let Some(child) = value.as_heap_handle() {
                    self.mark_handle(child);
                }
            }
        }
    }

    /// Apply the write barrier for a pointer store.
    fn write_barrier(&mut self, source_marked: bool, value: Value) {
        if !self.gc_state.phase.is_marking() {
            return;
        }

        if !source_marked {
            return;
        }

        if let Some(handle) = value.as_heap_handle() {
            self.mark_handle(handle);
        }
    }

    /// Sweep all unmarked cells and reclaim their slots.
    fn sweep(&mut self) -> (usize, u64) {
        let mut freed_cells = 0;
        let mut freed_bytes: u64 = 0;

        for index in 1..self.cells.len() {
            let should_free = matches!(self.cells[index].as_ref(), Some(cell) if !cell.marked);
            if !should_free {
                continue;
            }

            if let Some(cell) = self.cells[index].take() {
                freed_cells += 1;
                freed_bytes = freed_bytes.saturating_add(Self::cell_size_for(&cell));
                self.free_list.push(index);
                self.allocated_cells = self.allocated_cells.saturating_sub(1);
            }
        }

        self.allocated_bytes = self.allocated_bytes.saturating_sub(freed_bytes);

        (freed_cells, freed_bytes)
    }

    /// Estimate the size of a cell by handle.
    fn cell_size_bytes(&self, handle: HeapHandle) -> u64 {
        self.cells
            .get(handle.id() as usize)
            .and_then(|cell| cell.as_ref())
            .map(Self::cell_size_for)
            .unwrap_or(0)
    }

    /// Estimate the size of a cell by value.
    fn cell_size_for(cell: &HeapCell) -> u64 {
        let slot_count = cell.slots.len() as u64;
        let slot_bytes = slot_count.saturating_mul(VALUE_BYTES);
        CELL_HEADER_BYTES.saturating_add(slot_bytes)
    }
}
