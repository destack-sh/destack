use destack_dir as dir;
use destack_source::ModuleId;
use indexmap::IndexMap;

use crate::check::{GenericInstance, GenericTemplateId, StaticOperand, TypeOperand};

/// Checked declaration definitions for one component.
#[derive(Debug, Default)]
pub(in crate::check) struct DefinitionTable {
    /// Definitions keyed by declaring symbol.
    definitions: IndexMap<dir::GlobalSymbolId, Definition>,
    /// Extension symbols keyed by nominal target root.
    extensions_by_target: IndexMap<dir::GlobalSymbolId, Vec<dir::GlobalSymbolId>>,
    /// Blanket extension symbols.
    blanket_extensions: Vec<dir::GlobalSymbolId>,
}

impl DefinitionTable {
    /// Create an empty definition table.
    pub(in crate::check) fn new() -> Self {
        Self {
            definitions: IndexMap::new(),
            extensions_by_target: IndexMap::new(),
            blanket_extensions: Vec::new(),
        }
    }

    /// Insert one checked definition.
    pub(in crate::check) fn insert(&mut self, symbol: dir::GlobalSymbolId, definition: Definition) {
        if self.definitions.contains_key(&symbol) {
            unreachable!("definition symbol {symbol:?} already has a definition");
        }

        // index extension roots as definitions are inserted
        if let Definition::Extension(extension) = &definition {
            match extension.target.nominal_root() {
                Some(target) => self
                    .extensions_by_target
                    .entry(target)
                    .or_default()
                    .push(symbol),
                None => self.blanket_extensions.push(symbol),
            }
        }

        self.definitions.insert(symbol, definition);
    }

    /// Return one checked definition.
    pub(in crate::check) fn definition(&self, symbol: dir::GlobalSymbolId) -> Option<&Definition> {
        self.definitions.get(&symbol)
    }

    /// Return whether one checked definition exists.
    pub(in crate::check) fn contains_definition(&self, symbol: dir::GlobalSymbolId) -> bool {
        self.definitions.contains_key(&symbol)
    }

    /// Iterate definitions declared by one module.
    pub(in crate::check) fn definitions_in(
        &self,
        module: ModuleId,
    ) -> impl Iterator<Item = (dir::GlobalSymbolId, &Definition)> + '_ {
        self.definitions
            .iter()
            .filter_map(move |(symbol, definition)| {
                (symbol.module_id == module).then_some((*symbol, definition))
            })
    }

    /// Return extension symbols declared in one module for one target.
    pub(in crate::check) fn extension_symbols_for_target(
        &self,
        module: ModuleId,
        target: dir::GlobalSymbolId,
    ) -> Vec<dir::GlobalSymbolId> {
        self.extensions_by_target
            .get(&target)
            .into_iter()
            .flatten()
            .copied()
            .filter(|symbol| symbol.module_id == module)
            .collect()
    }

    /// Return blanket extension symbols declared in one module.
    pub(in crate::check) fn blanket_extension_symbols(
        &self,
        module: ModuleId,
    ) -> Vec<dir::GlobalSymbolId> {
        self.blanket_extensions
            .iter()
            .copied()
            .filter(|symbol| symbol.module_id == module)
            .collect()
    }
}

/// Checked declaration data for one symbol.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum Definition {
    /// Transparent type alias declaration.
    TypeAlias(TypeAliasDefinition),
    /// Struct declaration.
    Struct(StructDefinition),
    /// Class declaration.
    Class(ClassDefinition),
    /// Interface declaration.
    Interface(InterfaceDefinition),
    /// Enum declaration.
    Enum(EnumDefinition),
    /// Newtype declaration.
    Newtype(NewtypeDefinition),
    /// Extension declaration.
    Extension(ExtensionDefinition),
}

impl Definition {
    /// Return the source declaration node.
    pub(in crate::check) fn source(&self) -> dir::GlobalNodeIdAny {
        match self {
            Self::TypeAlias(definition) => definition.source,
            Self::Struct(definition) => definition.source,
            Self::Class(definition) => definition.source,
            Self::Interface(definition) => definition.source,
            Self::Enum(definition) => definition.source,
            Self::Newtype(definition) => definition.source,
            Self::Extension(definition) => definition.source,
        }
    }

    /// Return instance type members for one key.
    pub(in crate::check) fn type_members(&self, key: &dir::StaticKey) -> Vec<TypeMemberDefinition> {
        match self {
            Self::Struct(definition) => definition.type_members(key),
            Self::Class(definition) => definition.type_members(key),
            Self::Interface(definition) => definition.type_members(key),
            Self::Enum(definition) => definition.type_members(key),
            Self::Extension(definition) => definition.type_members(key),
            Self::TypeAlias(_) | Self::Newtype(_) => Vec::new(),
        }
    }

    /// Return all named instance type members.
    pub(in crate::check) fn named_type_members(&self) -> Vec<TypeMemberDefinition> {
        match self {
            Self::Struct(definition) => definition.named_type_members(),
            Self::Class(definition) => definition.named_type_members(),
            Self::Interface(definition) => definition.named_type_members(),
            Self::Enum(definition) => definition.named_type_members(),
            Self::Extension(definition) => definition.named_type_members(),
            Self::TypeAlias(_) | Self::Newtype(_) => Vec::new(),
        }
    }

    /// Return static value members for one key.
    pub(in crate::check) fn static_members(
        &self,
        key: &dir::StaticKey,
    ) -> Vec<StaticMemberDefinition> {
        match self {
            Self::Struct(definition) => definition.static_members(key),
            Self::Class(definition) => definition.static_members(key),
            Self::Interface(definition) => definition.static_members(key),
            Self::Enum(definition) => definition.static_members(key),
            Self::Extension(definition) => definition.static_members(key),
            Self::TypeAlias(_) | Self::Newtype(_) => Vec::new(),
        }
    }
}

/// Checked declaration data for one transparent type alias.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct TypeAliasDefinition {
    /// The source declaration node.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The generic template declared by the alias.
    pub(in crate::check) template: Option<GenericTemplateId>,
    /// The checked alias value.
    pub(in crate::check) value: TypeOperand,
}

/// Checked declaration data for one struct.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct StructDefinition {
    /// The source declaration node.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The generic template declared by the struct.
    pub(in crate::check) template: Option<GenericTemplateId>,
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

impl StructDefinition {
    /// Return instance type members for one key.
    pub(in crate::check) fn type_members(&self, key: &dir::StaticKey) -> Vec<TypeMemberDefinition> {
        let mut members = Vec::new();

        // collect fields
        for field in &self.fields {
            if let Some(member) = field.type_member(key) {
                members.push(member);
            }
        }

        // collect methods
        for method in &self.methods {
            if let Some(member) = method.type_member(key) {
                members.push(member);
            }
        }

        // collect associated types
        for associated_type in &self.associated_types {
            if let Some(member) = associated_type.type_member(key) {
                members.push(member);
            }
        }

        members
    }

    /// Return all named instance type members.
    pub(in crate::check) fn named_type_members(&self) -> Vec<TypeMemberDefinition> {
        let mut members = Vec::new();

        // collect named fields
        for field in &self.fields {
            members.push(field.named_type_member());
        }

        // collect named methods
        for method in &self.methods {
            if let Some(member) = method.named_type_member() {
                members.push(member);
            }
        }

        // collect associated types
        for associated_type in &self.associated_types {
            if let Some(member) = associated_type.named_type_member() {
                members.push(member);
            }
        }

        members
    }

    /// Return static value members for one key.
    pub(in crate::check) fn static_members(
        &self,
        key: &dir::StaticKey,
    ) -> Vec<StaticMemberDefinition> {
        let mut members = Vec::new();

        // collect associated constants
        for associated_const in &self.associated_consts {
            if let Some(member) = associated_const.static_member(key) {
                members.push(member);
            }
        }

        members
    }
}

/// Checked declaration data for one class.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct ClassDefinition {
    /// The source declaration node.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The generic template declared by the class.
    pub(in crate::check) template: Option<GenericTemplateId>,
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

impl ClassDefinition {
    /// Return instance type members for one key.
    pub(in crate::check) fn type_members(&self, key: &dir::StaticKey) -> Vec<TypeMemberDefinition> {
        let mut members = Vec::new();

        // collect fields
        for field in &self.fields {
            if let Some(member) = field.type_member(key) {
                members.push(member);
            }
        }

        // collect methods
        for method in &self.methods {
            if let Some(member) = method.type_member(key) {
                members.push(member);
            }
        }

        // collect associated types
        for associated_type in &self.associated_types {
            if let Some(member) = associated_type.type_member(key) {
                members.push(member);
            }
        }

        members
    }

    /// Return all named instance type members.
    pub(in crate::check) fn named_type_members(&self) -> Vec<TypeMemberDefinition> {
        let mut members = Vec::new();

        // collect named fields
        for field in &self.fields {
            members.push(field.named_type_member());
        }

        // collect named methods
        for method in &self.methods {
            if let Some(member) = method.named_type_member() {
                members.push(member);
            }
        }

        // collect associated types
        for associated_type in &self.associated_types {
            if let Some(member) = associated_type.named_type_member() {
                members.push(member);
            }
        }

        members
    }

    /// Return static value members for one key.
    pub(in crate::check) fn static_members(
        &self,
        key: &dir::StaticKey,
    ) -> Vec<StaticMemberDefinition> {
        let mut members = Vec::new();

        // collect associated constants
        for associated_const in &self.associated_consts {
            if let Some(member) = associated_const.static_member(key) {
                members.push(member);
            }
        }

        members
    }
}

/// Checked declaration data for one interface.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct InterfaceDefinition {
    /// The source declaration node.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The generic template declared by the interface.
    pub(in crate::check) template: Option<GenericTemplateId>,
    /// Whether the interface has nominal identity.
    pub(in crate::check) is_nominal: bool,
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

impl InterfaceDefinition {
    /// Return instance type members for one key.
    pub(in crate::check) fn type_members(&self, key: &dir::StaticKey) -> Vec<TypeMemberDefinition> {
        let mut members = Vec::new();

        // collect fields
        for field in &self.fields {
            if let Some(member) = field.type_member(key) {
                members.push(member);
            }
        }

        // collect methods
        for method in &self.methods {
            if let Some(member) = method.type_member(key) {
                members.push(member);
            }
        }

        // collect associated types
        for associated_type in &self.associated_types {
            if let Some(member) = associated_type.type_member(key) {
                members.push(member);
            }
        }

        members
    }

    /// Return all named instance type members.
    pub(in crate::check) fn named_type_members(&self) -> Vec<TypeMemberDefinition> {
        let mut members = Vec::new();

        // collect named fields
        for field in &self.fields {
            members.push(field.named_type_member());
        }

        // collect named methods
        for method in &self.methods {
            if let Some(member) = method.named_type_member() {
                members.push(member);
            }
        }

        // collect associated types
        for associated_type in &self.associated_types {
            if let Some(member) = associated_type.named_type_member() {
                members.push(member);
            }
        }

        members
    }

    /// Return static value members for one key.
    pub(in crate::check) fn static_members(
        &self,
        key: &dir::StaticKey,
    ) -> Vec<StaticMemberDefinition> {
        let mut members = Vec::new();

        // collect associated constants
        for associated_const in &self.associated_consts {
            if let Some(member) = associated_const.static_member(key) {
                members.push(member);
            }
        }

        members
    }
}

/// Checked declaration data for one enum.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct EnumDefinition {
    /// The source declaration node.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The generic template declared by the enum.
    pub(in crate::check) template: Option<GenericTemplateId>,
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

impl EnumDefinition {
    /// Return instance type members for one key.
    pub(in crate::check) fn type_members(&self, key: &dir::StaticKey) -> Vec<TypeMemberDefinition> {
        let mut members = Vec::new();

        // collect methods
        for method in &self.methods {
            if let Some(member) = method.type_member(key) {
                members.push(member);
            }
        }

        // collect associated types
        for associated_type in &self.associated_types {
            if let Some(member) = associated_type.type_member(key) {
                members.push(member);
            }
        }

        members
    }

    /// Return all named instance type members.
    pub(in crate::check) fn named_type_members(&self) -> Vec<TypeMemberDefinition> {
        let mut members = Vec::new();

        // collect named methods
        for method in &self.methods {
            if let Some(member) = method.named_type_member() {
                members.push(member);
            }
        }

        // collect associated types
        for associated_type in &self.associated_types {
            if let Some(member) = associated_type.named_type_member() {
                members.push(member);
            }
        }

        members
    }

    /// Return static value members for one key.
    pub(in crate::check) fn static_members(
        &self,
        key: &dir::StaticKey,
    ) -> Vec<StaticMemberDefinition> {
        let mut members = Vec::new();

        // collect associated constants
        for associated_const in &self.associated_consts {
            if let Some(member) = associated_const.static_member(key) {
                members.push(member);
            }
        }

        // collect variants
        for variant in &self.variants {
            if let Some(member) = variant.static_member(key) {
                members.push(member);
            }
        }

        members
    }
}

/// Checked declaration data for one newtype.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct NewtypeDefinition {
    /// The source declaration node.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The generic template declared by the newtype.
    pub(in crate::check) template: Option<GenericTemplateId>,
    /// The nominal backing type before solve reduction.
    pub(in crate::check) value: TypeOperand,
}

/// Checked declaration data for one extension.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct ExtensionDefinition {
    /// The source declaration node.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The extension visibility form.
    pub(in crate::check) form: dir::ExtensionForm,
    /// The checked receiver target.
    pub(in crate::check) target: ExtensionTarget,
    /// The implemented interfaces.
    pub(in crate::check) implements: Vec<NominalHeritage>,
    /// The checked where clauses that gate this extension.
    pub(in crate::check) where_clauses: Vec<ExtensionWhereClause>,
    /// The extension fields.
    pub(in crate::check) fields: Vec<FieldDefinition>,
    /// The extension static fields.
    pub(in crate::check) static_fields: Vec<FieldDefinition>,
    /// The extension methods.
    pub(in crate::check) methods: Vec<MethodDefinition>,
    /// The extension static methods.
    pub(in crate::check) static_methods: Vec<MethodDefinition>,
    /// The associated types.
    pub(in crate::check) associated_types: Vec<AssociatedTypeDefinition>,
    /// The associated constants.
    pub(in crate::check) associated_consts: Vec<AssociatedConstDefinition>,
}

impl ExtensionDefinition {
    /// Return instance type members for one key.
    pub(in crate::check) fn type_members(&self, key: &dir::StaticKey) -> Vec<TypeMemberDefinition> {
        let mut members = Vec::new();

        // collect fields
        for field in &self.fields {
            if let Some(member) = field.type_member(key) {
                members.push(member);
            }
        }

        // collect methods
        for method in &self.methods {
            if let Some(member) = method.type_member(key) {
                members.push(member);
            }
        }

        // collect associated types
        for associated_type in &self.associated_types {
            if let Some(member) = associated_type.type_member(key) {
                members.push(member);
            }
        }

        members
    }

    /// Return all named instance type members.
    pub(in crate::check) fn named_type_members(&self) -> Vec<TypeMemberDefinition> {
        let mut members = Vec::new();

        // collect named fields
        for field in &self.fields {
            members.push(field.named_type_member());
        }

        // collect named methods
        for method in &self.methods {
            if let Some(member) = method.named_type_member() {
                members.push(member);
            }
        }

        // collect associated types
        for associated_type in &self.associated_types {
            if let Some(member) = associated_type.named_type_member() {
                members.push(member);
            }
        }

        members
    }

    /// Return static value members for one key.
    pub(in crate::check) fn static_members(
        &self,
        key: &dir::StaticKey,
    ) -> Vec<StaticMemberDefinition> {
        let mut members = Vec::new();

        // collect associated constants
        for associated_const in &self.associated_consts {
            if let Some(member) = associated_const.static_member(key) {
                members.push(member);
            }
        }

        members
    }
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

/// A checked where clause attached to one extension.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) struct ExtensionWhereClause {
    /// The source where clause node.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The constrained type.
    pub(in crate::check) left: TypeOperand,
    /// The required constraint type.
    pub(in crate::check) right: TypeOperand,
}

/// Checked extension receiver target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum ExtensionTarget {
    /// Extension whose receiver type has a nominal root.
    Nominal {
        /// The nominal root used for member lookup.
        root: dir::GlobalSymbolId,
        /// The checked receiver type.
        ty: TypeOperand,
    },
    /// Extension over an open receiver type.
    Blanket {
        /// The checked receiver type.
        ty: TypeOperand,
    },
}

impl ExtensionTarget {
    /// Return the checked receiver type.
    pub(in crate::check) fn r#type(&self) -> TypeOperand {
        match self {
            Self::Nominal { ty, .. } | Self::Blanket { ty } => *ty,
        }
    }

    /// Return the nominal lookup root when this target has one.
    pub(in crate::check) fn nominal_root(&self) -> Option<dir::GlobalSymbolId> {
        match self {
            Self::Nominal { root, .. } => Some(*root),
            Self::Blanket { .. } => None,
        }
    }
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

impl FieldDefinition {
    /// Return this field as a type member when its key matches.
    pub(in crate::check) fn type_member(
        &self,
        key: &dir::StaticKey,
    ) -> Option<TypeMemberDefinition> {
        if !self.key.matches(key) {
            return None;
        }

        Some(self.named_type_member())
    }

    /// Return this field as a named type member.
    pub(in crate::check) fn named_type_member(&self) -> TypeMemberDefinition {
        TypeMemberDefinition {
            symbol: Some(self.symbol),
            key: self.key,
            ty: self.ty,
        }
    }
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

impl MethodDefinition {
    /// Return this method as a type member when its key matches.
    pub(in crate::check) fn type_member(
        &self,
        key: &dir::StaticKey,
    ) -> Option<TypeMemberDefinition> {
        let dir::MemberSlot::Key(member_key) = self.slot else {
            return None;
        };
        if !member_key.matches(key) {
            return None;
        }

        Some(TypeMemberDefinition {
            symbol: self.symbol,
            key: member_key,
            ty: self.ty,
        })
    }

    /// Return this method as a named type member.
    pub(in crate::check) fn named_type_member(&self) -> Option<TypeMemberDefinition> {
        let dir::MemberSlot::Key(key) = self.slot else {
            return None;
        };

        Some(TypeMemberDefinition {
            symbol: self.symbol,
            key,
            ty: self.ty,
        })
    }
}

/// One checked associated type.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct AssociatedTypeDefinition {
    /// The associated type symbol.
    pub(in crate::check) symbol: dir::GlobalSymbolId,
    /// The source member node.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The associated type key.
    pub(in crate::check) key: dir::StaticKey,
    /// The upper bound required by this associated type.
    pub(in crate::check) constraint: Option<TypeOperand>,
    /// The concrete associated type value.
    pub(in crate::check) value: Option<TypeOperand>,
}

impl AssociatedTypeDefinition {
    /// Return this associated type as a type member when its key matches.
    pub(in crate::check) fn type_member(
        &self,
        key: &dir::StaticKey,
    ) -> Option<TypeMemberDefinition> {
        if !self.key.matches(key) {
            return None;
        }

        self.named_type_member()
    }

    /// Return this associated type as a named type member.
    pub(in crate::check) fn named_type_member(&self) -> Option<TypeMemberDefinition> {
        let ty = self.value.or(self.constraint)?;

        Some(TypeMemberDefinition {
            symbol: Some(self.symbol),
            key: self.key,
            ty,
        })
    }
}

/// One associated constant in component operands.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct AssociatedConstDefinition {
    /// The associated const symbol.
    pub(in crate::check) symbol: dir::GlobalSymbolId,
    /// The source member node.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The associated const key.
    pub(in crate::check) key: dir::StaticKey,
    /// The static type operand.
    pub(in crate::check) ty: TypeOperand,
    /// The static value operand.
    pub(in crate::check) value: Option<StaticOperand>,
}

impl AssociatedConstDefinition {
    /// Return this associated const as a static value member.
    pub(in crate::check) fn static_member(
        &self,
        key: &dir::StaticKey,
    ) -> Option<StaticMemberDefinition> {
        if !self.key.matches(key) {
            return None;
        }
        let value = self.value?;

        Some(StaticMemberDefinition {
            symbol: self.symbol,
            key: self.key,
            value,
        })
    }
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

impl VariantDefinition {
    /// Return this variant as a static value member.
    pub(in crate::check) fn static_member(
        &self,
        key: &dir::StaticKey,
    ) -> Option<StaticMemberDefinition> {
        if !self.key.matches(key) {
            return None;
        }
        let value = self.value?;

        Some(StaticMemberDefinition {
            symbol: self.symbol,
            key: self.key,
            value,
        })
    }
}

/// One checked symbol-free signature member.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct SignatureDefinition {
    /// The source member node.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The checked signature type.
    pub(in crate::check) ty: TypeOperand,
}

/// One checked type member selected from a declaration definition.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct TypeMemberDefinition {
    /// The declaring member symbol.
    pub(in crate::check) symbol: Option<dir::GlobalSymbolId>,
    /// The member key.
    pub(in crate::check) key: dir::StaticKey,
    /// The checked member type.
    pub(in crate::check) ty: TypeOperand,
}

/// One checked static member selected from a declaration definition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) struct StaticMemberDefinition {
    /// The declaring member symbol.
    pub(in crate::check) symbol: dir::GlobalSymbolId,
    /// The member key.
    pub(in crate::check) key: dir::StaticKey,
    /// The checked static value.
    pub(in crate::check) value: StaticOperand,
}
