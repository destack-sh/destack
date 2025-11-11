use dyst_ast::StringId;

use crate::{
    Argument, AssignOperator, Asynchrony, BinaryOperator, Block, BlockTarget, Definition,
    DependencyItem, DependencyKind, ExportType, MatchCase, MatchFile, Mutability, Node, NodeId,
    NodeType, Parameter, Path, Pattern, ScalarLiteral, ScopedMutability, TemplateLiteral, Type,
    TypeBinaryOperator, TypeLiteral, TypeUnaryOperator, UnaryOperator, VarianceBound,
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
    Import {
        kind: DependencyKind,
        asynchrony: Asynchrony,
        items: Vec<NodeId<DependencyItem>>,
        arguments: Option<Vec<NodeId<Argument>>>,
    },
    /// Export dependency declaration (flattened for items like `export { bar } from foo`)
    Export {
        mode: ExportType,
        kind: DependencyKind,
        items: Option<Vec<NodeId<DependencyItem>>>,
        value: Option<NodeId<Expression>>,
    },
    /// Let or var binding for constant or mutable variables (without a value, i.e. not a condition).
    Let {
        mutability: ScopedMutability,
        pattern: NodeId<Pattern>,
        ty: Option<NodeId<Type>>,
        value: Option<NodeId<Expression>>,
    },
    /// Type alias binding.
    LetType {
        mutability: Option<Mutability>,
        name: StringId,
        static_parameters: Option<Vec<NodeId<Parameter>>>,
        value: NodeId<Expression>,
    },

    /// Type unary operation.
    TypeUnary {
        operator: TypeUnaryOperator,
        right: NodeId<Expression>,
    },
    /// Type binary operation.
    TypeBinary {
        left: NodeId<Expression>,
        operator: TypeBinaryOperator,
        right: NodeId<Expression>,
    },
    /// Unary operation (except reference/dereference, e.g., `-x`).
    Unary {
        operator: UnaryOperator,
        right: NodeId<Expression>,
    },
    /// Value operation (e.g., `^x`).
    ValueOf {
        mutability: Option<ScopedMutability>,
        variance: Option<VarianceBound>,
        right: NodeId<Expression>,
    },
    /// Reference of operation (e.g., `&x`).
    ReferenceOf {
        mutability: Option<ScopedMutability>,
        variance: Option<VarianceBound>,
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
        static_arguments: Option<Vec<NodeId<Argument>>>,
    },
    /// Call to a function.
    Call {
        left: NodeId<Expression>,
        dynamic_arguments: Vec<NodeId<Argument>>,
    },
    /// New constructor call (for #Compatibility).
    New {
        left: Path,
        static_arguments: Option<Vec<NodeId<Argument>>>,
        dynamic_arguments: Vec<NodeId<Argument>>,
    },
    /// Delete expression (for #Compatibility).
    Delete { value: NodeId<Expression> },
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
    Path {
        path: Path,
        static_arguments: Option<Vec<NodeId<Argument>>>,
    },
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
    ArrayLiteral { elements: Vec<NodeId<Argument>> },
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
        kind: IfKind,
        condition: NodeId<Expression>,
        then_expression: NodeId<Expression>,
        else_expression: Option<NodeId<Expression>>,
    },
    /// Loop expression.
    Loop {
        condition: NodeId<Expression>,
        body: NodeId<Block>,
        source: LoopFile,
    },
    /// For each loop.
    ForEach {
        asynchrony: Asynchrony,
        kind: ForEachKind,
        pattern: NodeId<Pattern>,
        iterator: NodeId<Expression>,
        body: NodeId<Block>,
    },
    /// For three-part loop.
    For {
        initialization: Option<NodeId<Expression>>,
        condition: Option<NodeId<Expression>>,
        increment: Option<NodeId<Expression>>,
        body: NodeId<Block>,
    },
    /// Match expression.
    Match {
        value: NodeId<Expression>,
        cases: Vec<NodeId<MatchCase>>,
        source: MatchFile,
    },
    /// Break expression.
    Break {
        target: Option<BlockTarget>,
        value: Option<NodeId<Expression>>,
    },
    /// Continue expression.
    Continue { target: Option<BlockTarget> },
    /// Defer expression.
    Defer { expression: NodeId<Expression> },
    /// Throw expression (for #Compatibility).
    Throw { value: Option<NodeId<Expression>> },
    /// Return expression.
    Return { value: Option<NodeId<Expression>> },

    /// Error expression.
    Error,
}

impl Node for Expression {
    const TYPE: NodeType = NodeType::Expression;
}

impl Expression {
    // nocheckin TODO #Broken: revisit Compiler is_evaluated/evaluate logic (after load, ...)
    /// Whether the expression is considered evaluated at the outermost level (ignoring child nodes).
    pub fn is_evaluated(&self) -> bool {
        match self {
            // values
            Expression::Path { path, .. } => path.is_evaluated(),
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
            | Expression::For { .. }
            | Expression::ForEach { .. }
            | Expression::Match { .. }
            | Expression::Break { .. }
            | Expression::Continue { .. }
            | Expression::Defer { .. }
            | Expression::Throw { .. }
            | Expression::Return { .. } => false,

            // definitions and declarations
            Expression::Definition { .. }
            | Expression::With { .. }
            | Expression::Import { .. }
            | Expression::Export { .. }
            | Expression::Let { .. }
            | Expression::LetType { .. } => true,

            // operators
            Expression::Block { .. }
            | Expression::Unary { .. }
            | Expression::TypeBinary { .. }
            | Expression::ReferenceOf { .. }
            | Expression::ValueOf { .. }
            | Expression::Binary { .. }
            | Expression::TypeUnary { .. }
            | Expression::AssignDirect { .. }
            | Expression::AssignBinary { .. }
            | Expression::Member { .. }
            | Expression::Call { .. }
            | Expression::Index { .. }
            | Expression::Maybe { .. }
            | Expression::Must { .. }
            | Expression::New { .. }
            | Expression::Delete { .. } => false,

            // error
            Expression::Error => false,
        }
    }
}

/// A LoopFile is where the loop was lowered from.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum LoopFile {
    /// For loop.
    For,
    /// For loop.
    ForEach,
    /// While loop.
    While,
    /// Loop loop.
    Loop,
}

/// The style of if expression.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IfKind {
    /// If expression.
    If,
    /// If ternary expression.
    Ternary,
}

/// The kind of a while expression.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WhileKind {
    /// While expression.
    While,
    /// Do-while expression.
    DoWhile,
}

/// The kind of a for each expression.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ForEachKind {
    /// In expression.
    In,
    /// Of expression.
    Of,
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
    const TYPE: NodeType = NodeType::WithClause;
}
