use serde::{Deserialize, Serialize};

use super::{CardSet, EdgeId};
use crate::LayoutId;
use crate::arena::{Bitmap, PageView};

/// One frozen managed span root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct SmallSpanImage {
    /// The size class for this span in bytes.
    pub size_class: usize,
    /// The number of slots in this span.
    pub slot_count: usize,
    /// The occupied slots in this span.
    pub occupied: Bitmap,
    /// The per-slot edge-map ids for this span.
    pub edge_ids: Box<[u32]>,
    /// The per-slot layout ids for this span.
    pub layout_ids: Box<[Option<LayoutId>]>,
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
    /// The per-slot edge-map ids for this span.
    pub(crate) edge_ids: Box<[EdgeId]>,
    /// The per-slot layout ids for this span.
    pub(crate) layout_ids: Box<[Option<LayoutId>]>,
    /// The arena pages for this span.
    pub(crate) pages: PageView,
    /// The dirty cards remembered for young tracing.
    pub(crate) dirty_cards: CardSet,
    /// Whether this span is already queued for dirty-card scanning.
    pub(crate) is_dirty_queued: bool,
}

impl SmallSpan {
    /// Return the layout id for one slot in this span.
    pub(crate) fn layout_id(&self, slot_index: usize) -> Option<LayoutId> {
        self.layout_ids.get(slot_index).copied().flatten()
    }

    /// Set the layout id for one slot in this span.
    pub(crate) fn set_layout_id(&mut self, slot_index: usize, layout_id: Option<LayoutId>) {
        if let Some(entry) = self.layout_ids.get_mut(slot_index) {
            *entry = layout_id;
        }
    }

    /// Set the edge-map id for one slot in this span.
    pub(crate) fn set_edge_id(&mut self, slot_index: usize, edge_id: EdgeId) {
        if let Some(entry) = self.edge_ids.get_mut(slot_index) {
            *entry = edge_id;
        }
    }
}
