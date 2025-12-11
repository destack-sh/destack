use crate::{BindingModifier, Expression, FunctionSignature, Key, LocalNodeId, Node, NodeType};

/// A Property is a property of an object literal (may be a field, method, or spread).
#[derive(Debug, Clone, PartialEq)]
pub enum Property {
    /// Named field (like `x: int32`).
    Field {
        modifiers: Option<BindingModifier>,
        key: Option<Key>,
        value: Option<LocalNodeId<Expression>>,
        default: Option<LocalNodeId<Expression>>,
    },
    /// Named member function (like `foo()` or `<T>(): T`).
    Method {
        modifiers: Option<BindingModifier>,
        key: Option<Key>,
        signature: FunctionSignature,
        body: Option<LocalNodeId<Expression>>,
    },
    /// Spread property (like `...a`).
    Spread {
        modifiers: Option<BindingModifier>,
        value: LocalNodeId<Expression>,
    },
}

impl Node for Property {
    const TYPE: NodeType = NodeType::Property;
}

/// A Member is a member of a class-like declaration.
#[derive(Debug, Clone, PartialEq)]
pub enum Member {
    /// Named field (like `x: int32`).
    Field {
        modifiers: Option<BindingModifier>,
        key: Option<Key>,
        value: Option<LocalNodeId<Expression>>,
        default: Option<LocalNodeId<Expression>>,
    },
    /// Named member function (like `foo()` or `<T>(): T`).
    Method {
        modifiers: Option<BindingModifier>,
        key: Option<Key>,
        signature: FunctionSignature,
        body: Option<LocalNodeId<Expression>>,
    },
    /// Static initialization block (like `static { ... }`).
    StaticBlock { body: LocalNodeId<Expression> },
}

impl Node for Member {
    const TYPE: NodeType = NodeType::Member;
}
