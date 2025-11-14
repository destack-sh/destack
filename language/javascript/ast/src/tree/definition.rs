use crate::{
    BindingModifier, BindingScope, Block, ExportType, Expression, Name, Node, NodeId, NodeType,
    Parameter, StringId, Type, Visibility,
};

/// The kind of declaration.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum DeclarationKind {
    Declaration,
    Definition,
}

/// The meta data for a definition.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct DefinitionMeta {
    pub kind: DeclarationKind = DeclarationKind::Definition,
    pub scope: BindingScope = BindingScope::Container,
    pub name: Option<Name> = None,
    pub key: Option<NodeId<Expression>> = None,
    pub visibility: Option<Visibility> = None,
    pub export: Option<ExportType> = None,
}

/// A Definition is a declaration in some namespace.
#[derive(Debug, Clone, PartialEq)]
pub enum Definition {
    /// Namespace definition (TS-only).
    Namespace {
        meta: DefinitionMeta,
        definitions: Vec<NodeId<Definition>>,
    },
    /// Class definition.
    Class {
        meta: DefinitionMeta,
        static_parameters: Option<Vec<NodeId<Parameter>>>,
        fields: Vec<NodeId<Field>>,
        definitions: Vec<NodeId<Definition>>,
    },
    /// Interface definition.
    Interface {
        meta: DefinitionMeta,
        static_parameters: Option<Vec<NodeId<Parameter>>>,
        fields: Vec<NodeId<Field>>,
        definitions: Vec<NodeId<Definition>>,
    },
    /// Enum definition.
    Enum {
        meta: DefinitionMeta,
        fields: Vec<NodeId<EnumField>>,
    },
    /// Function definition.
    Function {
        meta: DefinitionMeta,
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
pub enum Field { // nocheckin: turn JS Field -> Property
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
