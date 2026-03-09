use std::sync::Arc;

use serde::{Deserialize, Serialize};

use super::page::{PageImage, PageKind, PageReference};
use super::slot::SlotStorage;
use crate::value::{RawPointer, Value};

const FIRST_ALLOCATED_SLOT_ID: u64 = 1;

/// Immutable raw heap image.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawHeapImage {
    /// Captured raw pages.
    pub pages: Arc<[Arc<PageImage<RawCell>>]>,
    /// The next slot id to allocate.
    pub next_unused_id: u64,
    /// The captured free slot ids.
    pub free_list: Arc<[u64]>,
    /// The number of allocated raw cells.
    pub allocated_cells: usize,
}

/// Raw heap storage for either value slots or byte buffers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RawCellStorage {
    /// Slot storage for value based cells.
    Values(SlotStorage),
    /// Byte buffer storage for raw payloads.
    Bytes(Vec<u8>),
}

impl RawCellStorage {
    /// Return the number of slots or bytes.
    pub fn len(&self) -> usize {
        match self {
            Self::Values(slots) => slots.len(),
            Self::Bytes(bytes) => bytes.len(),
        }
    }

    /// Report whether the storage is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// A raw heap cell.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawCell {
    /// The raw storage backing this cell.
    pub storage: RawCellStorage,
}

impl Default for RawCell {
    fn default() -> Self {
        Self::new()
    }
}

impl RawCell {
    /// Create a new empty value cell.
    pub fn new() -> Self {
        Self {
            storage: RawCellStorage::Values(SlotStorage::default()),
        }
    }

    /// Create a value cell with the given slot count.
    pub fn with_slots(count: usize) -> Self {
        Self {
            storage: RawCellStorage::Values(SlotStorage::with_slots(count)),
        }
    }

    /// Create a value cell with the given slot values.
    pub fn with_values(values: Vec<Value>) -> Self {
        Self {
            storage: RawCellStorage::Values(SlotStorage::from_values(values)),
        }
    }

    /// Create a byte cell with the given payload.
    pub fn with_bytes(bytes: Vec<u8>) -> Self {
        Self {
            storage: RawCellStorage::Bytes(bytes),
        }
    }
}

/// A raw heap for manual memory management.
#[derive(Debug)]
pub struct RawHeap {
    /// Raw pages keyed by global slot id.
    pub(super) pages: Vec<PageReference<RawCell>>,
    /// Free global slot ids available for reuse.
    pub(super) free_list: Vec<u64>,
    /// The next global slot id to allocate.
    pub(super) next_unused_id: u64,
    /// Count of live heap cells, excluding the null slot.
    pub(super) allocated_cells: usize,
}

impl Default for RawHeap {
    fn default() -> Self {
        Self::new()
    }
}

impl RawHeap {
    /// Create a new empty raw heap.
    pub fn new() -> Self {
        Self {
            pages: vec![PageReference::new(PageKind::Raw)],
            free_list: Vec::new(),
            next_unused_id: FIRST_ALLOCATED_SLOT_ID,
            allocated_cells: 0,
        }
    }

    /// Capture one immutable raw heap image.
    pub fn image(&mut self) -> RawHeapImage {
        // capture page contents
        let pages = self
            .pages
            .iter_mut()
            .map(PageReference::freeze)
            .collect::<Vec<_>>()
            .into();

        // capture allocator state
        let free_list = self.free_list.clone().into();

        RawHeapImage {
            pages,
            next_unused_id: self.next_unused_id,
            free_list,
            allocated_cells: self.allocated_cells,
        }
    }

    /// Create one raw heap from an immutable image.
    pub fn from_image(image: &RawHeapImage) -> Self {
        // rebuild page storage from the immutable images
        let pages = image
            .pages
            .iter()
            .cloned()
            .map(PageReference::from_image)
            .collect::<Vec<_>>();

        // restore allocator state
        let free_list = image.free_list.iter().copied().collect();

        Self {
            pages,
            free_list,
            next_unused_id: image.next_unused_id,
            allocated_cells: image.allocated_cells,
        }
    }

    /// Allocate a new cell and return its pointer.
    pub fn allocate(&mut self) -> RawPointer {
        self.allocate_cell(RawCell::new())
    }

    /// Allocate a cell with a given number of slots.
    pub fn allocate_with_slots(&mut self, slot_count: usize) -> RawPointer {
        self.allocate_cell(RawCell::with_slots(slot_count))
    }

    /// Allocate a cell with the given slot values.
    pub fn allocate_with_values(&mut self, slots: Vec<Value>) -> RawPointer {
        self.allocate_cell(RawCell::with_values(slots))
    }

    /// Allocate a cell with the given byte payload.
    pub fn allocate_with_bytes(&mut self, bytes: &[u8]) -> RawPointer {
        self.allocate_cell(RawCell::with_bytes(bytes.to_vec()))
    }

    // reserve one slot and write the cell into its page
    fn allocate_cell(&mut self, cell: RawCell) -> RawPointer {
        let slot_id = if let Some(slot_id) = self.free_list.pop() {
            slot_id
        } else {
            let slot_id = self.next_unused_id;
            self.next_unused_id = self.next_unused_id.saturating_add(1);
            self.ensure_slot(slot_id);
            slot_id
        };

        let pointer = RawPointer::new(slot_id);
        let (target_page, target_offset) = pointer.page_position();
        self.pages[target_page].set(target_offset, cell);
        self.allocated_cells += 1;

        pointer
    }

    /// Get a cell by pointer.
    #[inline]
    pub fn get(&self, pointer: RawPointer) -> Option<&RawCell> {
        let slot_id = pointer.id();
        if slot_id == 0 || slot_id >= self.next_unused_id {
            return None;
        }

        let (target_page, target_offset) = pointer.page_position();
        let page = self.pages.get(target_page)?;

        page.get(target_offset)
    }

    /// Get a mutable reference to a cell.
    #[inline]
    pub fn get_mut(&mut self, pointer: RawPointer) -> Option<&mut RawCell> {
        let slot_id = pointer.id();
        if slot_id == 0 || slot_id >= self.next_unused_id {
            return None;
        }

        let (target_page, target_offset) = pointer.page_position();
        let page = self.pages.get_mut(target_page)?;

        page.get_mut(target_offset)
    }

    /// Free a cell by pointer. Returns true if the cell existed.
    #[inline]
    pub fn free(&mut self, pointer: RawPointer) -> bool {
        let slot_id = pointer.id();
        if slot_id == 0 || slot_id >= self.next_unused_id {
            return false;
        }

        let (target_page, target_offset) = pointer.page_position();
        let Some(_) = self.pages[target_page].take(target_offset) else {
            return false;
        };

        self.free_list.push(slot_id);
        debug_assert!(self.allocated_cells > 0, "heap allocation count underflow");
        self.allocated_cells -= 1;
        true
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
        self.pages.clear();
        self.pages.push(PageReference::new(PageKind::Raw));
        self.free_list.clear();
        self.next_unused_id = FIRST_ALLOCATED_SLOT_ID;
        self.allocated_cells = 0;
    }

    // ensure the selected slot has page storage allocated
    fn ensure_slot(&mut self, slot_id: u64) {
        let required_page = RawPointer::new(slot_id).page_index();

        while self.pages.len() <= required_page {
            self.pages.push(PageReference::new(PageKind::Raw));
        }
    }
}
