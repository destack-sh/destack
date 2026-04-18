use serde::{Deserialize, Serialize};

use crate::arena::{Bitmap, PageView};
use crate::{EdgeMap, LayoutId};

/// One frozen shared managed small-span root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SharedManagedSmallSpanImage {
    /// The size class for this span in bytes.
    pub size_class: usize,
    /// The number of slots in this span.
    pub slot_count: usize,
    /// The occupied slots in this span.
    pub occupied: Bitmap,
    /// The per-slot edge maps for this span.
    pub edge_maps: Box<[EdgeMap]>,
    /// The per-slot layout ids for this span.
    pub layout_ids: Box<[Option<LayoutId>]>,
    /// The arena pages for this span.
    pub pages: PageView,
}

/// One live shared managed span.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SharedSmallSpan {
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
    /// The per-slot edge maps for this span.
    pub(crate) edge_maps: Box<[EdgeMap]>,
    /// The per-slot layout ids for this span.
    pub(crate) layout_ids: Box<[Option<LayoutId>]>,
    /// The arena pages for this span.
    pub(crate) pages: PageView,
}

impl SharedSmallSpan {
    /// Return the edge map for one slot in this span.
    pub(crate) fn edge_map(&self, slot_index: usize) -> Option<&EdgeMap> {
        self.edge_maps.get(slot_index)
    }

    /// Return the layout id for one slot in this span.
    pub(crate) fn layout_id(&self, slot_index: usize) -> Option<LayoutId> {
        self.layout_ids.get(slot_index).copied().flatten()
    }

    /// Set the edge map for one slot in this span.
    pub(crate) fn set_edge_map(&mut self, slot_index: usize, edge_map: EdgeMap) {
        if let Some(entry) = self.edge_maps.get_mut(slot_index) {
            *entry = edge_map;
        }
    }

    /// Set the layout id for one slot in this span.
    pub(crate) fn set_layout_id(&mut self, slot_index: usize, layout_id: Option<LayoutId>) {
        if let Some(entry) = self.layout_ids.get_mut(slot_index) {
            *entry = layout_id;
        }
    }
}
