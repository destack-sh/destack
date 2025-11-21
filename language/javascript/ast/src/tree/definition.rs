use crate::{
    BindingScope, Block, ExportType, Expression, FunctionSignature, Generics, Heritage,
    LocalNodeId, Name, Node, NodeType, Property, StringId,
};

/// The kind of declaration.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum DeclarationKind {
    /// Declare without link.
    Declaration,
    /// Inline definition.
    Definition,
}

/// The descriptor for a definition.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct DeclarationDescriptor {
    /// The kind of declaration.
    pub kind: DeclarationKind = DeclarationKind::Definition,
    /// The scope of the declaration.
    pub scope: BindingScope = BindingScope::Instance,
    /// The name of the definition.
    pub name: Option<Name> = None,
    /// The export type of the definition.
    pub export: Option<ExportType> = None,
}

/// A Definition is a declaration in some namespace.
#[derive(Debug, Clone, PartialEq)]
pub enum Definition {
    /// Namespace definition (TS-only).
    Namespace {
        descriptor: DeclarationDescriptor,
        definitions: Vec<LocalNodeId<Definition>>,
    },
    /// Class definition.
    Class {
        descriptor: DeclarationDescriptor,
        generics: Generics,
        heritage: Heritage,
        properties: Vec<LocalNodeId<Property>>,
    },
    /// Interface definition.
    Interface {
        descriptor: DeclarationDescriptor,
        generics: Generics,
        heritage: Heritage,
        properties: Vec<LocalNodeId<Property>>,
    },
    /// Enum definition.
    Enum {
        descriptor: DeclarationDescriptor,
        fields: Vec<LocalNodeId<EnumField>>,
    },
    /// Function definition.
    Function {
        descriptor: DeclarationDescriptor,
        signature: FunctionSignature,
        body: Option<LocalNodeId<Block>>,
    },
}

impl Node for Definition {
    const TYPE: NodeType = NodeType::Definition;
}

/// An EnumField is a named field of an enum definition.
#[derive(Debug, Clone, PartialEq)]
pub struct EnumField {
    /// The name of the enum field.
    pub name: StringId,
    /// The value of the enum field.
    pub value: Option<LocalNodeId<Expression>>,
}

impl Node for EnumField {
    const TYPE: NodeType = NodeType::EnumField;
}
