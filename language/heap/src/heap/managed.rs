use std::sync::Arc;

use serde::{Deserialize, Serialize};

use super::page::{PageImage, PageKind, PageReference};
use super::slot::{HeapCell, SlotStorage};
use super::{GcPhase, GcState, GcStats};
use crate::value::{ManagedPointer, Value};

const CELL_HEADER_BYTES: u64 = 24;
const VALUE_BYTES: u64 = std::mem::size_of::<Value>() as u64;
const FIRST_ALLOCATED_SLOT_ID: u64 = 1;

/// Immutable managed heap snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManagedHeapSnapshot {
    /// Captured managed pages.
    pub pages: Arc<[Arc<PageImage<HeapCell>>]>,
    /// The next slot id to allocate.
    pub next_unused_id: u64,
    /// The captured free slot ids.
    pub free_list: Arc<[u64]>,
    /// The number of allocated managed cells.
    pub allocated_cells: usize,
    /// The number of allocated managed bytes.
    pub allocated_bytes: u64,
    /// The captured gc state.
    pub gc_state: GcState,
}

/// A managed heap for interpreter allocations.
#[derive(Debug)]
pub struct ManagedHeap {
    /// Managed pages keyed by global slot id.
    pub(super) pages: Vec<PageReference<HeapCell>>,
    /// Free global slot ids available for reuse.
    pub(super) free_list: Vec<u64>,
    /// The next global slot id to allocate.
    pub(super) next_unused_id: u64,
    /// Count of live heap cells, excluding the null slot.
    pub(super) allocated_cells: usize,
    /// Approximate heap bytes in use.
    pub(super) allocated_bytes: u64,
    /// GC phase and cycle state.
    pub(super) gc_state: GcState,
    /// Pending mark worklist.
    pub(super) mark_queue: Vec<ManagedPointer>,
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
            pages: vec![PageReference::new(PageKind::Managed)],
            free_list: Vec::new(),
            next_unused_id: FIRST_ALLOCATED_SLOT_ID,
            allocated_cells: 0,
            allocated_bytes: 0,
            gc_state: GcState::default(),
            mark_queue: Vec::new(),
        }
    }

    /// Capture one immutable managed heap snapshot.
    pub fn snapshot(&mut self) -> ManagedHeapSnapshot {
        debug_assert!(
            self.is_checkpoint_ready(),
            "managed heap snapshot requires idle gc state"
        );

        // capture page contents
        let pages = self
            .pages
            .iter_mut()
            .map(PageReference::snapshot)
            .collect::<Vec<_>>()
            .into();

        // capture allocator and gc state
        let free_list = self.free_list.clone().into();

        ManagedHeapSnapshot {
            pages,
            next_unused_id: self.next_unused_id,
            free_list,
            allocated_cells: self.allocated_cells,
            allocated_bytes: self.allocated_bytes,
            gc_state: self.gc_state.clone(),
        }
    }

    /// Restore one managed heap from an immutable snapshot.
    pub fn restore(snapshot: &ManagedHeapSnapshot) -> Self {
        // rebuild page storage from the immutable images
        let pages = snapshot
            .pages
            .iter()
            .cloned()
            .map(PageReference::from_image)
            .collect::<Vec<_>>();

        // restore allocator and gc state
        let free_list = snapshot.free_list.iter().copied().collect();

        Self {
            pages,
            free_list,
            next_unused_id: snapshot.next_unused_id,
            allocated_cells: snapshot.allocated_cells,
            allocated_bytes: snapshot.allocated_bytes,
            gc_state: snapshot.gc_state.clone(),
            mark_queue: Vec::new(),
        }
    }

    /// Return the GC state.
    pub fn gc_state(&self) -> &GcState {
        &self.gc_state
    }

    /// Report whether this heap is ready for checkpoint capture.
    pub fn is_checkpoint_ready(&self) -> bool {
        self.gc_state.phase == GcPhase::Idle && self.mark_queue.is_empty()
    }

    /// Return a mutable reference to the GC state.
    pub fn gc_state_mut(&mut self) -> &mut GcState {
        &mut self.gc_state
    }

    /// Return the last completed GC stats snapshot.
    pub fn gc_stats(&self) -> Option<&GcStats> {
        self.gc_state.last_stats.as_ref()
    }

    /// Return the current heap byte estimate.
    pub fn heap_bytes(&self) -> u64 {
        self.allocated_bytes
    }

    /// Return the current heap cell count.
    pub fn cell_count(&self) -> usize {
        self.allocated_cells
    }

    /// Check whether a handle points to a live heap cell.
    pub fn is_allocated(&self, handle: ManagedPointer) -> bool {
        self.get(handle).is_some()
    }

    /// Check if the heap is empty.
    pub fn is_empty(&self) -> bool {
        self.cell_count() == 0
    }

    /// Allocate a new cell and return its handle.
    pub fn allocate(&mut self) -> ManagedPointer {
        self.allocate_cell(HeapCell::new())
    }

    /// Allocate a cell with a given number of slots.
    pub fn allocate_with_slots(&mut self, slot_count: usize) -> ManagedPointer {
        self.allocate_cell(HeapCell::with_slots(slot_count))
    }

    /// Allocate a cell with the given slot values.
    pub fn allocate_with_values(&mut self, slots: Vec<Value>) -> ManagedPointer {
        self.allocate_cell(HeapCell {
            slots: SlotStorage::from_values(slots),
        })
    }

    /// Allocate a cell with exactly 2 slot values.
    #[inline]
    pub fn allocate_pair(&mut self, first: Value, second: Value) -> ManagedPointer {
        self.allocate_cell(HeapCell {
            slots: SlotStorage::from_pair(first, second),
        })
    }

    /// Allocate a cell with exactly 1 slot value.
    #[inline]
    pub fn allocate_single(&mut self, value: Value) -> ManagedPointer {
        self.allocate_cell(HeapCell {
            slots: SlotStorage::from_single(value),
        })
    }

    // reserve one slot and write the cell into its page
    #[inline]
    fn allocate_cell(&mut self, cell: HeapCell) -> ManagedPointer {
        let cell_bytes = Self::cell_size_for(&cell);

        let slot_id = if let Some(slot_id) = self.free_list.pop() {
            slot_id
        } else {
            let slot_id = self.next_unused_id;
            self.next_unused_id = self.next_unused_id.saturating_add(1);
            self.ensure_slot(slot_id);
            slot_id
        };

        let handle = ManagedPointer::new(slot_id);
        let target_page = handle.page_index();
        let target_offset = handle.page_offset();
        self.pages[target_page].set(target_offset, cell);

        self.allocated_cells += 1;
        self.allocated_bytes = self.allocated_bytes.saturating_add(cell_bytes);

        // keep new allocations visible to the current mark phase
        if self.gc_state.phase.is_marking() {
            self.mark_pointer(handle);
        }

        handle
    }

    /// Get a cell by handle.
    #[inline(always)]
    pub fn get(&self, handle: ManagedPointer) -> Option<&HeapCell> {
        let slot_id = handle.id();
        if slot_id == 0 || slot_id >= self.next_unused_id {
            return None;
        }

        let target_page = handle.page_index();
        let target_offset = handle.page_offset();
        let page = self.pages.get(target_page)?;

        page.get(target_offset)
    }

    /// Get a mutable reference to a cell.
    #[inline(always)]
    pub fn get_mut(&mut self, handle: ManagedPointer) -> Option<&mut HeapCell> {
        let slot_id = handle.id();
        if slot_id == 0 || slot_id >= self.next_unused_id {
            return None;
        }

        let target_page = handle.page_index();
        let target_offset = handle.page_offset();
        let page = self.pages.get_mut(target_page)?;

        page.get_mut(target_offset)
    }

    /// Get a cell by handle without bounds checks.
    ///
    /// # Safety
    /// Caller must ensure the handle points to a valid allocated cell.
    #[inline(always)]
    pub unsafe fn get_unchecked(&self, handle: ManagedPointer) -> &HeapCell {
        let slot_id = handle.id();
        let target_page = handle.page_index();
        let target_offset = handle.page_offset();

        debug_assert!(slot_id != 0, "managed pointer is null");
        debug_assert!(
            slot_id < self.next_unused_id,
            "managed pointer is out of bounds"
        );
        debug_assert!(target_page < self.pages.len(), "heap page out of bounds");
        debug_assert!(
            self.pages[target_page].is_occupied(target_offset),
            "managed pointer points to freed cell"
        );

        unsafe {
            self.pages
                .get_unchecked(target_page)
                .get_unchecked(target_offset)
        }
    }

    /// Get a mutable cell by handle without bounds checks.
    ///
    /// # Safety
    /// Caller must ensure the handle points to a valid allocated cell.
    #[inline(always)]
    pub unsafe fn get_unchecked_mut(&mut self, handle: ManagedPointer) -> &mut HeapCell {
        let slot_id = handle.id();
        let target_page = handle.page_index();
        let target_offset = handle.page_offset();

        debug_assert!(slot_id != 0, "managed pointer is null");
        debug_assert!(
            slot_id < self.next_unused_id,
            "managed pointer is out of bounds"
        );
        debug_assert!(target_page < self.pages.len(), "heap page out of bounds");
        debug_assert!(
            self.pages[target_page].is_occupied(target_offset),
            "managed pointer points to freed cell"
        );

        unsafe {
            self.pages
                .get_unchecked_mut(target_page)
                .get_unchecked_mut(target_offset)
        }
    }

    /// Get an immutable slot value.
    #[inline(always)]
    pub fn get_slot(&self, handle: ManagedPointer, index: usize) -> Option<&Value> {
        let cell = self.get(handle)?;
        cell.slots.get(index)
    }

    /// Get a mutable slot value.
    #[inline(always)]
    pub fn get_slot_mut(&mut self, handle: ManagedPointer, index: usize) -> Option<&mut Value> {
        let cell = self.get_mut(handle)?;
        cell.slots.get_mut(index)
    }

    /// Set a slot value.
    #[inline(always)]
    pub fn set_slot(&mut self, handle: ManagedPointer, index: usize, value: Value) -> bool {
        let source_marked = self.is_marked(handle);
        self.write_barrier(source_marked, value);

        let Some(slot) = self.get_slot_mut(handle, index) else {
            return false;
        };

        *slot = value;
        true
    }

    /// Resize the slot storage for a cell.
    pub fn resize_slots(&mut self, handle: ManagedPointer, len: usize) -> bool {
        let Some(cell) = self.get_mut(handle) else {
            return false;
        };

        let old_bytes = Self::cell_size_for(cell);
        cell.slots.resize(len, Value::VOID);
        let new_bytes = Self::cell_size_for(cell);

        if new_bytes >= old_bytes {
            self.allocated_bytes = self
                .allocated_bytes
                .saturating_add(new_bytes.saturating_sub(old_bytes));
        } else {
            self.allocated_bytes = self
                .allocated_bytes
                .saturating_sub(old_bytes.saturating_sub(new_bytes));
        }

        true
    }

    /// Free a cell by handle.
    pub fn free(&mut self, handle: ManagedPointer) -> bool {
        let slot_id = handle.id();
        if slot_id == 0 || slot_id >= self.next_unused_id {
            return false;
        }

        let target_page = handle.page_index();
        let target_offset = handle.page_offset();

        let Some(cell) = self.pages[target_page].take(target_offset) else {
            return false;
        };

        self.free_list.push(slot_id);
        debug_assert!(self.allocated_cells > 0, "heap allocation count underflow");
        self.allocated_cells -= 1;
        self.allocated_bytes = self
            .allocated_bytes
            .saturating_sub(Self::cell_size_for(&cell));

        true
    }

    /// Clear all allocations.
    pub fn clear(&mut self) {
        self.pages.clear();
        self.pages.push(PageReference::new(PageKind::Managed));
        self.free_list.clear();
        self.next_unused_id = FIRST_ALLOCATED_SLOT_ID;
        self.allocated_cells = 0;
        self.allocated_bytes = 0;
        self.gc_state = GcState::default();
        self.mark_queue.clear();
    }

    // ensure the selected slot has page storage allocated
    fn ensure_slot(&mut self, slot_id: u64) {
        let required_page = ManagedPointer::new(slot_id).page_index();

        while self.pages.len() <= required_page {
            self.pages.push(PageReference::new(PageKind::Managed));
        }
    }

    /// Begin one GC cycle.
    pub fn begin_gc_cycle(&mut self, roots: impl IntoIterator<Item = Value>) {
        if self.gc_state.phase != GcPhase::Idle {
            return;
        }

        // start one new cycle and clear stale page marks
        self.gc_state.begin_cycle();
        self.mark_queue.clear();

        for page in &mut self.pages {
            page.clear_marks();
        }

        // seed the mark queue from the provided roots
        for root in roots {
            self.mark_value(root);
        }
    }

    /// Advance one incremental GC step.
    pub fn gc_step(&mut self, budget: usize) {
        let mut remaining = budget;
        if remaining == 0 {
            return;
        }

        // mark phase
        if self.gc_state.phase == GcPhase::Mark {
            while remaining > 0 {
                let Some(handle) = self.mark_queue.pop() else {
                    self.gc_state.phase = GcPhase::Sweep;
                    break;
                };

                remaining -= 1;
                self.trace_pointer(handle);
            }
        }

        // sweep phase
        if self.gc_state.phase == GcPhase::Sweep {
            self.finish_gc_cycle();
        }
    }

    /// Run one full GC cycle immediately.
    pub fn collect_handles(&mut self, roots: impl IntoIterator<Item = ManagedPointer>) -> GcStats {
        self.begin_gc_cycle(roots.into_iter().map(Value::managed_reference));
        self.finish_gc_cycle();

        self.gc_state.last_stats.unwrap_or_default()
    }

    /// Complete one GC cycle immediately.
    pub fn finish_gc_cycle(&mut self) {
        if self.gc_state.phase == GcPhase::Idle {
            return;
        }

        // drain remaining mark work before sweeping
        while let Some(pointer) = self.mark_queue.pop() {
            self.trace_pointer(pointer);
        }

        // sweep all currently allocated slots
        let mut freed = 0_usize;
        let previous_next_unused_id = self.next_unused_id;
        for slot_id in FIRST_ALLOCATED_SLOT_ID..previous_next_unused_id {
            let handle = ManagedPointer::new(slot_id);
            let target_page = handle.page_index();
            let target_offset = handle.page_offset();
            let is_occupied = self.pages[target_page].is_occupied(target_offset);
            let should_free = is_occupied && !self.pages[target_page].is_marked(target_offset);
            if !should_free {
                continue;
            }

            let cell = self.pages[target_page]
                .take(target_offset)
                .expect("sweep expected occupied managed cell");
            self.free_list.push(slot_id);
            self.allocated_cells = self.allocated_cells.saturating_sub(1);
            self.allocated_bytes = self
                .allocated_bytes
                .saturating_sub(Self::cell_size_for(&cell));
            freed += 1;
        }

        // publish cycle stats and return to idle
        let stats = GcStats {
            freed_cells: freed,
            live_cells: self.allocated_cells,
            freed_bytes: 0,
            live_bytes: self.allocated_bytes,
            heap_bytes: self.allocated_bytes,
        };

        self.gc_state.finish_cycle(stats);
        self.mark_queue.clear();
    }

    /// Mark one pointer if it has not already been marked this cycle.
    pub fn mark_pointer(&mut self, pointer: ManagedPointer) {
        let slot_id = pointer.id();
        if slot_id == 0 || slot_id >= self.next_unused_id {
            return;
        }

        let target_page = pointer.page_index();
        let target_offset = pointer.page_offset();
        let page = &mut self.pages[target_page];
        if !page.is_occupied(target_offset) || page.is_marked(target_offset) {
            return;
        }

        page.mark(target_offset);
        self.mark_queue.push(pointer);
    }

    /// Mark one runtime value if it contains a managed pointer.
    pub fn mark_value(&mut self, value: Value) {
        if let Some(pointer) = value.as_managed_pointer() {
            self.mark_pointer(pointer);
        }
    }

    /// Return whether one pointer is marked in the current cycle.
    pub fn is_marked(&self, pointer: ManagedPointer) -> bool {
        let slot_id = pointer.id();
        if slot_id == 0 || slot_id >= self.next_unused_id {
            return false;
        }

        let target_page = pointer.page_index();
        let target_offset = pointer.page_offset();
        self.pages[target_page].is_marked(target_offset)
    }

    // maintain tri color invariant for in place pointer writes
    fn write_barrier(&mut self, source_marked: bool, value: Value) {
        if !source_marked || !self.gc_state.phase.is_marking() {
            return;
        }

        self.mark_value(value);
    }

    // trace one pointer's outgoing references
    fn trace_pointer(&mut self, pointer: ManagedPointer) {
        let Some(cell) = self.get(pointer) else {
            return;
        };

        let mut values = Vec::with_capacity(cell.slots.len());
        values.extend(cell.slots.as_slice().iter().copied());

        for value in values {
            self.mark_value(value);
        }
    }

    /// Return a cloned list of the currently allocated pointers.
    pub fn allocated_pointers(&self) -> Vec<ManagedPointer> {
        let mut pointers = Vec::with_capacity(self.allocated_cells);
        let previous_next_unused_id = self.next_unused_id;

        for slot_id in FIRST_ALLOCATED_SLOT_ID..previous_next_unused_id {
            let pointer = ManagedPointer::new(slot_id);
            let target_page = pointer.page_index();
            let target_offset = pointer.page_offset();
            if self.pages[target_page].is_occupied(target_offset) {
                pointers.push(pointer);
            }
        }

        pointers
    }

    /// Return the approximate byte size for one cell.
    pub fn cell_size(&self, pointer: ManagedPointer) -> Option<u64> {
        let cell = self.get(pointer)?;
        Some(Self::cell_size_for(cell))
    }

    /// Return the approximate byte size for one cell.
    pub fn cell_size_for(cell: &HeapCell) -> u64 {
        let slot_bytes = VALUE_BYTES.saturating_mul(cell.slots.len() as u64);
        CELL_HEADER_BYTES.saturating_add(slot_bytes)
    }
}
