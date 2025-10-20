use dyst_ast::StringId;

use crate::{
    Argument, AssignOperator, BinaryOperator, Block, Definition, Destination, ExportMode, ImportItem, MatchCase, MatchSource, Mutability, Node, NodeId, NodeType, Parameter, Path, Pattern, Runtime, ScalarLiteral, ScopedMutability, TemplateLiteral, Type, TypeLiteral, UnaryOperator, Visibility
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
    /// Import dependency declaration (flattened for grouped items like `import foo.{bar, baz}`)
    Import { items: Vec<NodeId<ImportItem>> },
    /// Export dependency declaration (flattened for grouped items like `export { bar } from foo`)
    Export {
        mode: ExportMode,
        items: Vec<NodeId<ImportItem>>,
    },
    /// Let or var binding for constant or mutable variables (without a value, i.e. not a condition).
    Let {
        mutability: ScopedMutability,
        visibility: Option<Visibility>,
        pattern: NodeId<Pattern>,
        ty: Option<NodeId<Type>>,
        value: Option<NodeId<Expression>>,
    },
    /// Type alias binding.
    LetType {
        mutability: Option<Mutability>,
        name: Option<StringId>,
        static_parameters: Option<Vec<NodeId<Parameter>>>,
        visibility: Option<Visibility>,
        value: NodeId<Expression>,
    },
    /// Type expression.
    Type {
        mutability: Option<Mutability>,
        value: NodeId<Expression>,
    },

    /// Unary operation (except reference/dereference, e.g., `-x`).
    Unary {
        operator: UnaryOperator,
        expression: NodeId<Expression>,
    },
    /// Reference operation (e.g., `&x`).
    Reference {
        mutability: Option<ScopedMutability>,
        right: NodeId<Expression>,
    },
    /// Dynamic operation (e.g., `$x`).
    Dynamic {
        mutability: Option<ScopedMutability>,
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
    /// Template literal value.
    TemplateLiteral { value: TemplateLiteral },
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

impl Expression {
    /// Whether the expression is resolved (ignoring child nodes).
    /// Whether the expression is considered evaluated at the outermost level (ignoring child nodes).
    pub fn is_resolved(&self) -> bool {
        match self {
            // values
            Expression::Path { path } => path.is_resolved(),
            Expression::ScalarLiteral { .. }
            | Expression::TemplateLiteral { .. }
            | Expression::TypeLiteral { .. }
            | Expression::RangeLiteral { .. }
            | Expression::ArrayLiteral { .. }
            | Expression::TupleLiteral { .. }
            | Expression::StructLiteral { .. }
            | Expression::TreeLiteral { .. } => true,

            // control flow
            Expression::If { .. }
            | Expression::Loop { .. }
            | Expression::Match { .. }
            | Expression::Break { .. }
            | Expression::Continue { .. }
            | Expression::Defer { .. }
            | Expression::Return { .. } => false,

            // definitions and declarations
            Expression::Definition { .. }
            | Expression::With { .. }
            | Expression::Import { .. }
            | Expression::Export { .. }
            | Expression::Let { .. }
            | Expression::Type { .. }
            | Expression::LetType { .. } => true,

            // operators
            Expression::Block { .. }
            | Expression::Unary { .. }
            | Expression::Reference { .. }
            | Expression::Dynamic { .. }
            | Expression::Binary { .. }
            | Expression::AssignDirect { .. }
            | Expression::AssignBinary { .. }
            | Expression::Member { .. }
            | Expression::Call { .. }
            | Expression::Index { .. }
            | Expression::Maybe { .. }
            | Expression::Must { .. } => false,

            // error
            Expression::Error => false,
        }
    }
}

/// A LoopSource is where the loop was lowered from.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum LoopSource {
    /// For loop.
    ForThree,
    /// For loop.
    ForEach,
    /// While loop.
    While,
    /// Loop loop.
    Loop,
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
