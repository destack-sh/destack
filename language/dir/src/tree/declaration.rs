use crate::{
    BindingAnchor, DependencyMode, Expression, FunctionSignature, Generics, GlobalSymbolId,
    Heritage, LocalNodeId, LocalScopeId, LocalSymbolId, Node, NodeType, Property, StringId,
};

/// The kind of declaration.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum DeclarationKind {
    /// Declare.
    Declaration,
    /// Definition.
    Definition,
}

/// The meta data for a declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct DeclarationDescriptor {
    /// The kind of declaration.
    pub kind: DeclarationKind,
    /// The anchor of the declaration.
    pub anchor: BindingAnchor,
    /// The name of the declaration.
    pub name: Option<StringId>,
    /// The export type of the declaration.
    pub export: Option<DependencyMode>,
    /// The symbol of the declaration.
    pub symbol: LocalSymbolId,
}

/// Declaration introduces a type or function into its scope.
#[derive(Debug, Clone, PartialEq)]
pub enum Declaration {
    /// Namespace declaration.
    Namespace {
        descriptor: DeclarationDescriptor,
        generics: Generics,
        scope: LocalScopeId,
        expressions: Vec<LocalNodeId<Expression>>,
    },
    /// Struct or class declaration.
    Struct {
        descriptor: DeclarationDescriptor,
        kind: StructKind,
        generics: Generics,
        heritage: Heritage,
        scope: LocalScopeId,
        properties: Vec<LocalNodeId<Property>>,
    },
    /// Enum declaration.
    Enum {
        descriptor: DeclarationDescriptor,
        generics: Generics,
        heritage: Heritage,
        scope: LocalScopeId,
        fields: Vec<LocalNodeId<EnumField>>,
        properties: Vec<LocalNodeId<Property>>,
    },
    /// Interface declaration.
    Interface {
        descriptor: DeclarationDescriptor,
        generics: Generics,
        heritage: Heritage,
        scope: LocalScopeId,
        properties: Vec<LocalNodeId<Property>>,
    },
    /// Function declaration. Nested declarations are lifted from the body.
    Function {
        descriptor: DeclarationDescriptor,
        signature: FunctionSignature,
        scope: LocalScopeId,
        body: Option<LocalNodeId<Expression>>,
    },
    /// Extension declaration.
    Extension {
        descriptor: DeclarationDescriptor,
        generics: Generics,
        target_type: LocalNodeId<Expression>,
        target_symbol: Option<GlobalSymbolId>,
        heritage: Heritage,
        scope: LocalScopeId,
        properties: Vec<LocalNodeId<Property>>,
    },
}

impl Node for Declaration {
    const TYPE: NodeType = NodeType::Declaration;

    fn is_resolved(&self) -> bool {
        true
    }
}

impl Declaration {
    /// Get the name of this kind of declaration.
    pub fn kind_name(&self) -> &'static str {
        match self {
            Declaration::Namespace { .. } => "namespace",
            Declaration::Struct { .. } => "struct",
            Declaration::Enum { .. } => "enum",
            Declaration::Interface { .. } => "interface",
            Declaration::Function { .. } => "function",
            Declaration::Extension { .. } => "extension",
        }
    }

    /// Get the descriptor of the declaration.
    pub fn descriptor(&self) -> &DeclarationDescriptor {
        match self {
            Declaration::Namespace { descriptor, .. } => descriptor,
            Declaration::Struct { descriptor, .. } => descriptor,
            Declaration::Enum { descriptor, .. } => descriptor,
            Declaration::Interface { descriptor, .. } => descriptor,
            Declaration::Function { descriptor, .. } => descriptor,
            Declaration::Extension { descriptor, .. } => descriptor,
        }
    }

    /// Get the symbol of the declaration.
    pub fn symbol(&self) -> LocalSymbolId {
        self.descriptor().symbol
    }

    /// Get the scope of the declaration.
    pub fn scope(&self) -> LocalScopeId {
        match self {
            Declaration::Namespace { scope, .. } => *scope,
            Declaration::Struct { scope, .. } => *scope,
            Declaration::Enum { scope, .. } => *scope,
            Declaration::Interface { scope, .. } => *scope,
            Declaration::Function { scope, .. } => *scope,
            Declaration::Extension { scope, .. } => *scope,
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

/// An enum field is a named field of an enum declaration.
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
