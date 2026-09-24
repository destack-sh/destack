use std::sync::Arc;

use destack_core::FxIndexMap as IndexMap;
use destack_serde::Reflect;
use destack_source::ModuleId;
use serde::{Deserialize, Serialize};

use crate::{
    ClassConstructorDefinition, DefinitionMember, FunctionRole, GlobalGenericTemplateId,
    GlobalNodeIdAny, GlobalSymbolId, GlobalTypeId, NameResolution, PropertyAccess, SegmentView,
    StaticKey, TypeFold, TypeListId,
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

    /// Create a member table by appending a borrowed tail segment.
    pub fn with_tail<'b>(&'b self, tail: &'b MemberSegment) -> MemberTable<'b> {
        MemberTable::from_view(self.segments.with_tail(tail))
    }

    /// Return the lookup subject and member key selected at one source site.
    pub fn subject(&self, site: MemberSite) -> Option<(MemberSubject, Option<StaticKey>)> {
        self.segments
            .iter()
            .rev()
            .find_map(|segment| segment.subject(site))
    }

    /// Return the member bindings flattened for one declared owner.
    pub fn bindings(&self, owner: GlobalSymbolId, space: MemberSpace) -> Option<&[MemberBinding]> {
        self.segments
            .iter()
            .rev()
            .find_map(|segment| segment.bindings(owner, space))
    }

    /// Return the membership projected at one source site.
    pub fn membership(&self, site: MemberSite) -> Option<&Membership> {
        let (subject, _) = self.subject(site)?;

        self.segments
            .iter()
            .rev()
            .find_map(|segment| segment.membership(&subject))
    }

    /// Return the members selected to satisfy one `implements` clause.
    pub fn conformance_members(&self, source: GlobalNodeIdAny) -> Option<&[MemberConformance]> {
        self.segments
            .iter()
            .rev()
            .find_map(|segment| segment.conformance_members(source))
    }

    /// Iterate every selected member conformance.
    pub fn member_conformances(&self) -> impl Iterator<Item = &MemberConformance> + '_ {
        self.segments
            .iter()
            .flat_map(|segment| segment.member_conformances())
    }

    /// Return the constructable backing alternatives derived for one newtype.
    pub fn newtype_constructors(&self, symbol: GlobalSymbolId) -> Option<&[NewtypeConstructor]> {
        self.segments
            .iter()
            .rev()
            .find_map(|segment| segment.newtype_constructors(symbol))
    }

    /// Return the construct candidates derived for one class.
    pub fn class_constructors(
        &self,
        symbol: GlobalSymbolId,
    ) -> Option<&[ClassConstructorDefinition]> {
        self.segments
            .iter()
            .rev()
            .find_map(|segment| segment.class_constructors(symbol))
    }

    /// Iterate member implementation edges.
    pub fn member_implementations(&self) -> impl Iterator<Item = ImplementationEdge> + '_ {
        self.member_conformances()
            .map(|conformance| ImplementationEdge {
                declaration: conformance.requirement,
                implementation: conformance.member,
            })
    }

    /// Iterate declarations satisfied by one member symbol.
    pub fn member_declarations(
        &self,
        symbol: GlobalSymbolId,
    ) -> impl Iterator<Item = GlobalSymbolId> + '_ {
        self.member_conformances().filter_map(move |conformance| {
            (conformance.member == symbol).then_some(conformance.requirement)
        })
    }

    /// Return one member binding projected at a source site.
    pub fn binding(&self, site: MemberSite, key: StaticKey) -> Option<&MemberBinding> {
        // read the membership the site's subject selects
        let (subject, _) = self.subject(site)?;
        let membership = self.membership(site)?;

        // answer from the structural bindings the subject projected
        if let Some(binding) = membership
            .structural
            .iter()
            .find(|binding| binding.key == key)
        {
            return Some(binding);
        }

        // answer from the first source owner declaring the key
        membership.sources.iter().find_map(|source| {
            self.bindings(source.owner, subject.space)?
                .iter()
                .find(|binding| binding.key == key)
        })
    }

    /// Return whether this table has no member bindings.
    pub fn is_empty(&self) -> bool {
        self.segments.iter().all(MemberSegment::is_empty)
    }
}

/// One source contributing declared members to a projected subject membership.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct MemberSource {
    /// The declaring owner whose flattened bindings contribute.
    pub owner: GlobalSymbolId,
    /// The type arguments reopening the owner's parameters, in declaration order.
    pub arguments: TypeListId,
}

/// The membership one settled subject selects.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct Membership {
    /// The receiver the sources' `this` types reopen at.
    pub receiver: GlobalTypeId,
    /// The contributing sources, the nominal owner first.
    pub sources: Vec<MemberSource>,
    /// Structural bindings whose types are already concrete.
    pub structural: Vec<MemberBinding>,
}

/// Member bindings added by one DIR phase.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct MemberSegment {
    /// The module id of the member segment.
    pub module_id: ModuleId,
    /// Lookup subjects selected at source sites, each with the member key a keyed lookup names.
    subjects: IndexMap<MemberSite, (MemberSubject, Option<StaticKey>)>,
    /// Member bindings flattened once per declared owner and space.
    bindings: IndexMap<(GlobalSymbolId, MemberSpace), Vec<MemberBinding>>,
    /// The membership each settled subject selects.
    memberships: IndexMap<MemberSubject, Membership>,
    /// The members selected to satisfy each `implements` clause, keyed by its source node.
    conformances: IndexMap<GlobalNodeIdAny, Vec<MemberConformance>>,
    /// The constructable backing alternatives derived per newtype, in selection order.
    newtype_constructors: IndexMap<GlobalSymbolId, Vec<NewtypeConstructor>>,
    /// The construct candidates derived per class, declared, forwarded, or default.
    class_constructors: IndexMap<GlobalSymbolId, Vec<ClassConstructorDefinition>>,
}

/// One member satisfying an interface requirement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, TypeFold)]
pub struct MemberConformance {
    /// The implementing member symbol.
    pub member: GlobalSymbolId,
    /// The required interface member symbol.
    pub requirement: GlobalSymbolId,
}

/// One constructable newtype backing alternative.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, TypeFold)]
pub struct NewtypeConstructor {
    /// The reduced backing alternative selected by this constructor.
    pub backing: GlobalTypeId,
    /// The callable constructor type.
    pub ty: GlobalTypeId,
}

/// One exact member implementation edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ImplementationEdge {
    /// The declared member requirement.
    pub declaration: GlobalSymbolId,
    /// The member satisfying the declaration.
    pub implementation: GlobalSymbolId,
}

impl MemberSegment {
    /// Create an empty member segment.
    pub fn new(module_id: ModuleId) -> Self {
        Self {
            module_id,
            subjects: IndexMap::default(),
            bindings: IndexMap::default(),
            memberships: IndexMap::default(),
            conformances: IndexMap::default(),
            newtype_constructors: IndexMap::default(),
            class_constructors: IndexMap::default(),
        }
    }

    /// Set the constructable backing alternatives derived for one newtype.
    pub fn set_newtype_constructors(
        &mut self,
        symbol: GlobalSymbolId,
        constructors: Vec<NewtypeConstructor>,
    ) {
        self.newtype_constructors.insert(symbol, constructors);
    }

    /// Return the constructable backing alternatives derived for one newtype.
    pub fn newtype_constructors(&self, symbol: GlobalSymbolId) -> Option<&[NewtypeConstructor]> {
        self.newtype_constructors.get(&symbol).map(Vec::as_slice)
    }

    /// Set the construct candidates derived for one class.
    pub fn set_class_constructors(
        &mut self,
        symbol: GlobalSymbolId,
        constructors: Vec<ClassConstructorDefinition>,
    ) {
        self.class_constructors.insert(symbol, constructors);
    }

    /// Return the construct candidates derived for one class.
    pub fn class_constructors(
        &self,
        symbol: GlobalSymbolId,
    ) -> Option<&[ClassConstructorDefinition]> {
        self.class_constructors.get(&symbol).map(Vec::as_slice)
    }

    /// Set the members selected to satisfy one `implements` clause.
    pub fn set_conformance_members(
        &mut self,
        source: GlobalNodeIdAny,
        members: Vec<MemberConformance>,
    ) {
        self.conformances.insert(source, members);
    }

    /// Return the members selected to satisfy one `implements` clause.
    pub fn conformance_members(&self, source: GlobalNodeIdAny) -> Option<&[MemberConformance]> {
        self.conformances.get(&source).map(Vec::as_slice)
    }

    /// Iterate every selected member conformance.
    pub fn member_conformances(&self) -> impl Iterator<Item = &MemberConformance> + '_ {
        self.conformances.values().flatten()
    }

    /// Commit the member lookup subject and key at one source site, where the resolved one wins.
    pub fn commit_subject(
        &mut self,
        site: MemberSite,
        subject: MemberSubject,
        key: Option<StaticKey>,
    ) {
        self.subjects.insert(site, (subject, key));
    }

    /// Set the member bindings flattened for one declared owner.
    pub fn set_bindings(
        &mut self,
        owner: GlobalSymbolId,
        space: MemberSpace,
        bindings: Vec<MemberBinding>,
    ) {
        self.bindings.insert((owner, space), bindings);
    }

    /// Return the member bindings flattened for one declared owner.
    pub fn bindings(&self, owner: GlobalSymbolId, space: MemberSpace) -> Option<&[MemberBinding]> {
        self.bindings.get(&(owner, space)).map(Vec::as_slice)
    }

    /// Iterate the member bindings flattened per declared owner.
    pub fn iter_bindings(
        &self,
    ) -> impl Iterator<Item = (GlobalSymbolId, MemberSpace, &[MemberBinding])> + '_ {
        self.bindings
            .iter()
            .map(|((owner, space), bindings)| (*owner, *space, bindings.as_slice()))
    }

    /// Set the membership projected for one resolved site subject.
    pub fn set_membership(&mut self, subject: MemberSubject, membership: Membership) {
        self.memberships.insert(subject, membership);
    }

    /// Return the membership projected for one resolved site subject.
    pub fn membership(&self, subject: &MemberSubject) -> Option<&Membership> {
        self.memberships.get(subject)
    }

    /// Iterate the memberships projected per resolved site subject.
    pub fn iter_memberships(&self) -> impl Iterator<Item = (&MemberSubject, &Membership)> + '_ {
        self.memberships.iter()
    }

    /// Iterate the member lookup subjects and keys stored at source sites.
    pub fn iter_subjects(
        &self,
    ) -> impl Iterator<Item = (MemberSite, MemberSubject, Option<StaticKey>)> + '_ {
        self.subjects
            .iter()
            .map(|(site, (subject, key))| (*site, *subject, *key))
    }

    /// Return the lookup subject and member key selected at one source site.
    pub fn subject(&self, site: MemberSite) -> Option<(MemberSubject, Option<StaticKey>)> {
        self.subjects.get(&site).copied()
    }

    /// Return whether this segment has no member bindings.
    pub fn is_empty(&self) -> bool {
        self.subjects.is_empty()
            && self.bindings.is_empty()
            && self.memberships.is_empty()
            && self.conformances.is_empty()
            && self.newtype_constructors.is_empty()
            && self.class_constructors.is_empty()
    }
}

/// The exact input to member lookup.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct MemberSubject {
    /// The use-site receiver type before implicit projections.
    pub receiver: GlobalTypeId,
    /// The type searched by member lookup.
    pub target: GlobalTypeId,
    /// The selected instance or static member space.
    pub space: MemberSpace,
    /// The generic template assumptions active at the source site.
    pub scope: Option<GlobalGenericTemplateId>,
    /// The type whose member keys are enumerated.
    pub key_source: GlobalTypeId,
}

impl MemberSubject {
    /// Create one member lookup subject.
    pub fn new(receiver: GlobalTypeId, target: GlobalTypeId, space: MemberSpace) -> Self {
        Self {
            receiver,
            target,
            space,
            scope: None,
            key_source: target,
        }
    }

    /// Attach the generic assumptions active for this lookup.
    pub fn with_scope(mut self, scope: Option<GlobalGenericTemplateId>) -> Self {
        self.scope = scope;

        self
    }

    /// Enumerate keys from another type while retaining the lookup target.
    pub fn with_key_source(mut self, key_source: GlobalTypeId) -> Self {
        self.key_source = key_source;

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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect, TypeFold)]
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

    /// Return the declaration a read of this member selects.
    pub fn read_declaration(&self) -> Option<&MemberDeclaration> {
        match self.declarations.as_slice() {
            [declaration] => Some(declaration),
            declarations => declarations
                .iter()
                .find(|declaration| declaration.role == MemberRole::Getter),
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect, TypeFold)]
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
}

impl From<Option<FunctionRole>> for MemberRole {
    /// Convert one function role into its member use-site role.
    fn from(role: Option<FunctionRole>) -> Self {
        match role {
            Some(FunctionRole::Getter) => Self::Getter,
            Some(FunctionRole::Setter) => Self::Setter,
            Some(FunctionRole::Constructor)
            | Some(FunctionRole::New)
            | Some(FunctionRole::Call)
            | None => Self::Method,
        }
    }
}

impl TryFrom<&DefinitionMember> for MemberRole {
    type Error = ();

    /// Convert one definition member into its use-site role.
    fn try_from(member: &DefinitionMember) -> Result<Self, Self::Error> {
        Self::from_definition(member).ok_or(())
    }
}

impl MemberRole {
    /// Return whether this role selects a callable declaration.
    pub fn is_callable(self) -> bool {
        matches!(self, Self::Method | Self::Getter | Self::Setter)
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

/// The declaration family one member came from, ordered by shadowing precedence.
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
