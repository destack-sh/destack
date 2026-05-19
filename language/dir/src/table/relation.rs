use std::sync::Arc;

use destack_source::ModuleId;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::{GlobalSymbolId, Relation, RelationKind, SegmentView};

/// Cumulative checked relations for one DIR module.
#[derive(Debug, Clone)]
pub struct RelationTable<'a> {
    /// The module id of the relation table.
    pub module_id: ModuleId,
    /// The ordered relation table segments.
    segments: SegmentView<'a, RelationSegment>,
}

impl RelationTable<'static> {
    /// Create a relation table from ordered segments.
    pub fn from_segments(segments: Vec<Arc<RelationSegment>>) -> Self {
        let segments = SegmentView::from_segments(segments);

        Self::from_view(segments)
    }

    /// Create a relation table from one segment.
    pub fn from_segment(segment: Arc<RelationSegment>) -> Self {
        Self::from_segments(vec![segment])
    }
}

impl<'a> RelationTable<'a> {
    /// Create a relation table from a segment view.
    pub fn from_view(segments: SegmentView<'a, RelationSegment>) -> Self {
        let first = segments
            .first()
            .unwrap_or_else(|| panic!("relation table needs at least one segment"));
        let module_id = first.module_id;

        // require a single module owner
        for segment in segments.iter() {
            assert_eq!(
                segment.module_id, module_id,
                "relation table segment belongs to a different module"
            );
        }

        Self {
            module_id,
            segments,
        }
    }

    /// Create a relation table by appending a borrowed tail segment.
    pub fn with_tail<'b>(&'b self, tail: &'b RelationSegment) -> RelationTable<'b> {
        RelationTable::from_view(self.segments.with_tail(tail))
    }

    /// Return the latest parent relation for a symbol.
    pub fn extends(&self, symbol: GlobalSymbolId) -> Option<Relation> {
        for segment in self.segments.iter().rev() {
            if let Some(relation) = segment.extends(symbol) {
                return Some(relation);
            }
        }

        None
    }

    /// Iterate visible parent relations.
    pub fn extends_entries(&self) -> impl Iterator<Item = (GlobalSymbolId, Relation)> + '_ {
        self.segments
            .iter()
            .enumerate()
            .flat_map(move |(segment_index, segment)| {
                segment
                    .extends
                    .iter()
                    .filter_map(move |(symbol, relation)| {
                        let is_shadowed = self
                            .segments
                            .iter()
                            .skip(segment_index + 1)
                            .any(|segment| segment.extends.contains_key(symbol));

                        (!is_shadowed).then_some((*symbol, *relation))
                    })
            })
    }

    /// Return visible implemented relations for a symbol.
    pub fn implements(&self, symbol: GlobalSymbolId) -> impl Iterator<Item = Relation> + '_ {
        self.segments
            .iter()
            .flat_map(move |segment| segment.implements(symbol))
    }

    /// Iterate visible implemented relations.
    pub fn implements_entries(&self) -> impl Iterator<Item = (GlobalSymbolId, Relation)> + '_ {
        self.segments
            .iter()
            .flat_map(|segment| segment.implements_entries())
    }

    /// Return whether this table has no relations.
    pub fn is_empty(&self) -> bool {
        self.segments.iter().all(|segment| segment.is_empty())
    }
}

/// Relations added by one DIR phase.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationSegment {
    /// The module id of the relation segment.
    pub module_id: ModuleId,
    /// Single inheritance parent relation by declaration symbol.
    pub(crate) extends: IndexMap<GlobalSymbolId, Relation>,
    /// Implemented interface relations by declaration symbol.
    pub(crate) implements: IndexMap<GlobalSymbolId, Vec<Relation>>,
}

impl RelationSegment {
    /// Create an empty relation segment.
    pub fn new(module_id: ModuleId) -> Self {
        Self {
            module_id,
            extends: IndexMap::new(),
            implements: IndexMap::new(),
        }
    }

    /// Set the parent relation for a declaration symbol.
    pub fn set_extends(&mut self, symbol: GlobalSymbolId, relation: Relation) {
        assert_eq!(
            relation.kind,
            RelationKind::Extends,
            "extends relation must have extends kind"
        );
        self.extends.insert(symbol, relation);
    }

    /// Return the parent relation for a declaration symbol.
    pub fn extends(&self, symbol: GlobalSymbolId) -> Option<Relation> {
        self.extends.get(&symbol).copied()
    }

    /// Iterate parent relations in insertion order.
    pub fn extends_entries(&self) -> impl Iterator<Item = (GlobalSymbolId, Relation)> + '_ {
        self.extends
            .iter()
            .map(|(symbol, relation)| (*symbol, *relation))
    }

    /// Add one implemented relation for a declaration symbol.
    pub fn push_implements(&mut self, symbol: GlobalSymbolId, relation: Relation) {
        assert_eq!(
            relation.kind,
            RelationKind::Implements,
            "implements relation must have implements kind"
        );
        self.implements.entry(symbol).or_default().push(relation);
    }

    /// Return implemented relations for a declaration symbol.
    pub fn implements(&self, symbol: GlobalSymbolId) -> impl Iterator<Item = Relation> + '_ {
        self.implements.get(&symbol).into_iter().flatten().copied()
    }

    /// Iterate implemented relations in insertion order.
    pub fn implements_entries(&self) -> impl Iterator<Item = (GlobalSymbolId, Relation)> + '_ {
        self.implements.iter().flat_map(|(symbol, relations)| {
            relations.iter().map(move |relation| (*symbol, *relation))
        })
    }

    /// Return whether this segment has no relations.
    pub fn is_empty(&self) -> bool {
        self.extends.is_empty() && self.implements.is_empty()
    }
}
