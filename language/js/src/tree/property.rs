use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

use crate::{
    Block, Expression, FunctionRole, FunctionSignature, Key, LocalNodeId, MemberModifier, Node,
    NodeType, Parameter,
};

/// A Property is a property of an object literal (may be a field, method, or spread).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub enum Property {
    /// Named field such as `x: value`.
    Field {
        key: Key,
        value: LocalNodeId<Expression>,
        is_shorthand: bool,
    },
    /// Named method such as `foo()`.
    Method {
        key: Key,
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
        key: Key,
        default: Option<LocalNodeId<Expression>>,
    },
    /// Named method.
    Method {
        modifiers: MemberModifier,
        key: Key,
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
