use std::sync::Arc;

use destack_serde::Reflect;
use destack_source::ModuleId;
use serde::{Deserialize, Serialize};

use crate::{GlobalNodeIdAny, GlobalSymbolId, GlobalTypeId, LanguageItem, SegmentView};

/// Interface whose implementation can be provided by compiler rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum AutoInterface {
    /// Ordered comparison interface.
    Compare,
    /// Complete by-value storage representation.
    Concrete,
    /// Implicit duplication without ownership transfer.
    Copy,
    /// Explicit clone interface.
    Clone,
    /// Debug formatting interface.
    Debug,
    /// Conventional default-value interface.
    Default,
    /// Structured deserialization interface.
    Deserialize,
    /// Runtime erasure marker for `Dynamic<T>`.
    DynamicSafe,
    /// Equality interface.
    Equal,
    /// Builtin float marker for any float format.
    Float,
    /// Hashing interface.
    Hash,
    /// Builtin integer marker for any integer width.
    Integer,
    /// Non-exclusive overwrite marker.
    OverwriteStable,
    /// Partial ordered comparison interface.
    PartialCompare,
    /// Partial equality interface.
    PartialEqual,
    /// Structured serialization interface.
    Serialize,
    /// Worker-send capability.
    Send,
    /// Shared-storage capability.
    Sync,
    /// Pin-move capability.
    Unpin,
    /// Zero-byte initialization capability.
    Zeroable,
}

impl AutoInterface {
    /// Return the auto interface named by one language item.
    pub fn from_language_item(item: LanguageItem) -> Option<Self> {
        match item {
            LanguageItem::Compare => Some(Self::Compare),
            LanguageItem::Concrete => Some(Self::Concrete),
            LanguageItem::Copy => Some(Self::Copy),
            LanguageItem::Clone => Some(Self::Clone),
            LanguageItem::Debug => Some(Self::Debug),
            LanguageItem::Default => Some(Self::Default),
            LanguageItem::Deserialize => Some(Self::Deserialize),
            LanguageItem::DynamicSafe => Some(Self::DynamicSafe),
            LanguageItem::Equal => Some(Self::Equal),
            LanguageItem::Float => Some(Self::Float),
            LanguageItem::Hash => Some(Self::Hash),
            LanguageItem::Integer => Some(Self::Integer),
            LanguageItem::OverwriteStable => Some(Self::OverwriteStable),
            LanguageItem::PartialCompare => Some(Self::PartialCompare),
            LanguageItem::PartialEqual => Some(Self::PartialEqual),
            LanguageItem::Serialize => Some(Self::Serialize),
            LanguageItem::Send => Some(Self::Send),
            LanguageItem::Sync => Some(Self::Sync),
            LanguageItem::Unpin => Some(Self::Unpin),
            LanguageItem::Zeroable => Some(Self::Zeroable),
            _ => None,
        }
    }

    /// Return the source-facing interface name.
    pub fn name(self) -> &'static str {
        match self {
            Self::Compare => "Compare",
            Self::Concrete => "Concrete",
            Self::Copy => "Copy",
            Self::Clone => "Clone",
            Self::Debug => "Debug",
            Self::Default => "Default",
            Self::Deserialize => "Deserialize",
            Self::DynamicSafe => "DynamicSafe",
            Self::Equal => "Equal",
            Self::Float => "Float",
            Self::Hash => "Hash",
            Self::Integer => "Integer",
            Self::OverwriteStable => "OverwriteStable",
            Self::PartialCompare => "PartialCompare",
            Self::PartialEqual => "PartialEqual",
            Self::Serialize => "Serialize",
            Self::Send => "Send",
            Self::Sync => "Sync",
            Self::Unpin => "Unpin",
            Self::Zeroable => "Zeroable",
        }
    }

    /// Return whether this interface is memberless.
    pub fn is_marker(self) -> bool {
        match self {
            Self::Concrete
            | Self::Copy
            | Self::DynamicSafe
            | Self::OverwriteStable
            | Self::Send
            | Self::Sync
            | Self::Unpin
            | Self::Zeroable
            | Self::Integer
            | Self::Float => true,
            Self::Compare
            | Self::Clone
            | Self::Debug
            | Self::Default
            | Self::Deserialize
            | Self::Equal
            | Self::Hash
            | Self::PartialCompare
            | Self::PartialEqual
            | Self::Serialize => false,
        }
    }

    /// Return whether this interface is a scalar marker.
    pub fn is_scalar_marker(self) -> bool {
        matches!(self, Self::Integer | Self::Float)
    }

    /// Return whether satisfying this interface can generate members.
    pub fn has_generated_members(self) -> bool {
        match self {
            Self::Compare
            | Self::Clone
            | Self::Debug
            | Self::Default
            | Self::Deserialize
            | Self::Equal
            | Self::Hash
            | Self::PartialCompare
            | Self::PartialEqual
            | Self::Serialize => true,
            Self::Copy
            | Self::DynamicSafe
            | Self::OverwriteStable
            | Self::Send
            | Self::Sync
            | Self::Unpin
            | Self::Zeroable
            | Self::Integer
            | Self::Float
            | Self::Concrete => false,
        }
    }

    /// Return whether the checker proves this interface by auto conformance.
    pub fn has_auto_conformance(self) -> bool {
        self.is_marker() && self != Self::Concrete
    }
}

/// Generated implementation for one auto interface.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct AutoDerivedImplementation {
    /// The source node that requested this implementation.
    pub source: GlobalNodeIdAny,
    /// The generated interface.
    pub interface: AutoInterface,
    /// The implemented type.
    pub target: GlobalTypeId,
    /// The generated members.
    pub members: Vec<AutoImplementationMember>,
}

impl AutoDerivedImplementation {
    /// Map every type id embedded in this implementation.
    pub fn map_type_ids(&mut self, map: &mut impl FnMut(GlobalTypeId) -> GlobalTypeId) {
        self.target = map(self.target);
        for member in &mut self.members {
            member.map_type_ids(map);
        }
    }
}

/// Member generated for one auto-derived implementation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct AutoImplementationMember {
    /// The source node that owns the generated member.
    pub source: GlobalNodeIdAny,
    /// The generated member symbol.
    pub symbol: GlobalSymbolId,
    /// The generated member type.
    pub ty: GlobalTypeId,
}

impl AutoImplementationMember {
    /// Map every type id embedded in this member.
    pub fn map_type_ids(&mut self, map: &mut impl FnMut(GlobalTypeId) -> GlobalTypeId) {
        self.ty = map(self.ty);
    }
}

/// Cumulative auto-derived implementations for one DIR module.
#[derive(Debug, Clone)]
pub struct AutoTable<'a> {
    /// The module id of the auto table.
    pub module_id: ModuleId,
    /// The ordered auto table segments.
    segments: SegmentView<'a, AutoSegment>,
}

impl AutoTable<'static> {
    /// Create an auto table from ordered segments.
    pub fn from_segments(segments: Vec<Arc<AutoSegment>>) -> Self {
        let segments = SegmentView::from_segments(segments);

        Self::from_view(segments)
    }

    /// Create an auto table from one segment.
    pub fn from_segment(segment: Arc<AutoSegment>) -> Self {
        Self::from_segments(vec![segment])
    }
}

impl<'a> AutoTable<'a> {
    /// Create an auto table from a segment view.
    pub fn from_view(segments: SegmentView<'a, AutoSegment>) -> Self {
        let first = segments
            .first()
            .unwrap_or_else(|| panic!("auto table needs at least one segment"));
        let module_id = first.module_id;

        // require a single module owner
        for segment in segments.iter() {
            assert_eq!(
                segment.module_id, module_id,
                "auto table segment belongs to a different module"
            );
        }

        Self {
            module_id,
            segments,
        }
    }

    /// Create an auto table by appending a borrowed tail segment.
    pub fn with_tail<'b>(&'b self, tail: &'b AutoSegment) -> AutoTable<'b> {
        AutoTable::from_view(self.segments.with_tail(tail))
    }

    /// Iterate visible auto-derived implementations.
    pub fn implementations(&self) -> impl Iterator<Item = &AutoDerivedImplementation> + '_ {
        self.segments
            .iter()
            .flat_map(|segment| segment.implementations())
    }

    /// Return whether this table has no auto-derived implementations.
    pub fn is_empty(&self) -> bool {
        self.segments.iter().all(|segment| segment.is_empty())
    }
}

/// Auto-derived implementations added by one DIR phase.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct AutoSegment {
    /// The module id of the auto segment.
    pub module_id: ModuleId,
    /// Auto-derived implementations in emission order.
    implementations: Vec<AutoDerivedImplementation>,
}

impl AutoSegment {
    /// Create an empty auto segment.
    pub fn new(module_id: ModuleId) -> Self {
        Self {
            module_id,
            implementations: Vec::new(),
        }
    }

    /// Append one auto-derived implementation.
    pub fn push_implementation(&mut self, implementation: AutoDerivedImplementation) {
        self.implementations.push(implementation);
    }

    /// Iterate auto-derived implementations in insertion order.
    pub fn implementations(&self) -> impl Iterator<Item = &AutoDerivedImplementation> + '_ {
        self.implementations.iter()
    }

    /// Return whether this segment has no implementations.
    pub fn is_empty(&self) -> bool {
        self.implementations.is_empty()
    }

    /// Map every type id embedded in this segment.
    pub fn map_type_ids(&mut self, map: &mut impl FnMut(GlobalTypeId) -> GlobalTypeId) {
        for implementation in &mut self.implementations {
            implementation.map_type_ids(map);
        }
    }
}
