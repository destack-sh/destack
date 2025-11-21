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
/// It may be named or positional. Named shorthands are only supported in struct-like literals.
/// Can be used in static and dynamic contexts (e.g. in [..] or (..)).
///
/// Examples:
/// ```
/// x: 1
/// x?: 1
/// y: foo()
/// y
/// false
/// ...args
/// ...args: int32[]
/// foo()
/// ["Content-Type"]: "application/json"
/// [x: string]: any
/// [string]: woof
/// [var] = "hello"
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum Argument {
    /// Named argument (like `x: 1` or `y: foo()`).
    Named {
        modifiers: Option<BindingModifier>,
        name: Name,
        value: LocalNodeId<Expression>,
    },
    /// Named shorthand argument (like `y`, only in certain contexts like struct literals).
    Shorthand {
        modifiers: Option<BindingModifier>,
        name: StringId,
    },
    /// Positional argument (like `1` or `foo()`).
    Positional {
        modifiers: Option<BindingModifier>,
        value: LocalNodeId<Expression>,
    },
    /// Spread argument (like `...args` or `...args: int32[]`).
    Spread {
        modifiers: Option<BindingModifier>,
        name: Option<StringId>,
        value: LocalNodeId<Expression>,
    },
}

impl Node for Argument {
    const TYPE: NodeType = NodeType::Argument;
}
