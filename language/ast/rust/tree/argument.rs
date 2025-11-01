use crate::{BindingModifiers, Expression, Node, NodeId, NodeType, Pattern, StringId};

/// A Name is a regular or string identifier.
#[derive(Debug, Clone, Copy, PartialEq)]
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
        modifiers: Option<BindingModifiers>,
        name: StringId,
        ty: Option<NodeId<Expression>>,
        default: Option<NodeId<Expression>>,
    },
    /// Pattern parameter (like `_` or `{ x }` or `{ x }: MyType = Foo`).
    Pattern {
        modifiers: Option<BindingModifiers>,
        pattern: NodeId<Pattern>,
        ty: Option<NodeId<Expression>>,
        default: Option<NodeId<Expression>>,
    },
    /// Variadic parameter (like `..T` or `...x: int32[]`).
    Variadic {
        modifiers: Option<BindingModifiers>,
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
        modifiers: Option<BindingModifiers>,
        name: Name,
        value: NodeId<Expression>,
    },
    /// Named shorthand argument (like `y`, only in certain contexts like struct literals).
    Shorthand {
        modifiers: Option<BindingModifiers>,
        name: StringId,
    },
    /// Positional argument (like `1` or `foo()`).
    Positional {
        modifiers: Option<BindingModifiers>,
        value: NodeId<Expression>,
    },
    /// Spread argument (like `...args` or `...args: int32[]`).
    Spread {
        modifiers: Option<BindingModifiers>,
        name: Option<StringId>,
        value: NodeId<Expression>,
    },
    /// Dynamic argument (like `{ [variable]: 2 }`).
    Dynamic {
        modifiers: Option<BindingModifiers>,
        name: Option<StringId>,
        key: NodeId<Expression>,
        value: NodeId<Expression>,
    },
    /// Named shorthand function argument (like `foo()` or `<T>(): T`, only in struct literals).
    Function {
        modifiers: Option<BindingModifiers>,
        name: Option<Name>,
        value: NodeId<Expression>,
    },
    /// Dynamic function argument (like `[x: string](): T`, only in struct literals).
    DynamicFunction {
        modifiers: Option<BindingModifiers>,
        name: Option<StringId>,
        key: NodeId<Expression>,
        value: NodeId<Expression>,
    },
}

impl Node for Argument {
    const KIND: NodeType = NodeType::Argument;
}
