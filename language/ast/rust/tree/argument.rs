use crate::{Expression, Node, NodeId, NodeType, Pattern, StringId};

/// A Name is a regular or string identifier.
#[derive(Debug, Clone, PartialEq)]
pub enum Name {
    /// A regular identifier (regular `x` or `someThing`).
    Identifier(StringId),
    /// A string identifier (like `["Content-Type"]`, only in certain contexts).
    String(StringId),
}

impl Name {
    /// Get the string identifier.
    #[inline]
    pub fn string(&self) -> StringId {
        match self {
            Name::Identifier(id) => *id,
            Name::String(id) => *id,
        }
    }
}

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
        name: StringId,
        ty: Option<NodeId<Expression>>,
        default: Option<NodeId<Expression>>,
    },
    /// Pattern parameter (like `_` or `{ x }` or `{ x }: MyType = Foo`).
    Pattern {
        pattern: NodeId<Pattern>,
        ty: Option<NodeId<Expression>>,
        default: Option<NodeId<Expression>>,
    },
    /// Variadic parameter (like `..T` or `...x: int32[]`).
    Variadic {
        name: StringId,
        ty: Option<NodeId<Expression>>,
    },
}

impl Node for Parameter {
    const KIND: NodeType = NodeType::Parameter;
}

/// An Argument is an argument to a function call.
/// It may be named or positional. Named shorthands are only supported in struct-like literals.
/// Can be used in static and dynamic contexts (e.g. in [..] or (..)).
///
/// Examples:
/// ```
/// x: 1
/// y: foo()
/// y
/// false
/// ...args
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
        name: Name,
        value: NodeId<Expression>,
    },
    /// Named shorthand argument (like `y`, only in certain contexts like struct literals).
    NamedShorthand { name: StringId },
    /// Named shorthand function argument (like `foo()`, only in certain contexts like struct literals).
    NamedFunction {
        name: StringId,
        value: NodeId<Expression>,
    },
    /// Positional argument (like `1` or `foo()`).
    Positional { value: NodeId<Expression> },
    /// Positional spread argument (like `...args`).
    Spread { value: NodeId<Expression> },
    /// Dynamic argument (like `{ [variable]: 2 }`).
    Dynamic {
        name: Option<StringId>,
        key: NodeId<Expression>,
        value: NodeId<Expression>,
    },
}

impl Node for Argument {
    const KIND: NodeType = NodeType::Argument;
}
