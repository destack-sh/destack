use std::collections::BTreeMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tspp_core::{FxIndexMap, FxIndexSet};
use tspp_serde::Reflect;
use tspp_source::ModuleId;

use crate::{AccessPath, GlobalSymbolId, LocalNodeIdAny, LocalSymbolId, SegmentView};

/// One binding's recorded uses inside a DIR module.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct BindingUse(u8);

impl BindingUse {
    /// The symbol is read.
    pub const READ: Self = Self(1 << 0);
    /// The symbol is written after initialization.
    pub const WRITE: Self = Self(1 << 1);
    /// The symbol is captured by a closure.
    pub const CAPTURE: Self = Self(1 << 2);
    /// The use mutates the binding storage.
    pub const MUTATE: Self = Self(1 << 3);
    /// The use takes mutable access to the value, in place or through a handle.
    pub const MUTABLE: Self = Self(1 << 4);
    /// The use takes the value out of the binding by value.
    pub const MOVE: Self = Self(1 << 5);
    /// The use requires exclusion of conflicting access.
    pub const EXCLUSIVE: Self = Self(1 << 6);

    /// Return whether every bit of `other` is set.
    pub fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// Return whether no use is recorded.
    pub fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Return these uses with the bits in `other` cleared.
    pub fn without(self, other: Self) -> Self {
        Self(self.0 & !other.0)
    }

    /// Return whether the recorded use may mutate binding storage or its value.
    pub fn may_mutate(self) -> bool {
        self.contains(Self::WRITE) || self.contains(Self::MUTATE)
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

    /// Iterate the recorded binding uses by occurrence node.
    pub fn binding_occurrences(&self) -> impl Iterator<Item = BindingOccurrence> {
        let mut occurrences = BTreeMap::new();

        // merge the independent uses recorded across phases
        for occurrence in self
            .segments
            .iter()
            .flat_map(|segment| &segment.binding_occurrences)
        {
            *occurrences
                .entry((occurrence.node, occurrence.symbol))
                .or_insert(BindingUse::default()) |= occurrence.uses;
        }

        occurrences
            .into_iter()
            .map(|((node, symbol), uses)| BindingOccurrence { node, symbol, uses })
    }

    /// Iterate the recorded stable access uses by occurrence node.
    pub fn access_occurrences(&self) -> impl Iterator<Item = AccessOccurrence> {
        let mut occurrences = FxIndexMap::default();

        // merge the independent uses recorded across phases
        for occurrence in self
            .segments
            .iter()
            .flat_map(|segment| &segment.access_occurrences)
        {
            *occurrences
                .entry((occurrence.node, occurrence.path.clone()))
                .or_insert(BindingUse::default()) |= occurrence.uses;
        }

        occurrences
            .into_iter()
            .map(|((node, path), uses)| AccessOccurrence { node, path, uses })
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

    /// Return whether flow proves one loop runs its body at most once.
    pub fn is_single_pass(&self, node: LocalNodeIdAny) -> bool {
        self.segments
            .iter()
            .any(|segment| segment.single_pass.contains(&node))
    }

    /// Iterate the recorded module symbol uses.
    pub fn binding_uses(&self) -> impl Iterator<Item = (LocalSymbolId, BindingUse)> {
        let mut recorded = BTreeMap::new();

        // merge occurrences by local symbol
        for occurrence in self
            .segments
            .iter()
            .flat_map(|segment| &segment.binding_occurrences)
        {
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
        for occurrence in self
            .segments
            .iter()
            .flat_map(|segment| &segment.binding_occurrences)
        {
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

    /// Iterate the loops flow proves run their body at most once.
    pub fn single_pass_nodes(&self) -> impl Iterator<Item = LocalNodeIdAny> {
        let mut nodes = self
            .segments
            .iter()
            .flat_map(|segment| segment.single_pass.iter().copied())
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
    /// Loops flow proves run their body at most once.
    single_pass: FxIndexSet<LocalNodeIdAny>,
    /// Proved binding uses.
    binding_occurrences: FxIndexSet<BindingOccurrence>,
    /// Proved stable access uses.
    access_occurrences: FxIndexSet<AccessOccurrence>,
}

/// One binding use at its occurrence node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct BindingOccurrence {
    /// The occurrence node.
    pub node: LocalNodeIdAny,
    /// The used symbol.
    pub symbol: GlobalSymbolId,
    /// The recorded uses.
    pub uses: BindingUse,
}

/// One stable storage access at its occurrence node.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct AccessOccurrence {
    /// The occurrence node.
    pub node: LocalNodeIdAny,
    /// The selected storage path.
    pub path: AccessPath,
    /// The recorded uses.
    pub uses: BindingUse,
}

impl FlowSegment {
    /// Create an empty flow segment.
    pub fn new(module_id: ModuleId) -> Self {
        Self {
            module_id,
            unreachable: FxIndexSet::default(),
            diverging: FxIndexSet::default(),
            single_pass: FxIndexSet::default(),
            binding_occurrences: FxIndexSet::default(),
            access_occurrences: FxIndexSet::default(),
        }
    }

    /// Create a new empty segment after an existing flow segment.
    pub fn from_base(base: &Self) -> Self {
        Self::new(base.module_id)
    }

    /// Iterate the binding occurrences this segment recorded.
    pub fn binding_occurrences(&self) -> impl Iterator<Item = &BindingOccurrence> {
        self.binding_occurrences.iter()
    }

    /// Set one node unreachable.
    pub fn set_unreachable(&mut self, node: LocalNodeIdAny) {
        self.unreachable.insert(node);
    }

    /// Set one node diverging.
    pub fn set_diverging(&mut self, node: LocalNodeIdAny) {
        self.diverging.insert(node);
    }

    /// Set one loop running its body at most once.
    pub fn set_single_pass(&mut self, node: LocalNodeIdAny) {
        self.single_pass.insert(node);
    }

    /// Commit one proved binding use.
    pub fn commit_binding_use(
        &mut self,
        node: LocalNodeIdAny,
        symbol: GlobalSymbolId,
        uses: BindingUse,
    ) {
        self.binding_occurrences
            .insert(BindingOccurrence { node, symbol, uses });
    }

    /// Commit one proved stable access use.
    pub fn commit_access_use(&mut self, node: LocalNodeIdAny, path: AccessPath, uses: BindingUse) {
        self.access_occurrences
            .insert(AccessOccurrence { node, path, uses });
    }
}
