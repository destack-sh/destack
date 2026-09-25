use std::sync::Arc;
use tspp_serde::Reflect;

use serde::{Deserialize, Serialize};
use tspp_core::FxIndexMap as IndexMap;
use tspp_source::ModuleId;

use crate::{
    Arena, GlobalStaticId, GlobalSymbolId, GlobalTypeId, LocalNodeIdAny, LocalStaticId,
    SegmentView, StaticTerm, TypeFold, View,
};

/// Cumulative static values for one DIR module.
#[derive(Debug, Clone)]
pub struct StaticTable<'a> {
    /// The module id of the static table.
    pub module_id: ModuleId,
    /// The ordered static table segments.
    segments: SegmentView<'a, StaticSegment>,
}

impl StaticTable<'static> {
    /// Create a static table from ordered segments.
    pub fn from_segments(segments: Vec<Arc<StaticSegment>>) -> Self {
        let segments = SegmentView::from_segments(segments);

        Self::from_view(segments)
    }

    /// Create a static table from one segment.
    pub fn from_segment(segment: Arc<StaticSegment>) -> Self {
        Self::from_segments(vec![segment])
    }
}

impl<'a> StaticTable<'a> {
    /// Create a static table from a segment view.
    pub fn from_view(segments: SegmentView<'a, StaticSegment>) -> Self {
        let first = segments
            .first()
            .unwrap_or_else(|| panic!("static table needs at least one segment"));
        let module_id = first.module_id;

        // require a single module owner
        for segment in segments.iter() {
            assert_eq!(
                segment.module_id, module_id,
                "static table segment belongs to a different module"
            );
        }

        Self {
            module_id,
            segments,
        }
    }

    /// Create a static table by appending a borrowed tail segment.
    pub fn with_tail<'b>(&'b self, tail: &'b StaticSegment) -> StaticTable<'b> {
        StaticTable::from_view(self.segments.with_tail(tail))
    }

    /// Iterate over all static value ids.
    pub fn iter_static_ids(&self) -> impl Iterator<Item = LocalStaticId> + '_ {
        self.segments
            .iter()
            .flat_map(|segment| segment.iter_static_ids())
    }

    /// Iterate effective static values keyed by symbol.
    pub fn symbol_statics(&self) -> impl Iterator<Item = (GlobalSymbolId, GlobalStaticId)> + '_ {
        let mut entries = IndexMap::default();

        // apply later segment values over earlier ones
        for segment in self.segments.iter() {
            for (symbol_id, static_id) in &segment.static_by_symbol_id {
                entries.insert(*symbol_id, *static_id);
            }
        }

        entries.into_iter()
    }

    /// Get a static value by its id.
    pub fn get_static(&self, static_id: LocalStaticId) -> &StaticTerm {
        self.get_static_maybe(static_id)
            .unwrap_or_else(|| panic!("DIR static value {static_id:?} is not visible"))
    }

    /// Get a static value by its id when present.
    pub fn get_static_maybe(&self, static_id: LocalStaticId) -> Option<&StaticTerm> {
        for segment in self.segments.iter().rev() {
            if let Some(term) = segment.get_static_maybe(static_id) {
                return Some(term);
            }
        }

        None
    }

    /// Get the static value id for a symbol.
    pub fn get_symbol_static_id(&self, symbol_id: GlobalSymbolId) -> Option<GlobalStaticId> {
        for segment in self.segments.iter().rev() {
            if let Some(static_id) = segment.get_symbol_static_id(symbol_id) {
                return Some(static_id);
            }
        }

        None
    }

    /// Return whether one source node is inside a statically absent subtree.
    pub fn is_absent(&self, view: View<'_>, node: LocalNodeIdAny) -> bool {
        let mut current = Some(node);

        // inspect the node and each enclosing static gate
        while let Some(node) = current {
            if self.presence(node) == Some(StaticPresence::Absent) {
                return true;
            }

            current = view.get_parent_any(node);
        }

        false
    }

    /// Return one node's static gate decision.
    pub fn presence(&self, node: LocalNodeIdAny) -> Option<StaticPresence> {
        self.segments
            .iter()
            .find_map(|segment| segment.presence(node))
    }

    /// Find one exact static value by shape.
    pub fn find_static(&self, expected: &StaticTerm) -> Option<LocalStaticId> {
        self.iter_static_ids()
            .find(|&static_id| self.get_static(static_id) == expected)
    }

    /// Intern one static value into a mutable tail segment.
    pub fn intern_static(&self, tail: &mut StaticSegment, term: StaticTerm) -> LocalStaticId {
        assert_eq!(
            self.module_id, tail.module_id,
            "static table tail belongs to a different module"
        );

        if let Some(static_id) = self.find_static(&term) {
            return static_id;
        }

        if let Some(static_id) = tail.find_static(&term) {
            return static_id;
        }

        tail.push_static(term)
    }

    /// Get the number of static values in the table.
    pub fn static_count(&self) -> u32 {
        self.segments
            .last()
            .map(|segment| segment.static_count())
            .unwrap_or(0)
    }

    /// Return true when this table has no entries.
    pub fn is_empty(&self) -> bool {
        self.segments.iter().all(|segment| segment.is_empty())
    }
}

/// Source presence decided by one closed static gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum StaticPresence {
    /// The node is absent.
    Absent,
    /// The node is present.
    Present,
}

/// Static values added by one DIR phase.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct StaticSegment {
    /// The module id of the static segment.
    pub module_id: ModuleId,
    /// The first static id owned by this table segment.
    pub(crate) first_static_id: u32,
    /// Interned static values.
    pub(crate) statics: Arena<StaticTerm>,
    /// Checked static value keyed by symbol.
    pub(crate) static_by_symbol_id: IndexMap<GlobalSymbolId, GlobalStaticId>,
    /// Gate decisions by decorated node.
    pub(crate) gates: IndexMap<LocalNodeIdAny, StaticPresence>,
}

impl StaticSegment {
    /// Create a new static segment.
    pub fn new(module_id: ModuleId) -> Self {
        Self {
            module_id,
            first_static_id: 0,
            statics: Arena::new(),
            static_by_symbol_id: IndexMap::default(),
            gates: IndexMap::default(),
        }
    }

    /// Create a new empty segment after an existing static table segment.
    pub fn from_base(base: &Self) -> Self {
        Self {
            module_id: base.module_id,
            first_static_id: base.static_count(),
            statics: Arena::new(),
            static_by_symbol_id: IndexMap::default(),
            gates: IndexMap::default(),
        }
    }

    /// Append a static value to this segment.
    pub fn push_static(&mut self, term: StaticTerm) -> LocalStaticId {
        let static_id = LocalStaticId::new(self.static_count());
        self.statics.allocate(term);

        static_id
    }

    /// Set the static value for a symbol.
    pub fn set_symbol_static(&mut self, symbol_id: GlobalSymbolId, static_id: GlobalStaticId) {
        self.static_by_symbol_id.insert(symbol_id, static_id);
    }

    /// Commit one node's static gate decision.
    pub fn commit_presence(&mut self, node: LocalNodeIdAny, gate: StaticPresence) {
        self.gates.insert(node, gate);
    }

    /// Return one node's static gate decision.
    pub fn presence(&self, node: LocalNodeIdAny) -> Option<StaticPresence> {
        self.gates.get(&node).copied()
    }

    /// Iterate static values keyed by symbol.
    pub fn symbol_statics(&self) -> impl Iterator<Item = (GlobalSymbolId, GlobalStaticId)> + '_ {
        self.static_by_symbol_id
            .iter()
            .map(|(symbol_id, static_id)| (*symbol_id, *static_id))
    }

    /// Find one exact static value by shape.
    pub fn find_static(&self, expected: &StaticTerm) -> Option<LocalStaticId> {
        self.iter_static_ids()
            .find(|&static_id| self.get_static(static_id) == expected)
    }

    /// Get a static value by its id.
    pub fn get_static(&self, static_id: LocalStaticId) -> &StaticTerm {
        self.get_static_maybe(static_id).unwrap_or_else(|| {
            panic!("DIR static value {static_id:?} is not allocated in this segment")
        })
    }

    /// Get a static value by its id when present.
    pub fn get_static_maybe(&self, static_id: LocalStaticId) -> Option<&StaticTerm> {
        self.contains_static_id(static_id)
            .then(|| self.statics.get(static_id.0 - self.first_static_id))
    }

    /// Get the static value id for a symbol.
    pub fn get_symbol_static_id(&self, symbol_id: GlobalSymbolId) -> Option<GlobalStaticId> {
        self.static_by_symbol_id.get(&symbol_id).copied()
    }

    /// Iterate over all static value ids.
    pub fn iter_static_ids(&self) -> impl Iterator<Item = LocalStaticId> + '_ {
        let end = self.static_count();

        (self.first_static_id..end).map(LocalStaticId::new)
    }

    /// Get the number of static values in the segment.
    pub fn static_count(&self) -> u32 {
        self.first_static_id + self.statics.len() as u32
    }

    /// Return true when this segment has no entries.
    pub fn is_empty(&self) -> bool {
        self.statics.is_empty() && self.static_by_symbol_id.is_empty() && self.gates.is_empty()
    }

    /// Return whether this segment contains the given static id.
    fn contains_static_id(&self, static_id: LocalStaticId) -> bool {
        static_id.0 >= self.first_static_id && static_id.0 < self.static_count()
    }
}

impl TypeFold for StaticSegment {
    fn map_types<E>(
        &mut self,
        map: &mut impl FnMut(GlobalTypeId) -> Result<GlobalTypeId, E>,
    ) -> Result<(), E> {
        for term in self.statics.iter_mut() {
            term.map_types(map)?;
        }

        Ok(())
    }
}
