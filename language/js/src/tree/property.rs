use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::{
    Block, ClassElementName, Expression, FunctionRole, FunctionSignature, Identifier, LocalNodeId,
    MemberModifier, Node, NodeType, Parameter, PropertyName,
};

/// One object literal property.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub enum Property {
    /// Named field such as `x: value`.
    Field {
        key: PropertyName,
        value: LocalNodeId<Expression>,
    },
    /// One shorthand identifier field such as `x`.
    Shorthand { value: Identifier },
    /// Named method such as `foo()`.
    Method {
        key: PropertyName,
        role: Option<FunctionRole>,
        signature: FunctionSignature,
        body: LocalNodeId<Block>,
    },
    /// Spread property (like `...a`).
    Spread { value: LocalNodeId<Expression> },
}

impl Node for Property {
    const TYPE: NodeType = NodeType::Property;
}

/// One class member.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub enum Member {
    /// Named field such as `x = value`.
    Field {
        modifiers: MemberModifier,
        key: ClassElementName,
        default: Option<LocalNodeId<Expression>>,
    },
    /// Named method.
    Method {
        modifiers: MemberModifier,
        key: ClassElementName,
        role: Option<FunctionRole>,
        signature: FunctionSignature,
        body: LocalNodeId<Block>,
    },
    /// Constructor method.
    Constructor {
        parameters: Vec<LocalNodeId<Parameter>>,
        body: LocalNodeId<Block>,
    },
    /// Static initialization block (like `static { ... }`).
    StaticBlock { body: LocalNodeId<Block> },
}

impl Node for Member {
    const TYPE: NodeType = NodeType::Member;
}
