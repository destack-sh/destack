use std::sync::Arc;

use destack_source::ModuleId;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::{GlobalSymbolId, LocalStaticId, LocalTypeId, SegmentView, VarianceModifier};

/// Cumulative checked generic slots for one DIR module.
#[derive(Debug, Clone)]
pub struct GenericTable<'a> {
    /// The module id of the generic table.
    pub module_id: ModuleId,
    /// The ordered generic table segments.
    segments: SegmentView<'a, GenericSegment>,
}

impl GenericTable<'static> {
    /// Create a generic table from ordered segments.
    pub fn from_segments(segments: Vec<Arc<GenericSegment>>) -> Self {
        let segments = SegmentView::from_segments(segments);

        Self::from_view(segments)
    }

    /// Create a generic table from one segment.
    pub fn from_segment(segment: Arc<GenericSegment>) -> Self {
        Self::from_segments(vec![segment])
    }
}

impl<'a> GenericTable<'a> {
    /// Create a generic table from a segment view.
    pub fn from_view(segments: SegmentView<'a, GenericSegment>) -> Self {
        let first = segments
            .first()
            .unwrap_or_else(|| panic!("generic table needs at least one segment"));
        let module_id = first.module_id;

        // require a single module owner
        for segment in segments.iter() {
            assert_eq!(
                segment.module_id, module_id,
                "generic table segment belongs to a different module"
            );
        }

        Self {
            module_id,
            segments,
        }
    }

    /// Create a generic table by appending a borrowed tail segment.
    pub fn with_tail<'b>(&'b self, tail: &'b GenericSegment) -> GenericTable<'b> {
        GenericTable::from_view(self.segments.with_tail(tail))
    }

    /// Iterate effective generic slots keyed by owner declaration symbol.
    pub fn declarations(&self) -> impl Iterator<Item = (GlobalSymbolId, &GenericSlots)> + '_ {
        self.segments
            .iter()
            .enumerate()
            .flat_map(move |(segment_index, segment)| {
                segment
                    .declarations
                    .iter()
                    .filter_map(move |(owner, slots)| {
                        let is_shadowed = self
                            .segments
                            .iter()
                            .skip(segment_index + 1)
                            .any(|segment| segment.declarations.contains_key(owner));

                        (!is_shadowed).then_some((*owner, slots))
                    })
            })
    }

    /// Get checked generic slots for a declaration symbol.
    pub fn slots(&self, owner: GlobalSymbolId) -> Option<&GenericSlots> {
        for segment in self.segments.iter().rev() {
            if let Some(slots) = segment.slots(owner) {
                return Some(slots);
            }
        }

        None
    }

    /// Get the checked generic slot for a parameter symbol.
    pub fn slot(&self, symbol_id: GlobalSymbolId) -> Option<&GenericSlot> {
        for (_, slots) in self.declarations() {
            if let Some(slot) = slots.slot(symbol_id) {
                return Some(slot);
            }
        }

        None
    }

    /// Return true when this table has no entries.
    pub fn is_empty(&self) -> bool {
        self.segments.iter().all(|segment| segment.is_empty())
    }
}

/// Generic slots added by one DIR phase.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenericSegment {
    /// The module id of the generic segment.
    pub module_id: ModuleId,
    /// Checked generic slots keyed by owner declaration symbol.
    pub(crate) declarations: IndexMap<GlobalSymbolId, GenericSlots>,
}

impl GenericSegment {
    /// Create a new generic segment.
    pub fn new(module_id: ModuleId) -> Self {
        Self {
            module_id,
            declarations: IndexMap::new(),
        }
    }

    /// Set checked generic slots for a declaration symbol.
    pub fn set_slots(&mut self, owner: GlobalSymbolId, slots: GenericSlots) {
        self.declarations.insert(owner, slots);
    }

    /// Get checked generic slots for a declaration symbol.
    pub fn slots(&self, owner: GlobalSymbolId) -> Option<&GenericSlots> {
        self.declarations.get(&owner)
    }

    /// Iterate generic slots keyed by owner declaration symbol.
    pub fn declarations(&self) -> impl Iterator<Item = (GlobalSymbolId, &GenericSlots)> + '_ {
        self.declarations
            .iter()
            .map(|(owner, slots)| (*owner, slots))
    }

    /// Return true when this segment has no entries.
    pub fn is_empty(&self) -> bool {
        self.declarations.is_empty()
    }
}

/// Checked generic slots for one declaration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GenericSlots {
    /// The declaration's formal generic slots in application order.
    pub slots: Vec<GenericSlot>,
}

impl GenericSlots {
    /// Create checked generic slots from ordered slots.
    pub fn new(slots: Vec<GenericSlot>) -> Self {
        Self { slots }
    }

    /// Get the slot for a parameter symbol.
    pub fn slot(&self, symbol_id: GlobalSymbolId) -> Option<&GenericSlot> {
        self.slots.iter().find(|slot| slot.symbol() == symbol_id)
    }

    /// Return true when this slot list has no entries.
    pub fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }
}

/// One checked formal generic slot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GenericSlot {
    /// Type generic slot.
    Type {
        /// The symbol bound by this slot.
        symbol: GlobalSymbolId,
        /// The constraint type for this slot.
        constraint: Option<LocalTypeId>,
        /// The default type argument.
        default: Option<LocalTypeId>,
        /// The variance for this slot.
        variance: Option<VarianceModifier>,
    },
    /// Static generic slot.
    Static {
        /// The symbol bound by this slot.
        symbol: GlobalSymbolId,
        /// The constraint type for this slot.
        constraint: Option<LocalTypeId>,
        /// The default static argument.
        default: Option<LocalStaticId>,
    },
}

impl GenericSlot {
    /// Return the symbol bound by this slot.
    pub fn symbol(&self) -> GlobalSymbolId {
        match self {
            Self::Type { symbol, .. } | Self::Static { symbol, .. } => *symbol,
        }
    }
}
