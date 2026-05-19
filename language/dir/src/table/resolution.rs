use std::sync::Arc;

use destack_source::ModuleId;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::{
    CallResolution, GlobalNodeIdAny, GlobalSymbolId, LabelResolution, MemberResolution,
    NameResolution, SegmentView,
};

/// Cumulative checked resolutions for one DIR module.
#[derive(Debug, Clone)]
pub struct ResolutionTable<'a> {
    /// The module id of the resolution table.
    pub module_id: ModuleId,
    /// The ordered resolution table segments.
    segments: SegmentView<'a, ResolutionSegment>,
}

impl ResolutionTable<'static> {
    /// Create a resolution table from ordered segments.
    pub fn from_segments(segments: Vec<Arc<ResolutionSegment>>) -> Self {
        let segments = SegmentView::from_segments(segments);

        Self::from_view(segments)
    }

    /// Create a resolution table from one segment.
    pub fn from_segment(segment: Arc<ResolutionSegment>) -> Self {
        Self::from_segments(vec![segment])
    }
}

impl<'a> ResolutionTable<'a> {
    /// Create a resolution table from a segment view.
    pub fn from_view(segments: SegmentView<'a, ResolutionSegment>) -> Self {
        let first = segments
            .first()
            .unwrap_or_else(|| panic!("resolution table needs at least one segment"));
        let module_id = first.module_id;

        // require a single module owner
        for segment in segments.iter() {
            assert_eq!(
                segment.module_id, module_id,
                "resolution table segment belongs to a different module"
            );
        }

        Self {
            module_id,
            segments,
        }
    }

    /// Create a resolution table by appending a borrowed tail segment.
    pub fn with_tail<'b>(&'b self, tail: &'b ResolutionSegment) -> ResolutionTable<'b> {
        ResolutionTable::from_view(self.segments.with_tail(tail))
    }

    /// Iterate visible name resolutions.
    pub fn name_entries(&self) -> impl Iterator<Item = (GlobalNodeIdAny, &NameResolution)> + '_ {
        self.visible_entries(|segment| &segment.names)
    }

    /// Iterate visible label resolutions.
    pub fn label_entries(&self) -> impl Iterator<Item = (GlobalNodeIdAny, &LabelResolution)> + '_ {
        self.visible_entries(|segment| &segment.labels)
    }

    /// Iterate visible member resolutions.
    pub fn member_entries(
        &self,
    ) -> impl Iterator<Item = (GlobalNodeIdAny, &MemberResolution)> + '_ {
        self.visible_entries(|segment| &segment.members)
    }

    /// Iterate visible call resolutions.
    pub fn call_entries(&self) -> impl Iterator<Item = (GlobalNodeIdAny, &CallResolution)> + '_ {
        self.visible_entries(|segment| &segment.calls)
    }

    /// Get the lexical symbol resolution for a node.
    pub fn symbol_resolution(&self, node_id: GlobalNodeIdAny) -> Option<GlobalSymbolId> {
        self.name_resolution(node_id)
            .map(|resolution| resolution.symbol)
    }

    /// Get the name resolution for a node.
    pub fn name_resolution(&self, node_id: GlobalNodeIdAny) -> Option<&NameResolution> {
        self.lookup(node_id, |segment| &segment.names)
    }

    /// Get the label resolution for a node.
    pub fn label_resolution(&self, node_id: GlobalNodeIdAny) -> Option<LabelResolution> {
        self.lookup(node_id, |segment| &segment.labels).copied()
    }

    /// Get the member resolution for a node.
    pub fn member_resolution(&self, node_id: GlobalNodeIdAny) -> Option<&MemberResolution> {
        self.lookup(node_id, |segment| &segment.members)
    }

    /// Get the call resolution for a node.
    pub fn call_resolution(&self, node_id: GlobalNodeIdAny) -> Option<&CallResolution> {
        self.lookup(node_id, |segment| &segment.calls)
    }

    /// Return whether this table has no resolutions.
    pub fn is_empty(&self) -> bool {
        self.segments.iter().all(|segment| segment.is_empty())
    }

    /// Look up the latest visible entry in one resolution column.
    fn lookup<T>(
        &self,
        node_id: GlobalNodeIdAny,
        column: impl Fn(&ResolutionSegment) -> &IndexMap<GlobalNodeIdAny, T>,
    ) -> Option<&T> {
        for segment in self.segments.iter().rev() {
            if let Some(resolution) = column(segment).get(&node_id) {
                return Some(resolution);
            }
        }

        None
    }

    /// Iterate the visible entries in one resolution column.
    fn visible_entries<'b, T: 'b>(
        &'b self,
        column: impl Fn(&ResolutionSegment) -> &IndexMap<GlobalNodeIdAny, T> + Copy + 'b,
    ) -> impl Iterator<Item = (GlobalNodeIdAny, &'b T)> + 'b {
        self.segments
            .iter()
            .enumerate()
            .flat_map(move |(segment_index, segment)| {
                column(segment)
                    .iter()
                    .filter_map(move |(node_id, resolution)| {
                        let is_shadowed = self
                            .segments
                            .iter()
                            .skip(segment_index + 1)
                            .any(|segment| column(segment).contains_key(node_id));

                        (!is_shadowed).then_some((*node_id, resolution))
                    })
            })
    }
}

/// Resolutions added by one DIR phase.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolutionSegment {
    /// The module id of the resolution segment.
    pub module_id: ModuleId,
    /// Checked lexical or path resolutions keyed by DIR node.
    pub(crate) names: IndexMap<GlobalNodeIdAny, NameResolution>,
    /// Checked label resolutions keyed by DIR node.
    pub(crate) labels: IndexMap<GlobalNodeIdAny, LabelResolution>,
    /// Checked member resolutions keyed by DIR node.
    pub(crate) members: IndexMap<GlobalNodeIdAny, MemberResolution>,
    /// Checked call resolutions keyed by DIR node.
    pub(crate) calls: IndexMap<GlobalNodeIdAny, CallResolution>,
}

impl ResolutionSegment {
    /// Create an empty resolution segment.
    pub fn new(module_id: ModuleId) -> Self {
        Self {
            module_id,
            names: IndexMap::new(),
            labels: IndexMap::new(),
            members: IndexMap::new(),
            calls: IndexMap::new(),
        }
    }

    /// Copy node-owned resolutions from one node to another.
    pub fn copy_node_relations(&mut self, source: GlobalNodeIdAny, target: GlobalNodeIdAny) {
        if let Some(resolution) = self.names.get(&source).copied() {
            self.names.insert(target, resolution);
        }

        if let Some(resolution) = self.labels.get(&source).copied() {
            self.labels.insert(target, resolution);
        }

        if let Some(resolution) = self.members.get(&source).cloned() {
            self.members.insert(target, resolution);
        }

        if let Some(resolution) = self.calls.get(&source).cloned() {
            self.calls.insert(target, resolution);
        }
    }

    /// Set the lexical symbol resolution for a node.
    pub fn set_symbol_resolution(&mut self, node_id: GlobalNodeIdAny, symbol_id: GlobalSymbolId) {
        self.set_name_resolution(node_id, NameResolution::new(symbol_id));
    }

    /// Get the lexical symbol resolution for a node.
    pub fn symbol_resolution(&self, node_id: GlobalNodeIdAny) -> Option<GlobalSymbolId> {
        self.name_resolution(node_id)
            .map(|resolution| resolution.symbol)
    }

    /// Set the name resolution for a node.
    pub fn set_name_resolution(&mut self, node_id: GlobalNodeIdAny, resolution: NameResolution) {
        self.names.insert(node_id, resolution);
    }

    /// Get the name resolution for a node.
    pub fn name_resolution(&self, node_id: GlobalNodeIdAny) -> Option<&NameResolution> {
        self.names.get(&node_id)
    }

    /// Set the label resolution for a node.
    pub fn set_label_resolution(&mut self, node_id: GlobalNodeIdAny, resolution: LabelResolution) {
        self.labels.insert(node_id, resolution);
    }

    /// Get the label resolution for a node.
    pub fn label_resolution(&self, node_id: GlobalNodeIdAny) -> Option<LabelResolution> {
        self.labels.get(&node_id).copied()
    }

    /// Set the member resolution for a node.
    pub fn set_member_resolution(
        &mut self,
        node_id: GlobalNodeIdAny,
        resolution: MemberResolution,
    ) {
        self.members.insert(node_id, resolution);
    }

    /// Get the member resolution for a node.
    pub fn member_resolution(&self, node_id: GlobalNodeIdAny) -> Option<&MemberResolution> {
        self.members.get(&node_id)
    }

    /// Set the call resolution for a node.
    pub fn set_call_resolution(&mut self, node_id: GlobalNodeIdAny, resolution: CallResolution) {
        self.calls.insert(node_id, resolution);
    }

    /// Get the call resolution for a node.
    pub fn call_resolution(&self, node_id: GlobalNodeIdAny) -> Option<&CallResolution> {
        self.calls.get(&node_id)
    }

    /// Iterate visible name resolutions.
    pub fn name_entries(&self) -> impl Iterator<Item = (GlobalNodeIdAny, &NameResolution)> + '_ {
        self.names
            .iter()
            .map(|(node_id, resolution)| (*node_id, resolution))
    }

    /// Iterate visible label resolutions.
    pub fn label_entries(&self) -> impl Iterator<Item = (GlobalNodeIdAny, &LabelResolution)> + '_ {
        self.labels
            .iter()
            .map(|(node_id, resolution)| (*node_id, resolution))
    }

    /// Iterate visible member resolutions.
    pub fn member_entries(
        &self,
    ) -> impl Iterator<Item = (GlobalNodeIdAny, &MemberResolution)> + '_ {
        self.members
            .iter()
            .map(|(node_id, resolution)| (*node_id, resolution))
    }

    /// Iterate visible call resolutions.
    pub fn call_entries(&self) -> impl Iterator<Item = (GlobalNodeIdAny, &CallResolution)> + '_ {
        self.calls
            .iter()
            .map(|(node_id, resolution)| (*node_id, resolution))
    }

    /// Return whether this segment has no resolutions.
    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
            && self.labels.is_empty()
            && self.members.is_empty()
            && self.calls.is_empty()
    }
}
