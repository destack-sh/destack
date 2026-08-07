use std::sync::Arc;

use destack_core::FxIndexMap as IndexMap;
use destack_serde::Reflect;
use destack_source::ModuleId;
use serde::{Deserialize, Serialize};

use crate::{
    DefinitionMember, FunctionRole, GlobalGenericTemplateId, GlobalNodeIdAny, GlobalSymbolId,
    GlobalTypeId, NameResolution, PropertyAccess, SegmentView, StaticKey,
};

/// Cumulative member bindings for one DIR module.
#[derive(Debug, Clone)]
pub struct MemberTable<'a> {
    /// The module id of the member table.
    pub module_id: ModuleId,
    /// The ordered member table segments.
    segments: SegmentView<'a, MemberSegment>,
}

impl MemberTable<'static> {
    /// Create a member table from ordered segments.
    pub fn from_segments(segments: Vec<Arc<MemberSegment>>) -> Self {
        let segments = SegmentView::from_segments(segments);

        Self::from_view(segments)
    }

    /// Create a member table from one segment.
    pub fn from_segment(segment: Arc<MemberSegment>) -> Self {
        Self::from_segments(vec![segment])
    }
}

impl<'a> MemberTable<'a> {
    /// Create a member table from a segment view.
    pub fn from_view(segments: SegmentView<'a, MemberSegment>) -> Self {
        let first = segments
            .first()
            .unwrap_or_else(|| panic!("member table needs at least one segment"));
        let module_id = first.module_id;

        // require a single module owner
        for segment in segments.iter() {
            assert_eq!(
                segment.module_id, module_id,
                "member table segment belongs to a different module"
            );
        }

        Self {
            module_id,
            segments,
        }
    }

    /// Return the member bindings stored for one subject.
    pub fn subject_bindings(&self, subject: &MemberSubject) -> Option<&[MemberBinding]> {
        for segment in self.segments.iter().rev() {
            if let Some(bindings) = segment.subject_bindings(subject) {
                return Some(bindings);
            }
        }

        None
    }

    /// Create a member table by appending a borrowed tail segment.
    pub fn with_tail<'b>(&'b self, tail: &'b MemberSegment) -> MemberTable<'b> {
        MemberTable::from_view(self.segments.with_tail(tail))
    }

    /// Return the lookup subject selected at one source site.
    pub fn subject(&self, site: MemberSite) -> Option<MemberSubject> {
        self.segments
            .iter()
            .rev()
            .find_map(|segment| segment.subject(site))
    }

    /// Return the member bindings available at one source site.
    pub fn members(&self, site: MemberSite) -> Option<&[MemberBinding]> {
        let subject = self.subject(site)?;

        self.segments
            .iter()
            .rev()
            .find_map(|segment| segment.members(subject))
    }

    /// Return one member binding available at a source site.
    pub fn binding(&self, site: MemberSite, key: StaticKey) -> Option<&MemberBinding> {
        self.members(site)?
            .iter()
            .find(|binding| binding.key == key)
    }

    /// Return whether this table has no member bindings.
    pub fn is_empty(&self) -> bool {
        self.segments.iter().all(MemberSegment::is_empty)
    }
}

/// Member bindings added by one DIR phase.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct MemberSegment {
    /// The module id of the member segment.
    pub module_id: ModuleId,
    /// Lookup subjects selected at source sites.
    subjects: IndexMap<MemberSite, MemberSubject>,
    /// Member bindings stored once per lookup subject.
    bindings: IndexMap<MemberSubject, Vec<MemberBinding>>,
}

/// One rollback position in a member segment.
#[derive(Debug, Clone, Copy)]
pub struct MemberMark {
    /// The recorded site count.
    subjects: usize,
}

impl MemberSegment {
    /// Create an empty member segment.
    pub fn new(module_id: ModuleId) -> Self {
        Self {
            module_id,
            subjects: IndexMap::default(),
            bindings: IndexMap::default(),
        }
    }

    /// Record the member lookup subject selected at one source site.
    pub fn record_subject(&mut self, site: MemberSite, subject: MemberSubject) {
        // require one stable subject per source site
        if let Some(recorded) = self.subjects.get(&site) {
            assert_eq!(
                *recorded, subject,
                "member site was recorded with a different subject"
            );
        } else {
            self.subjects.insert(site, subject);
        }
    }

    /// Return the member bindings stored for one subject.
    pub fn subject_bindings(&self, subject: &MemberSubject) -> Option<&[MemberBinding]> {
        self.bindings.get(subject).map(Vec::as_slice)
    }

    /// Set the member bindings stored for one lookup subject.
    pub fn set_bindings(&mut self, subject: MemberSubject, bindings: Vec<MemberBinding>) {
        self.bindings.insert(subject, bindings);
    }

    /// Iterate the member lookup subjects stored at source sites.
    pub fn iter_subjects(&self) -> impl Iterator<Item = (MemberSite, MemberSubject)> + '_ {
        self.subjects
            .iter()
            .map(|(site, subject)| (*site, *subject))
    }

    /// Return a rollback position for this segment.
    pub fn mark(&self) -> MemberMark {
        MemberMark {
            subjects: self.subjects.len(),
        }
    }

    /// Truncate this segment to a previous rollback position.
    pub fn truncate_to(&mut self, mark: MemberMark) {
        self.subjects.truncate(mark.subjects);
    }

    /// Return the lookup subject selected at one source site.
    pub fn subject(&self, site: MemberSite) -> Option<MemberSubject> {
        self.subjects.get(&site).copied()
    }

    /// Return the bindings stored for one lookup subject.
    pub fn members(&self, subject: MemberSubject) -> Option<&[MemberBinding]> {
        self.bindings.get(&subject).map(Vec::as_slice)
    }

    /// Return one member binding available at a source site.
    pub fn binding(&self, site: MemberSite, key: StaticKey) -> Option<&MemberBinding> {
        let subject = self.subject(site)?;

        self.members(subject)?
            .iter()
            .find(|binding| binding.key == key)
    }

    /// Return whether this segment has no member bindings.
    pub fn is_empty(&self) -> bool {
        self.subjects.is_empty() && self.bindings.is_empty()
    }
}

/// The exact input to member lookup.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct MemberSubject {
    /// The non-nullish receiver type.
    pub receiver: GlobalTypeId,
    /// The type searched by member lookup.
    pub target: GlobalTypeId,
    /// The selected instance or static member space.
    pub space: MemberSpace,
    /// The generic template assumptions active at the source site.
    pub scope: Option<GlobalGenericTemplateId>,
    /// The type whose member keys are enumerated.
    pub key_type: GlobalTypeId,
}

impl MemberSubject {
    /// Create one member lookup subject.
    pub fn new(receiver: GlobalTypeId, target: GlobalTypeId, space: MemberSpace) -> Self {
        Self {
            receiver,
            target,
            space,
            scope: None,
            key_type: target,
        }
    }

    /// Attach the generic assumptions active for this lookup.
    pub fn with_scope(mut self, scope: Option<GlobalGenericTemplateId>) -> Self {
        self.scope = scope;

        self
    }

    /// Enumerate keys from another type while retaining the lookup target.
    pub fn with_key_type(mut self, key_type: GlobalTypeId) -> Self {
        self.key_type = key_type;

        self
    }
}

/// One source member lookup location.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum MemberSite {
    /// A member expression or member type node.
    Node(GlobalNodeIdAny),
    /// One segment of a flat reference path.
    Path {
        /// The reference path node.
        node: GlobalNodeIdAny,
        /// The selected path segment.
        segment: u16,
    },
}

impl MemberSite {
    /// Return the source node containing this lookup.
    pub fn node(self) -> GlobalNodeIdAny {
        match self {
            Self::Node(node) | Self::Path { node, .. } => node,
        }
    }
}

/// One member selected by lookup.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct MemberBinding {
    /// The available member key.
    pub key: StaticKey,
    /// The available member kind.
    pub kind: MemberKind,
    /// The selected read and write types.
    pub access: PropertyAccess,
    /// Whether the member may be absent.
    pub is_optional: bool,
    /// The declarations contributing to this binding.
    pub declarations: Vec<MemberDeclaration>,
}

impl MemberBinding {
    /// Create one member binding.
    pub fn new(
        key: StaticKey,
        kind: MemberKind,
        access: PropertyAccess,
        is_optional: bool,
        declarations: Vec<MemberDeclaration>,
    ) -> Self {
        Self {
            key,
            kind,
            access,
            is_optional,
            declarations,
        }
    }

    /// Return the selected declarations as a name resolution.
    pub fn declaration_resolution(&self) -> Option<NameResolution> {
        let symbols = self
            .declarations
            .iter()
            .map(|declaration| declaration.symbol)
            .collect::<Vec<_>>();
        if symbols.is_empty() {
            None
        } else {
            Some(NameResolution::from_symbols(symbols))
        }
    }
}

/// One declaration contributing to a checked member binding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct MemberDeclaration {
    /// The selected declaration symbol.
    pub symbol: GlobalSymbolId,
    /// The declaration that exposed this member, through heritage.
    pub owner: GlobalSymbolId,
    /// The declaration family that exposed this member.
    pub origin: MemberOrigin,
    /// How the member behaves at a use site.
    pub role: MemberRole,
    /// The substituted callable type, when callable.
    pub callable_type: Option<GlobalTypeId>,
}

/// How one declaration member behaves at a use site.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum MemberRole {
    /// Field members use regular assignability and project their own type.
    Field,
    /// Method members use receiver-free method assignability.
    Method,
    /// Getter members project their return type.
    Getter,
    /// Setter members accept their first parameter type.
    Setter,
    /// Associated members use regular assignability.
    Associated,
    /// Variant values select one unit case.
    VariantValue,
    /// Variant constructors accept one payload value.
    VariantConstructor,
}

impl MemberRole {
    /// Return whether this role selects a callable declaration.
    pub fn is_callable(self) -> bool {
        matches!(
            self,
            Self::Method | Self::Getter | Self::Setter | Self::VariantConstructor
        )
    }

    /// Return the use-site role of one definition member.
    pub fn from_definition(member: &DefinitionMember) -> Option<Self> {
        match member {
            DefinitionMember::Field(_) => Some(Self::Field),
            DefinitionMember::Method(method) if method.role == Some(FunctionRole::Getter) => {
                Some(Self::Getter)
            }
            DefinitionMember::Method(method) if method.role == Some(FunctionRole::Setter) => {
                Some(Self::Setter)
            }
            DefinitionMember::Method(_) => Some(Self::Method),
            DefinitionMember::AssociatedType(_) | DefinitionMember::AssociatedConst(_) => {
                Some(Self::Associated)
            }
            DefinitionMember::EnumVariant(_) => Some(Self::VariantValue),
            DefinitionMember::TaggedKey(_) => Some(Self::VariantValue),
            DefinitionMember::TaggedVariant(variant) if variant.argument.is_some() => {
                Some(Self::VariantConstructor)
            }
            DefinitionMember::TaggedVariant(_) => Some(Self::VariantValue),
            DefinitionMember::CallSignature(_)
            | DefinitionMember::ConstructSignature(_)
            | DefinitionMember::IndexSignature(_) => None,
        }
    }

    /// Return whether this role can be read by member access.
    pub fn is_readable(self) -> bool {
        !matches!(self, Self::Setter)
    }
}

/// One member kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Reflect)]
pub enum MemberKind {
    /// Field member.
    Field,
    /// Getter or setter property member.
    Property,
    /// Method member.
    Method,
    /// Constructor member.
    Constructor,
    /// Call signature member.
    CallSignature,
    /// Construct signature member.
    ConstructSignature,
    /// Index signature member.
    IndexSignature,
    /// Associated type member.
    AssociatedType,
    /// Associated constant member.
    AssociatedConst,
    /// Enum variant member.
    Variant,
}

/// Member namespace selected by member lookup.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum MemberSpace {
    /// Instance members selected from a runtime receiver.
    Instance,
    /// Static members selected from a declaration receiver.
    Static,
}

/// The declaration family one member came from.
///
/// Direct declarations shadow rooted extensions, which shadow blanket extensions.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Reflect,
)]
pub enum MemberOrigin {
    /// A member declared by the owning declaration itself.
    Declaration,
    /// A member declared by an extension rooted at its target.
    RootedExtension,
    /// A member declared by an open blanket extension.
    BlanketExtension,
}
