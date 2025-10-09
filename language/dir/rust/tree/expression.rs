use dyst_ast::StringId;

use crate::{
    Argument, AssignOperator, BinaryOperator, Block, Definition, Destination, Node, NodeId,
    NodeType, Path, Pattern, ScalarLiteral, Type, TypeLiteral, UnaryOperator,
};

#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    // Block of "statements" (inside `{}` usually)
    Block(NodeId<Block>),

    // With context declaration (flattened, like `with Foo, Bar` for `with Foo.Bar`)
    With {
        clauses: Vec<NodeId<WithClause>>,
        body: Option<NodeId<Block>>,
    },

    // Use dependency declaration (flattened, like `use foo.bar` for `use foo.bar, baz.quz`)
    Use {
        items: Vec<NodeId<UseItem>>,
        body: Option<NodeId<Block>>,
    },

    /// Unary operation (except reference/dereference, e.g., `-x`).
    Unary {
        operator: UnaryOperator,
        right: NodeId<Expression>,
    },
    /// Reference operation (e.g., `&x`).
    Reference {
        right: NodeId<Expression>,
    },
    /// Dereference operation (e.g., `*x`).
    Dereference {
        right: NodeId<Expression>,
    },
    /// Binary operation.
    Binary {
        left: NodeId<Expression>,
        operator: BinaryOperator,
        right: NodeId<Expression>,
    },
    /// Assignment (e.g., `x = y`).
    AssignDirect {
        left: NodeId<Expression>,
        right: NodeId<Expression>,
    },
    /// Assignment with operator (except direct assignment, e.g., `x += y`).
    AssignBinary {
        left: NodeId<Expression>,
        operator: AssignOperator,
        right: NodeId<Expression>,
    },

    /// Member access.
    Member {
        left: NodeId<Expression>,
        path: Path,
    },
    /// Call to a function.
    Call {
        left: NodeId<Expression>,
        dynamic_arguments: Vec<NodeId<Argument>>,
    },
    /// Index into an array or slice.
    Index {
        left: NodeId<Expression>,
        right: NodeId<Expression>,
    },
    /// Cast to a type.
    Cast {
        value: NodeId<Expression>,
        ty: NodeId<Type>,
    },
    // nocheckin TODO #Incomplete: Expression.Let?
    /// --------------------------------
    /// Literals.
    /// --------------------------------
    /// Path.
    /// --------------------------------
    Path {
        path: Path,
    },
    // Inline definition as a value (with a name or anonymous)
    InlineDefinition {
        definition: NodeId<Definition>,
    },
    /// Scalar literal value.
    ScalarLiteral {
        value: ScalarLiteral,
    },
    /// Type literal value.
    TypeLiteral {
        value: TypeLiteral,
    },
    /// Struct creation.
    StructLiteral {
        ty: NodeId<Type>,
        fields: Vec<NodeId<Argument>>,
    },
    /// Tuple creation.
    TupleLiteral {
        elements: Vec<NodeId<Argument>>,
    },
    /// Array creation.
    ArrayLiteral {
        elements: Vec<NodeId<Expression>>,
    },

    /// --------------------------------
    /// Control flow.
    /// --------------------------------

    /// If expression.
    If {
        condition: NodeId<Expression>,
        then_block: NodeId<Block>,
        else_block: Option<NodeId<Expression>>,
    },
    /// Loop expression.
    Loop {
        condition: NodeId<Expression>,
        body: NodeId<Block>,
        source: LoopSource,
    },
    /// Match expression.
    Match {
        value: NodeId<Expression>,
        cases: Vec<NodeId<MatchCase>>,
        source: MatchSource,
    },
    /// Break expression.
    Break {
        destination: Destination,
        value: Option<NodeId<Expression>>,
    },
    /// Continue expression.
    Continue {
        destination: Destination,
    },
    /// Defer expression.
    Defer {
        destination: Destination,
        body: NodeId<Expression>,
    },
    /// Return expression.
    Return {
        destination: Destination,
        value: Option<NodeId<Expression>>,
    },

    /// Error expression.
    Error,
}

impl Node for Expression {
    const KIND: NodeType = NodeType::Expression;
}

/// A LoopSource is where the loop was lowered from.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum LoopSource {
    /// For loop.
    For,
    /// While loop.
    While,
    /// Loop loop.
    Loop,
}

/// A UseItem is an item to use in a use clause.
#[derive(Debug, Clone, PartialEq)]
pub struct UseItem {
    /// The source of the item.
    pub source: Path,
    /// The source name of the item (like `foo` in `foo as bar`)
    pub name: StringId,
    /// The alias to use for the item (like `bar` in `foo as bar`)
    pub alias: Option<StringId>,
}

impl Node for UseItem {
    const KIND: NodeType = NodeType::UseItem;
}

/// A WithClause is a single clause in a with Context declaration or definition.
#[derive(Debug, Clone, PartialEq)]
pub struct WithClause {
    /// The name of the declaration (the `T` in `T: Foo`).
    pub alias: Option<StringId>,
    /// The type of the declaration (the `Foo` in `T: Foo` or `!Foo`).
    pub right: NodeId<Expression>,
}

impl Node for WithClause {
    const KIND: NodeType = NodeType::WithClause;
}

/// A MatchSource is where the match was lowered from.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum MatchSource {
    /// Match expression (regular match with cases).
    Match,
    /// Explicit try expression or block (`try { ... }` with optional catch).
    Try,
    /// Maybe unary expression (postfix `?`).
    Maybe,
    /// Must unary expression (postfix `!`).
    Must,
}

/// A MatchCase is a match case inside a Match expression.
/// MatchCases can be any Pattern and can have an optional `if` guard.
#[derive(Debug, Clone, PartialEq)]
pub enum MatchCase {
    /// A match case with an expression body.
    Expression {
        pattern: NodeId<Pattern>,
        body: NodeId<Expression>,
        guard: Option<NodeId<Expression>>,
    },
    /// A match case with a block body.
    Block {
        pattern: NodeId<Pattern>,
        body: NodeId<Block>,
        guard: Option<NodeId<Expression>>,
    },
}

impl Node for MatchCase {
    const KIND: NodeType = NodeType::MatchCase;
}
