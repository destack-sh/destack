use std::sync::Arc;

use destack_source::ModuleId;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::{
    GlobalNodeIdAny, GlobalStaticId, GlobalSymbolId, GlobalTypeId, LocalGenericApplicationId,
    LocalGenericTemplateId, MemberSlot, SegmentView, StaticKey,
};

/// Cumulative nominal declarations for one DIR module.
#[derive(Debug, Clone)]
pub struct NominalTable<'a> {
    /// The module id of the nominal table.
    pub module_id: ModuleId,
    /// The ordered nominal table segments.
    segments: SegmentView<'a, NominalSegment>,
}

impl NominalTable<'static> {
    /// Create a nominal table from ordered segments.
    pub fn from_segments(segments: Vec<Arc<NominalSegment>>) -> Self {
        let segments = SegmentView::from_segments(segments);

        Self::from_view(segments)
    }

    /// Create a nominal table from one segment.
    pub fn from_segment(segment: Arc<NominalSegment>) -> Self {
        Self::from_segments(vec![segment])
    }
}

impl<'a> NominalTable<'a> {
    /// Create a nominal table from a segment view.
    pub fn from_view(segments: SegmentView<'a, NominalSegment>) -> Self {
        let first = segments
            .first()
            .unwrap_or_else(|| panic!("nominal table needs at least one segment"));
        let module_id = first.module_id;

        // require a single module owner
        for segment in segments.iter() {
            assert_eq!(
                segment.module_id, module_id,
                "nominal table segment belongs to a different module"
            );
        }

        Self {
            module_id,
            segments,
        }
    }

    /// Return one nominal definition by symbol.
    pub fn definition(&self, symbol: GlobalSymbolId) -> Option<&NominalDefinition> {
        for segment in self.segments.iter().rev() {
            if let Some(definition) = segment.definition(symbol) {
                return Some(definition);
            }
        }

        None
    }

    /// Return the source declaration node for one nominal definition.
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
            Some(NominalDefinition::Newtype(definition)) => Some(definition),
            _ => None,
        }
    }

    /// Return one enum definition by symbol.
    pub fn enum_definition(&self, symbol: GlobalSymbolId) -> Option<&EnumDefinition> {
        match self.definition(symbol) {
            Some(NominalDefinition::Enum(definition)) => Some(definition),
            _ => None,
        }
    }

    /// Iterate nominal definitions in phase order.
    pub fn iter_definitions(
        &self,
    ) -> impl Iterator<Item = (GlobalSymbolId, &NominalDefinition)> + '_ {
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

/// Nominal declarations added by one DIR phase.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NominalSegment {
    /// The module id of the nominal segment.
    pub module_id: ModuleId,
    /// Source declaration nodes keyed by declaring symbol.
    pub(crate) sources: IndexMap<GlobalSymbolId, GlobalNodeIdAny>,
    /// Nominal definitions keyed by declaring symbol.
    pub(crate) definitions: IndexMap<GlobalSymbolId, NominalDefinition>,
}

impl NominalSegment {
    /// Create a new nominal segment.
    pub fn new(module_id: ModuleId) -> Self {
        Self {
            module_id,
            sources: IndexMap::new(),
            definitions: IndexMap::new(),
        }
    }

    /// Insert one nominal definition.
    pub fn insert_definition(
        &mut self,
        symbol: GlobalSymbolId,
        source: GlobalNodeIdAny,
        definition: NominalDefinition,
    ) {
        self.sources.insert(symbol, source);
        self.definitions.insert(symbol, definition);
    }

    /// Return the source declaration node for one nominal definition.
    pub fn definition_source(&self, symbol: GlobalSymbolId) -> GlobalNodeIdAny {
        *self
            .sources
            .get(&symbol)
            .unwrap_or_else(|| panic!("DIR nominal symbol {symbol:?} has no source"))
    }

    /// Return one nominal definition by symbol.
    pub fn definition(&self, symbol: GlobalSymbolId) -> Option<&NominalDefinition> {
        self.definitions.get(&symbol)
    }

    /// Iterate nominal definitions in insertion order.
    pub fn iter_definitions(
        &self,
    ) -> impl Iterator<Item = (GlobalSymbolId, &NominalDefinition)> + '_ {
        self.definitions
            .iter()
            .map(|(symbol, definition)| (*symbol, definition))
    }

    /// Return true when this segment has no definitions.
    pub fn is_empty(&self) -> bool {
        self.definitions.is_empty()
    }
}

/// Checked declaration data for one nominal symbol.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NominalDefinition {
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
    /// nominal interface Reader { read(): string }
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
    /// The generic application used at the relation site.
    pub application: Option<LocalGenericApplicationId>,
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
