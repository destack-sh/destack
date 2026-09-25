use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tspp_core::FxIndexMap as IndexMap;
use tspp_serde::Reflect;
use tspp_source::ModuleId;

use crate::{GlobalNodeIdAny, GlobalSymbolId, NameResolution, Path, SegmentView};

/// Cumulative lexical resolutions for one DIR module.
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

    /// Get the name resolution for a node.
    pub fn name_resolution(&self, node_id: GlobalNodeIdAny) -> Option<&NameResolution> {
        self.segments
            .iter()
            .rev()
            .find_map(|segment| segment.name_resolution(node_id))
    }

    /// Get the name resolution for one path segment of a node.
    pub fn path_resolution(
        &self,
        node_id: GlobalNodeIdAny,
        segment: u16,
    ) -> Option<&NameResolution> {
        self.segments
            .iter()
            .rev()
            .find_map(|inner| inner.path_resolution(node_id, segment))
    }

    /// Get the unresolved reference path for a node.
    pub fn unresolved_reference(&self, node_id: GlobalNodeIdAny) -> Option<&Path> {
        self.segments
            .iter()
            .rev()
            .find_map(|segment| segment.unresolved_reference(node_id))
    }

    /// Iterate visible lexical name resolutions.
    pub fn name_entries(&self) -> impl Iterator<Item = (GlobalNodeIdAny, &NameResolution)> + '_ {
        self.segments
            .iter()
            .flat_map(|segment| segment.name_entries())
    }

    /// Iterate visible path segment resolutions.
    pub fn path_entries(
        &self,
    ) -> impl Iterator<Item = ((GlobalNodeIdAny, u16), &NameResolution)> + '_ {
        self.segments
            .iter()
            .flat_map(|segment| segment.path_entries())
    }

    /// Iterate visible unresolved reference paths.
    pub fn unresolved_entries(&self) -> impl Iterator<Item = (GlobalNodeIdAny, &Path)> + '_ {
        self.segments
            .iter()
            .flat_map(|segment| segment.unresolved_entries())
    }

    /// Return whether this table has no resolutions.
    pub fn is_empty(&self) -> bool {
        self.segments.iter().all(ResolutionSegment::is_empty)
    }
}

/// Lexical resolutions added by one DIR phase's walk.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct ResolutionSegment {
    /// The module id of the resolution segment.
    pub module_id: ModuleId,
    /// Resolved lexical names keyed by DIR node.
    names: IndexMap<GlobalNodeIdAny, NameResolution>,
    /// Resolved path segment names keyed by DIR node and segment index.
    paths: IndexMap<(GlobalNodeIdAny, u16), NameResolution>,
    /// Unresolved reference paths keyed by DIR node.
    unresolved: IndexMap<GlobalNodeIdAny, Path>,
}

impl ResolutionSegment {
    /// Create an empty resolution segment.
    pub fn new(module_id: ModuleId) -> Self {
        Self {
            module_id,
            names: IndexMap::default(),
            paths: IndexMap::default(),
            unresolved: IndexMap::default(),
        }
    }

    /// Set the lexical symbol resolution for a node.
    pub fn set_symbol_resolution(&mut self, node_id: GlobalNodeIdAny, symbol_id: GlobalSymbolId) {
        self.set_name_resolution(node_id, NameResolution::new(symbol_id));
    }

    /// Set the lexical name resolution for a node.
    pub fn set_name_resolution(&mut self, node_id: GlobalNodeIdAny, resolution: NameResolution) {
        self.names.insert(node_id, resolution);
    }

    /// Get the name resolution for a node.
    pub fn name_resolution(&self, node_id: GlobalNodeIdAny) -> Option<&NameResolution> {
        self.names.get(&node_id)
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

    /// Set the unresolved reference path for a node.
    pub fn set_unresolved_reference(&mut self, node_id: GlobalNodeIdAny, path: Path) {
        self.unresolved.insert(node_id, path);
    }

    /// Get the unresolved reference path for a node.
    pub fn unresolved_reference(&self, node_id: GlobalNodeIdAny) -> Option<&Path> {
        self.unresolved.get(&node_id)
    }

    /// Iterate lexical name resolutions.
    pub fn name_entries(&self) -> impl Iterator<Item = (GlobalNodeIdAny, &NameResolution)> + '_ {
        self.names.iter().map(|(node, name)| (*node, name))
    }

    /// Iterate the path segment resolutions in this segment.
    pub fn path_entries(
        &self,
    ) -> impl Iterator<Item = ((GlobalNodeIdAny, u16), &NameResolution)> + '_ {
        self.paths
            .iter()
            .map(|(key, resolution)| (*key, resolution))
    }

    /// Iterate the unresolved reference paths in this segment.
    pub fn unresolved_entries(&self) -> impl Iterator<Item = (GlobalNodeIdAny, &Path)> + '_ {
        self.unresolved
            .iter()
            .map(|(node_id, path)| (*node_id, path))
    }

    /// Drop every resolution an earlier sealed segment already carries identically.
    pub fn drop_carried(&mut self, sealed: &ResolutionSegment) {
        self.names
            .retain(|node, name| sealed.names.get(node) != Some(name));
        self.paths
            .retain(|key, resolution| sealed.paths.get(key) != Some(resolution));
        self.unresolved
            .retain(|node, path| sealed.unresolved.get(node) != Some(path));
    }

    /// Return whether this segment has no resolutions.
    pub fn is_empty(&self) -> bool {
        self.names.is_empty() && self.paths.is_empty() && self.unresolved.is_empty()
    }
}
