use crate::{
    ExportType, Expression, FunctionSignature, Generics, Heritage, Node, NodeId, NodeType,
    Property, StringId, Type,
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
#[derive(Debug, Clone, PartialEq, Default)]
pub struct DeclarationDescriptor {
    /// The kind of declaration.
    pub kind: DeclarationKind = DeclarationKind::Definition,
    /// The name of the definition.
    pub name: Option<StringId> = None,
    /// The export type of the definition.
    pub export: Option<ExportType> = None,
}

/// Definition introduces a type or function into its scope.
#[derive(Debug, Clone, PartialEq)]
pub enum Definition {
    /// Namespace definition.
    Namespace {
        descriptor: DeclarationDescriptor,
        generics: Option<Generics>,
        definitions: Vec<NodeId<Definition>>,
    },
    /// Struct or class definition.
    Struct {
        descriptor: DeclarationDescriptor,
        kind: StructKind,
        generics: Option<Generics>,
        heritage: Option<Heritage>,
        properties: Vec<NodeId<Property>>,
    },
    /// Enum definition.
    Enum {
        descriptor: DeclarationDescriptor,
        generics: Option<Generics>,
        heritage: Option<Heritage>,
        fields: Vec<NodeId<EnumField>>,
        properties: Vec<NodeId<Property>>,
    },
    /// Interface definition.
    Interface {
        descriptor: DeclarationDescriptor,
        generics: Option<Generics>,
        heritage: Option<Heritage>,
        properties: Vec<NodeId<Property>>,
    },
    /// Function definition. Nested definitions are lifted from the body.
    Function {
        descriptor: DeclarationDescriptor,
        signature: FunctionSignature,
        definitions: Vec<NodeId<Definition>>,
        body: Option<NodeId<Expression>>,
    },
    /// Implement definition.
    Implement {
        descriptor: DeclarationDescriptor,
        generics: Option<Generics>,
        target_type: NodeId<Type>,
        heritage: Option<Heritage>,
        properties: Vec<NodeId<Property>>,
    },
}

impl Node for Definition {
    const TYPE: NodeType = NodeType::Definition;
}

impl Definition {
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
    pub value: Option<NodeId<Expression>>,
}

impl Node for EnumField {
    const TYPE: NodeType = NodeType::EnumField;
}
