use destack_serde::Reflect;
use std::sync::Arc;

use destack_core::FxIndexMap as IndexMap;
use destack_source::ModuleId;
use serde::{Deserialize, Serialize};

use crate::{
    AccessResolution, AssignPatternResolution, AssignmentResolution, CallResolution,
    ConstructResolution, GlobalNodeIdAny, GlobalSymbolId, GlobalTypeId, GuardResolution,
    InstantiationResolution, MemberResolution, NameResolution, OperatorResolution, Path,
    PatternResolution, PlaceResolution, ReceiverResolution, SegmentView, SubscriptResolution,
    TreeResolution,
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

    /// Iterate visible path segment resolutions.
    pub fn path_entries(
        &self,
    ) -> impl Iterator<Item = (GlobalNodeIdAny, u16, &NameResolution)> + '_ {
        self.visible_entries(|segment| &segment.paths)
            .map(|((node_id, index), resolution)| (node_id, index, resolution))
    }

    /// Iterate visible generic instantiation resolutions.
    pub fn instantiation_entries(
        &self,
    ) -> impl Iterator<Item = (GlobalNodeIdAny, &InstantiationResolution)> + '_ {
        self.visible_entries(|segment| &segment.instantiations)
    }

    /// Iterate visible label resolutions.
    pub fn label_entries(&self) -> impl Iterator<Item = (GlobalNodeIdAny, &GlobalSymbolId)> + '_ {
        self.visible_entries(|segment| &segment.labels)
    }

    /// Iterate visible receiver resolutions.
    pub fn receiver_entries(
        &self,
    ) -> impl Iterator<Item = (GlobalNodeIdAny, &ReceiverResolution)> + '_ {
        self.visible_entries(|segment| &segment.receivers)
    }

    /// Iterate visible access resolutions.
    pub fn access_entries(
        &self,
    ) -> impl Iterator<Item = (GlobalNodeIdAny, &AccessResolution)> + '_ {
        self.visible_entries(|segment| &segment.accesses)
    }

    /// Iterate visible member resolutions.
    pub fn member_entries(
        &self,
    ) -> impl Iterator<Item = (GlobalNodeIdAny, &MemberResolution)> + '_ {
        self.visible_entries(|segment| &segment.members)
    }

    /// Iterate visible operator resolutions.
    pub fn operator_entries(
        &self,
    ) -> impl Iterator<Item = (GlobalNodeIdAny, &OperatorResolution)> + '_ {
        self.visible_entries(|segment| &segment.operators)
    }

    /// Iterate visible call resolutions.
    pub fn call_entries(&self) -> impl Iterator<Item = (GlobalNodeIdAny, &CallResolution)> + '_ {
        self.visible_entries(|segment| &segment.calls)
    }

    /// Iterate visible subscript resolutions.
    pub fn subscript_entries(
        &self,
    ) -> impl Iterator<Item = (GlobalNodeIdAny, &SubscriptResolution)> + '_ {
        self.visible_entries(|segment| &segment.subscripts)
    }

    /// Iterate visible place resolutions.
    pub fn place_entries(&self) -> impl Iterator<Item = (GlobalNodeIdAny, &PlaceResolution)> + '_ {
        self.visible_entries(|segment| &segment.places)
    }

    /// Iterate visible assignment resolutions.
    pub fn assignment_entries(
        &self,
    ) -> impl Iterator<Item = (GlobalNodeIdAny, &AssignmentResolution)> + '_ {
        self.visible_entries(|segment| &segment.assignments)
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

    /// Iterate visible unresolved reference paths.
    pub fn unresolved_entries(&self) -> impl Iterator<Item = (GlobalNodeIdAny, &Path)> + '_ {
        self.visible_entries(|segment| &segment.unresolved)
    }

    /// Iterate visible tree resolutions.
    pub fn tree_entries(&self) -> impl Iterator<Item = (GlobalNodeIdAny, &TreeResolution)> + '_ {
        self.visible_entries(|segment| &segment.trees)
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
        self.visible_entries(|segment| &segment.assigns)
    }

    /// Get the lexical symbol resolution for a node.
    pub fn symbol_resolution(&self, node_id: GlobalNodeIdAny) -> Option<GlobalSymbolId> {
        self.name_resolution(node_id).map(NameResolution::symbol)
    }

    /// Get the name resolution for a node.
    pub fn name_resolution(&self, node_id: GlobalNodeIdAny) -> Option<&NameResolution> {
        self.lookup(node_id, |segment| &segment.names)
    }

    /// Get the name resolution for one path segment of a node.
    pub fn path_resolution(
        &self,
        node_id: GlobalNodeIdAny,
        segment: u16,
    ) -> Option<&NameResolution> {
        self.lookup((node_id, segment), |segment| &segment.paths)
    }

    /// Get the explicit generic instantiation for a node.
    pub fn instantiation_resolution(
        &self,
        node_id: GlobalNodeIdAny,
    ) -> Option<&InstantiationResolution> {
        self.lookup(node_id, |segment| &segment.instantiations)
    }

    /// Get the label resolution for a node.
    pub fn label_resolution(&self, node_id: GlobalNodeIdAny) -> Option<GlobalSymbolId> {
        self.lookup(node_id, |segment| &segment.labels).copied()
    }

    /// Get the receiver resolution for a node.
    pub fn receiver_resolution(&self, node_id: GlobalNodeIdAny) -> Option<&ReceiverResolution> {
        self.lookup(node_id, |segment| &segment.receivers)
    }

    /// Get the checked access resolution for a node.
    pub fn access_resolution(&self, node_id: GlobalNodeIdAny) -> Option<&AccessResolution> {
        self.lookup(node_id, |segment| &segment.accesses)
    }

    /// Get the member resolution for a node.
    pub fn member_resolution(&self, node_id: GlobalNodeIdAny) -> Option<&MemberResolution> {
        self.lookup(node_id, |segment| &segment.members)
    }

    /// Get the operator resolution for a node.
    pub fn operator_resolution(&self, node_id: GlobalNodeIdAny) -> Option<&OperatorResolution> {
        self.lookup(node_id, |segment| &segment.operators)
    }

    /// Get the unresolved reference path recorded for a node.
    pub fn unresolved_reference(&self, node_id: GlobalNodeIdAny) -> Option<&Path> {
        self.lookup(node_id, |segment| &segment.unresolved)
    }

    /// Get the call resolution for a node.
    pub fn call_resolution(&self, node_id: GlobalNodeIdAny) -> Option<&CallResolution> {
        self.lookup(node_id, |segment| &segment.calls)
    }

    /// Get the subscript resolution for a node.
    pub fn subscript_resolution(&self, node_id: GlobalNodeIdAny) -> Option<&SubscriptResolution> {
        self.lookup(node_id, |segment| &segment.subscripts)
    }

    /// Get the place resolution for a node.
    pub fn place_resolution(&self, node_id: GlobalNodeIdAny) -> Option<&PlaceResolution> {
        self.lookup(node_id, |segment| &segment.places)
    }

    /// Get the assignment resolution for a node.
    pub fn assignment_resolution(&self, node_id: GlobalNodeIdAny) -> Option<&AssignmentResolution> {
        self.lookup(node_id, |segment| &segment.assignments)
    }

    /// Get the guard resolution for a node.
    pub fn guard_resolution(&self, node_id: GlobalNodeIdAny) -> Option<&GuardResolution> {
        self.lookup(node_id, |segment| &segment.guards)
    }

    /// Get the construct resolution for a node.
    pub fn construct_resolution(&self, node_id: GlobalNodeIdAny) -> Option<&ConstructResolution> {
        self.lookup(node_id, |segment| &segment.constructs)
    }

    /// Get the tree resolution for a node.
    pub fn tree_resolution(&self, node_id: GlobalNodeIdAny) -> Option<&TreeResolution> {
        self.lookup(node_id, |segment| &segment.trees)
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
        self.lookup(node_id, |segment| &segment.assigns)
    }

    /// Return whether this table has no resolutions.
    pub fn is_empty(&self) -> bool {
        self.segments.iter().all(|segment| segment.is_empty())
    }

    /// Look up the latest visible entry in one resolution column.
    fn lookup<K: std::hash::Hash + Eq + 'a, T>(
        &self,
        key: K,
        column: impl Fn(&ResolutionSegment) -> &IndexMap<K, T>,
    ) -> Option<&T> {
        for segment in self.segments.iter().rev() {
            if let Some(resolution) = column(segment).get(&key) {
                return Some(resolution);
            }
        }

        None
    }

    /// Iterate the visible entries in one resolution column.
    fn visible_entries<'b, K: std::hash::Hash + Eq + Copy + 'b, T: 'b>(
        &'b self,
        column: impl Fn(&ResolutionSegment) -> &IndexMap<K, T> + Copy + 'b,
    ) -> impl Iterator<Item = (K, &'b T)> + 'b {
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
    /// Checked path segment resolutions keyed by DIR node and segment index.
    pub(crate) paths: IndexMap<(GlobalNodeIdAny, u16), NameResolution>,
    /// Checked generic instantiations keyed by DIR node.
    pub(crate) instantiations: IndexMap<GlobalNodeIdAny, InstantiationResolution>,
    /// Checked label resolutions keyed by DIR node.
    pub(crate) labels: IndexMap<GlobalNodeIdAny, GlobalSymbolId>,
    /// Checked receiver resolutions keyed by DIR node.
    pub(crate) receivers: IndexMap<GlobalNodeIdAny, ReceiverResolution>,
    /// Checked stable storage accesses keyed by DIR node.
    pub(crate) accesses: IndexMap<GlobalNodeIdAny, AccessResolution>,
    /// Checked member resolutions keyed by DIR node.
    pub(crate) members: IndexMap<GlobalNodeIdAny, MemberResolution>,
    /// Checked operator resolutions keyed by DIR node.
    pub(crate) operators: IndexMap<GlobalNodeIdAny, OperatorResolution>,
    /// Checked call resolutions keyed by DIR node.
    pub(crate) calls: IndexMap<GlobalNodeIdAny, CallResolution>,
    /// Checked subscript resolutions keyed by DIR node.
    pub(crate) subscripts: IndexMap<GlobalNodeIdAny, SubscriptResolution>,
    /// Checked place resolutions keyed by DIR node.
    pub(crate) places: IndexMap<GlobalNodeIdAny, PlaceResolution>,
    /// Checked assignment resolutions keyed by DIR node.
    pub(crate) assignments: IndexMap<GlobalNodeIdAny, AssignmentResolution>,
    /// Checked guard resolutions keyed by DIR node.
    pub(crate) guards: IndexMap<GlobalNodeIdAny, GuardResolution>,
    /// Checked construct resolutions keyed by DIR node.
    pub(crate) constructs: IndexMap<GlobalNodeIdAny, ConstructResolution>,
    /// Checked tree literal resolutions keyed by expression node.
    pub(crate) trees: IndexMap<GlobalNodeIdAny, TreeResolution>,
    /// Checked pattern resolutions keyed by DIR node.
    pub(crate) patterns: IndexMap<GlobalNodeIdAny, PatternResolution>,
    /// Checked assignment pattern resolutions keyed by DIR node.
    pub(crate) assigns: IndexMap<GlobalNodeIdAny, AssignPatternResolution>,
    /// Unresolved reference paths keyed by DIR node.
    pub(crate) unresolved: IndexMap<GlobalNodeIdAny, Path>,
}

impl ResolutionSegment {
    /// Drop every resolution an earlier sealed segment already carries identically.
    pub fn drop_carried(&mut self, sealed: &ResolutionSegment) {
        self.names
            .retain(|node, resolution| sealed.names.get(node) != Some(resolution));
        self.paths
            .retain(|key, resolution| sealed.paths.get(key) != Some(resolution));
        self.instantiations
            .retain(|node, resolution| sealed.instantiations.get(node) != Some(resolution));
        self.labels
            .retain(|node, resolution| sealed.labels.get(node) != Some(resolution));
        self.receivers
            .retain(|node, resolution| sealed.receivers.get(node) != Some(resolution));
        self.accesses
            .retain(|node, resolution| sealed.accesses.get(node) != Some(resolution));
        self.places
            .retain(|node, resolution| sealed.places.get(node) != Some(resolution));
        self.members
            .retain(|node, resolution| sealed.members.get(node) != Some(resolution));
        self.operators
            .retain(|node, resolution| sealed.operators.get(node) != Some(resolution));
        self.calls
            .retain(|node, resolution| sealed.calls.get(node) != Some(resolution));
        self.subscripts
            .retain(|node, resolution| sealed.subscripts.get(node) != Some(resolution));
        self.assignments
            .retain(|node, resolution| sealed.assignments.get(node) != Some(resolution));
        self.unresolved
            .retain(|node, path| sealed.unresolved.get(node) != Some(path));
        self.guards
            .retain(|node, resolution| sealed.guards.get(node) != Some(resolution));
        self.constructs
            .retain(|node, resolution| sealed.constructs.get(node) != Some(resolution));
        self.trees
            .retain(|node, resolution| sealed.trees.get(node) != Some(resolution));
        self.patterns
            .retain(|node, resolution| sealed.patterns.get(node) != Some(resolution));
        self.assigns
            .retain(|node, resolution| sealed.assigns.get(node) != Some(resolution));
    }
}

/// Per-kind resolution counts marking one segment position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolutionMark {
    /// The per-kind map lengths at the mark.
    lengths: [usize; 17],
}

impl ResolutionSegment {
    /// Mark the current segment position for later truncation.
    pub fn mark(&self) -> ResolutionMark {
        ResolutionMark {
            lengths: [
                self.names.len(),
                self.paths.len(),
                self.instantiations.len(),
                self.labels.len(),
                self.receivers.len(),
                self.accesses.len(),
                self.members.len(),
                self.operators.len(),
                self.calls.len(),
                self.subscripts.len(),
                self.places.len(),
                self.assignments.len(),
                self.guards.len(),
                self.constructs.len(),
                self.trees.len(),
                self.patterns.len(),
                self.assigns.len(),
            ],
        }
    }

    /// Truncate resolutions back to one mark, newest first.
    pub fn truncate_to(&mut self, mark: ResolutionMark) {
        let [
            names,
            paths,
            instantiations,
            labels,
            receivers,
            accesses,
            members,
            operators,
            calls,
            subscripts,
            places,
            assignments,
            guards,
            constructs,
            trees,
            patterns,
            assigns,
        ] = mark.lengths;
        Self::truncate_map(&mut self.names, names);
        Self::truncate_map(&mut self.paths, paths);
        Self::truncate_map(&mut self.instantiations, instantiations);
        Self::truncate_map(&mut self.labels, labels);
        Self::truncate_map(&mut self.receivers, receivers);
        Self::truncate_map(&mut self.accesses, accesses);
        Self::truncate_map(&mut self.members, members);
        Self::truncate_map(&mut self.operators, operators);
        Self::truncate_map(&mut self.calls, calls);
        Self::truncate_map(&mut self.subscripts, subscripts);
        Self::truncate_map(&mut self.places, places);
        Self::truncate_map(&mut self.assignments, assignments);
        Self::truncate_map(&mut self.guards, guards);
        Self::truncate_map(&mut self.constructs, constructs);
        Self::truncate_map(&mut self.trees, trees);
        Self::truncate_map(&mut self.patterns, patterns);
        Self::truncate_map(&mut self.assigns, assigns);
    }

    /// Drop map entries added past one length.
    fn truncate_map<K: std::hash::Hash + Eq, T>(map: &mut IndexMap<K, T>, length: usize) {
        while map.len() > length {
            map.pop();
        }
    }
}

impl ResolutionSegment {
    /// Create an empty resolution segment.
    pub fn new(module_id: ModuleId) -> Self {
        Self {
            module_id,
            names: IndexMap::default(),
            paths: IndexMap::default(),
            instantiations: IndexMap::default(),
            labels: IndexMap::default(),
            receivers: IndexMap::default(),
            accesses: IndexMap::default(),
            members: IndexMap::default(),
            operators: IndexMap::default(),
            calls: IndexMap::default(),
            subscripts: IndexMap::default(),
            places: IndexMap::default(),
            assignments: IndexMap::default(),
            guards: IndexMap::default(),
            constructs: IndexMap::default(),
            trees: IndexMap::default(),
            patterns: IndexMap::default(),
            assigns: IndexMap::default(),
            unresolved: IndexMap::default(),
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

    /// Set the name resolution for one path segment of a node.
    pub fn set_path_resolution(
        &mut self,
        node_id: GlobalNodeIdAny,
        segment: u16,
        resolution: NameResolution,
    ) {
        self.paths.insert((node_id, segment), resolution);
    }

    /// Get the name resolution for one path segment of a node.
    pub fn path_resolution(
        &self,
        node_id: GlobalNodeIdAny,
        segment: u16,
    ) -> Option<&NameResolution> {
        self.paths.get(&(node_id, segment))
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
    pub fn set_label_resolution(&mut self, node_id: GlobalNodeIdAny, symbol: GlobalSymbolId) {
        self.labels.insert(node_id, symbol);
    }

    /// Get the label resolution for a node.
    pub fn label_resolution(&self, node_id: GlobalNodeIdAny) -> Option<GlobalSymbolId> {
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

    /// Set the checked access resolution for a node.
    pub fn set_access_resolution(
        &mut self,
        node_id: GlobalNodeIdAny,
        resolution: AccessResolution,
    ) {
        self.accesses.insert(node_id, resolution);
    }

    /// Get the checked access resolution for a node.
    pub fn access_resolution(&self, node_id: GlobalNodeIdAny) -> Option<&AccessResolution> {
        self.accesses.get(&node_id)
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

    /// Set the unresolved reference path for a node.
    pub fn set_unresolved_reference(&mut self, node_id: GlobalNodeIdAny, path: Path) {
        self.unresolved.insert(node_id, path);
    }

    /// Get the unresolved reference path for a node.
    pub fn unresolved_reference(&self, node_id: GlobalNodeIdAny) -> Option<&Path> {
        self.unresolved.get(&node_id)
    }

    /// Set the operator resolution for a node.
    pub fn set_operator_resolution(
        &mut self,
        node_id: GlobalNodeIdAny,
        resolution: OperatorResolution,
    ) {
        self.operators.insert(node_id, resolution);
    }

    /// Get the operator resolution for a node.
    pub fn operator_resolution(&self, node_id: GlobalNodeIdAny) -> Option<&OperatorResolution> {
        self.operators.get(&node_id)
    }

    /// Set the call resolution for a node.
    pub fn set_call_resolution(&mut self, node_id: GlobalNodeIdAny, resolution: CallResolution) {
        self.calls.insert(node_id, resolution);
    }

    /// Get the call resolution for a node.
    pub fn call_resolution(&self, node_id: GlobalNodeIdAny) -> Option<&CallResolution> {
        self.calls.get(&node_id)
    }

    /// Set the subscript resolution for a node.
    pub fn set_subscript_resolution(
        &mut self,
        node_id: GlobalNodeIdAny,
        resolution: SubscriptResolution,
    ) {
        self.subscripts.insert(node_id, resolution);
    }

    /// Get the subscript resolution for a node.
    pub fn subscript_resolution(&self, node_id: GlobalNodeIdAny) -> Option<&SubscriptResolution> {
        self.subscripts.get(&node_id)
    }

    /// Set the place resolution for a node.
    pub fn set_place_resolution(&mut self, node_id: GlobalNodeIdAny, resolution: PlaceResolution) {
        self.places.insert(node_id, resolution);
    }

    /// Get the place resolution for a node.
    pub fn place_resolution(&self, node_id: GlobalNodeIdAny) -> Option<&PlaceResolution> {
        self.places.get(&node_id)
    }

    /// Set the assignment resolution for a node.
    pub fn set_assignment_resolution(
        &mut self,
        node_id: GlobalNodeIdAny,
        resolution: AssignmentResolution,
    ) {
        self.assignments.insert(node_id, resolution);
    }

    /// Get the assignment resolution for a node.
    pub fn assignment_resolution(&self, node_id: GlobalNodeIdAny) -> Option<&AssignmentResolution> {
        self.assignments.get(&node_id)
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

    /// Set the tree resolution for a node.
    pub fn set_tree_resolution(&mut self, node_id: GlobalNodeIdAny, resolution: TreeResolution) {
        self.trees.insert(node_id, resolution);
    }

    /// Get the tree resolution for a node.
    pub fn tree_resolution(&self, node_id: GlobalNodeIdAny) -> Option<&TreeResolution> {
        self.trees.get(&node_id)
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
        self.assigns.insert(node_id, resolution);
    }

    /// Get the assignment pattern resolution for a node.
    pub fn assign_pattern_resolution(
        &self,
        node_id: GlobalNodeIdAny,
    ) -> Option<&AssignPatternResolution> {
        self.assigns.get(&node_id)
    }

    /// Iterate visible name resolutions.
    pub fn name_entries(&self) -> impl Iterator<Item = (GlobalNodeIdAny, &NameResolution)> + '_ {
        self.names
            .iter()
            .map(|(node_id, resolution)| (*node_id, resolution))
    }

    /// Iterate the path segment resolutions in this segment.
    pub fn path_entries(
        &self,
    ) -> impl Iterator<Item = (GlobalNodeIdAny, u16, &NameResolution)> + '_ {
        self.paths
            .iter()
            .map(|((node_id, segment), resolution)| (*node_id, *segment, resolution))
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
    pub fn label_entries(&self) -> impl Iterator<Item = (GlobalNodeIdAny, &GlobalSymbolId)> + '_ {
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

    /// Iterate checked access resolutions.
    pub fn access_entries(
        &self,
    ) -> impl Iterator<Item = (GlobalNodeIdAny, &AccessResolution)> + '_ {
        self.accesses
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

    /// Iterate visible operator resolutions.
    pub fn operator_entries(
        &self,
    ) -> impl Iterator<Item = (GlobalNodeIdAny, &OperatorResolution)> + '_ {
        self.operators
            .iter()
            .map(|(node_id, resolution)| (*node_id, resolution))
    }

    /// Iterate visible call resolutions.
    pub fn call_entries(&self) -> impl Iterator<Item = (GlobalNodeIdAny, &CallResolution)> + '_ {
        self.calls
            .iter()
            .map(|(node_id, resolution)| (*node_id, resolution))
    }

    /// Iterate visible subscript resolutions.
    pub fn subscript_entries(
        &self,
    ) -> impl Iterator<Item = (GlobalNodeIdAny, &SubscriptResolution)> + '_ {
        self.subscripts
            .iter()
            .map(|(node_id, resolution)| (*node_id, resolution))
    }

    /// Iterate visible place resolutions.
    pub fn place_entries(&self) -> impl Iterator<Item = (GlobalNodeIdAny, &PlaceResolution)> + '_ {
        self.places
            .iter()
            .map(|(node_id, resolution)| (*node_id, resolution))
    }

    /// Iterate visible assignment resolutions.
    pub fn assignment_entries(
        &self,
    ) -> impl Iterator<Item = (GlobalNodeIdAny, &AssignmentResolution)> + '_ {
        self.assignments
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

    /// Iterate visible tree resolutions.
    pub fn tree_entries(&self) -> impl Iterator<Item = (GlobalNodeIdAny, &TreeResolution)> + '_ {
        self.trees
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
        self.assigns
            .iter()
            .map(|(node_id, resolution)| (*node_id, resolution))
    }

    /// Return whether this segment has no resolutions.
    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
            && self.paths.is_empty()
            && self.instantiations.is_empty()
            && self.labels.is_empty()
            && self.receivers.is_empty()
            && self.accesses.is_empty()
            && self.members.is_empty()
            && self.operators.is_empty()
            && self.calls.is_empty()
            && self.subscripts.is_empty()
            && self.places.is_empty()
            && self.assignments.is_empty()
            && self.guards.is_empty()
            && self.constructs.is_empty()
            && self.trees.is_empty()
            && self.patterns.is_empty()
            && self.unresolved.is_empty()
            && self.assigns.is_empty()
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
        for resolution in self.operators.values_mut() {
            resolution.map_type_ids(map);
        }
        for resolution in self.calls.values_mut() {
            resolution.map_type_ids(map);
        }
        for resolution in self.subscripts.values_mut() {
            resolution.map_type_ids(map);
        }
        for resolution in self.places.values_mut() {
            resolution.map_type_ids(map);
        }
        for resolution in self.assignments.values_mut() {
            resolution.map_type_ids(map);
        }
        for resolution in self.guards.values_mut() {
            resolution.map_type_ids(map);
        }
        for resolution in self.trees.values_mut() {
            resolution.map_type_ids(map);
        }
        for resolution in self.constructs.values_mut() {
            resolution.map_type_ids(map);
        }
        for resolution in self.patterns.values_mut() {
            resolution.map_type_ids(map);
        }
        for resolution in self.assigns.values_mut() {
            resolution.map_type_ids(map);
        }
    }
}
