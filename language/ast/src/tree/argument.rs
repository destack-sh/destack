use crate::{BindingModifier, Expression, LocalNodeId, Name, Node, NodeType, Pattern, StringId};

/// A Parameter is a parameter to some construct.
///
/// Examples:
/// ```
/// x
/// T
/// x: int32
/// y: (int32, boolean, Vector2)
/// Validate: boolean = true
/// z: int32 = 4
/// _
/// { x }
/// { x }: MyType = Foo
/// ..T
/// ...x: int32[]
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum Parameter {
    /// Named scalar parameter (like `T`, `x: int32` or `Validate: boolean = true`).
    Named {
        modifiers: Option<BindingModifier>,
        name: StringId,
        ty: Option<LocalNodeId<Expression>>,
        default: Option<LocalNodeId<Expression>>,
    },
    /// Pattern parameter (like `_` or `{ x }` or `{ x }: MyType = Foo`).
    Pattern {
        modifiers: Option<BindingModifier>,
        pattern: LocalNodeId<Pattern>,
        ty: Option<LocalNodeId<Expression>>,
        default: Option<LocalNodeId<Expression>>,
    },
    /// Variadic parameter (like `..T` or `...x: int32[]`).
    Variadic {
        modifiers: Option<BindingModifier>,
        name: StringId,
        ty: Option<LocalNodeId<Expression>>,
    },
}

impl Node for Parameter {
    const TYPE: NodeType = NodeType::Parameter;
}

/// An Argument is an argument to some construct.
/// Named arguments are only valid in tree literals (JSX-like attributes).
/// Labeled arguments are only valid in tuple types (TypeScript labeled tuple elements).
/// All other arguments (dynamic arguments, static arguments, tuples) must be positional or spread.
///
/// Examples:
/// ```
/// false                              // positional
/// foo()                              // positional
/// ...args                            // spread
/// <Component name="foo" />           // named in tree literal only
/// [start: number, end: number]       // labeled in tuple type only
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum Argument {
    /// Named argument (only valid in tree literals for JSX-like attributes).
    Named {
        name: Name,
        value: LocalNodeId<Expression>,
    },
    /// Labeled tuple element (only valid in tuple types, e.g., `[start: number, end: number]`).
    /// (Labels are purely for documentation/tooling and don't affect type checking directly.)
    Labeled {
        label: StringId,
        value: LocalNodeId<Expression>,
    },
    /// Positional argument (like `1` or `foo()`).
    Positional { value: LocalNodeId<Expression> },
    /// Spread argument (like `...args`).
    Spread { value: LocalNodeId<Expression> },
}

impl Node for Argument {
    const TYPE: NodeType = NodeType::Argument;
}
