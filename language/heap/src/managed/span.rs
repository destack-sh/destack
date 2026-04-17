use serde::{Deserialize, Serialize};

use super::MapId;
use crate::StorageLayoutId;
use crate::alloc::{Bitmap, PageView};
use crate::gc::CardSet;

/// One frozen managed span root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct SmallSpanImage {
    /// The size class for this span in bytes.
    pub size_class: usize,
    /// The number of slots in this span.
    pub slot_count: usize,
    /// The occupied slots in this span.
    pub occupied: Bitmap,
    /// The per-slot reference-map ids for this span.
    pub map_ids: Box<[u32]>,
    /// The per-slot layout ids for this span.
    pub layout_ids: Box<[Option<StorageLayoutId>]>,
    /// The arena pages for this span.
    pub pages: PageView,
}

/// One live managed span.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SmallSpan {
    /// The slot payload size in bytes.
    pub(crate) size_class: usize,
    /// The number of slots in this span.
    pub(crate) slot_count: usize,
    /// The number of occupied slots in this span.
    pub(crate) occupied_count: usize,
    /// The next likely free slot.
    pub(crate) next_free_slot: usize,
    /// The occupied slots in this span.
    pub(crate) occupied: Bitmap,
    /// The per-slot reference-map ids for this span.
    pub(crate) map_ids: Box<[MapId]>,
    /// The per-slot layout ids for this span.
    pub(crate) layout_ids: Box<[Option<StorageLayoutId>]>,
    /// The arena pages for this span.
    pub(crate) pages: PageView,
    /// The dirty cards remembered for young tracing.
    pub(crate) dirty_cards: CardSet,
    /// Whether this span is already queued for dirty-card scanning.
    pub(crate) is_dirty_queued: bool,
}

impl SmallSpan {
    /// Return the layout id for one slot in this span.
    pub(crate) fn layout_id(&self, slot_index: usize) -> Option<StorageLayoutId> {
        self.layout_ids.get(slot_index).copied().flatten()
    }

    /// Set the layout id for one slot in this span.
    pub(crate) fn set_layout_id(&mut self, slot_index: usize, layout_id: Option<StorageLayoutId>) {
        if let Some(entry) = self.layout_ids.get_mut(slot_index) {
            *entry = layout_id;
        }
    }

    /// Set the reference-map id for one slot in this span.
    pub(crate) fn set_map_id(&mut self, slot_index: usize, map_id: MapId) {
        if let Some(entry) = self.map_ids.get_mut(slot_index) {
            *entry = map_id;
        }
    }
}
