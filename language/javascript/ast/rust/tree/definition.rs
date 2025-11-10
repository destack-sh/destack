use crate::{
    BindingScope, Block, ExportType, Expression, Name, Node, NodeId, NodeType, Parameter, StringId,
    Type, Visibility,
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
    Namespace {
        meta: DefinitionMeta,
        definitions: Vec<NodeId<Definition>>,
    },
    Class {
        meta: DefinitionMeta,
        static_parameters: Option<Vec<NodeId<Parameter>>>,
        fields: Vec<NodeId<Field>>,
        definitions: Vec<NodeId<Definition>>,
    },
    Interface {
        meta: DefinitionMeta,
        fields: Vec<NodeId<Field>>,
        definitions: Vec<NodeId<Definition>>,
    },
    Enum {
        meta: DefinitionMeta,
        fields: Vec<NodeId<EnumField>>,
    },
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
pub struct Field {
    pub name: StringId,
    pub ty: Option<NodeId<Type>>,
    pub default: Option<NodeId<Expression>>,
}

impl Node for Field {
    const TYPE: NodeType = NodeType::Field;
}

/// An EnumField is a named field of an enum definition.
#[derive(Debug, Clone, PartialEq)]
pub struct EnumField {
    pub name: StringId,
    pub value: Option<NodeId<Expression>>,
}

impl Node for EnumField {
    const TYPE: NodeType = NodeType::EnumField;
}
