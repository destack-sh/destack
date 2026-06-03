use destack_dir as dir;
use destack_source::ModuleId;
use indexmap::IndexMap;

use crate::check::{GenericInstance, GenericTemplate, StaticOperand, TypeOperand};

/// Checked nominal declarations for one component.
#[derive(Debug, Default)]
pub(in crate::check) struct NominalTable {
    /// Nominal definitions keyed by declaring symbol.
    definitions: IndexMap<dir::GlobalSymbolId, NominalDefinition>,
}

impl NominalTable {
    /// Create an empty nominal table.
    pub(in crate::check) fn new() -> Self {
        Self {
            definitions: IndexMap::new(),
        }
    }

    /// Insert one nominal definition.
    pub(in crate::check) fn insert_definition(
        &mut self,
        symbol: dir::GlobalSymbolId,
        definition: NominalDefinition,
    ) {
        if self.definitions.contains_key(&symbol) {
            panic!("check nominal symbol {symbol:?} already has a definition");
        }

        self.definitions.insert(symbol, definition);
    }

    /// Return one nominal definition.
    pub(in crate::check) fn definition(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> Option<&NominalDefinition> {
        self.definitions.get(&symbol)
    }

    /// Iterate definitions declared by one module.
    pub(in crate::check) fn definitions_in(
        &self,
        module: ModuleId,
    ) -> impl Iterator<Item = (dir::GlobalSymbolId, &NominalDefinition)> + '_ {
        self.definitions
            .iter()
            .filter_map(move |(symbol, definition)| {
                (symbol.module_id == module).then_some((*symbol, definition))
            })
    }
}

/// Checked declaration data for one nominal symbol.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum NominalDefinition {
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
}

impl NominalDefinition {
    /// Return the source declaration node.
    pub(in crate::check) fn source(&self) -> dir::GlobalNodeIdAny {
        match self {
            Self::Struct(definition) => definition.source,
            Self::Class(definition) => definition.source,
            Self::Interface(definition) => definition.source,
            Self::Enum(definition) => definition.source,
            Self::Newtype(definition) => definition.source,
        }
    }
}

/// Checked declaration data for one nominal struct.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct StructDefinition {
    /// The source declaration node.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The generic template declared by the struct.
    pub(in crate::check) template: Option<GenericTemplate>,
    /// The implemented interfaces.
    pub(in crate::check) implements: Vec<NominalHeritage>,
    /// The instance fields.
    pub(in crate::check) fields: Vec<FieldDefinition>,
    /// The static fields.
    pub(in crate::check) static_fields: Vec<FieldDefinition>,
    /// The instance methods.
    pub(in crate::check) methods: Vec<MethodDefinition>,
    /// The static methods.
    pub(in crate::check) static_methods: Vec<MethodDefinition>,
    /// The associated types.
    pub(in crate::check) associated_types: Vec<AssociatedTypeDefinition>,
    /// The associated constants.
    pub(in crate::check) associated_consts: Vec<AssociatedConstDefinition>,
}

/// Checked declaration data for one nominal class.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct ClassDefinition {
    /// The source declaration node.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The generic template declared by the class.
    pub(in crate::check) template: Option<GenericTemplate>,
    /// The extended class.
    pub(in crate::check) extends: Option<NominalHeritage>,
    /// The implemented interfaces.
    pub(in crate::check) implements: Vec<NominalHeritage>,
    /// The instance fields.
    pub(in crate::check) fields: Vec<FieldDefinition>,
    /// The static fields.
    pub(in crate::check) static_fields: Vec<FieldDefinition>,
    /// The instance methods.
    pub(in crate::check) methods: Vec<MethodDefinition>,
    /// The static methods.
    pub(in crate::check) static_methods: Vec<MethodDefinition>,
    /// The associated types.
    pub(in crate::check) associated_types: Vec<AssociatedTypeDefinition>,
    /// The associated constants.
    pub(in crate::check) associated_consts: Vec<AssociatedConstDefinition>,
}

/// Checked declaration data for one nominal interface.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct InterfaceDefinition {
    /// The source declaration node.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The generic template declared by the interface.
    pub(in crate::check) template: Option<GenericTemplate>,
    /// The inherited interfaces.
    pub(in crate::check) extends: Vec<NominalHeritage>,
    /// The instance fields.
    pub(in crate::check) fields: Vec<FieldDefinition>,
    /// The static fields.
    pub(in crate::check) static_fields: Vec<FieldDefinition>,
    /// The instance methods.
    pub(in crate::check) methods: Vec<MethodDefinition>,
    /// The static methods.
    pub(in crate::check) static_methods: Vec<MethodDefinition>,
    /// The call signatures.
    pub(in crate::check) call_signatures: Vec<SignatureDefinition>,
    /// The construct signatures.
    pub(in crate::check) construct_signatures: Vec<SignatureDefinition>,
    /// The index signatures.
    pub(in crate::check) index_signatures: Vec<SignatureDefinition>,
    /// The associated types.
    pub(in crate::check) associated_types: Vec<AssociatedTypeDefinition>,
    /// The associated constants.
    pub(in crate::check) associated_consts: Vec<AssociatedConstDefinition>,
}

/// Checked declaration data for one nominal enum.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct EnumDefinition {
    /// The source declaration node.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The generic template declared by the enum.
    pub(in crate::check) template: Option<GenericTemplate>,
    /// The implemented interfaces.
    pub(in crate::check) implements: Vec<NominalHeritage>,
    /// The enum variants.
    pub(in crate::check) variants: Vec<VariantDefinition>,
    /// The static fields.
    pub(in crate::check) static_fields: Vec<FieldDefinition>,
    /// The instance methods.
    pub(in crate::check) methods: Vec<MethodDefinition>,
    /// The static methods.
    pub(in crate::check) static_methods: Vec<MethodDefinition>,
    /// The associated types.
    pub(in crate::check) associated_types: Vec<AssociatedTypeDefinition>,
    /// The associated constants.
    pub(in crate::check) associated_consts: Vec<AssociatedConstDefinition>,
}

/// Checked declaration data for one newtype.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct NewtypeDefinition {
    /// The source declaration node.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The generic template declared by the newtype.
    pub(in crate::check) template: Option<GenericTemplate>,
    /// The nominal backing type before solve reduction.
    pub(in crate::check) value: TypeOperand,
}

/// One nominal heritage.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct NominalHeritage {
    /// The source type expression node.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The heritage nominal symbol.
    pub(in crate::check) symbol: dir::GlobalSymbolId,
    /// The generic instance used at the relation site.
    pub(in crate::check) instance: Option<GenericInstance>,
}

/// One checked field member.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct FieldDefinition {
    /// The field symbol.
    pub(in crate::check) symbol: dir::GlobalSymbolId,
    /// The source member node.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The field key.
    pub(in crate::check) key: dir::StaticKey,
    /// The checked field type.
    pub(in crate::check) ty: TypeOperand,
}

/// One method member.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct MethodDefinition {
    /// The method symbol.
    pub(in crate::check) symbol: Option<dir::GlobalSymbolId>,
    /// The source member node.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The nominal member slot.
    pub(in crate::check) slot: dir::MemberSlot,
    /// The checked method type.
    pub(in crate::check) ty: TypeOperand,
}

/// One checked associated type.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct AssociatedTypeDefinition {
    /// The associated type symbol.
    pub(in crate::check) symbol: dir::GlobalSymbolId,
    /// The source member node.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The upper bound required by this associated type.
    pub(in crate::check) constraint: Option<TypeOperand>,
    /// The concrete associated type value.
    pub(in crate::check) value: Option<TypeOperand>,
}

/// One associated constant in component operands.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct AssociatedConstDefinition {
    /// The associated const symbol.
    pub(in crate::check) symbol: dir::GlobalSymbolId,
    /// The source member node.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The static type operand.
    pub(in crate::check) ty: TypeOperand,
    /// The static value operand.
    pub(in crate::check) value: Option<StaticOperand>,
}

/// One checked enum variant.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct VariantDefinition {
    /// The variant symbol.
    pub(in crate::check) symbol: dir::GlobalSymbolId,
    /// The source enum field node.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The variant key.
    pub(in crate::check) key: dir::StaticKey,
    /// The checked variant value.
    pub(in crate::check) value: Option<StaticOperand>,
}

/// One checked symbol-free signature member.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct SignatureDefinition {
    /// The source member node.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The checked signature type.
    pub(in crate::check) ty: TypeOperand,
}
