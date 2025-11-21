use crate::{Expression, Mutability, Node, LocalNodeId, NodeType, Pattern, StringId, Type, Visibility};

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
    Instance,
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

/// Named or positional parameter to some construct.
#[derive(Debug, Clone, PartialEq)]
pub enum Parameter {
    /// Named parameter (like `x: int32` or `Validate: boolean = true`).
    Named {
        modifiers: Option<BindingModifier>,
        name: StringId,
        ty: Option<LocalNodeId<Type>>,
        default: Option<LocalNodeId<Expression>>,
    },
    /// Pattern parameter (like `_` or `{ x }` or `{ x, ..rest }: MyType = Foo`).
    Pattern {
        modifiers: Option<BindingModifier>,
        pattern: LocalNodeId<Pattern>,
        ty: Option<LocalNodeId<Type>>,
        default: Option<LocalNodeId<Expression>>,
    },
    /// Variadic parameter (like `...args: int32[]`).
    Variadic {
        modifiers: Option<BindingModifier>,
        name: StringId,
        ty: Option<LocalNodeId<Type>>,
    },
}

impl Node for Parameter {
    const TYPE: NodeType = NodeType::Parameter;
}

/// Positional argument to some construct.
#[derive(Debug, Clone, PartialEq)]
pub enum Argument {
    /// Positional argument (like `1` or `foo()`).
    Positional { value: LocalNodeId<Expression> },
    /// Spread argument (like `...args`).
    Spread { value: LocalNodeId<Expression> },
    /// Dynamic argument (like `[variable]: 2`).
    Dynamic {
        key: LocalNodeId<Expression>,
        value: LocalNodeId<Expression>,
    },
}

impl Node for Argument {
    const TYPE: NodeType = NodeType::Argument;
}
