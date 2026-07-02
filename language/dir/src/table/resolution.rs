use destack_serde::Reflect;
use std::sync::Arc;

use destack_source::ModuleId;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::{
    AssignPatternResolution, CallResolution, ConstructResolution, GlobalNodeIdAny, GlobalSymbolId,
    GlobalTypeId, GuardResolution, InstantiationResolution, LabelResolution, MemberResolution,
    NameResolution, PatternResolution, PlaceResolution, ReceiverResolution, SegmentView,
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

    /// Iterate visible generic instantiation resolutions.
    pub fn instantiation_entries(
        &self,
    ) -> impl Iterator<Item = (GlobalNodeIdAny, &InstantiationResolution)> + '_ {
        self.visible_entries(|segment| &segment.instantiations)
    }

    /// Iterate visible label resolutions.
    pub fn label_entries(&self) -> impl Iterator<Item = (GlobalNodeIdAny, &LabelResolution)> + '_ {
        self.visible_entries(|segment| &segment.labels)
    }

    /// Iterate visible receiver resolutions.
    pub fn receiver_entries(
        &self,
    ) -> impl Iterator<Item = (GlobalNodeIdAny, &ReceiverResolution)> + '_ {
        self.visible_entries(|segment| &segment.receivers)
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

    /// Iterate visible place resolutions.
    pub fn place_entries(&self) -> impl Iterator<Item = (GlobalNodeIdAny, &PlaceResolution)> + '_ {
        self.visible_entries(|segment| &segment.places)
    }

    /// Iterate visible guard resolutions.
    pub fn guard_entries(&self) -> impl Iterator<Item = (GlobalNodeIdAny, &GuardResolution)> + '_ {
        self.visible_entries(|segment| &segment.guards)
    }

    /// Iterate visible construct resolutions.
    pub fn construct_entries(
        &self,
    ) -> impl Iterator<Item = (GlobalNodeIdAny, &ConstructResolution)> + '_ {
        self.visible_entries(|segment| &segment.constructs)
    }

    /// Iterate visible pattern resolutions.
    pub fn pattern_entries(
        &self,
    ) -> impl Iterator<Item = (GlobalNodeIdAny, &PatternResolution)> + '_ {
        self.visible_entries(|segment| &segment.patterns)
    }

    /// Iterate visible assignment pattern resolutions.
    pub fn assign_pattern_entries(
        &self,
    ) -> impl Iterator<Item = (GlobalNodeIdAny, &AssignPatternResolution)> + '_ {
        self.visible_entries(|segment| &segment.assign_patterns)
    }

    /// Get the lexical symbol resolution for a node.
    pub fn symbol_resolution(&self, node_id: GlobalNodeIdAny) -> Option<GlobalSymbolId> {
        self.name_resolution(node_id).map(NameResolution::symbol)
    }

    /// Get the name resolution for a node.
    pub fn name_resolution(&self, node_id: GlobalNodeIdAny) -> Option<&NameResolution> {
        self.lookup(node_id, |segment| &segment.names)
    }

    /// Get the explicit generic instantiation for a node.
    pub fn instantiation_resolution(
        &self,
        node_id: GlobalNodeIdAny,
    ) -> Option<&InstantiationResolution> {
        self.lookup(node_id, |segment| &segment.instantiations)
    }

    /// Get the label resolution for a node.
    pub fn label_resolution(&self, node_id: GlobalNodeIdAny) -> Option<LabelResolution> {
        self.lookup(node_id, |segment| &segment.labels).copied()
    }

    /// Get the receiver resolution for a node.
    pub fn receiver_resolution(&self, node_id: GlobalNodeIdAny) -> Option<&ReceiverResolution> {
        self.lookup(node_id, |segment| &segment.receivers)
    }

    /// Get the member resolution for a node.
    pub fn member_resolution(&self, node_id: GlobalNodeIdAny) -> Option<&MemberResolution> {
        self.lookup(node_id, |segment| &segment.members)
    }

    /// Get the call resolution for a node.
    pub fn call_resolution(&self, node_id: GlobalNodeIdAny) -> Option<&CallResolution> {
        self.lookup(node_id, |segment| &segment.calls)
    }

    /// Get the place resolution for a node.
    pub fn place_resolution(&self, node_id: GlobalNodeIdAny) -> Option<&PlaceResolution> {
        self.lookup(node_id, |segment| &segment.places)
    }

    /// Get the guard resolution for a node.
    pub fn guard_resolution(&self, node_id: GlobalNodeIdAny) -> Option<&GuardResolution> {
        self.lookup(node_id, |segment| &segment.guards)
    }

    /// Get the construct resolution for a node.
    pub fn construct_resolution(&self, node_id: GlobalNodeIdAny) -> Option<&ConstructResolution> {
        self.lookup(node_id, |segment| &segment.constructs)
    }

    /// Get the pattern resolution for a node.
    pub fn pattern_resolution(&self, node_id: GlobalNodeIdAny) -> Option<&PatternResolution> {
        self.lookup(node_id, |segment| &segment.patterns)
    }

    /// Get the assignment pattern resolution for a node.
    pub fn assign_pattern_resolution(
        &self,
        node_id: GlobalNodeIdAny,
    ) -> Option<&AssignPatternResolution> {
        self.lookup(node_id, |segment| &segment.assign_patterns)
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
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct ResolutionSegment {
    /// The module id of the resolution segment.
    pub module_id: ModuleId,
    /// Checked lexical or path resolutions keyed by DIR node.
    pub(crate) names: IndexMap<GlobalNodeIdAny, NameResolution>,
    /// Checked generic instantiations keyed by DIR node.
    pub(crate) instantiations: IndexMap<GlobalNodeIdAny, InstantiationResolution>,
    /// Checked label resolutions keyed by DIR node.
    pub(crate) labels: IndexMap<GlobalNodeIdAny, LabelResolution>,
    /// Checked receiver resolutions keyed by DIR node.
    pub(crate) receivers: IndexMap<GlobalNodeIdAny, ReceiverResolution>,
    /// Checked member resolutions keyed by DIR node.
    pub(crate) members: IndexMap<GlobalNodeIdAny, MemberResolution>,
    /// Checked call resolutions keyed by DIR node.
    pub(crate) calls: IndexMap<GlobalNodeIdAny, CallResolution>,
    /// Checked place resolutions keyed by DIR node.
    pub(crate) places: IndexMap<GlobalNodeIdAny, PlaceResolution>,
    /// Checked guard resolutions keyed by DIR node.
    pub(crate) guards: IndexMap<GlobalNodeIdAny, GuardResolution>,
    /// Checked construct resolutions keyed by DIR node.
    pub(crate) constructs: IndexMap<GlobalNodeIdAny, ConstructResolution>,
    /// Checked pattern resolutions keyed by DIR node.
    pub(crate) patterns: IndexMap<GlobalNodeIdAny, PatternResolution>,
    /// Checked assignment pattern resolutions keyed by DIR node.
    pub(crate) assign_patterns: IndexMap<GlobalNodeIdAny, AssignPatternResolution>,
}

impl ResolutionSegment {
    /// Create an empty resolution segment.
    pub fn new(module_id: ModuleId) -> Self {
        Self {
            module_id,
            names: IndexMap::new(),
            instantiations: IndexMap::new(),
            labels: IndexMap::new(),
            receivers: IndexMap::new(),
            members: IndexMap::new(),
            calls: IndexMap::new(),
            places: IndexMap::new(),
            guards: IndexMap::new(),
            constructs: IndexMap::new(),
            patterns: IndexMap::new(),
            assign_patterns: IndexMap::new(),
        }
    }

    /// Copy node-owned resolutions from one node to another.
    pub fn copy_node_relations(&mut self, source: GlobalNodeIdAny, target: GlobalNodeIdAny) {
        if let Some(resolution) = self.names.get(&source).cloned() {
            self.names.insert(target, resolution);
        }

        if let Some(resolution) = self.instantiations.get(&source).cloned() {
            self.instantiations.insert(target, resolution);
        }

        if let Some(resolution) = self.labels.get(&source).copied() {
            self.labels.insert(target, resolution);
        }

        if let Some(resolution) = self.receivers.get(&source).copied() {
            self.receivers.insert(target, resolution);
        }

        if let Some(resolution) = self.members.get(&source).cloned() {
            self.members.insert(target, resolution);
        }

        if let Some(resolution) = self.calls.get(&source).cloned() {
            self.calls.insert(target, resolution);
        }

        if let Some(resolution) = self.places.get(&source).cloned() {
            self.places.insert(target, resolution);
        }

        if let Some(resolution) = self.guards.get(&source).cloned() {
            self.guards.insert(target, resolution);
        }

        if let Some(resolution) = self.constructs.get(&source).cloned() {
            self.constructs.insert(target, resolution);
        }

        if let Some(resolution) = self.patterns.get(&source).cloned() {
            self.patterns.insert(target, resolution);
        }

        if let Some(resolution) = self.assign_patterns.get(&source).cloned() {
            self.assign_patterns.insert(target, resolution);
        }
    }

    /// Set the lexical symbol resolution for a node.
    pub fn set_symbol_resolution(&mut self, node_id: GlobalNodeIdAny, symbol_id: GlobalSymbolId) {
        self.set_name_resolution(node_id, NameResolution::new(symbol_id));
    }

    /// Get the lexical symbol resolution for a node.
    pub fn symbol_resolution(&self, node_id: GlobalNodeIdAny) -> Option<GlobalSymbolId> {
        self.name_resolution(node_id).map(NameResolution::symbol)
    }

    /// Set the name resolution for a node.
    pub fn set_name_resolution(&mut self, node_id: GlobalNodeIdAny, resolution: NameResolution) {
        self.names.insert(node_id, resolution);
    }

    /// Get the name resolution for a node.
    pub fn name_resolution(&self, node_id: GlobalNodeIdAny) -> Option<&NameResolution> {
        self.names.get(&node_id)
    }

    /// Set the explicit generic instantiation for a node.
    pub fn set_instantiation_resolution(
        &mut self,
        node_id: GlobalNodeIdAny,
        resolution: InstantiationResolution,
    ) {
        self.instantiations.insert(node_id, resolution);
    }

    /// Get the explicit generic instantiation for a node.
    pub fn instantiation_resolution(
        &self,
        node_id: GlobalNodeIdAny,
    ) -> Option<&InstantiationResolution> {
        self.instantiations.get(&node_id)
    }

    /// Set the label resolution for a node.
    pub fn set_label_resolution(&mut self, node_id: GlobalNodeIdAny, resolution: LabelResolution) {
        self.labels.insert(node_id, resolution);
    }

    /// Get the label resolution for a node.
    pub fn label_resolution(&self, node_id: GlobalNodeIdAny) -> Option<LabelResolution> {
        self.labels.get(&node_id).copied()
    }

    /// Set the receiver resolution for a node.
    pub fn set_receiver_resolution(
        &mut self,
        node_id: GlobalNodeIdAny,
        resolution: ReceiverResolution,
    ) {
        self.receivers.insert(node_id, resolution);
    }

    /// Get the receiver resolution for a node.
    pub fn receiver_resolution(&self, node_id: GlobalNodeIdAny) -> Option<&ReceiverResolution> {
        self.receivers.get(&node_id)
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

    /// Set the place resolution for a node.
    pub fn set_place_resolution(&mut self, node_id: GlobalNodeIdAny, resolution: PlaceResolution) {
        self.places.insert(node_id, resolution);
    }

    /// Get the place resolution for a node.
    pub fn place_resolution(&self, node_id: GlobalNodeIdAny) -> Option<&PlaceResolution> {
        self.places.get(&node_id)
    }

    /// Set the guard resolution for a node.
    pub fn set_guard_resolution(&mut self, node_id: GlobalNodeIdAny, resolution: GuardResolution) {
        self.guards.insert(node_id, resolution);
    }

    /// Get the guard resolution for a node.
    pub fn guard_resolution(&self, node_id: GlobalNodeIdAny) -> Option<&GuardResolution> {
        self.guards.get(&node_id)
    }

    /// Set the construct resolution for a node.
    pub fn set_construct_resolution(
        &mut self,
        node_id: GlobalNodeIdAny,
        resolution: ConstructResolution,
    ) {
        self.constructs.insert(node_id, resolution);
    }

    /// Get the construct resolution for a node.
    pub fn construct_resolution(&self, node_id: GlobalNodeIdAny) -> Option<&ConstructResolution> {
        self.constructs.get(&node_id)
    }

    /// Set the pattern resolution for a node.
    pub fn set_pattern_resolution(
        &mut self,
        node_id: GlobalNodeIdAny,
        resolution: PatternResolution,
    ) {
        self.patterns.insert(node_id, resolution);
    }

    /// Get the pattern resolution for a node.
    pub fn pattern_resolution(&self, node_id: GlobalNodeIdAny) -> Option<&PatternResolution> {
        self.patterns.get(&node_id)
    }

    /// Set the assignment pattern resolution for a node.
    pub fn set_assign_pattern_resolution(
        &mut self,
        node_id: GlobalNodeIdAny,
        resolution: AssignPatternResolution,
    ) {
        self.assign_patterns.insert(node_id, resolution);
    }

    /// Get the assignment pattern resolution for a node.
    pub fn assign_pattern_resolution(
        &self,
        node_id: GlobalNodeIdAny,
    ) -> Option<&AssignPatternResolution> {
        self.assign_patterns.get(&node_id)
    }

    /// Iterate visible name resolutions.
    pub fn name_entries(&self) -> impl Iterator<Item = (GlobalNodeIdAny, &NameResolution)> + '_ {
        self.names
            .iter()
            .map(|(node_id, resolution)| (*node_id, resolution))
    }

    /// Iterate visible generic instantiation resolutions.
    pub fn instantiation_entries(
        &self,
    ) -> impl Iterator<Item = (GlobalNodeIdAny, &InstantiationResolution)> + '_ {
        self.instantiations
            .iter()
            .map(|(node_id, resolution)| (*node_id, resolution))
    }

    /// Iterate visible label resolutions.
    pub fn label_entries(&self) -> impl Iterator<Item = (GlobalNodeIdAny, &LabelResolution)> + '_ {
        self.labels
            .iter()
            .map(|(node_id, resolution)| (*node_id, resolution))
    }

    /// Iterate visible receiver resolutions.
    pub fn receiver_entries(
        &self,
    ) -> impl Iterator<Item = (GlobalNodeIdAny, &ReceiverResolution)> + '_ {
        self.receivers
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

    /// Iterate visible place resolutions.
    pub fn place_entries(&self) -> impl Iterator<Item = (GlobalNodeIdAny, &PlaceResolution)> + '_ {
        self.places
            .iter()
            .map(|(node_id, resolution)| (*node_id, resolution))
    }

    /// Iterate visible guard resolutions.
    pub fn guard_entries(&self) -> impl Iterator<Item = (GlobalNodeIdAny, &GuardResolution)> + '_ {
        self.guards
            .iter()
            .map(|(node_id, resolution)| (*node_id, resolution))
    }

    /// Iterate visible construct resolutions.
    pub fn construct_entries(
        &self,
    ) -> impl Iterator<Item = (GlobalNodeIdAny, &ConstructResolution)> + '_ {
        self.constructs
            .iter()
            .map(|(node_id, resolution)| (*node_id, resolution))
    }

    /// Iterate visible pattern resolutions.
    pub fn pattern_entries(
        &self,
    ) -> impl Iterator<Item = (GlobalNodeIdAny, &PatternResolution)> + '_ {
        self.patterns
            .iter()
            .map(|(node_id, resolution)| (*node_id, resolution))
    }

    /// Iterate visible assignment pattern resolutions.
    pub fn assign_pattern_entries(
        &self,
    ) -> impl Iterator<Item = (GlobalNodeIdAny, &AssignPatternResolution)> + '_ {
        self.assign_patterns
            .iter()
            .map(|(node_id, resolution)| (*node_id, resolution))
    }

    /// Return whether this segment has no resolutions.
    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
            && self.instantiations.is_empty()
            && self.labels.is_empty()
            && self.receivers.is_empty()
            && self.members.is_empty()
            && self.calls.is_empty()
            && self.places.is_empty()
            && self.guards.is_empty()
            && self.constructs.is_empty()
            && self.patterns.is_empty()
            && self.assign_patterns.is_empty()
    }

    /// Apply one mapping to every type id stored in this segment.
    ///
    /// Names and labels resolve to symbols only, so they need no mapping.
    pub fn map_type_ids(&mut self, map: &mut impl FnMut(GlobalTypeId) -> GlobalTypeId) {
        for resolution in self.instantiations.values_mut() {
            resolution.map_type_ids(map);
        }
        for resolution in self.receivers.values_mut() {
            resolution.map_type_ids(map);
        }
        for resolution in self.members.values_mut() {
            resolution.map_type_ids(map);
        }
        for resolution in self.calls.values_mut() {
            resolution.map_type_ids(map);
        }
        for resolution in self.places.values_mut() {
            resolution.map_type_ids(map);
        }
        for resolution in self.guards.values_mut() {
            resolution.map_type_ids(map);
        }
        for resolution in self.constructs.values_mut() {
            resolution.map_type_ids(map);
        }
        for resolution in self.patterns.values_mut() {
            resolution.map_type_ids(map);
        }
        for resolution in self.assign_patterns.values_mut() {
            resolution.map_type_ids(map);
        }
    }
}
