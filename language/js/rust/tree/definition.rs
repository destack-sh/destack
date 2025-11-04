use dyst_source::StringId;

use crate::{Expression, Node, NodeId, NodeType, Type};

#[derive(Debug, Clone, PartialEq)]
pub enum Definition {
    Namespace {},
    Class {},
    Interface { fields: Vec<NodeId<Field>> },
    Enum { fields: Vec<NodeId<EnumField>> },
    Function {},
}

impl Node for Definition {
    const TYPE: NodeType = NodeType::Definition;
}

#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    name: StringId,
    ty: NodeId<Type>,
}

impl Node for Field {
    const TYPE: NodeType = NodeType::Field;
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnumField {
    name: StringId,
    value: Option<NodeId<Expression>>,
}

impl Node for EnumField {
    const TYPE: NodeType = NodeType::EnumField;
}
