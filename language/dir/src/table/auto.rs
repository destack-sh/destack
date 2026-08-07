use std::sync::Arc;

use destack_serde::Reflect;
use destack_source::ModuleId;
use serde::{Deserialize, Serialize};

use crate::{GlobalNodeIdAny, GlobalSymbolId, GlobalTypeId, LanguageItem, SegmentView};

/// Interface whose implementation can be provided by compiler rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum AutoInterface {
    /// Values supported by atomic storage.
    AtomicSafe,
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
    /// Float literals and builtin float formats.
    FloatDomain,
    /// Hashing interface.
    Hash,
    /// Builtin integer marker for any integer width.
    Integer,
    /// Integer literals, intervals, and builtin integer types.
    IntegerDomain,
    /// Non-exclusive overwrite marker.
    OverwriteStable,
    /// Partial ordered comparison interface.
    PartialCompare,
    /// Partial equality interface.
    PartialEqual,
    /// Structured serialization interface.
    Serialize,
    /// Shared-storage safety marker.
    SharedSafe,
    /// Pin-move capability.
    Unpin,
    /// Zero-byte initialization capability.
    Zeroable,
}

impl AutoInterface {
    /// The representation markers sealed on every concrete nominal.
    pub const REPRESENTATION: [Self; 4] = [
        Self::Copy,
        Self::SharedSafe,
        Self::OverwriteStable,
        Self::DynamicSafe,
    ];
}

impl AutoInterface {
    /// Return the auto interface named by one language item.
    pub fn from_language_item(item: LanguageItem) -> Option<Self> {
        match item {
            LanguageItem::AtomicSafe => Some(Self::AtomicSafe),
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
            LanguageItem::FloatDomain => Some(Self::FloatDomain),
            LanguageItem::Hash => Some(Self::Hash),
            LanguageItem::Integer => Some(Self::Integer),
            LanguageItem::IntegerDomain => Some(Self::IntegerDomain),
            LanguageItem::OverwriteStable => Some(Self::OverwriteStable),
            LanguageItem::PartialCompare => Some(Self::PartialCompare),
            LanguageItem::PartialEqual => Some(Self::PartialEqual),
            LanguageItem::Serialize => Some(Self::Serialize),
            LanguageItem::SharedSafe => Some(Self::SharedSafe),
            LanguageItem::Unpin => Some(Self::Unpin),
            LanguageItem::Zeroable => Some(Self::Zeroable),
            _ => None,
        }
    }

    /// Return the source-facing interface name.
    pub fn name(self) -> &'static str {
        match self {
            Self::AtomicSafe => "AtomicSafe",
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
            Self::FloatDomain => "FloatDomain",
            Self::Hash => "Hash",
            Self::Integer => "Integer",
            Self::IntegerDomain => "IntegerDomain",
            Self::OverwriteStable => "OverwriteStable",
            Self::PartialCompare => "PartialCompare",
            Self::PartialEqual => "PartialEqual",
            Self::Serialize => "Serialize",
            Self::SharedSafe => "SharedSafe",
            Self::Unpin => "Unpin",
            Self::Zeroable => "Zeroable",
        }
    }

    /// Return whether this interface is memberless.
    pub fn is_marker(self) -> bool {
        match self {
            Self::AtomicSafe
            | Self::Concrete
            | Self::Copy
            | Self::DynamicSafe
            | Self::OverwriteStable
            | Self::SharedSafe
            | Self::Unpin
            | Self::Zeroable
            | Self::Integer
            | Self::IntegerDomain
            | Self::Float
            | Self::FloatDomain => true,
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
            Self::AtomicSafe
            | Self::Copy
            | Self::DynamicSafe
            | Self::OverwriteStable
            | Self::SharedSafe
            | Self::Unpin
            | Self::Zeroable
            | Self::Integer
            | Self::IntegerDomain
            | Self::Float
            | Self::FloatDomain
            | Self::Concrete => false,
        }
    }

    /// Return whether an unsafe extension may assume this interface.
    pub fn permits_unsafe_implementation(self) -> bool {
        matches!(
            self,
            Self::OverwriteStable | Self::SharedSafe | Self::Unpin | Self::Zeroable
        )
    }
}

impl From<AutoInterface> for LanguageItem {
    fn from(interface: AutoInterface) -> Self {
        match interface {
            AutoInterface::AtomicSafe => Self::AtomicSafe,
            AutoInterface::Compare => Self::Compare,
            AutoInterface::Concrete => Self::Concrete,
            AutoInterface::Copy => Self::Copy,
            AutoInterface::Clone => Self::Clone,
            AutoInterface::Debug => Self::Debug,
            AutoInterface::Default => Self::Default,
            AutoInterface::Deserialize => Self::Deserialize,
            AutoInterface::DynamicSafe => Self::DynamicSafe,
            AutoInterface::Equal => Self::Equal,
            AutoInterface::Float => Self::Float,
            AutoInterface::FloatDomain => Self::FloatDomain,
            AutoInterface::Hash => Self::Hash,
            AutoInterface::Integer => Self::Integer,
            AutoInterface::IntegerDomain => Self::IntegerDomain,
            AutoInterface::OverwriteStable => Self::OverwriteStable,
            AutoInterface::PartialCompare => Self::PartialCompare,
            AutoInterface::PartialEqual => Self::PartialEqual,
            AutoInterface::Serialize => Self::Serialize,
            AutoInterface::SharedSafe => Self::SharedSafe,
            AutoInterface::Unpin => Self::Unpin,
            AutoInterface::Zeroable => Self::Zeroable,
        }
    }
}

/// One checked marker conformance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct AutoConformance {
    /// The satisfied marker interface.
    pub interface: AutoInterface,
    /// The conforming type.
    pub target: GlobalTypeId,
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

    /// Return whether one type satisfies one marker interface.
    pub fn conforms(&self, target: GlobalTypeId, interface: AutoInterface) -> bool {
        self.segments
            .iter()
            .any(|segment| segment.conforms(target, interface))
    }

    /// Iterate visible checked marker conformances.
    pub fn conformances(&self) -> impl Iterator<Item = &AutoConformance> + '_ {
        self.segments
            .iter()
            .flat_map(|segment| segment.conformances())
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
    /// Checked marker conformances in emission order.
    conformances: Vec<AutoConformance>,
}

impl AutoSegment {
    /// Create an empty auto segment.
    pub fn new(module_id: ModuleId) -> Self {
        Self {
            module_id,
            implementations: Vec::new(),
            conformances: Vec::new(),
        }
    }

    /// Record one checked marker conformance.
    pub fn push_conformance(&mut self, conformance: AutoConformance) {
        self.conformances.push(conformance);
    }

    /// Record one generated implementation.
    pub fn push_implementation(&mut self, implementation: AutoDerivedImplementation) {
        self.implementations.push(implementation);
    }

    /// Return whether one type satisfies one marker interface in this segment.
    pub fn conforms(&self, target: GlobalTypeId, interface: AutoInterface) -> bool {
        self.conformances
            .iter()
            .any(|conformance| conformance.target == target && conformance.interface == interface)
    }

    /// Iterate checked marker conformances in insertion order.
    pub fn conformances(&self) -> impl Iterator<Item = &AutoConformance> + '_ {
        self.conformances.iter()
    }

    /// Iterate auto-derived implementations in insertion order.
    pub fn implementations(&self) -> impl Iterator<Item = &AutoDerivedImplementation> + '_ {
        self.implementations.iter()
    }

    /// Return whether this segment has no implementations or conformances.
    pub fn is_empty(&self) -> bool {
        self.implementations.is_empty() && self.conformances.is_empty()
    }
}
