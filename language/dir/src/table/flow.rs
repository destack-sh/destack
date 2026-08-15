use std::collections::BTreeMap;
use std::sync::Arc;

use destack_core::FxIndexSet;
use destack_serde::Reflect;
use destack_source::ModuleId;
use serde::{Deserialize, Serialize};

use crate::{GlobalSymbolId, LocalNodeIdAny, LocalSymbolId, SegmentView};

/// One symbol's recorded uses inside a DIR module.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct BindingUse(u8);

impl BindingUse {
    /// The symbol is read.
    pub const READ: Self = Self(1 << 0);
    /// The symbol is written after initialization.
    pub const WRITTEN: Self = Self(1 << 1);
    /// The symbol is captured by a closure.
    pub const CAPTURED: Self = Self(1 << 2);
    /// The binding storage requires mutable or exclusive access.
    pub const MUTABLE: Self = Self(1 << 3);

    /// Return whether every bit of `other` is set.
    pub fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// Return whether no use is recorded.
    pub fn is_empty(self) -> bool {
        self.0 == 0
    }
}

impl std::ops::BitOrAssign for BindingUse {
    fn bitor_assign(&mut self, other: Self) {
        self.0 |= other.0;
    }
}

/// Cumulative flow conclusions for one DIR module.
#[derive(Debug, Clone)]
pub struct FlowTable<'a> {
    /// The module id of the flow table.
    pub module_id: ModuleId,
    /// The ordered flow table segments.
    segments: SegmentView<'a, FlowSegment>,
}

impl FlowTable<'static> {
    /// Create a flow table from ordered segments.
    pub fn from_segments(segments: Vec<Arc<FlowSegment>>) -> Self {
        let segments = SegmentView::from_segments(segments);

        Self::from_view(segments)
    }

    /// Create a flow table from one segment.
    pub fn from_segment(segment: Arc<FlowSegment>) -> Self {
        Self::from_segments(vec![segment])
    }
}

impl<'a> FlowTable<'a> {
    /// Create a flow table from a segment view.
    pub fn from_view(segments: SegmentView<'a, FlowSegment>) -> Self {
        let first = segments
            .first()
            .unwrap_or_else(|| panic!("flow table needs at least one segment"));
        let module_id = first.module_id;

        // require a single module owner
        for segment in segments.iter() {
            assert_eq!(
                segment.module_id, module_id,
                "flow table segment belongs to a different module"
            );
        }

        Self {
            module_id,
            segments,
        }
    }

    /// Create a flow table by appending a borrowed tail segment.
    pub fn with_tail<'b>(&'b self, tail: &'b FlowSegment) -> FlowTable<'b> {
        FlowTable::from_view(self.segments.with_tail(tail))
    }

    /// Iterate the recorded symbol uses by occurrence node.
    pub fn occurrences(&self) -> impl Iterator<Item = BindingOccurrence> {
        let mut occurrences = BTreeMap::new();

        // merge the independent uses recorded across phases
        for occurrence in self.segments.iter().flat_map(|segment| &segment.uses) {
            *occurrences
                .entry((occurrence.node, occurrence.symbol))
                .or_insert(BindingUse::default()) |= occurrence.uses;
        }

        occurrences
            .into_iter()
            .map(|((node, symbol), uses)| BindingOccurrence { node, symbol, uses })
    }

    /// Return whether flow proves one node unreachable.
    pub fn is_unreachable(&self, node: LocalNodeIdAny) -> bool {
        self.segments
            .iter()
            .any(|segment| segment.unreachable.contains(&node))
    }

    /// Return whether flow proves one node never returns.
    pub fn is_diverging(&self, node: LocalNodeIdAny) -> bool {
        self.segments
            .iter()
            .any(|segment| segment.diverging.contains(&node))
    }

    /// Iterate the recorded module symbol uses.
    pub fn uses(&self) -> impl Iterator<Item = (LocalSymbolId, BindingUse)> {
        let mut recorded = BTreeMap::new();

        // merge occurrences by local symbol
        for occurrence in self.segments.iter().flat_map(|segment| &segment.uses) {
            if occurrence.symbol.module_id == self.module_id {
                *recorded
                    .entry(occurrence.symbol.local_id)
                    .or_insert(BindingUse::default()) |= occurrence.uses;
            }
        }

        recorded.into_iter()
    }

    /// Iterate the recorded foreign symbol uses.
    pub fn foreign_uses(&self) -> impl Iterator<Item = (GlobalSymbolId, BindingUse)> {
        let mut recorded = BTreeMap::new();

        // merge occurrences by foreign symbol
        for occurrence in self.segments.iter().flat_map(|segment| &segment.uses) {
            if occurrence.symbol.module_id != self.module_id {
                *recorded
                    .entry(occurrence.symbol)
                    .or_insert(BindingUse::default()) |= occurrence.uses;
            }
        }

        recorded.into_iter()
    }

    /// Iterate the nodes flow proves unreachable.
    pub fn unreachable_nodes(&self) -> impl Iterator<Item = LocalNodeIdAny> {
        let mut nodes = self
            .segments
            .iter()
            .flat_map(|segment| segment.unreachable.iter().copied())
            .collect::<Vec<_>>();
        nodes.sort_unstable();
        nodes.dedup();

        nodes.into_iter()
    }

    /// Iterate the nodes flow proves never return.
    pub fn diverging_nodes(&self) -> impl Iterator<Item = LocalNodeIdAny> {
        let mut nodes = self
            .segments
            .iter()
            .flat_map(|segment| segment.diverging.iter().copied())
            .collect::<Vec<_>>();
        nodes.sort_unstable();
        nodes.dedup();

        nodes.into_iter()
    }
}

/// Flow conclusions added by one DIR phase.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct FlowSegment {
    /// The module id of the flow segment.
    pub module_id: ModuleId,
    /// Nodes flow proves unreachable.
    unreachable: FxIndexSet<LocalNodeIdAny>,
    /// Nodes flow proves never return.
    diverging: FxIndexSet<LocalNodeIdAny>,
    /// Proved symbol uses.
    uses: FxIndexSet<BindingOccurrence>,
}

/// One symbol use at its occurrence node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct BindingOccurrence {
    /// The occurrence node.
    pub node: LocalNodeIdAny,
    /// The used symbol.
    pub symbol: GlobalSymbolId,
    /// The recorded uses.
    pub uses: BindingUse,
}

/// One rollback position in a flow segment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FlowMark {
    /// The per-collection lengths at the mark.
    lengths: [usize; 3],
}

impl FlowSegment {
    /// Create an empty flow segment.
    pub fn new(module_id: ModuleId) -> Self {
        Self {
            module_id,
            unreachable: FxIndexSet::default(),
            diverging: FxIndexSet::default(),
            uses: FxIndexSet::default(),
        }
    }

    /// Create a new empty segment after an existing flow segment.
    pub fn from_base(base: &Self) -> Self {
        Self::new(base.module_id)
    }

    /// Mark one node unreachable.
    pub fn mark_unreachable(&mut self, node: LocalNodeIdAny) {
        self.unreachable.insert(node);
    }

    /// Mark one node diverging.
    pub fn mark_diverging(&mut self, node: LocalNodeIdAny) {
        self.diverging.insert(node);
    }

    /// Record one proved symbol use.
    pub fn record_use(&mut self, node: LocalNodeIdAny, symbol: GlobalSymbolId, uses: BindingUse) {
        self.uses.insert(BindingOccurrence { node, symbol, uses });
    }

    /// Return a rollback position for this segment.
    pub fn mark(&self) -> FlowMark {
        FlowMark {
            lengths: [
                self.unreachable.len(),
                self.diverging.len(),
                self.uses.len(),
            ],
        }
    }

    /// Truncate this segment to a previous rollback position.
    pub fn truncate_to(&mut self, mark: FlowMark) {
        let [unreachable, diverging, uses] = mark.lengths;
        self.unreachable.truncate(unreachable);
        self.diverging.truncate(diverging);
        self.uses.truncate(uses);
    }
}
