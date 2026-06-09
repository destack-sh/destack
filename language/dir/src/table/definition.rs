use std::sync::Arc;

use destack_source::ModuleId;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::{
    Extension, GlobalNodeIdAny, GlobalStaticId, GlobalSymbolId, GlobalTypeId,
    LocalGenericInstanceId, LocalGenericTemplateId, MemberSlot, SegmentView, StaticKey,
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
    pub fn extension_definition(&self, symbol: GlobalSymbolId) -> Option<&Extension> {
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
    pub fn iter_extensions(&self) -> impl Iterator<Item = (GlobalSymbolId, &Extension)> + '_ {
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
            match extension.target.nominal_root() {
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
}

/// Checked declaration data for one symbol.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
    Extension(Extension),
}

impl Definition {
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypeAliasDefinition {
    /// The generic template declared by the alias.
    pub template: Option<LocalGenericTemplateId>,
    /// The checked alias value.
    pub value: GlobalTypeId,
}

/// Checked declaration data for one nominal struct.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StructDefinition {
    /// The generic template declared by the struct.
    pub template: Option<LocalGenericTemplateId>,
    /// The implemented interfaces.
    pub implements: Vec<NominalHeritage>,
    /// The instance fields.
    pub fields: Vec<FieldDefinition>,
    /// The static fields.
    pub static_fields: Vec<FieldDefinition>,
    /// The instance methods.
    pub methods: Vec<MethodDefinition>,
    /// The static methods.
    pub static_methods: Vec<MethodDefinition>,
    /// The associated types.
    pub associated_types: Vec<AssociatedTypeDefinition>,
    /// The associated constants.
    pub associated_consts: Vec<AssociatedConstDefinition>,
}

/// Checked declaration data for one nominal class.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClassDefinition {
    /// The generic template declared by the class.
    pub template: Option<LocalGenericTemplateId>,
    /// The extended class.
    pub extends: Option<NominalHeritage>,
    /// The implemented interfaces.
    pub implements: Vec<NominalHeritage>,
    /// The instance fields.
    pub fields: Vec<FieldDefinition>,
    /// The static fields.
    pub static_fields: Vec<FieldDefinition>,
    /// The instance methods.
    pub methods: Vec<MethodDefinition>,
    /// The static methods.
    pub static_methods: Vec<MethodDefinition>,
    /// The associated types.
    pub associated_types: Vec<AssociatedTypeDefinition>,
    /// The associated constants.
    pub associated_consts: Vec<AssociatedConstDefinition>,
}

/// Checked declaration data for one nominal interface.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InterfaceDefinition {
    /// The generic template declared by the interface.
    pub template: Option<LocalGenericTemplateId>,
    /// Whether the interface has nominal identity.
    pub is_nominal: bool,
    /// The inherited interfaces.
    pub extends: Vec<NominalHeritage>,
    /// The instance fields.
    pub fields: Vec<FieldDefinition>,
    /// The static fields.
    pub static_fields: Vec<FieldDefinition>,
    /// The instance methods.
    pub methods: Vec<MethodDefinition>,
    /// The static methods.
    pub static_methods: Vec<MethodDefinition>,
    /// The call signatures.
    pub call_signatures: Vec<SignatureDefinition>,
    /// The construct signatures.
    pub construct_signatures: Vec<SignatureDefinition>,
    /// The index signatures.
    pub index_signatures: Vec<SignatureDefinition>,
    /// The associated types.
    pub associated_types: Vec<AssociatedTypeDefinition>,
    /// The associated constants.
    pub associated_consts: Vec<AssociatedConstDefinition>,
}

/// Checked declaration data for one nominal enum.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnumDefinition {
    /// The generic template declared by the enum.
    pub template: Option<LocalGenericTemplateId>,
    /// The implemented interfaces.
    pub implements: Vec<NominalHeritage>,
    /// The enum variants.
    pub variants: Vec<VariantDefinition>,
    /// The static fields.
    pub static_fields: Vec<FieldDefinition>,
    /// The instance methods.
    pub methods: Vec<MethodDefinition>,
    /// The static methods.
    pub static_methods: Vec<MethodDefinition>,
    /// The associated types.
    pub associated_types: Vec<AssociatedTypeDefinition>,
    /// The associated constants.
    pub associated_consts: Vec<AssociatedConstDefinition>,
}

/// Checked declaration data for one nominal type alias.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewtypeDefinition {
    /// The generic template declared by the newtype.
    pub template: Option<LocalGenericTemplateId>,
    /// The nominal backing type.
    pub value: GlobalTypeId,
}

/// One nominal heritage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NominalHeritage {
    /// The source heritage node.
    pub source: GlobalNodeIdAny,
    /// The heritage nominal symbol.
    pub symbol: GlobalSymbolId,
    /// The generic instance used at the relation site.
    pub instance: Option<LocalGenericInstanceId>,
}

/// One checked field member.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FieldDefinition {
    /// The field symbol.
    pub symbol: GlobalSymbolId,
    /// The source member node.
    pub source: GlobalNodeIdAny,
    /// The field key.
    pub key: StaticKey,
    /// The checked field type.
    pub ty: GlobalTypeId,
}

/// One method member.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MethodDefinition {
    /// The method symbol.
    pub symbol: Option<GlobalSymbolId>,
    /// The source member node.
    pub source: GlobalNodeIdAny,
    /// The nominal member slot.
    pub slot: MemberSlot,
    /// The checked method type.
    pub ty: GlobalTypeId,
}

/// One checked associated type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

/// One checked associated constant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssociatedConstDefinition {
    /// The associated const symbol.
    pub symbol: GlobalSymbolId,
    /// The source member node.
    pub source: GlobalNodeIdAny,
    /// The associated const key.
    pub key: StaticKey,
    /// The checked static type.
    pub ty: GlobalTypeId,
    /// The checked static value.
    pub value: Option<GlobalStaticId>,
}

/// One checked enum variant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

/// One checked symbol-free signature member.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignatureDefinition {
    /// The source member node.
    pub source: GlobalNodeIdAny,
    /// The checked signature type.
    pub ty: GlobalTypeId,
}
