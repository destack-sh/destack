use serde::{Deserialize, Serialize};

use crate::{
    Expression, LocalNodeId, Mutability, Node, NodeType, Pattern, StringId, TypeExpression,
    Visibility,
};

/// The type of a binding.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub enum BindingKind {
    /// Must binding (like `x`).
    Must,
    /// Maybe binding (like `x?`).
    Maybe,
}

/// Variance annotation for type parameters.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub enum VarianceModifier {
    /// Contravariant type parameter.
    In,
    /// Covariant type parameter.
    Out,
    /// Invariant type parameter.
    InOut,
}

/// The scope of a binding (dynamic or static).
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub enum BindingAnchor {
    /// Container scope.
    Instance,
    /// Static scope.
    Static,
}

/// The operator to apply to the binding.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub enum BindingOperator {
    /// Apply `as const` to the value of the binding.
    AsConst,
}

/// The accessor kind of a binding.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub enum AccessorKind {
    /// Auto-accessor (generates getter/setter).
    Accessor,
}

/// The modifiers of a field-like item.
#[derive(Debug, Copy, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct BindingModifier {
    /// The kind of the binding.
    pub kind: Option<BindingKind>,
    /// The variance of a type parameter.
    pub variance: Option<VarianceModifier>,
    /// The scope of the binding.
    pub anchor: Option<BindingAnchor>,
    /// The mutability of the field.
    pub mutability: Option<Mutability>,
    /// The visibility of the field.
    pub visibility: Option<Visibility>,
    /// The operator to apply to the binding.
    pub operator: Option<BindingOperator>,
    /// Whether the binding uses a definite assignment assertion.
    pub definite: bool,
    /// The accessor kind of the binding.
    pub accessor: Option<AccessorKind>,
}

/// One generic parameter.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GenericParameter {
    /// Type parameter.
    Type {
        modifiers: Option<BindingModifier>,
        name: StringId,
        constraint: Option<LocalNodeId<TypeExpression>>,
        default: Option<LocalNodeId<TypeExpression>>,
    },
}

impl Node for GenericParameter {
    const TYPE: NodeType = NodeType::GenericParameter;
}

/// Named or positional parameter to some construct.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Parameter {
    /// Named parameter (like `x: int32` or `Validate: boolean = true`).
    Named {
        modifiers: Option<BindingModifier>,
        name: StringId,
        ty: Option<LocalNodeId<TypeExpression>>,
        default: Option<LocalNodeId<Expression>>,
    },
    /// Pattern parameter (like `_` or `{ x }` or `{ x, ..rest }: MyType = Foo`).
    Pattern {
        modifiers: Option<BindingModifier>,
        pattern: LocalNodeId<Pattern>,
        ty: Option<LocalNodeId<TypeExpression>>,
        default: Option<LocalNodeId<Expression>>,
    },
    /// Variadic parameter with a named binding (like `...args: int32[]`).
    VariadicNamed {
        modifiers: Option<BindingModifier>,
        name: StringId,
        ty: Option<LocalNodeId<TypeExpression>>,
    },
    /// Variadic parameter with a pattern binding (like `...[a, b]`).
    VariadicPattern {
        modifiers: Option<BindingModifier>,
        pattern: LocalNodeId<Pattern>,
        ty: Option<LocalNodeId<TypeExpression>>,
    },
}

impl Node for Parameter {
    const TYPE: NodeType = NodeType::Parameter;
}

/// Positional argument to some construct.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Argument {
    /// Positional argument (like `1` or `foo()`).
    Positional { value: LocalNodeId<Expression> },
    /// Spread argument (like `...args`).
    Spread { value: LocalNodeId<Expression> },
}

impl Node for Argument {
    const TYPE: NodeType = NodeType::Argument;
}
