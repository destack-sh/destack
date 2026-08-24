use std::sync::Arc;

use destack_core::FxIndexMap as IndexMap;
use destack_serde::Reflect;
use destack_source::ModuleId;
use serde::{Deserialize, Serialize};

use crate::{
    GlobalGenericTemplateId, GlobalNodeIdAny, GlobalSymbolId, GlobalTypeId, LanguageItem,
    SegmentView, TypeFold,
};

/// Interface whose implementation can be provided by compiler rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum AutoInterface {
    /// Values supported by atomic storage.
    AtomicSafe,
    /// Values safe to hold across a suspension point.
    SuspendSafe,
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
    /// User-facing display formatting interface.
    Display,
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
    /// Builtin strict equality marker.
    StrictEqual,
    /// Pin-move capability.
    Unpin,
    /// Zero-byte initialization capability.
    Zeroable,
}

impl AutoInterface {
    /// Every auto interface in declaration order.
    const ALL: [Self; 25] = [
        Self::AtomicSafe,
        Self::SuspendSafe,
        Self::Compare,
        Self::Concrete,
        Self::Copy,
        Self::Clone,
        Self::Debug,
        Self::Display,
        Self::Default,
        Self::Deserialize,
        Self::DynamicSafe,
        Self::Equal,
        Self::Float,
        Self::FloatDomain,
        Self::Hash,
        Self::Integer,
        Self::IntegerDomain,
        Self::OverwriteStable,
        Self::PartialCompare,
        Self::PartialEqual,
        Self::Serialize,
        Self::SharedSafe,
        Self::StrictEqual,
        Self::Unpin,
        Self::Zeroable,
    ];

    /// The representation markers sealed on every concrete nominal.
    pub const REPRESENTATION: [Self; 5] = [
        Self::Copy,
        Self::SharedSafe,
        Self::SuspendSafe,
        Self::OverwriteStable,
        Self::DynamicSafe,
    ];

    /// Return the auto interface named by one language item.
    pub fn from_language_item(item: LanguageItem) -> Option<Self> {
        match item {
            LanguageItem::AtomicSafe => Some(Self::AtomicSafe),
            LanguageItem::SuspendSafe => Some(Self::SuspendSafe),
            LanguageItem::Compare => Some(Self::Compare),
            LanguageItem::Concrete => Some(Self::Concrete),
            LanguageItem::Copy => Some(Self::Copy),
            LanguageItem::Clone => Some(Self::Clone),
            LanguageItem::Debug => Some(Self::Debug),
            LanguageItem::Default => Some(Self::Default),
            LanguageItem::Deserialize => Some(Self::Deserialize),
            LanguageItem::Display => Some(Self::Display),
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
            LanguageItem::StrictEqual => Some(Self::StrictEqual),
            LanguageItem::Unpin => Some(Self::Unpin),
            LanguageItem::Zeroable => Some(Self::Zeroable),
            _ => None,
        }
    }

    /// Return the source-facing interface name.
    pub fn name(self) -> &'static str {
        match self {
            Self::AtomicSafe => "AtomicSafe",
            Self::SuspendSafe => "SuspendSafe",
            Self::Compare => "Compare",
            Self::Concrete => "Concrete",
            Self::Copy => "Copy",
            Self::Clone => "Clone",
            Self::Debug => "Debug",
            Self::Default => "Default",
            Self::Deserialize => "Deserialize",
            Self::Display => "Display",
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
            Self::StrictEqual => "StrictEqual",
            Self::Unpin => "Unpin",
            Self::Zeroable => "Zeroable",
        }
    }

    /// Return whether this interface is memberless.
    pub fn is_marker(self) -> bool {
        match self {
            Self::AtomicSafe
            | Self::SuspendSafe
            | Self::Concrete
            | Self::Copy
            | Self::DynamicSafe
            | Self::OverwriteStable
            | Self::SharedSafe
            | Self::StrictEqual
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
            | Self::Display
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
            | Self::Display
            | Self::Equal
            | Self::Hash
            | Self::PartialCompare
            | Self::PartialEqual
            | Self::Serialize => true,
            Self::AtomicSafe
            | Self::SuspendSafe
            | Self::Copy
            | Self::DynamicSafe
            | Self::OverwriteStable
            | Self::SharedSafe
            | Self::StrictEqual
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
            Self::SuspendSafe
                | Self::OverwriteStable
                | Self::SharedSafe
                | Self::Unpin
                | Self::Zeroable
        )
    }

    /// Return whether the compiler derives this interface field-wise without annotation.
    pub fn is_auto_derivable(self) -> bool {
        matches!(
            self,
            Self::Copy
                | Self::Clone
                | Self::Debug
                | Self::Display
                | Self::Equal
                | Self::PartialEqual
                | Self::Hash
                | Self::SharedSafe
        )
    }

    /// Return whether the compiler implements this interface without declarations.
    pub fn has_builtin_implementation(self) -> bool {
        // include scalar ordering, which the compiler decides outside the auto set
        self.is_marker()
            || self.is_auto_derivable()
            || matches!(self, Self::Compare | Self::PartialCompare | Self::Default)
    }

    /// Return whether this interface takes the compared value as its argument.
    pub fn has_receiver_argument(self) -> bool {
        matches!(
            self,
            Self::Equal
                | Self::PartialEqual
                | Self::Compare
                | Self::PartialCompare
                | Self::StrictEqual
        )
    }

    /// Iterate every auto interface in declaration order.
    pub fn all() -> impl Iterator<Item = Self> {
        Self::ALL.into_iter()
    }

    /// Return whether a written derive decorator may name this interface.
    pub fn is_derivable(self) -> bool {
        self.is_auto_derivable()
            || matches!(
                self,
                Self::Default
                    | Self::Compare
                    | Self::PartialCompare
                    | Self::Serialize
                    | Self::Deserialize
            )
    }
}

impl From<AutoInterface> for LanguageItem {
    fn from(interface: AutoInterface) -> Self {
        match interface {
            AutoInterface::AtomicSafe => Self::AtomicSafe,
            AutoInterface::SuspendSafe => Self::SuspendSafe,
            AutoInterface::Compare => Self::Compare,
            AutoInterface::Concrete => Self::Concrete,
            AutoInterface::Copy => Self::Copy,
            AutoInterface::Clone => Self::Clone,
            AutoInterface::Debug => Self::Debug,
            AutoInterface::Default => Self::Default,
            AutoInterface::Deserialize => Self::Deserialize,
            AutoInterface::Display => Self::Display,
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
            AutoInterface::StrictEqual => Self::StrictEqual,
            AutoInterface::Unpin => Self::Unpin,
            AutoInterface::Zeroable => Self::Zeroable,
        }
    }
}

/// Generated implementation for one auto interface.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect, TypeFold)]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect, TypeFold)]
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

    /// Return whether one type satisfies one marker interface under the given assuming template.
    pub fn conforms(
        &self,
        target: GlobalTypeId,
        scope: Option<GlobalGenericTemplateId>,
        interface: AutoInterface,
    ) -> bool {
        self.segments
            .iter()
            .any(|segment| segment.conforms(target, scope, interface))
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
    /// The satisfied marker interfaces per conforming type, under the template whose bounds
    /// a type open in parameters assumes.
    conformances: IndexMap<(GlobalTypeId, Option<GlobalGenericTemplateId>), Vec<AutoInterface>>,
    /// The selected implementation per concrete declared owner and interface root.
    selected: IndexMap<(GlobalSymbolId, GlobalSymbolId), Option<GlobalSymbolId>>,
}

impl AutoSegment {
    /// Create an empty auto segment.
    pub fn new(module_id: ModuleId) -> Self {
        Self {
            module_id,
            implementations: Vec::new(),
            conformances: IndexMap::default(),
            selected: IndexMap::default(),
        }
    }

    /// Record the selected implementation of one declared owner at one interface root.
    pub fn set_selected(
        &mut self,
        owner: GlobalSymbolId,
        root: GlobalSymbolId,
        implementation: Option<GlobalSymbolId>,
    ) {
        self.selected.insert((owner, root), implementation);
    }

    /// Return the selected implementation of one declared owner at one interface root.
    pub fn selected(
        &self,
        owner: GlobalSymbolId,
        root: GlobalSymbolId,
    ) -> Option<Option<GlobalSymbolId>> {
        self.selected.get(&(owner, root)).copied()
    }

    /// Record one checked marker conformance, under the assuming template of an open type.
    pub fn push_conformance(
        &mut self,
        target: GlobalTypeId,
        scope: Option<GlobalGenericTemplateId>,
        interface: AutoInterface,
    ) {
        let interfaces = self.conformances.entry((target, scope)).or_default();
        if !interfaces.contains(&interface) {
            interfaces.push(interface);
        }
    }

    /// Record one generated implementation.
    pub fn push_implementation(&mut self, implementation: AutoDerivedImplementation) {
        self.implementations.push(implementation);
    }

    /// Return whether one type satisfies one marker interface in this segment under the given
    /// assuming template, a closed type's conformance holding in every scope.
    pub fn conforms(
        &self,
        target: GlobalTypeId,
        scope: Option<GlobalGenericTemplateId>,
        interface: AutoInterface,
    ) -> bool {
        [scope, None].iter().any(|scope| {
            self.conformances
                .get(&(target, *scope))
                .is_some_and(|interfaces| interfaces.contains(&interface))
        })
    }

    /// Iterate auto-derived implementations in insertion order.
    pub fn implementations(&self) -> impl Iterator<Item = &AutoDerivedImplementation> + '_ {
        self.implementations.iter()
    }

    /// Return whether this segment carries no entries.
    pub fn is_empty(&self) -> bool {
        self.implementations.is_empty() && self.conformances.is_empty() && self.selected.is_empty()
    }
}

impl TypeFold for AutoSegment {
    fn map_types<E>(
        &mut self,
        map: &mut impl FnMut(GlobalTypeId) -> Result<GlobalTypeId, E>,
    ) -> Result<(), E> {
        let conformances = std::mem::take(&mut self.conformances);
        for ((mut target, scope), interfaces) in conformances {
            target.map_types(map)?;
            let entries = self.conformances.entry((target, scope)).or_default();
            for interface in interfaces {
                if !entries.contains(&interface) {
                    entries.push(interface);
                }
            }
        }
        for implementation in &mut self.implementations {
            implementation.map_types(map)?;
        }

        Ok(())
    }
}
