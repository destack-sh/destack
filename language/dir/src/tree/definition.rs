use crate::{
    BindingScope, ExportType, Expression, FunctionSignature, Generics, Heritage, Node, LocalNodeId,
    NodeType, Property, LocalScopeId, StringId, LocalSymbolId, Type,
};

/// The kind of declaration.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum DeclarationKind {
    /// Declare without link.
    Declaration,
    /// Inline definition.
    Definition,
}

/// The meta data for a definition.
#[derive(Debug, Clone, PartialEq)]
pub struct DeclarationDescriptor {
    /// The kind of declaration.
    pub kind: DeclarationKind,
    /// The scope of the declaration.
    pub scope: BindingScope,
    /// The name of the definition.
    pub name: Option<StringId>,
    /// The export type of the definition.
    pub export: Option<ExportType>,
    /// The symbol of the definition.
    pub symbol: LocalSymbolId,
}

/// Definition introduces a type or function into its scope.
#[derive(Debug, Clone, PartialEq)]
pub enum Definition {
    /// Namespace definition.
    Namespace {
        descriptor: DeclarationDescriptor,
        generics: Generics,
        scope: LocalScopeId,
        definitions: Vec<LocalNodeId<Definition>>,
    },
    /// Struct or class definition.
    Struct {
        descriptor: DeclarationDescriptor,
        kind: StructKind,
        generics: Generics,
        heritage: Heritage,
        scope: LocalScopeId,
        properties: Vec<LocalNodeId<Property>>,
    },
    /// Enum definition.
    Enum {
        descriptor: DeclarationDescriptor,
        generics: Generics,
        heritage: Heritage,
        scope: LocalScopeId,
        fields: Vec<LocalNodeId<EnumField>>,
        properties: Vec<LocalNodeId<Property>>,
    },
    /// Interface definition.
    Interface {
        descriptor: DeclarationDescriptor,
        generics: Generics,
        heritage: Heritage,
        scope: LocalScopeId,
        properties: Vec<LocalNodeId<Property>>,
    },
    /// Function definition. Nested definitions are lifted from the body.
    Function {
        descriptor: DeclarationDescriptor,
        signature: FunctionSignature,
        scope: LocalScopeId,
        definitions: Vec<LocalNodeId<Definition>>,
        body: Option<LocalNodeId<Expression>>,
    },
    /// Implement definition.
    Implement {
        descriptor: DeclarationDescriptor,
        generics: Generics,
        target_type: LocalNodeId<Type>,
        heritage: Heritage,
        scope: LocalScopeId,
        properties: Vec<LocalNodeId<Property>>,
    },
}

impl Node for Definition {
    const TYPE: NodeType = NodeType::Definition;

    fn is_resolved(&self) -> bool {
        true
    }
}

impl Definition {
    /// Get the name of this kind of definition.
    pub fn kind_name(&self) -> &'static str {
        match self {
            Definition::Namespace { .. } => "namespace",
            Definition::Struct { .. } => "struct",
            Definition::Enum { .. } => "enum",
            Definition::Interface { .. } => "interface",
            Definition::Function { .. } => "function",
            Definition::Implement { .. } => "implement",
        }
    }

    /// Get the descriptor of the definition.
    pub fn descriptor(&self) -> &DeclarationDescriptor {
        match self {
            Definition::Namespace { descriptor, .. } => descriptor,
            Definition::Struct { descriptor, .. } => descriptor,
            Definition::Enum { descriptor, .. } => descriptor,
            Definition::Interface { descriptor, .. } => descriptor,
            Definition::Function { descriptor, .. } => descriptor,
            Definition::Implement { descriptor, .. } => descriptor,
        }
    }

    /// Get the symbol of the definition.
    pub fn symbol(&self) -> LocalSymbolId {
        self.descriptor().symbol
    }

    /// Get the scope of the definition.
    pub fn scope(&self) -> LocalScopeId {
        match self {
            Definition::Namespace { scope, .. } => *scope,
            Definition::Struct { scope, .. } => *scope,
            Definition::Enum { scope, .. } => *scope,
            Definition::Interface { scope, .. } => *scope,
            Definition::Function { scope, .. } => *scope,
            Definition::Implement { scope, .. } => *scope,
        }
    }
}

/// The style of a struct or class.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum StructKind {
    /// Struct.
    Struct,
    /// Class.
    Class,
}

/// An enum field is a named field of an enum definition.
#[derive(Debug, Clone, PartialEq)]
pub struct EnumField {
    /// The name of the enum field.
    pub name: StringId,
    /// The value of the enum field.
    pub value: Option<LocalNodeId<Expression>>,
    /// The symbol of the enum field.
    pub symbol: LocalSymbolId,
}

impl Node for EnumField {
    const TYPE: NodeType = NodeType::EnumField;

    fn is_resolved(&self) -> bool {
        true
    }
}
