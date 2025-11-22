use crate::{
    BindingAnchor, Block, ExportType, Expression, FunctionSignature, Generics, Heritage,
    LocalNodeId, Name, Node, NodeType, Property, Statement, StringId,
};

/// The kind of declaration.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum DeclarationKind {
    /// Declare.
    Declaration,
    /// Definition.
    Definition,
}

/// The descriptor for a declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct DeclarationDescriptor {
    /// The kind of declaration.
    pub kind: DeclarationKind,
    /// The anchor of the declaration.
    pub anchor: BindingAnchor,
    /// The name of the declaration.
    pub name: Option<Name>,
    /// The export type of the declaration.
    pub export: Option<ExportType>,
}

/// A Declaration is a declaration in some namespace.
#[derive(Debug, Clone, PartialEq)]
pub enum Declaration {
    /// Namespace declaration (TS-only).
    Namespace {
        descriptor: DeclarationDescriptor,
        statements: Vec<LocalNodeId<Statement>>,
    },
    /// Class declaration.
    Class {
        descriptor: DeclarationDescriptor,
        generics: Generics,
        heritage: Heritage,
        properties: Vec<LocalNodeId<Property>>,
    },
    /// Interface declaration.
    Interface {
        descriptor: DeclarationDescriptor,
        generics: Generics,
        heritage: Heritage,
        properties: Vec<LocalNodeId<Property>>,
    },
    /// Enum declaration.
    Enum {
        descriptor: DeclarationDescriptor,
        fields: Vec<LocalNodeId<EnumField>>,
    },
    /// Function declaration.
    Function {
        descriptor: DeclarationDescriptor,
        signature: FunctionSignature,
        body: Option<LocalNodeId<Block>>,
    },
}

impl Node for Declaration {
    const TYPE: NodeType = NodeType::Declaration;
}

/// An EnumField is a named field of an enum declaration.
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
