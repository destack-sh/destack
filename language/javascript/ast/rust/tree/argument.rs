use crate::{Expression, Mutability, Node, NodeId, NodeType, Pattern, StringId, Type, Visibility};

/// The type of a binding.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum BindingKind {
    /// Must binding (like `x`).
    Must,
    /// Maybe binding (like `x?`).
    Maybe,
}

/// The scope of a binding (dynamic or static).
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum BindingScope {
    /// Container scope.
    Container,
    /// Static scope.
    Static,
}

/// The operator to apply to the binding.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum BindingOperator {
    /// Apply `as const` to the value of the binding.
    AsConst,
}

/// The modifiers of a field-like item.
#[derive(Debug, Copy, Clone, PartialEq, Default)]
pub struct BindingModifier {
    /// The kind of the binding.
    pub kind: Option<BindingKind> = None,
    /// The scope of the binding.
    pub scope: Option<BindingScope> = None,
    /// The mutability of the field.
    pub mutability: Option<Mutability> = None,
    /// The visibility of the field.
    pub visibility: Option<Visibility> = None,
    /// The operator to apply to the binding.
    pub operator: Option<BindingOperator> = None,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Parameter {
    /// Named parameter (like `x: int32` or `Validate: boolean = true`).
    Named {
        modifiers: Option<BindingModifier>,
        name: StringId,
        ty: Option<NodeId<Type>>,
        default: Option<NodeId<Expression>>,
    },
    /// Pattern parameter (like `_` or `{ x }` or `{ x, ..rest }: MyType = Foo`).
    Pattern {
        modifiers: Option<BindingModifier>,
        pattern: NodeId<Pattern>,
        ty: Option<NodeId<Type>>,
        default: Option<NodeId<Expression>>,
    },
    /// Variadic parameter (like `...args: int32[]`).
    Variadic {
        modifiers: Option<BindingModifier>,
        name: StringId,
        ty: Option<NodeId<Type>>,
    },
}

impl Node for Parameter {
    const TYPE: NodeType = NodeType::Parameter;
}

#[derive(Debug, Clone, PartialEq)]
pub enum Argument {
    /// Positional argument (like `1` or `foo()`).
    Positional { value: NodeId<Expression> },
    /// Spread argument (like `...args`).
    Spread { value: NodeId<Expression> },
    /// Dynamic argument (like `[variable]: 2`).
    Dynamic {
        key: NodeId<Expression>,
        value: NodeId<Expression>,
    },
}

impl Node for Argument {
    const TYPE: NodeType = NodeType::Argument;
}
