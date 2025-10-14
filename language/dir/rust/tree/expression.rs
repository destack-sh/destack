use dyst_ast::StringId;

use crate::{
    Argument, AssignOperator, BinaryOperator, Block, Definition, Destination, Node, NodeId,
    NodeType, Path, Pattern, Runtime, ScalarLiteral, ScopedMutability, Type, TypeLiteral,
    UnaryOperator, Visibility,
};

#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    /// Block of "statements" (inside `{}` usually)
    Block { block: NodeId<Block> },
    /// Definition as a value (with a name or anonymous)
    Definition { definition: NodeId<Definition> },

    /// With context declaration (flattened, like `with Foo, Bar` for `with Foo.Bar`)
    With {
        clauses: Vec<NodeId<WithClause>>,
        body: Option<NodeId<Block>>,
    },
    /// Import dependency declaration (flattened, like `import foo.bar` for `import foo.bar, baz.quz`)
    Import {
        visibility: Option<Visibility>,
        items: Vec<NodeId<ImportItem>>,
    },
    // NOTE #Incomplete: `export` modifier (and export expression?)
    /// Let or var binding for constant or mutable variables (without a value, i.e. not a condition).
    Let {
        mutability: ScopedMutability,
        visibility: Option<Visibility>,
        pattern: NodeId<Pattern>,
        ty: Option<NodeId<Type>>,
        value: Option<NodeId<Expression>>,
    },
    /// Type alias or expression to declare some value as a type.
    Type {
        name: Option<StringId>,
        visibility: Option<Visibility>,
        value: NodeId<Expression>,
    },

    /// Unary operation (except reference/dereference, e.g., `-x`).
    Unary {
        operator: UnaryOperator,
        right: NodeId<Expression>,
    },
    /// Reference operation (e.g., `&x`).
    Reference {
        mutability: ScopedMutability,
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
        runtime: Option<Runtime>,
        left: NodeId<Expression>,
        dynamic_arguments: Vec<NodeId<Argument>>,
    },
    /// Index into an array or slice.
    Index {
        left: NodeId<Expression>,
        right: Option<NodeId<Expression>>,
    },
    /// Maybe unwrap an expression with `?` and propagate.
    Maybe { left: NodeId<Expression> },
    /// Force unwrap an expression with `!` and propagate.
    Must { left: NodeId<Expression> },

    /// --------------------------------
    /// Values.
    /// --------------------------------

    /// Path.
    Path { path: Path },
    /// Scalar literal value.
    ScalarLiteral { value: ScalarLiteral },
    /// Type literal value.
    TypeLiteral { value: TypeLiteral },
    /// Range literal value.
    RangeLiteral {
        start: NodeId<Expression>,
        end: NodeId<Expression>,
        is_inclusive: bool,
    },
    /// Array creation.
    ArrayLiteral { elements: Vec<NodeId<Expression>> },
    /// Tuple creation.
    TupleLiteral {
        ty: Option<NodeId<Type>>,
        elements: Vec<NodeId<Argument>>,
    },
    /// Struct creation.
    StructLiteral {
        ty: Option<NodeId<Type>>,
        fields: Vec<NodeId<Argument>>,
    },
    /// Tree creation.
    TreeLiteral {
        path: Option<Path>,
        arguments: Option<Vec<NodeId<Argument>>>,
        elements: Option<Vec<NodeId<Argument>>>,
    },

    /// --------------------------------
    /// Control flow.
    /// --------------------------------

    /// If expression.
    If {
        runtime: Option<Runtime>,
        condition: NodeId<Expression>,
        then_expression: NodeId<Expression>,
        else_expression: Option<NodeId<Expression>>,
    },
    /// Loop expression.
    Loop {
        runtime: Option<Runtime>,
        condition: NodeId<Expression>,
        body: NodeId<Block>,
        source: LoopSource,
    },
    /// Match expression.
    Match {
        runtime: Option<Runtime>,
        value: NodeId<Expression>,
        cases: Vec<NodeId<MatchCase>>,
        source: MatchSource,
    },
    /// Break expression.
    Break {
        destination: Option<Destination>,
        value: Option<NodeId<Expression>>,
    },
    /// Continue expression.
    Continue { destination: Option<Destination> },
    /// Defer expression.
    Defer { expression: NodeId<Expression> },
    /// Return expression.
    Return { value: Option<NodeId<Expression>> },

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

/// A ImportItem is an item to use in a import clause.
#[derive(Debug, Clone, PartialEq)]
pub struct ImportItem {
    /// The source of the item.
    pub source: Path,
    /// The alias to use for the item.
    pub alias: Option<StringId>,
}

impl Node for ImportItem {
    const KIND: NodeType = NodeType::ImportItem;
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
