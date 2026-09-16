use std::sync::Arc;

use destack_core::{FxIndexMap as IndexMap, StringId};
use destack_serde::Reflect;
use destack_source::ModuleId;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use crate::{
    AutoInterface, EnumBackingType, EnumVariantValue, FunctionRole, GlobalNodeIdAny,
    GlobalSymbolId, GlobalTypeId, IntegerType, LocalGenericTemplateId, MemberKind, MemberSlot,
    MemberSpace, MethodAbstraction, PrimitiveType, SegmentView, Space, StaticKey, TypeFold,
    Visibility,
};

/// Cumulative declaration definitions for one DIR module.
#[derive(Debug, Clone)]
pub struct DefinitionTable<'a> {
    /// The module id of the definition table.
    pub module_id: ModuleId,
    /// The ordered definition table segments.
    segments: SegmentView<'a, DefinitionSegment>,
}

impl DefinitionTable<'static> {
    /// Create a definition table from ordered segments.
    pub fn from_segments(segments: Vec<Arc<DefinitionSegment>>) -> Self {
        let segments = SegmentView::from_segments(segments);

        Self::from_view(segments)
    }

    /// Create a definition table from one segment.
    pub fn from_segment(segment: Arc<DefinitionSegment>) -> Self {
        Self::from_segments(vec![segment])
    }
}

impl<'a> DefinitionTable<'a> {
    /// Create a definition table from a segment view.
    pub fn from_view(segments: SegmentView<'a, DefinitionSegment>) -> Self {
        let first = segments
            .first()
            .unwrap_or_else(|| panic!("definition table needs at least one segment"));
        let module_id = first.module_id;

        // require a single module owner
        for segment in segments.iter() {
            assert_eq!(
                segment.module_id, module_id,
                "definition table segment belongs to a different module"
            );
        }

        Self {
            module_id,
            segments,
        }
    }

    /// Return one definition by symbol.
    pub fn definition(&self, symbol: GlobalSymbolId) -> Option<&Definition> {
        self.definition_handle(symbol).map(Arc::as_ref)
    }

    /// Return one definition's shared handle by symbol.
    pub fn definition_handle(&self, symbol: GlobalSymbolId) -> Option<&Arc<Definition>> {
        self.segments
            .iter()
            .rev()
            .find_map(|segment| segment.definition_handle(symbol))
    }

    /// Return the source declaration node for one definition.
    pub fn definition_source(&self, symbol: GlobalSymbolId) -> Option<GlobalNodeIdAny> {
        for segment in self.segments.iter().rev() {
            if segment.definition(symbol).is_some() {
                return Some(segment.definition_source(symbol));
            }
        }

        None
    }

    /// Return one newtype definition by symbol.
    pub fn newtype_definition(&self, symbol: GlobalSymbolId) -> Option<&NewtypeDefinition> {
        match self.definition(symbol) {
            Some(Definition::Newtype(definition)) => Some(definition),
            _ => None,
        }
    }

    /// Return one enum definition by symbol.
    pub fn enum_definition(&self, symbol: GlobalSymbolId) -> Option<&EnumDefinition> {
        match self.definition(symbol) {
            Some(Definition::Enum(definition)) => Some(definition),
            _ => None,
        }
    }

    /// Return one extension definition by symbol.
    pub fn extension_definition(&self, symbol: GlobalSymbolId) -> Option<&ExtensionDefinition> {
        match self.definition(symbol) {
            Some(Definition::Extension(extension)) => Some(extension),
            _ => None,
        }
    }

    /// Iterate extension symbols targeting one root.
    pub fn root_extensions(&self, root: TypeRoot) -> impl Iterator<Item = GlobalSymbolId> + '_ {
        self.segments
            .iter()
            .flat_map(move |segment| segment.root_extensions(root).iter().copied())
    }

    /// Iterate blanket extension symbols.
    pub fn blanket_extensions(&self) -> impl Iterator<Item = GlobalSymbolId> + '_ {
        self.segments
            .iter()
            .flat_map(|segment| segment.blanket_extensions().iter().copied())
    }

    /// Iterate extensions in phase order.
    pub fn iter_extensions(
        &self,
    ) -> impl Iterator<Item = (GlobalSymbolId, &ExtensionDefinition)> + '_ {
        self.iter_definitions().filter_map(|(symbol, definition)| {
            if let Definition::Extension(extension) = definition {
                Some((symbol, extension))
            } else {
                None
            }
        })
    }

    /// Iterate definitions in phase order.
    pub fn iter_definitions(&self) -> impl Iterator<Item = (GlobalSymbolId, &Definition)> + '_ {
        let mut definitions = IndexMap::default();

        // apply later segment values over earlier ones
        for segment in self.segments.iter() {
            for (symbol, definition) in segment.iter_definitions() {
                definitions.insert(symbol, definition);
            }
        }

        definitions.into_iter()
    }

    /// Return one symbol-backed member and its declaring definition.
    pub fn member(
        &self,
        symbol: GlobalSymbolId,
    ) -> Option<(GlobalSymbolId, &Definition, &DefinitionMember)> {
        // resolve the member's declaring definition
        for segment in self.segments.iter().rev() {
            let Some(declaring) = segment.definition_by_member(symbol) else {
                continue;
            };

            // read the effective definition
            let definition = self.definition(declaring).unwrap_or_else(|| {
                panic!("DIR member symbol {symbol:?} has no declaring definition")
            });
            let member = definition
                .member(symbol)
                .unwrap_or_else(|| panic!("DIR definition {declaring:?} lost member {symbol:?}"));

            return Some((declaring, definition, member));
        }

        None
    }

    /// Return true when this table has no definitions.
    pub fn is_empty(&self) -> bool {
        self.segments.iter().all(|segment| segment.is_empty())
    }
}

/// Declaration definitions added by one DIR phase.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct DefinitionSegment {
    /// The module id of the definition segment.
    pub module_id: ModuleId,
    /// Source declaration nodes keyed by declaring symbol.
    pub(crate) sources: IndexMap<GlobalSymbolId, GlobalNodeIdAny>,
    /// Definitions keyed by declaring symbol.
    pub(crate) definitions: IndexMap<GlobalSymbolId, Arc<Definition>>,
    /// Declaring definition symbols keyed by member symbol.
    pub(crate) definitions_by_member: IndexMap<GlobalSymbolId, GlobalSymbolId>,
    /// Extension symbols by target root.
    pub(crate) extensions_by_root: IndexMap<TypeRoot, Vec<GlobalSymbolId>>,
    /// Blanket extension symbols.
    pub(crate) blanket_extensions: Vec<GlobalSymbolId>,
}

/// One indexable receiver root, keying extension lookup and conflicts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, TypeFold)]
pub enum TypeRoot {
    /// A declared nominal root.
    Declaration(GlobalSymbolId),
    /// A builtin primitive root.
    Primitive(PrimitiveType),
    /// The tuple constructor root.
    Tuple,
}

impl DefinitionSegment {
    /// Create a new definition segment.
    pub fn new(module_id: ModuleId) -> Self {
        Self {
            module_id,
            sources: IndexMap::default(),
            definitions: IndexMap::default(),
            definitions_by_member: IndexMap::default(),
            extensions_by_root: IndexMap::default(),
            blanket_extensions: Vec::new(),
        }
    }

    /// Insert one definition.
    pub fn insert_definition(
        &mut self,
        symbol: GlobalSymbolId,
        source: GlobalNodeIdAny,
        definition: Definition,
    ) {
        // index each symbol-backed member by its declaring definition
        for member in definition
            .members()
            .iter()
            .filter_map(DefinitionMember::symbol)
        {
            // reject reuse by another definition
            if let Some(previous) = self.definitions_by_member.insert(member, symbol) {
                assert_eq!(
                    previous, symbol,
                    "DIR member symbol {member:?} belongs to two definitions"
                );
            }
        }

        // index extensions by receiver root
        if let Definition::Extension(extension) = &definition {
            match extension.target.root() {
                Some(root) => {
                    self.extensions_by_root
                        .entry(root)
                        .or_default()
                        .push(symbol);
                }
                None => {
                    self.blanket_extensions.push(symbol);
                }
            }
        }

        // retain the definition and its source
        self.sources.insert(symbol, source);
        self.definitions.insert(symbol, Arc::new(definition));
    }

    /// Return the source declaration node for one definition when present.
    pub fn definition_source_maybe(&self, symbol: GlobalSymbolId) -> Option<GlobalNodeIdAny> {
        self.sources.get(&symbol).copied()
    }

    /// Return the source declaration node for one definition.
    pub fn definition_source(&self, symbol: GlobalSymbolId) -> GlobalNodeIdAny {
        *self
            .sources
            .get(&symbol)
            .unwrap_or_else(|| panic!("DIR definition symbol {symbol:?} has no source"))
    }

    /// Return one definition by symbol.
    pub fn definition(&self, symbol: GlobalSymbolId) -> Option<&Definition> {
        self.definitions.get(&symbol).map(Arc::as_ref)
    }

    /// Return one definition's shared handle by symbol.
    pub fn definition_handle(&self, symbol: GlobalSymbolId) -> Option<&Arc<Definition>> {
        self.definitions.get(&symbol)
    }

    /// Return the symbol of the definition declaring one member.
    pub fn definition_by_member(&self, symbol: GlobalSymbolId) -> Option<GlobalSymbolId> {
        self.definitions_by_member.get(&symbol).copied()
    }

    /// Get all extension symbols targeting one root.
    pub fn root_extensions(&self, root: TypeRoot) -> &[GlobalSymbolId] {
        self.extensions_by_root
            .get(&root)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    /// Get all blanket extension symbols.
    pub fn blanket_extensions(&self) -> &[GlobalSymbolId] {
        &self.blanket_extensions
    }

    /// Iterate definitions in insertion order.
    pub fn iter_definitions(&self) -> impl Iterator<Item = (GlobalSymbolId, &Definition)> + '_ {
        self.definitions
            .iter()
            .map(|(symbol, definition)| (*symbol, definition.as_ref()))
    }

    /// Return true when this segment has no definitions.
    pub fn is_empty(&self) -> bool {
        self.definitions.is_empty()
    }
}

/// Checked declaration data for one symbol.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect, TypeFold)]
pub enum Definition {
    /// Transparent type alias declaration.
    TypeAlias(TypeAliasDefinition),
    /// Struct declaration.
    Struct(StructDefinition),
    /// Class declaration.
    Class(ClassDefinition),
    /// Nominal interface declaration.
    Interface(InterfaceDefinition),
    /// Enum declaration.
    Enum(EnumDefinition),
    /// Newtype declaration.
    Newtype(NewtypeDefinition),
    /// Extension declaration.
    Extension(ExtensionDefinition),
}

impl Definition {
    /// Return whether this declaration's own shape admits one representation family.
    pub fn is_representable(&self, kind: RepresentationKind) -> bool {
        match (self, kind) {
            (Self::Struct(_), RepresentationKind::Destack | RepresentationKind::C)
            | (Self::Class(_), RepresentationKind::Destack) => true,
            (Self::Class(definition), RepresentationKind::C) => {
                !definition.declares_virtual_dispatch()
            }
            (Self::Struct(definition), RepresentationKind::Transparent) => {
                definition
                    .members
                    .iter()
                    .filter(|member| {
                        matches!(
                            member,
                            DefinitionMember::Field(field)
                                if field.space == MemberSpace::Instance
                        )
                    })
                    .count()
                    == 1
            }
            (Self::Enum(_), RepresentationKind::Destack | RepresentationKind::C) => true,
            (
                Self::Newtype(_),
                RepresentationKind::Destack
                | RepresentationKind::C
                | RepresentationKind::Transparent,
            ) => true,
            (Self::TypeAlias(_) | Self::Interface(_) | Self::Extension(_), _)
            | (Self::Struct(_), RepresentationKind::Integer(_))
            | (Self::Class(_), RepresentationKind::Transparent | RepresentationKind::Integer(_))
            | (Self::Enum(_), RepresentationKind::Transparent | RepresentationKind::Integer(_))
            | (Self::Newtype(_), RepresentationKind::Integer(_)) => false,
        }
    }

    /// Return this nominal declaration's concrete space.
    pub fn space(&self) -> Option<Space> {
        match self {
            Self::Struct(definition) => definition.space,
            Self::Class(definition) => definition.space,
            Self::Interface(definition) => definition.space,
            Self::Enum(definition) => definition.space,
            Self::Newtype(definition) => definition.space,
            Self::TypeAlias(_) | Self::Extension(_) => None,
        }
    }

    /// Return whether this definition has nominal identity.
    pub fn is_nominal(&self) -> bool {
        match self {
            Self::Struct(_) | Self::Class(_) | Self::Enum(_) | Self::Newtype(_) => true,
            Self::Interface(definition) => definition.is_nominal,
            Self::TypeAlias(_) | Self::Extension(_) => false,
        }
    }

    /// Return the generic template declared by this definition.
    pub fn generic_template(&self) -> Option<LocalGenericTemplateId> {
        match self {
            Self::TypeAlias(definition) => definition.template,
            Self::Struct(definition) => definition.template,
            Self::Class(definition) => definition.template,
            Self::Interface(definition) => definition.template,
            Self::Enum(definition) => definition.template,
            Self::Newtype(definition) => definition.template,
            Self::Extension(_) => None,
        }
    }
}

/// Checked declaration data for one transparent type alias.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect, TypeFold)]
pub struct TypeAliasDefinition {
    /// The generic template declared by the alias.
    pub template: Option<LocalGenericTemplateId>,
    /// The alias value.
    pub value: GlobalTypeId,
}

/// Checked declaration data for one nominal struct.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect, TypeFold)]
pub struct StructDefinition {
    /// The intrinsic instance space, or relative when absent.
    pub space: Option<Space>,
    /// The generic template declared by the struct.
    pub template: Option<LocalGenericTemplateId>,
    /// The implemented interfaces.
    pub implements: Vec<NominalConformance>,
    /// The written derive list replacing the auto set, if any.
    pub derives: Option<Vec<AutoInterface>>,
    /// The members in declaration order.
    pub members: Vec<DefinitionMember>,
}

/// Checked declaration data for one nominal class.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect, TypeFold)]
pub struct ClassDefinition {
    /// The intrinsic instance space, or relative when absent.
    pub space: Option<Space>,
    /// The generic template declared by the class.
    pub template: Option<LocalGenericTemplateId>,
    /// Whether the class is abstract.
    pub is_abstract: bool,
    /// Whether the class rejects subclasses.
    pub is_final: bool,
    /// The extended class.
    pub extends: Option<NominalHeritage>,
    /// The implemented interfaces.
    pub implements: Vec<NominalConformance>,
    /// The written derive list replacing the auto set, if any.
    pub derives: Option<Vec<AutoInterface>>,
    /// The members in declaration order.
    pub members: Vec<DefinitionMember>,
}

impl ClassDefinition {
    /// Return whether this class declares virtual dispatch.
    pub fn declares_virtual_dispatch(&self) -> bool {
        self.members.iter().any(|member| {
            matches!(
                member,
                DefinitionMember::Method(method)
                    if method.space == MemberSpace::Instance
                        && method.abstraction != MethodAbstraction::Concrete
            )
        })
    }
}

/// One class construct candidate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect, TypeFold)]
pub struct ClassConstructorDefinition {
    /// The selected constructor.
    pub constructor: ClassConstructor,
    /// The constructor signature.
    pub ty: GlobalTypeId,
}

/// Class construct candidate origin.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, TypeFold)]
pub enum ClassConstructor {
    /// Constructor explicitly declared by this class.
    Declared {
        /// The declared constructor symbol.
        symbol: GlobalSymbolId,
    },
    /// Default `new T()` candidate for a class with no declared constructor.
    Default,
    /// Constructor forwarded to an explicit base class constructor.
    ForwardedDeclared {
        /// The base class symbol.
        base: GlobalSymbolId,
        /// The selected base constructor symbol.
        symbol: GlobalSymbolId,
    },
    /// Constructor forwarded to a base class default constructor.
    ForwardedDefault {
        /// The base class symbol.
        base: GlobalSymbolId,
    },
}

impl ClassConstructor {
    /// Return the construct name for diagnostics.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Declared { .. } => "declared",
            Self::Default => "default",
            Self::ForwardedDeclared { .. } => "forwarded declared",
            Self::ForwardedDefault { .. } => "forwarded default",
        }
    }

    /// Return this constructor forwarded through one direct base class.
    pub fn forwarded(self, base: GlobalSymbolId) -> Self {
        match self {
            Self::Declared { symbol } | Self::ForwardedDeclared { symbol, .. } => {
                Self::ForwardedDeclared { base, symbol }
            }
            Self::Default | Self::ForwardedDefault { .. } => Self::ForwardedDefault { base },
        }
    }

    /// Return the function symbol called by this constructor, when one exists.
    pub fn call_symbol(&self) -> Option<GlobalSymbolId> {
        match self {
            Self::Declared { symbol } | Self::ForwardedDeclared { symbol, .. } => Some(*symbol),
            Self::Default | Self::ForwardedDefault { .. } => None,
        }
    }
}

/// Checked declaration data for one nominal interface.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect, TypeFold)]
pub struct InterfaceDefinition {
    /// The intrinsic instance space, or relative when absent.
    pub space: Option<Space>,
    /// The generic template declared by the interface.
    pub template: Option<LocalGenericTemplateId>,
    /// Whether the interface has nominal identity.
    pub is_nominal: bool,
    /// The inherited interfaces.
    pub extends: Vec<NominalHeritage>,
    /// The members in declaration order.
    pub members: Vec<DefinitionMember>,
}

/// Checked declaration data for one nominal enum.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect, TypeFold)]
pub struct EnumDefinition {
    /// The intrinsic instance space, or relative when absent.
    pub space: Option<Space>,
    /// The generic template declared by the enum.
    pub template: Option<LocalGenericTemplateId>,
    /// The scalar type backing every enum variant.
    pub backing: EnumBackingType,
    /// The implemented interfaces.
    pub implements: Vec<NominalConformance>,
    /// The written derive list replacing the auto set, if any.
    pub derives: Option<Vec<AutoInterface>>,
    /// The members in declaration order.
    pub members: Vec<DefinitionMember>,
}

/// Checked declaration data for one nominal type alias.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect, TypeFold)]
pub struct NewtypeDefinition {
    /// The intrinsic instance space, or relative when absent.
    pub space: Option<Space>,
    /// The generic template declared by the newtype.
    pub template: Option<LocalGenericTemplateId>,
    /// The nominal backing type.
    pub backing: GlobalTypeId,
    /// The backing visibility, gating construction and unwrapping outside the module.
    pub backing_visibility: Visibility,
    /// The written derive list replacing the auto set, if any.
    pub derives: Option<Vec<AutoInterface>>,
    /// The members in declaration order.
    pub members: Vec<DefinitionMember>,
}

/// How an extension declaration relates to its target type.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
    Reflect,
    TypeFold,
)]
pub enum ExtensionForm {
    /// Extension visible only inside its declaring module.
    Local,
    /// Extension visible outside its declaring module.
    Exported,
}

/// Checked declaration data for one extension.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect, TypeFold)]
pub struct ExtensionDefinition {
    /// The extension declaration's symbol.
    pub symbol: GlobalSymbolId,
    /// The extension declaration form.
    pub form: ExtensionForm,
    /// The extension's generic template.
    pub template: Option<LocalGenericTemplateId>,
    /// The receiver target.
    pub target: ExtensionTarget,
    /// The implemented interfaces.
    pub implements: Vec<NominalConformance>,
    /// The members in declaration order.
    pub members: Vec<DefinitionMember>,
}

impl ExtensionDefinition {
    /// Return the positively implemented interfaces.
    pub fn implementations(&self) -> impl Iterator<Item = &NominalConformance> {
        self.implements
            .iter()
            .filter(|conformance| conformance.polarity == Polarity::Positive)
    }

    /// Create a new extension.
    pub fn new(
        symbol: GlobalSymbolId,
        form: ExtensionForm,
        template: Option<LocalGenericTemplateId>,
        target: ExtensionTarget,
        implements: Vec<NominalConformance>,
        members: Vec<DefinitionMember>,
    ) -> Self {
        Self {
            symbol,
            form,
            template,
            target,
            implements,
            members,
        }
    }

    /// Return whether this extension is inherent.
    pub fn is_inherent(&self) -> bool {
        self.target
            .declaration()
            .is_some_and(|root| root.module_id == self.symbol.module_id)
    }

    /// Return whether this extension is exported.
    pub fn is_exported(&self) -> bool {
        matches!(self.form, ExtensionForm::Exported)
    }

    /// Return whether this extension is local.
    pub fn is_local(&self) -> bool {
        matches!(self.form, ExtensionForm::Local)
    }

    /// Return whether this extension is visible from one module.
    pub fn is_visible_from(&self, module_id: ModuleId) -> bool {
        self.symbol.module_id == module_id || self.is_exported()
    }
}

/// Extension lookup target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, TypeFold)]
pub enum ExtensionTarget {
    /// Extension whose receiver type names one indexable root.
    Rooted {
        /// The root used for member lookup and conflicts.
        root: TypeRoot,
        /// The receiver type.
        ty: GlobalTypeId,
    },
    /// Extension over an open receiver type.
    Blanket {
        /// The receiver type.
        ty: GlobalTypeId,
        /// The receiver coverage the declared bound decides.
        coverage: BlanketCoverage,
    },
}

/// Receiver coverage one blanket extension's declared bound decides.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, TypeFold)]
pub enum BlanketCoverage {
    /// The blanket covers every receiver type.
    Every,
    /// The blanket covers the receivers conforming to one interface.
    Interface(GlobalSymbolId),
    /// Use sites decide the coverage.
    Deferred,
}

impl ExtensionTarget {
    /// Return the receiver type.
    pub fn r#type(&self) -> GlobalTypeId {
        match self {
            Self::Rooted { ty, .. } | Self::Blanket { ty, .. } => *ty,
        }
    }

    /// Return the declaration symbol when this target roots at one.
    pub fn declaration(&self) -> Option<GlobalSymbolId> {
        match self {
            Self::Rooted {
                root: TypeRoot::Declaration(symbol),
                ..
            } => Some(*symbol),
            Self::Rooted { .. } | Self::Blanket { .. } => None,
        }
    }

    /// Return the lookup root when this target has one.
    pub fn root(&self) -> Option<TypeRoot> {
        match self {
            Self::Rooted { root, .. } => Some(*root),
            Self::Blanket { .. } => None,
        }
    }

    /// Return whether this is an open blanket target.
    pub fn is_blanket(&self) -> bool {
        matches!(self, Self::Blanket { .. })
    }

    /// Return the declared receiver coverage of this target.
    pub fn coverage(&self) -> BlanketCoverage {
        match self {
            Self::Rooted { .. } => BlanketCoverage::Deferred,
            Self::Blanket { coverage, .. } => *coverage,
        }
    }
}

/// One nominal heritage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect, TypeFold)]
pub struct NominalHeritage {
    /// The source heritage node.
    pub source: GlobalNodeIdAny,
    /// The applied heritage type.
    pub ty: GlobalTypeId,
}

/// One explicit interface conformance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect, TypeFold)]
pub struct NominalConformance {
    /// The source `implements` node.
    pub source: GlobalNodeIdAny,
    /// The applied interface type.
    pub interface: GlobalTypeId,
    /// Whether the declaration implements or refuses the interface.
    pub polarity: Polarity,
}

/// The direction of one implements clause.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, TypeFold)]
pub enum Polarity {
    /// `implements Copy`.
    Positive,
    /// `implements !Copy`.
    Negative,
}

/// One field member.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect, TypeFold)]
pub struct FieldDefinition {
    /// The member space declaring the field.
    pub space: MemberSpace,
    /// The member visibility.
    pub visibility: Visibility,
    /// The field symbol.
    pub symbol: GlobalSymbolId,
    /// The source member node.
    pub source: GlobalNodeIdAny,
    /// The field key.
    pub key: StaticKey,
    /// The field initializer expression, when one is declared.
    pub initializer: Option<GlobalNodeIdAny>,
    /// Whether the field is optional on its declaration.
    pub is_optional: bool,
    /// Whether the field rejects writes after initialization.
    pub is_readonly: bool,
    /// Whether subclasses must provide the field.
    pub is_abstract: bool,
    /// Whether the field overrides an inherited member.
    pub is_override: bool,
}

/// One method member.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect, TypeFold)]
pub struct MethodDefinition {
    /// The member space declaring the method.
    pub space: MemberSpace,
    /// The member visibility.
    pub visibility: Visibility,
    /// The method symbol.
    pub symbol: GlobalSymbolId,
    /// The source member node.
    pub source: GlobalNodeIdAny,
    /// The nominal member slot.
    pub slot: MemberSlot,
    /// The method role.
    pub role: Option<FunctionRole>,
    /// The abstraction mode governing overrides.
    pub abstraction: MethodAbstraction,
    /// Whether the method overrides an inherited member.
    pub is_override: bool,
    /// How the method receives its implementation.
    pub implementation: MemberImplementation,
}

/// How one member receives its implementation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, TypeFold)]
pub enum MemberImplementation {
    /// Implementers must supply the implementation.
    Required,
    /// The declaration supplies its own implementation.
    Own,
    /// The declaring interface supplies a fallback implementation.
    Default,
}

/// One associated type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect, TypeFold)]
pub struct AssociatedTypeDefinition {
    /// The associated type symbol.
    pub symbol: GlobalSymbolId,
    /// The source member node.
    pub source: GlobalNodeIdAny,
    /// The associated type key.
    pub key: StaticKey,
    /// The upper bound required by this associated type.
    pub constraint: Option<GlobalTypeId>,
    /// The concrete associated type value.
    pub value: Option<GlobalTypeId>,
    /// How the associated type receives its implementation.
    pub implementation: MemberImplementation,
}

/// One associated constant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect, TypeFold)]
pub struct AssociatedConstDefinition {
    /// The associated const symbol.
    pub symbol: GlobalSymbolId,
    /// The source member node.
    pub source: GlobalNodeIdAny,
    /// The associated const key.
    pub key: StaticKey,
    /// How the associated const receives its implementation.
    pub implementation: MemberImplementation,
}

/// One declared enum variant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect, TypeFold)]
pub struct EnumVariantDefinition {
    /// The variant symbol.
    pub symbol: GlobalSymbolId,
    /// The source enum field node.
    pub source: GlobalNodeIdAny,
    /// The variant key.
    pub key: StaticKey,
    /// The resolved scalar value.
    pub value: EnumVariantValue,
}

/// One symbol-free signature member.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect, TypeFold)]
pub struct SignatureDefinition {
    /// The source member node.
    pub source: GlobalNodeIdAny,
    /// The signature type.
    pub ty: GlobalTypeId,
}

/// One structural index signature member.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect, TypeFold)]
pub struct IndexSignatureDefinition {
    /// The source member node.
    pub source: GlobalNodeIdAny,
    /// The parameter name like `key`.
    pub name: StringId,
    /// The declared key domain.
    pub key_type: GlobalTypeId,
    /// The declared value type.
    pub value_type: GlobalTypeId,
    /// Whether the index signature may omit a value.
    pub is_optional: bool,
    /// Whether the index signature rejects writes.
    pub is_readonly: bool,
}

/// One declaration member.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect, TypeFold)]
pub enum DefinitionMember {
    /// Field member.
    Field(FieldDefinition),
    /// Method member.
    Method(MethodDefinition),
    /// Associated type member.
    AssociatedType(AssociatedTypeDefinition),
    /// Associated constant member.
    AssociatedConst(AssociatedConstDefinition),
    /// Declared enum variant member.
    EnumVariant(EnumVariantDefinition),
    /// Structural call signature member.
    CallSignature(SignatureDefinition),
    /// Structural construct signature member.
    ConstructSignature(SignatureDefinition),
    /// Structural index signature member.
    IndexSignature(IndexSignatureDefinition),
}

impl DefinitionMember {
    /// Return the member kind.
    pub fn kind(&self) -> MemberKind {
        match self {
            Self::Field(_) => MemberKind::Field,
            Self::Method(method)
                if matches!(
                    method.role,
                    Some(FunctionRole::Getter | FunctionRole::Setter)
                ) =>
            {
                MemberKind::Property
            }
            Self::Method(method) => match method.slot {
                MemberSlot::Constructor | MemberSlot::New => MemberKind::Constructor,
                MemberSlot::Call => MemberKind::CallSignature,
                MemberSlot::Key(_) => MemberKind::Method,
            },
            Self::AssociatedType(_) => MemberKind::AssociatedType,
            Self::AssociatedConst(_) => MemberKind::AssociatedConst,
            Self::EnumVariant(_) => MemberKind::Variant,
            Self::CallSignature(_) => MemberKind::CallSignature,
            Self::ConstructSignature(_) => MemberKind::ConstructSignature,
            Self::IndexSignature(_) => MemberKind::IndexSignature,
        }
    }

    /// Return whether this member has a source-level name.
    pub fn is_named(&self) -> bool {
        matches!(self.key(), Some(StaticKey::Name(_)))
            || matches!(
                self,
                Self::Method(MethodDefinition {
                    slot: MemberSlot::Constructor | MemberSlot::New,
                    ..
                })
            )
    }

    /// Return whether this member accepts writes.
    pub fn is_writable(&self) -> bool {
        matches!(self, Self::Field(field) if !field.is_readonly)
            || matches!(self, Self::Method(method) if method.role == Some(FunctionRole::Setter))
            || matches!(self, Self::IndexSignature(index) if !index.is_readonly)
    }

    /// Return whether this member may be absent.
    pub fn is_optional(&self) -> bool {
        matches!(self, Self::Field(field) if field.is_optional)
            || matches!(self, Self::IndexSignature(index) if index.is_optional)
    }

    /// Return the source node declaring this member.
    pub fn source(&self) -> GlobalNodeIdAny {
        match self {
            Self::Field(field) => field.source,
            Self::Method(method) => method.source,
            Self::AssociatedType(associated) => associated.source,
            Self::AssociatedConst(associated) => associated.source,
            Self::EnumVariant(variant) => variant.source,
            Self::CallSignature(signature) | Self::ConstructSignature(signature) => {
                signature.source
            }
            Self::IndexSignature(signature) => signature.source,
        }
    }

    /// Return whether this member can share a key as an overload.
    pub fn is_overloadable(&self) -> bool {
        matches!(self, Self::Method(_))
    }

    /// Return the member space declaring this member.
    pub fn space(&self) -> MemberSpace {
        match self {
            Self::Field(field) => field.space,
            Self::Method(method) => method.space,
            // associated members and variants live on the declaration
            Self::AssociatedType(_) | Self::AssociatedConst(_) | Self::EnumVariant(_) => {
                MemberSpace::Static
            }
            // structural signatures describe instances
            Self::CallSignature(_) | Self::ConstructSignature(_) | Self::IndexSignature(_) => {
                MemberSpace::Instance
            }
        }
    }

    /// Return whether this member is an associated type or const.
    pub fn is_associated(&self) -> bool {
        matches!(self, Self::AssociatedType(_) | Self::AssociatedConst(_))
    }

    /// Return the declaring member symbol.
    pub fn symbol(&self) -> Option<GlobalSymbolId> {
        match self {
            Self::Field(field) => Some(field.symbol),
            Self::Method(method) => Some(method.symbol),
            Self::AssociatedType(associated) => Some(associated.symbol),
            Self::AssociatedConst(associated) => Some(associated.symbol),
            Self::EnumVariant(variant) => Some(variant.symbol),
            Self::CallSignature(_) | Self::ConstructSignature(_) | Self::IndexSignature(_) => None,
        }
    }

    /// Return the member key when the member is keyed.
    pub fn key(&self) -> Option<StaticKey> {
        match self {
            Self::Field(field) => Some(field.key),
            Self::Method(method) => match method.slot {
                MemberSlot::Key(key) => Some(key),
                MemberSlot::Constructor | MemberSlot::New | MemberSlot::Call => None,
            },
            Self::AssociatedType(associated) => Some(associated.key),
            Self::AssociatedConst(associated) => Some(associated.key),
            Self::EnumVariant(variant) => Some(variant.key),
            Self::CallSignature(_) | Self::ConstructSignature(_) | Self::IndexSignature(_) => None,
        }
    }

    /// Return the symbol whose type carries this member's value.
    pub fn type_symbol(&self) -> Option<GlobalSymbolId> {
        // associated types carry their value type directly
        match self {
            Self::AssociatedType(_) => None,
            _ => self.symbol(),
        }
    }

    /// Return the value type carried directly by this member.
    pub fn value_type(&self) -> Option<GlobalTypeId> {
        match self {
            Self::AssociatedType(associated) => associated.value,
            Self::CallSignature(signature) | Self::ConstructSignature(signature) => {
                Some(signature.ty)
            }
            Self::IndexSignature(signature) => Some(signature.value_type),
            Self::Field(_) | Self::Method(_) | Self::AssociatedConst(_) | Self::EnumVariant(_) => {
                None
            }
        }
    }
}

impl Definition {
    /// Return the member carrying one exact symbol.
    pub fn member(&self, symbol: GlobalSymbolId) -> Option<&DefinitionMember> {
        self.members()
            .iter()
            .find(|member| member.symbol() == Some(symbol))
    }

    /// Return the nominal owner of this definition's members.
    pub fn member_owner(&self, declaring: GlobalSymbolId) -> Option<GlobalSymbolId> {
        match self {
            Self::Extension(extension) => extension.target.declaration(),
            _ => Some(declaring),
        }
    }

    /// Return the language item owner for this definition's members.
    pub fn language_member_owner(&self, declaring: GlobalSymbolId) -> Option<GlobalSymbolId> {
        let Self::Extension(extension) = self else {
            return Some(declaring);
        };

        // resolve package-owned extensions through their target
        match extension.target {
            ExtensionTarget::Rooted {
                root: TypeRoot::Declaration(target),
                ..
            } if extension.symbol.module_id.package_id == target.module_id.package_id => {
                Some(target)
            }
            ExtensionTarget::Blanket {
                coverage: BlanketCoverage::Interface(interface),
                ..
            } if extension.symbol.module_id.package_id == interface.module_id.package_id => {
                Some(interface)
            }
            ExtensionTarget::Rooted { .. } | ExtensionTarget::Blanket { .. } => None,
        }
    }

    /// Return the declared generic template, when the definition has one.
    pub fn template(&self) -> Option<LocalGenericTemplateId> {
        match self {
            Self::TypeAlias(definition) => definition.template,
            Self::Struct(definition) => definition.template,
            Self::Class(definition) => definition.template,
            Self::Interface(definition) => definition.template,
            Self::Enum(definition) => definition.template,
            Self::Newtype(definition) => definition.template,
            Self::Extension(extension) => extension.template,
        }
    }

    /// Return one declared member's visibility.
    pub fn member_visibility(&self, member: GlobalSymbolId) -> Option<Visibility> {
        self.members().iter().find_map(|declared| match declared {
            DefinitionMember::Field(field) if field.symbol == member => Some(field.visibility),
            DefinitionMember::Method(method) if method.symbol == member => Some(method.visibility),
            _ => None,
        })
    }

    /// Return the members in declaration order.
    pub fn members(&self) -> &[DefinitionMember] {
        match self {
            Self::Struct(definition) => &definition.members,
            Self::Class(definition) => &definition.members,
            Self::Interface(definition) => &definition.members,
            Self::Enum(definition) => &definition.members,
            Self::Newtype(definition) => &definition.members,
            Self::Extension(extension) => &extension.members,
            Self::TypeAlias(_) => &[],
        }
    }

    /// Return the members in declaration order for in-place mutation.
    pub fn members_mut(&mut self) -> &mut [DefinitionMember] {
        match self {
            Self::Struct(definition) => &mut definition.members,
            Self::Class(definition) => &mut definition.members,
            Self::Interface(definition) => &mut definition.members,
            Self::Enum(definition) => &mut definition.members,
            Self::Newtype(definition) => &mut definition.members,
            Self::Extension(extension) => &mut extension.members,
            Self::TypeAlias(_) => &mut [],
        }
    }

    /// Return the interfaces implemented by this definition.
    pub fn derives(&self) -> Option<&[AutoInterface]> {
        match self {
            Self::Struct(definition) => definition.derives.as_deref(),
            Self::Class(definition) => definition.derives.as_deref(),
            Self::Enum(definition) => definition.derives.as_deref(),
            Self::Newtype(definition) => definition.derives.as_deref(),
            Self::TypeAlias(_) | Self::Interface(_) | Self::Extension(_) => None,
        }
    }

    /// Return the implemented interfaces.
    pub fn implementations(&self) -> impl Iterator<Item = &NominalConformance> {
        self.conformances()
            .iter()
            .filter(|conformance| conformance.polarity == Polarity::Positive)
    }

    /// Return the negatively implemented interfaces.
    pub fn negatives(&self) -> impl Iterator<Item = &NominalConformance> {
        self.conformances()
            .iter()
            .filter(|conformance| conformance.polarity == Polarity::Negative)
    }

    /// Return every implements clause, positive and negative.
    pub fn conformances(&self) -> &[NominalConformance] {
        match self {
            Self::Struct(definition) => &definition.implements,
            Self::Class(definition) => &definition.implements,
            Self::Enum(definition) => &definition.implements,
            Self::Extension(definition) => &definition.implements,
            Self::TypeAlias(_) | Self::Interface(_) | Self::Newtype(_) => &[],
        }
    }

    /// Return the method member symbol declared at one source node.
    pub fn method_declared_at(&self, source: GlobalNodeIdAny) -> Option<GlobalSymbolId> {
        self.members().iter().find_map(|member| match member {
            DefinitionMember::Method(method) if method.source == source => Some(method.symbol),
            _ => None,
        })
    }

    /// Return the field declared by one member symbol.
    pub fn field(&self, symbol: GlobalSymbolId) -> Option<&FieldDefinition> {
        self.members().iter().find_map(|member| match member {
            DefinitionMember::Field(field) if field.symbol == symbol => Some(field),
            _ => None,
        })
    }

    /// Iterate the instance fields in declaration order.
    pub fn instance_fields(&self) -> impl Iterator<Item = &FieldDefinition> + '_ {
        self.members().iter().filter_map(|member| match member {
            DefinitionMember::Field(field) if field.space == MemberSpace::Instance => Some(field),
            _ => None,
        })
    }

    /// Iterate members in one member space.
    pub fn members_in(&self, space: MemberSpace) -> impl Iterator<Item = &DefinitionMember> + '_ {
        self.members()
            .iter()
            .filter(move |member| member.space() == space)
    }

    /// Iterate members with one key in one member space.
    pub fn members_with_key(
        &self,
        space: MemberSpace,
        key: StaticKey,
    ) -> impl Iterator<Item = &DefinitionMember> + '_ {
        self.members_in(space)
            .filter(move |member| member.key() == Some(key))
    }

    /// Return the base declarations whose members are inherited.
    pub fn bases(&self) -> SmallVec<[&NominalHeritage; 2]> {
        match self {
            Self::Class(definition) => definition.extends.iter().collect(),
            Self::Interface(definition) => definition.extends.iter().collect(),
            Self::Struct(_)
            | Self::Enum(_)
            | Self::Extension(_)
            | Self::TypeAlias(_)
            | Self::Newtype(_) => SmallVec::new(),
        }
    }

    /// Return every heritage edge, extended bases before implemented interfaces.
    pub fn heritage_edges(&self) -> SmallVec<[NominalHeritage; 4]> {
        let mut edges = self
            .bases()
            .into_iter()
            .cloned()
            .collect::<SmallVec<[NominalHeritage; 4]>>();
        edges.extend(self.implementations().map(|conformance| NominalHeritage {
            source: conformance.source,
            ty: conformance.interface,
        }));

        edges
    }
}

/// Runtime representation selected for one nominal declaration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, Reflect, TypeFold)]
pub struct Representation {
    /// The representation family.
    pub kind: RepresentationKind,
    /// The required byte alignment.
    pub alignment: Option<u64>,
    /// The maximum field alignment for packed layouts.
    pub packing: Option<u64>,
}

/// Runtime representation family for one nominal declaration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, Reflect, TypeFold)]
pub enum RepresentationKind {
    /// The native Destack layout.
    #[default]
    Destack,
    /// The target C ABI layout.
    C,
    /// The single-field backing layout without a wrapper.
    Transparent,
    /// An explicit integer scalar representation.
    Integer(IntegerType),
}

impl<'a> TryFrom<&'a str> for RepresentationKind {
    type Error = &'a str;

    /// Convert one standard name into a representation family.
    fn try_from(name: &'a str) -> Result<Self, Self::Error> {
        let integer = match name {
            "destack" => return Ok(Self::Destack),
            "C" => return Ok(Self::C),
            "transparent" => return Ok(Self::Transparent),
            "int" => IntegerType::Fixed {
                width: 64,
                is_signed: true,
            },
            "uint" => IntegerType::Fixed {
                width: 64,
                is_signed: false,
            },
            "isize" => IntegerType::Pointer { is_signed: true },
            "usize" => IntegerType::Pointer { is_signed: false },
            _ => {
                let (width, is_signed) = if let Some(width) = name.strip_prefix("int") {
                    (width, true)
                } else {
                    let Some(width) = name.strip_prefix("uint") else {
                        return Err(name);
                    };

                    (width, false)
                };
                let Ok(width) = width.parse::<u16>() else {
                    return Err(name);
                };
                if width == 0 {
                    return Err(name);
                }

                IntegerType::Fixed { width, is_signed }
            }
        };

        Ok(Self::Integer(integer))
    }
}

impl EnumDefinition {
    /// Iterate the enum variants in declaration order.
    pub fn variants(&self) -> impl Iterator<Item = &EnumVariantDefinition> {
        self.members.iter().filter_map(|member| match member {
            DefinitionMember::EnumVariant(variant) => Some(variant),
            _ => None,
        })
    }

    /// Return one variant's declaration position.
    pub fn variant_position(&self, symbol: GlobalSymbolId) -> Option<usize> {
        self.variants().position(|variant| variant.symbol == symbol)
    }

    /// Return the enum variant with one member key.
    pub fn variant_by_key(&self, key: StaticKey) -> Option<&EnumVariantDefinition> {
        self.variants().find(|variant| variant.key == key)
    }
}
