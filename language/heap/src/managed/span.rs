use serde::{Deserialize, Serialize};

use super::{ReferenceMapId, StoredLayoutId};
use crate::alloc::{Bitmap, CardSet, PageMap};

/// One frozen managed span root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct SpanImage {
    /// The size class for this span in bytes.
    pub size_class: usize,
    /// The number of slots in this span.
    pub slot_count: usize,
    /// The occupied slots in this span.
    pub occupied: Bitmap,
    /// The per-slot trace ids for this span.
    pub trace_ids: Box<[u32]>,
    /// The per-slot layout ids for this span.
    pub layout_ids: Box<[StoredLayoutId]>,
    /// The arena pages for this span.
    pub pages: PageMap,
}

/// One live managed span.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Span {
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
    /// The per-slot trace ids for this span.
    pub(crate) trace_ids: Box<[ReferenceMapId]>,
    /// The per-slot layout ids for this span.
    pub(crate) layout_ids: Box<[StoredLayoutId]>,
    /// The arena pages for this span.
    pub(crate) pages: PageMap,
    /// The live mark bitmap keyed by slot.
    pub(crate) marked: Bitmap,
    /// The live pinned slots keyed by slot.
    pub(crate) pinned: Bitmap,
    /// Additional pin counts for slots pinned more than once.
    pub(crate) extra_pin_counts: Vec<(usize, u16)>,
    /// The number of active pins in this span.
    pub(crate) active_pins: usize,
    /// The dirty cards remembered for young tracing.
    pub(crate) dirty_cards: CardSet,
    /// Whether this span is already queued for dirty-card scanning.
    pub(crate) is_dirty_queued: bool,
}
