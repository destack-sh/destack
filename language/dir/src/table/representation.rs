use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tspp_core::FxIndexMap as IndexMap;
use tspp_serde::Reflect;
use tspp_source::ModuleId;

use crate::{AutoInterface, GlobalSymbolId, GlobalTypeId, Ownership, SegmentView, Space};

/// Cumulative layout policies for one DIR module.
#[derive(Debug, Clone)]
pub struct RepresentationTable<'a> {
    /// The module id of the representation table.
    pub module_id: ModuleId,
    /// The ordered representation table segments.
    segments: SegmentView<'a, RepresentationSegment>,
}

impl RepresentationTable<'static> {
    /// Create a representation table from ordered segments.
    pub fn from_segments(segments: Vec<Arc<RepresentationSegment>>) -> Self {
        Self::from_view(SegmentView::from_segments(segments))
    }
}

impl<'a> RepresentationTable<'a> {
    /// Create a representation table from a segment view.
    pub fn from_view(segments: SegmentView<'a, RepresentationSegment>) -> Self {
        let first = segments
            .first()
            .unwrap_or_else(|| panic!("representation table needs at least one segment"));

        Self {
            module_id: first.module_id,
            segments,
        }
    }

    /// Return whether one nominal declaration derives Copy, holding when its stored values do.
    pub fn derives_copy(&self, symbol: GlobalSymbolId) -> Option<bool> {
        self.segments
            .iter()
            .rev()
            .find_map(|segment| segment.derives_copy(symbol))
    }

    /// Return the recorded verdict of one auto interface on one closed type.
    pub fn auto(&self, ty: GlobalTypeId, interface: AutoInterface) -> Option<bool> {
        self.segments
            .iter()
            .rev()
            .find_map(|segment| segment.auto(ty, interface))
    }

    /// Return whether values of one closed type copy.
    pub fn copies(&self, ty: GlobalTypeId) -> Option<bool> {
        self.auto(ty, AutoInterface::Copy)
    }

    /// Return the recorded ownership one type's head defaults to.
    pub fn ownership(&self, ty: GlobalTypeId) -> Option<DefaultOwnership> {
        self.segments
            .iter()
            .rev()
            .find_map(|segment| segment.ownership(ty))
    }

    /// Return the space one nominal declaration's instances live in.
    pub fn space(&self, symbol: GlobalSymbolId) -> Option<Space> {
        self.segments
            .iter()
            .rev()
            .find_map(|segment| segment.space(symbol))
    }
}

/// The ownership one type's head defaults to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum DefaultOwnership {
    /// An open head whose ownership its bounds decide.
    Open,
    /// A head defaulting to one ownership.
    Decided(Ownership),
}

impl From<Option<Ownership>> for DefaultOwnership {
    fn from(ownership: Option<Ownership>) -> Self {
        match ownership {
            Some(ownership) => Self::Decided(ownership),
            None => Self::Open,
        }
    }
}

/// The layout policies the check pass commits for one module's nominal declarations.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct RepresentationSegment {
    /// The module id of the representation segment.
    pub module_id: ModuleId,
    /// Whether each nominal declaration derives Copy, holding when its stored values do.
    copy_derivations: IndexMap<GlobalSymbolId, bool>,
    /// The verdict of each auto interface decided on a closed type.
    auto: IndexMap<(GlobalTypeId, AutoInterface), bool>,
    /// The ownership each type's head defaults to.
    ownerships: IndexMap<GlobalTypeId, DefaultOwnership>,
    /// The space each nominal declaration's instances live in.
    spaces: IndexMap<GlobalSymbolId, Space>,
}

impl RepresentationSegment {
    /// Create an empty representation segment.
    pub fn new(module_id: ModuleId) -> Self {
        Self {
            module_id,
            copy_derivations: IndexMap::default(),
            auto: IndexMap::default(),
            ownerships: IndexMap::default(),
            spaces: IndexMap::default(),
        }
    }

    /// Set whether one nominal declaration derives Copy.
    pub fn set_derives_copy(&mut self, symbol: GlobalSymbolId, derives_copy: bool) {
        self.copy_derivations.insert(symbol, derives_copy);
    }

    /// Return whether one nominal declaration derives Copy.
    pub fn derives_copy(&self, symbol: GlobalSymbolId) -> Option<bool> {
        self.copy_derivations.get(&symbol).copied()
    }

    /// Record the verdict of one auto interface on one closed type.
    pub fn set_auto(&mut self, ty: GlobalTypeId, interface: AutoInterface, holds: bool) {
        self.auto.insert((ty, interface), holds);
    }

    /// Return the recorded verdict of one auto interface on one closed type.
    pub fn auto(&self, ty: GlobalTypeId, interface: AutoInterface) -> Option<bool> {
        self.auto.get(&(ty, interface)).copied()
    }

    /// Record the ownership one type's head defaults to.
    pub fn set_ownership(&mut self, ty: GlobalTypeId, ownership: DefaultOwnership) {
        self.ownerships.insert(ty, ownership);
    }

    /// Return the recorded ownership one type's head defaults to.
    pub fn ownership(&self, ty: GlobalTypeId) -> Option<DefaultOwnership> {
        self.ownerships.get(&ty).copied()
    }

    /// Set the space one nominal declaration's instances live in.
    pub fn set_space(&mut self, symbol: GlobalSymbolId, space: Space) {
        self.spaces.insert(symbol, space);
    }

    /// Return the space one nominal declaration's instances live in.
    pub fn space(&self, symbol: GlobalSymbolId) -> Option<Space> {
        self.spaces.get(&symbol).copied()
    }

    /// Return whether the segment commits no policy.
    pub fn is_empty(&self) -> bool {
        self.copy_derivations.is_empty()
            && self.auto.is_empty()
            && self.ownerships.is_empty()
            && self.spaces.is_empty()
    }
}
