use destack_serde::Reflect;
use std::sync::Arc;

use destack_source::ModuleId;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use crate::{
    FunctionRole, GlobalNodeIdAny, GlobalStaticId, GlobalSymbolId, GlobalTypeId,
    LocalGenericTemplateId, MemberSlot, MethodAbstraction, SegmentView, Space, StaticKey,
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
        for segment in self.segments.iter().rev() {
            if let Some(definition) = segment.definition(symbol) {
                return Some(definition);
            }
        }

        None
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

    /// Iterate extension symbols targeting one nominal symbol.
    pub fn target_extensions(
        &self,
        target_symbol: GlobalSymbolId,
    ) -> impl Iterator<Item = GlobalSymbolId> + '_ {
        self.segments
            .iter()
            .flat_map(move |segment| segment.target_extensions(target_symbol).iter().copied())
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
        let mut definitions = IndexMap::new();

        // apply later segment values over earlier ones
        for segment in self.segments.iter() {
            for (symbol, definition) in segment.iter_definitions() {
                definitions.insert(symbol, definition);
            }
        }

        definitions.into_iter()
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
    pub(crate) definitions: IndexMap<GlobalSymbolId, Definition>,
    /// Extension symbols by target symbol.
    pub(crate) extensions_by_target_symbol: IndexMap<GlobalSymbolId, Vec<GlobalSymbolId>>,
    /// Blanket extension symbols.
    pub(crate) blanket_extensions: Vec<GlobalSymbolId>,
}

impl DefinitionSegment {
    /// Create a new definition segment.
    pub fn new(module_id: ModuleId) -> Self {
        Self {
            module_id,
            sources: IndexMap::new(),
            definitions: IndexMap::new(),
            extensions_by_target_symbol: IndexMap::new(),
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
        self.sources.insert(symbol, source);
        if let Definition::Extension(extension) = &definition {
            match extension.target.root() {
                Some(target_symbol) => {
                    self.extensions_by_target_symbol
                        .entry(target_symbol)
                        .or_default()
                        .push(symbol);
                }
                None => {
                    self.blanket_extensions.push(symbol);
                }
            }
        }

        self.definitions.insert(symbol, definition);
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
        self.definitions.get(&symbol)
    }

    /// Return one definition for in-place mutation.
    pub fn definition_mut(&mut self, symbol: GlobalSymbolId) -> Option<&mut Definition> {
        self.definitions.get_mut(&symbol)
    }

    /// Get all extension symbols targeting a specific type symbol.
    pub fn target_extensions(&self, target_symbol: GlobalSymbolId) -> &[GlobalSymbolId] {
        self.extensions_by_target_symbol
            .get(&target_symbol)
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
            .map(|(symbol, definition)| (*symbol, definition))
    }

    /// Return true when this segment has no definitions.
    pub fn is_empty(&self) -> bool {
        self.definitions.is_empty()
    }

    /// Apply one mapping to every type id stored in this segment.
    pub fn map_type_ids(&mut self, map: &mut impl FnMut(GlobalTypeId) -> GlobalTypeId) {
        for definition in self.definitions.values_mut() {
            definition.map_type_ids(map);
        }
    }
}

/// Checked declaration data for one symbol.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum Definition {
    /// Transparent type alias declaration.
    ///
    /// Example:
    /// ```ds
    /// type Json = string | number | boolean
    /// ```
    TypeAlias(TypeAliasDefinition),
    /// Struct declaration.
    ///
    /// Example:
    /// ```ds
    /// struct User { name: string }
    /// ```
    Struct(StructDefinition),
    /// Class declaration.
    ///
    /// Example:
    /// ```ds
    /// class User { name: string }
    /// ```
    Class(ClassDefinition),
    /// Nominal interface declaration.
    ///
    /// Example:
    /// ```ds
    /// interface Reader { read(): string }
    /// ```
    Interface(InterfaceDefinition),
    /// Enum declaration.
    ///
    /// Example:
    /// ```ds
    /// enum Status { Ready, Done }
    /// ```
    Enum(EnumDefinition),
    /// Newtype declaration.
    ///
    /// Example:
    /// ```ds
    /// newtype UserId = int64
    /// ```
    Newtype(NewtypeDefinition),
    /// Extension declaration.
    ///
    /// Example:
    /// ```ds
    /// extension Logger for Writer { write(message: string): void }
    /// ```
    Extension(ExtensionDefinition),
}

impl Definition {
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

    /// Set the effective space of this nominal declaration.
    pub fn set_space(&mut self, space: Space) -> bool {
        match self {
            Self::Struct(definition) => definition.space = Some(space),
            Self::Class(definition) => definition.space = Some(space),
            Self::Interface(definition) => definition.space = Some(space),
            Self::Enum(definition) => definition.space = Some(space),
            Self::Newtype(definition) => definition.space = Some(space),
            Self::TypeAlias(_) | Self::Extension(_) => return false,
        }

        true
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

    /// Apply one mapping to every type id stored in this definition.
    pub fn map_type_ids(&mut self, map: &mut impl FnMut(GlobalTypeId) -> GlobalTypeId) {
        // map the definition's own type values
        match self {
            Self::TypeAlias(definition) => definition.value = map(definition.value),
            Self::Struct(definition) => map_heritages(&mut definition.implements, map),
            Self::Class(definition) => {
                map_heritages(&mut definition.implements, map);
                for constructor in &mut definition.constructors {
                    constructor.ty = map(constructor.ty);
                }
            }
            Self::Interface(definition) => map_heritages(&mut definition.extends, map),
            Self::Enum(definition) => map_heritages(&mut definition.implements, map),
            Self::Newtype(definition) => definition.backing = map(definition.backing),
            Self::Extension(definition) => {
                match &mut definition.target {
                    ExtensionTarget::Rooted { ty, .. } | ExtensionTarget::Blanket { ty } => {
                        *ty = map(*ty);
                    }
                }
                map_heritages(&mut definition.implements, map);
            }
        }

        // map every member's type values
        for member in self.members_mut() {
            match member {
                DefinitionMember::AssociatedType(member) => {
                    if let Some(constraint) = &mut member.constraint {
                        *constraint = map(*constraint);
                    }
                    if let Some(value) = &mut member.value {
                        *value = map(*value);
                    }
                }
                DefinitionMember::CallSignature(member)
                | DefinitionMember::ConstructSignature(member) => member.ty = map(member.ty),
                DefinitionMember::IndexSignature(member) => {
                    member.key_type = map(member.key_type);
                    member.value_type = map(member.value_type);
                }
                DefinitionMember::Field(_)
                | DefinitionMember::Method(_)
                | DefinitionMember::AssociatedConst(_)
                | DefinitionMember::Variant(_) => {}
            }
        }
    }
}

/// Apply one type id mapping to a heritage list.
fn map_heritages(
    heritages: &mut [NominalHeritage],
    map: &mut impl FnMut(GlobalTypeId) -> GlobalTypeId,
) {
    for heritage in heritages {
        for argument in &mut heritage.arguments {
            *argument = map(*argument);
        }
    }
}

/// Checked declaration data for one transparent type alias.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct TypeAliasDefinition {
    /// The generic template declared by the alias.
    pub template: Option<LocalGenericTemplateId>,
    /// The checked alias value.
    pub value: GlobalTypeId,
}

/// Checked declaration data for one nominal struct.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct StructDefinition {
    /// The intrinsic instance space, or relative when absent.
    pub space: Option<Space>,
    /// The generic template declared by the struct.
    pub template: Option<LocalGenericTemplateId>,
    /// The implemented interfaces.
    pub implements: Vec<NominalHeritage>,
    /// The members in declaration order.
    pub members: Vec<DefinitionMember>,
}

/// Checked declaration data for one nominal class.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
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
    pub implements: Vec<NominalHeritage>,
    /// The class's direct construct candidates.
    pub constructors: Vec<ClassConstructorDefinition>,
    /// The members in declaration order.
    pub members: Vec<DefinitionMember>,
}

/// One class construct candidate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ClassConstructorDefinition {
    /// The selected constructor.
    pub constructor: ClassConstructor,
    /// The checked constructor signature.
    pub ty: GlobalTypeId,
}

/// Class construct candidate origin.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum ClassConstructor {
    /// Constructor explicitly declared by this class.
    ///
    /// Examples:
    /// ```ds
    /// class User {
    ///     constructor(name: string) {}
    /// }
    ///
    /// new User("ada")
    /// ```
    Declared {
        /// The declared constructor symbol.
        symbol: GlobalSymbolId,
    },
    /// Default `new T()` candidate for a class with no declared constructor.
    ///
    /// Examples:
    /// ```ds
    /// class User {}
    ///
    /// new User()
    /// ```
    Default,
    /// Constructor forwarded to an explicit base class constructor.
    ///
    /// Examples:
    /// ```ds
    /// class Parent {
    ///     constructor(name: string) {}
    /// }
    ///
    /// class Child extends Parent {}
    ///
    /// new Child("ada")
    /// ```
    ForwardedDeclared {
        /// The base class symbol.
        base: GlobalSymbolId,
        /// The selected base constructor symbol.
        symbol: GlobalSymbolId,
    },
    /// Constructor forwarded to a base class default constructor.
    ///
    /// Examples:
    /// ```ds
    /// class Parent {}
    ///
    /// class Child extends Parent {}
    ///
    /// new Child()
    /// ```
    ForwardedDefault {
        /// The base class symbol.
        base: GlobalSymbolId,
    },
}

impl ClassConstructor {
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct EnumDefinition {
    /// The intrinsic instance space, or relative when absent.
    pub space: Option<Space>,
    /// The generic template declared by the enum.
    pub template: Option<LocalGenericTemplateId>,
    /// The implemented interfaces.
    pub implements: Vec<NominalHeritage>,
    /// The members in declaration order.
    pub members: Vec<DefinitionMember>,
}

/// Checked declaration data for one nominal type alias.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct NewtypeDefinition {
    /// The intrinsic instance space, or relative when absent.
    pub space: Option<Space>,
    /// The generic template declared by the newtype.
    pub template: Option<LocalGenericTemplateId>,
    /// The nominal backing type.
    pub backing: GlobalTypeId,
    /// The members in declaration order.
    pub members: Vec<DefinitionMember>,
}

/// How an extension declaration relates to its target type.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Reflect,
)]
pub enum ExtensionForm {
    /// Extension visible only inside its declaring module.
    ///
    /// Examples:
    /// ```ds
    /// extension of string { shout(): string { ... } }
    /// ```
    Local,
    /// Extension visible outside its declaring module.
    ///
    /// Examples:
    /// ```ds
    /// export extension of string implements Hash { ... }
    /// ```
    Exported,
}

/// Checked declaration data for one extension.
///
/// Examples:
/// ```ds
/// extension<T> of Array<T> implements Iterable<T> { ... }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ExtensionDefinition {
    /// The extension declaration's symbol.
    pub symbol: GlobalSymbolId,
    /// The extension declaration form.
    pub form: ExtensionForm,
    /// The extension's generic template.
    pub template: Option<LocalGenericTemplateId>,
    /// The checked receiver target.
    pub target: ExtensionTarget,
    /// The implemented interfaces.
    pub implements: Vec<NominalHeritage>,
    /// The members in declaration order.
    pub members: Vec<DefinitionMember>,
}

impl ExtensionDefinition {
    /// Create a new extension.
    pub fn new(
        symbol: GlobalSymbolId,
        form: ExtensionForm,
        template: Option<LocalGenericTemplateId>,
        target: ExtensionTarget,
        implements: Vec<NominalHeritage>,
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
            .root()
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum ExtensionTarget {
    /// Extension whose receiver type has a lookup root.
    ///
    /// Example:
    /// ```ds
    /// extension<T> of ^Array<T> {}
    /// ```
    Rooted {
        /// The declaration root used for member lookup.
        root: GlobalSymbolId,
        /// The checked receiver type.
        ty: GlobalTypeId,
    },
    /// Extension over an open receiver type.
    ///
    /// Example:
    /// ```ds
    /// extension<T> of T where T: Copy {}
    /// ```
    Blanket {
        /// The checked receiver type.
        ty: GlobalTypeId,
    },
}

impl ExtensionTarget {
    /// Return the checked receiver type.
    pub fn r#type(&self) -> GlobalTypeId {
        match self {
            Self::Rooted { ty, .. } | Self::Blanket { ty } => *ty,
        }
    }

    /// Return the lookup root when this target has one.
    pub fn root(&self) -> Option<GlobalSymbolId> {
        match self {
            Self::Rooted { root, .. } => Some(*root),
            Self::Blanket { .. } => None,
        }
    }

    /// Return whether this is an open blanket target.
    pub fn is_blanket(&self) -> bool {
        matches!(self, Self::Blanket { .. })
    }
}

/// One nominal heritage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct NominalHeritage {
    /// The source heritage node.
    pub source: GlobalNodeIdAny,
    /// The heritage nominal symbol.
    pub symbol: GlobalSymbolId,
    /// The generic arguments used at the relation site, empty when not applied.
    pub arguments: Vec<GlobalTypeId>,
}

/// One field member.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct FieldDefinition {
    /// The member space declaring the field.
    pub space: MemberSpace,
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
    /// Whether the field asserts definite assignment outside constructors.
    pub is_definite: bool,
    /// Whether subclasses must provide the field.
    pub is_abstract: bool,
    /// Whether the field overrides an inherited member.
    pub is_override: bool,
}

/// One method member.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct MethodDefinition {
    /// The member space declaring the method.
    pub space: MemberSpace,
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
    pub implementation: MethodImplementation,
}

/// How one method receives its implementation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum MethodImplementation {
    /// Implementers must supply a body.
    Required,
    /// The declaration supplies its own body.
    Body,
    /// The declaring interface supplies a fallback body.
    Default,
    /// The compiler supplies the body.
    Intrinsic,
}

/// One associated type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
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
}

/// One associated constant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct AssociatedConstDefinition {
    /// The associated const symbol.
    pub symbol: GlobalSymbolId,
    /// The source member node.
    pub source: GlobalNodeIdAny,
    /// The associated const key.
    pub key: StaticKey,
    /// The checked static value.
    pub value: Option<GlobalStaticId>,
}

/// One enum variant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct VariantDefinition {
    /// The variant symbol.
    pub symbol: GlobalSymbolId,
    /// The source enum field node.
    pub source: GlobalNodeIdAny,
    /// The variant key.
    pub key: StaticKey,
    /// The checked variant value.
    pub value: Option<GlobalStaticId>,
}

/// One symbol-free signature member.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct SignatureDefinition {
    /// The source member node.
    pub source: GlobalNodeIdAny,
    /// The checked signature type.
    pub ty: GlobalTypeId,
}

/// One structural index signature member.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct IndexSignatureDefinition {
    /// The source member node.
    pub source: GlobalNodeIdAny,
    /// The declared key domain.
    pub key_type: GlobalTypeId,
    /// The declared value type.
    pub value_type: GlobalTypeId,
}

/// Member namespace selected by member lookup.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum MemberSpace {
    /// Instance members selected from a runtime receiver.
    Instance,
    /// Static members selected from a declaration receiver.
    Static,
}

/// One declaration member.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum DefinitionMember {
    /// Field member.
    Field(FieldDefinition),
    /// Method member.
    Method(MethodDefinition),
    /// Associated type member.
    AssociatedType(AssociatedTypeDefinition),
    /// Associated constant member.
    AssociatedConst(AssociatedConstDefinition),
    /// Enum variant member.
    Variant(VariantDefinition),
    /// Structural call signature member.
    CallSignature(SignatureDefinition),
    /// Structural construct signature member.
    ConstructSignature(SignatureDefinition),
    /// Structural index signature member.
    IndexSignature(IndexSignatureDefinition),
}

impl DefinitionMember {
    /// Return whether this member carries a default implementation.
    pub fn is_default(&self) -> bool {
        match self {
            Self::Method(method) => method.implementation == MethodImplementation::Default,
            Self::AssociatedType(associated) => associated.value.is_some(),
            _ => false,
        }
    }

    /// Return the source node declaring this member.
    pub fn source(&self) -> GlobalNodeIdAny {
        match self {
            Self::Field(field) => field.source,
            Self::Method(method) => method.source,
            Self::AssociatedType(associated) => associated.source,
            Self::AssociatedConst(associated) => associated.source,
            Self::Variant(variant) => variant.source,
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
            Self::AssociatedType(_) | Self::AssociatedConst(_) | Self::Variant(_) => {
                MemberSpace::Static
            }
            // structural signatures describe instances
            Self::CallSignature(_) | Self::ConstructSignature(_) | Self::IndexSignature(_) => {
                MemberSpace::Instance
            }
        }
    }

    /// Return the declaring member symbol.
    pub fn symbol(&self) -> Option<GlobalSymbolId> {
        match self {
            Self::Field(field) => Some(field.symbol),
            Self::Method(method) => Some(method.symbol),
            Self::AssociatedType(associated) => Some(associated.symbol),
            Self::AssociatedConst(associated) => Some(associated.symbol),
            Self::Variant(variant) => Some(variant.symbol),
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
            Self::Variant(variant) => Some(variant.key),
            Self::CallSignature(_) | Self::ConstructSignature(_) | Self::IndexSignature(_) => None,
        }
    }

    /// Return the committed member value when the member carries one.
    pub fn value(&self) -> Option<GlobalStaticId> {
        match self {
            Self::AssociatedConst(associated) => associated.value,
            Self::Variant(variant) => variant.value,
            _ => None,
        }
    }

    /// Return the symbol whose checked type carries this member's value.
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
            Self::Field(_) | Self::Method(_) | Self::AssociatedConst(_) | Self::Variant(_) => None,
        }
    }
}

impl Definition {
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

    /// Return the heritage clauses this definition relates to.
    pub fn heritages(&self) -> SmallVec<[&NominalHeritage; 2]> {
        match self {
            Self::Struct(definition) => definition.implements.iter().collect(),
            Self::Class(definition) => definition
                .extends
                .iter()
                .chain(definition.implements.iter())
                .collect(),
            Self::Interface(definition) => definition.extends.iter().collect(),
            Self::Enum(definition) => definition.implements.iter().collect(),
            Self::Extension(extension) => extension.implements.iter().collect(),
            Self::TypeAlias(_) | Self::Newtype(_) => SmallVec::new(),
        }
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
}
