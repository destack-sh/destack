use destack_dir as dir;
use destack_source::ModuleId;
use indexmap::IndexMap;

use crate::check::{GenericInstance, GenericTemplateId, StaticOperand, TypeOperand};
use crate::{CompilerError, CompilerResult};

/// Member namespace selected by member resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::check) enum MemberSpace {
    /// Instance members selected from a runtime receiver.
    Instance,
    /// Static members selected from a declaration receiver.
    Static,
}

/// Indexed member lookup key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::check) struct MemberKey {
    /// The member namespace.
    pub(in crate::check) space: MemberSpace,
    /// The member key.
    pub(in crate::check) key: dir::StaticKey,
}

impl MemberKey {
    /// Create a member lookup key.
    pub(in crate::check) fn new(space: MemberSpace, key: dir::StaticKey) -> Self {
        Self { space, key }
    }
}

/// Checked declaration definitions for one component.
#[derive(Debug, Default)]
pub(in crate::check) struct DefinitionTable {
    /// Definitions keyed by declaring symbol.
    definitions: IndexMap<dir::GlobalSymbolId, Definition>,
    /// Type members keyed by declaring owner and member key.
    members: IndexMap<(dir::GlobalSymbolId, MemberKey), Vec<TypeMemberDefinition>>,
    /// Extension symbols keyed by nominal target root and member key.
    extensions_by_target: IndexMap<(dir::GlobalSymbolId, MemberKey), Vec<dir::GlobalSymbolId>>,
    /// Blanket extension symbols keyed by member key.
    blanket_extensions: IndexMap<MemberKey, Vec<dir::GlobalSymbolId>>,
}

impl DefinitionTable {
    /// Create an empty definition table.
    pub(in crate::check) fn new() -> Self {
        Self {
            definitions: IndexMap::new(),
            members: IndexMap::new(),
            extensions_by_target: IndexMap::new(),
            blanket_extensions: IndexMap::new(),
        }
    }

    /// Insert one checked definition.
    pub(in crate::check) fn insert(
        &mut self,
        symbol: dir::GlobalSymbolId,
        definition: Definition,
    ) -> CompilerResult<()> {
        if self.definitions.contains_key(&symbol) {
            return Err(CompilerError::Internal {
                message: format!("definition symbol {symbol:?} already has a definition"),
            });
        }

        self.index_members(symbol, &definition);

        // index extension roots as definitions are inserted
        if let Definition::Extension(extension) = &definition {
            for key in extension.member_keys() {
                match extension.target.nominal_root() {
                    Some(target) => self
                        .extensions_by_target
                        .entry((target, key))
                        .or_default()
                        .push(symbol),
                    None => self.blanket_extensions.entry(key).or_default().push(symbol),
                }
            }
        }

        self.definitions.insert(symbol, definition);

        Ok(())
    }

    /// Index direct members declared by one definition.
    fn index_members(&mut self, symbol: dir::GlobalSymbolId, definition: &Definition) {
        for (key, member) in definition.indexed_members() {
            self.members.entry((symbol, key)).or_default().push(member);
        }
    }

    /// Return one checked definition.
    pub(in crate::check) fn definition(&self, symbol: dir::GlobalSymbolId) -> Option<&Definition> {
        self.definitions.get(&symbol)
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

    /// Return direct members declared by one owner.
    pub(in crate::check) fn members(
        &self,
        owner: dir::GlobalSymbolId,
        key: MemberKey,
    ) -> &[TypeMemberDefinition] {
        self.members
            .get(&(owner, key))
            .map(Vec::as_slice)
            .unwrap_or_default()
    }

    /// Return extension symbols declared in one module for one target and key.
    pub(in crate::check) fn extension_symbols(
        &self,
        module: ModuleId,
        target: dir::GlobalSymbolId,
        key: MemberKey,
    ) -> impl Iterator<Item = dir::GlobalSymbolId> + '_ {
        self.extensions_by_target
            .get(&(target, key))
            .into_iter()
            .flatten()
            .copied()
            .filter(move |symbol| symbol.module_id == module)
    }

    /// Return blanket extension symbols declared in one module for one key.
    pub(in crate::check) fn blanket_extension_symbols(
        &self,
        module: ModuleId,
        key: MemberKey,
    ) -> impl Iterator<Item = dir::GlobalSymbolId> + '_ {
        self.blanket_extensions
            .get(&key)
            .into_iter()
            .flatten()
            .copied()
            .filter(move |symbol| symbol.module_id == module)
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

    /// Return type-level members with their indexed keys.
    fn indexed_members(&self) -> Vec<(MemberKey, TypeMemberDefinition)> {
        match self {
            Self::Struct(definition) => definition.indexed_members(),
            Self::Class(definition) => definition.indexed_members(),
            Self::Interface(definition) => definition.indexed_members(),
            Self::Enum(definition) => definition.indexed_members(),
            Self::Extension(definition) => definition.indexed_members(),
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
    /// Return type-level members with their indexed keys.
    fn indexed_members(&self) -> Vec<(MemberKey, TypeMemberDefinition)> {
        indexed_type_members(
            &self.fields,
            &self.methods,
            &self.associated_types,
            &self.static_fields,
            &self.static_methods,
        )
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
            members.push(associated_type.named_type_member());
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
    /// Return type-level members with their indexed keys.
    fn indexed_members(&self) -> Vec<(MemberKey, TypeMemberDefinition)> {
        indexed_type_members(
            &self.fields,
            &self.methods,
            &self.associated_types,
            &self.static_fields,
            &self.static_methods,
        )
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
            members.push(associated_type.named_type_member());
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
    /// Return type-level members with their indexed keys.
    fn indexed_members(&self) -> Vec<(MemberKey, TypeMemberDefinition)> {
        indexed_type_members(
            &self.fields,
            &self.methods,
            &self.associated_types,
            &self.static_fields,
            &self.static_methods,
        )
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
            members.push(associated_type.named_type_member());
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
    /// Return type-level members with their indexed keys.
    fn indexed_members(&self) -> Vec<(MemberKey, TypeMemberDefinition)> {
        let fields = [];

        indexed_type_members(
            &fields,
            &self.methods,
            &self.associated_types,
            &self.static_fields,
            &self.static_methods,
        )
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
            members.push(associated_type.named_type_member());
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
    /// Return type-level members with their indexed keys.
    fn indexed_members(&self) -> Vec<(MemberKey, TypeMemberDefinition)> {
        indexed_type_members(
            &self.fields,
            &self.methods,
            &self.associated_types,
            &self.static_fields,
            &self.static_methods,
        )
    }

    /// Return all type-level member keys declared by this extension.
    fn member_keys(&self) -> Vec<MemberKey> {
        self.indexed_members()
            .into_iter()
            .map(|(key, _)| key)
            .collect()
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
            members.push(associated_type.named_type_member());
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
    /// Return this field as a named type member.
    pub(in crate::check) fn named_type_member(&self) -> TypeMemberDefinition {
        TypeMemberDefinition::Value {
            symbol: Some(self.symbol),
            key: self.key,
            role: None,
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
    /// The method role.
    pub(in crate::check) role: Option<dir::FunctionRole>,
    /// The checked method type.
    pub(in crate::check) ty: TypeOperand,
}

impl MethodDefinition {
    /// Return this method as a named type member.
    pub(in crate::check) fn named_type_member(&self) -> Option<TypeMemberDefinition> {
        let dir::MemberSlot::Key(key) = self.slot else {
            return None;
        };

        Some(TypeMemberDefinition::Value {
            symbol: self.symbol,
            key,
            role: self.role,
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
    /// Return this associated type as a named type member.
    pub(in crate::check) fn named_type_member(&self) -> TypeMemberDefinition {
        TypeMemberDefinition::AssociatedType {
            symbol: self.symbol,
            key: self.key,
            constraint: self.constraint,
            value: self.value,
        }
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum TypeMemberDefinition {
    /// Member with a concrete checked type.
    Value {
        /// The declaring member symbol.
        symbol: Option<dir::GlobalSymbolId>,
        /// The member key.
        key: dir::StaticKey,
        /// The member function role.
        role: Option<dir::FunctionRole>,
        /// The checked member type.
        ty: TypeOperand,
    },
    /// Associated type projection with an optional concrete value.
    AssociatedType {
        /// The associated type symbol.
        symbol: dir::GlobalSymbolId,
        /// The member key.
        key: dir::StaticKey,
        /// The upper bound required by this associated type.
        constraint: Option<TypeOperand>,
        /// The concrete associated type value.
        value: Option<TypeOperand>,
    },
}

impl TypeMemberDefinition {
    /// Return the declaring member symbol.
    pub(in crate::check) fn symbol(&self) -> Option<dir::GlobalSymbolId> {
        match self {
            Self::Value { symbol, .. } => *symbol,
            Self::AssociatedType { symbol, .. } => Some(*symbol),
        }
    }

    /// Return the member key.
    pub(in crate::check) fn key(&self) -> dir::StaticKey {
        match self {
            Self::Value { key, .. } | Self::AssociatedType { key, .. } => *key,
        }
    }

    /// Return the member function role.
    pub(in crate::check) fn role(&self) -> Option<dir::FunctionRole> {
        match self {
            Self::Value { role, .. } => *role,
            Self::AssociatedType { .. } => None,
        }
    }

    /// Return whether this member is an abstract associated type projection.
    pub(in crate::check) fn is_abstract_associated_type(&self) -> bool {
        matches!(self, Self::AssociatedType { value: None, .. })
    }

    /// Return the concrete member type when this member has one.
    pub(in crate::check) fn value(&self) -> Option<TypeOperand> {
        match self {
            Self::Value { ty, .. } => Some(*ty),
            Self::AssociatedType { value, .. } => *value,
        }
    }
}

/// Return indexed type-level members from a declaration member list.
fn indexed_type_members(
    fields: &[FieldDefinition],
    methods: &[MethodDefinition],
    associated_types: &[AssociatedTypeDefinition],
    static_fields: &[FieldDefinition],
    static_methods: &[MethodDefinition],
) -> Vec<(MemberKey, TypeMemberDefinition)> {
    let mut members = Vec::new();

    // collect instance fields and methods
    for field in fields {
        let member = field.named_type_member();
        let key = MemberKey::new(MemberSpace::Instance, member.key());

        members.push((key, member));
    }
    for method in methods {
        let Some(member) = method.named_type_member() else {
            continue;
        };
        let key = MemberKey::new(MemberSpace::Instance, member.key());

        members.push((key, member));
    }

    // collect associated types
    for associated_type in associated_types {
        let member = associated_type.named_type_member();
        let key = MemberKey::new(MemberSpace::Instance, member.key());

        members.push((key, member));
    }

    // collect static fields and methods
    for field in static_fields {
        let member = field.named_type_member();
        let key = MemberKey::new(MemberSpace::Static, member.key());

        members.push((key, member));
    }
    for method in static_methods {
        let Some(member) = method.named_type_member() else {
            continue;
        };
        let key = MemberKey::new(MemberSpace::Static, member.key());

        members.push((key, member));
    }

    members
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
