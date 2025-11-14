use crate::{
    BindingModifier, BindingScope, Block, ExportType, Expression, Name, Node, NodeId, NodeType,
    Parameter, StringId, Type, Visibility,
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
    pub scope: BindingScope = BindingScope::Container,
    /// The name of the definition.
    pub name: Option<Name> = None,
    /// The key of the definition.
    pub key: Option<NodeId<Expression>> = None,
    /// The visibility of the definition.
    pub visibility: Option<Visibility> = None,
    /// The export type of the definition.
    pub export: Option<ExportType> = None,
}

/// A Definition is a declaration in some namespace.
#[derive(Debug, Clone, PartialEq)]
pub enum Definition {
    /// Namespace definition (TS-only).
    Namespace {
        descriptor: DeclarationDescriptor,
        definitions: Vec<NodeId<Definition>>,
    },
    /// Class definition.
    Class {
        descriptor: DeclarationDescriptor,
        static_parameters: Option<Vec<NodeId<Parameter>>>,
        fields: Vec<NodeId<Field>>,
        definitions: Vec<NodeId<Definition>>,
    },
    /// Interface definition.
    Interface {
        descriptor: DeclarationDescriptor,
        static_parameters: Option<Vec<NodeId<Parameter>>>,
        fields: Vec<NodeId<Field>>,
        definitions: Vec<NodeId<Definition>>,
    },
    /// Enum definition.
    Enum {
        descriptor: DeclarationDescriptor,
        fields: Vec<NodeId<EnumField>>,
    },
    /// Function definition.
    Function {
        descriptor: DeclarationDescriptor,
        static_parameters: Option<Vec<NodeId<Parameter>>>,
        dynamic_parameters: Vec<NodeId<Parameter>>,
        return_type: Option<NodeId<Type>>,
        body: Option<NodeId<Block>>,
    },
}

impl Node for Definition {
    const TYPE: NodeType = NodeType::Definition;
}

/// A Field is a named property of a definition.
#[derive(Debug, Clone, PartialEq)]
pub enum Field {
    // nocheckin: turn JS Field -> Property
    /// Named field (like `x: int32`).
    Named {
        modifiers: Option<BindingModifier>,
        name: Name,
        ty: NodeId<Type>,
        default: Option<NodeId<Expression>>,
    },
    /// Dynamic field (like `[x: string]: any`).
    Dynamic {
        modifiers: Option<BindingModifier>,
        name: Option<StringId>,
        ty: NodeId<Type>,
        key: NodeId<Expression>,
        default: Option<NodeId<Expression>>,
    },
}

impl Node for Field {
    const TYPE: NodeType = NodeType::Field;
}

/// An EnumField is a named field of an enum definition.
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
