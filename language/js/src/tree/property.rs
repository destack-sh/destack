use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::{
    Block, ClassElementName, Expression, FunctionSignature, Identifier, LocalNodeId, Node,
    NodeType, Parameter, Pattern, PropertyName,
};

/// One object literal property.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub enum Property {
    /// Named field like `x: value`.
    Field {
        key: PropertyName,
        value: LocalNodeId<Expression>,
    },
    /// Shorthand field like `x`.
    Shorthand { value: Identifier },
    /// Ordinary method like `method()`.
    Method {
        key: PropertyName,
        signature: FunctionSignature,
        body: LocalNodeId<Block>,
    },
    /// Getter method like `get value()`.
    Getter {
        key: PropertyName,
        body: LocalNodeId<Block>,
    },
    /// Setter method like `set value(next)`.
    Setter {
        key: PropertyName,
        parameter: LocalNodeId<Parameter>,
        body: LocalNodeId<Block>,
    },
    /// Spread property like `...value`.
    Spread { value: LocalNodeId<Expression> },
}

impl Node for Property {
    const TYPE: NodeType = NodeType::Property;
}

/// One class member.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub enum Member {
    /// Named field like `x = value`.
    Field {
        key: ClassElementName,
        default: Option<LocalNodeId<Expression>>,
        is_static: bool,
    },
    /// Ordinary method.
    Method {
        key: ClassElementName,
        signature: FunctionSignature,
        body: LocalNodeId<Block>,
        is_static: bool,
    },
    /// Getter method.
    Getter {
        key: ClassElementName,
        body: LocalNodeId<Block>,
        is_static: bool,
    },
    /// Setter method.
    Setter {
        key: ClassElementName,
        parameter: LocalNodeId<Parameter>,
        body: LocalNodeId<Block>,
        is_static: bool,
    },
    /// Constructor method.
    Constructor {
        parameters: Vec<LocalNodeId<Parameter>>,
        rest: Option<LocalNodeId<Pattern>>,
        body: LocalNodeId<Block>,
    },
    /// Static initialization block like `static { ... }`.
    StaticBlock { body: LocalNodeId<Block> },
}

impl Node for Member {
    const TYPE: NodeType = NodeType::Member;
}
