use dyst_ast::StringId;

use crate::{
    Argument, AssignOperator, Asynchrony, BinaryOperator, Block, Definition, DependencyItem,
    DependencyKind, ExportType, MatchCase, MatchSource, ModuleId, Mutability, Node, NodeId,
    NodeType, Parameter, Path, Pattern, Property, ScalarLiteral, ScopeId, SymbolId,
    TemplateLiteral, Type, TypeBinaryOperator, TypeKind, TypeLiteral, TypeUnaryOperator,
    UnaryOperator, VarianceBound,
};

/// An Expression is a generic container for all constructs.
#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    /// Definition as a value (with a name or anonymous).
    Definition { definition: NodeId<Definition> },

    /// Block of "statements" (inside `{}` usually).
    Block { block: NodeId<Block> },

    /// Statement expression (explicit statement with a `;` terminator).
    Statement { statement: NodeId<Expression> },

    /// With context declaration (like `with Foo, Bar` for `with Foo.Bar`).
    With {
        clauses: Vec<NodeId<WithClause>>,
        scope: ScopeId,
        body: Option<NodeId<Block>>,
    },
    /// Unresolved import dependency declaration (like `import "foo"`).
    UnresolvedImport {
        kind: DependencyKind,
        target: StringId,
        items: Vec<NodeId<DependencyItem>>,
        arguments: Option<Vec<NodeId<Argument>>>,
    },
    /// Unresolved re-export dependency declaration (like `export { bar } from foo`).
    UnresolvedReExport {
        mode: ExportType,
        target: StringId,
        kind: DependencyKind,
        items: Vec<NodeId<DependencyItem>>,
    },
    /// Import dependency (like `import "foo"` or `import { bar } from "foo"`).
    Import {
        kind: DependencyKind,
        target: StringId,
        module: ModuleId,
        items: Vec<NodeId<DependencyItem>>,
        arguments: Option<Vec<NodeId<Argument>>>,
    },
    /// Re-export dependency (like `export { bar } from "foo"` or `export * as foo from "foo"`).
    ReExport {
        mode: ExportType,
        target: StringId,
        module: ModuleId,
        kind: DependencyKind,
        items: Vec<NodeId<DependencyItem>>,
    },
    /// Export dependency (like `export { bar }` or `export = foo`).
    Export {
        mode: ExportType,
        kind: DependencyKind,
        items: Vec<NodeId<DependencyItem>>,
    },

    /// Let or var binding for constant or mutable variables (without a value, i.e. not a condition).
    Let {
        mutability: Mutability,
        pattern: NodeId<Pattern>,
        ty: Option<NodeId<Type>>,
        value: Option<NodeId<Expression>>,
        symbol: SymbolId,
    },
    /// Type alias binding.
    LetType {
        kind: TypeKind,
        mutability: Option<Mutability>,
        name: StringId,
        static_parameters: Option<Vec<NodeId<Parameter>>>,
        value: NodeId<Expression>,
        symbol: SymbolId,
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
    /// Unresolved unary operation (may be operator-overloaded).
    UnresolvedUnary {
        operator: UnaryOperator,
        right: NodeId<Expression>,
    },
    /// Unary operation (except reference/dereference, e.g., `-x`).
    Unary {
        operator: UnaryOperator,
        right: NodeId<Expression>,
    },
    /// Value operation (e.g., `^x`).
    ValueOf {
        mutability: Option<Mutability>,
        variance: Option<VarianceBound>,
        right: NodeId<Expression>,
    },
    /// Reference of operation (e.g., `&x`).
    ReferenceOf {
        mutability: Option<Mutability>,
        variance: Option<VarianceBound>,
        right: NodeId<Expression>,
    },
    /// Unresolved binary operation (may be operator-overloaded).
    UnresolvedBinary {
        left: NodeId<Expression>,
        operator: BinaryOperator,
        right: NodeId<Expression>,
    },
    /// Binary operation.
    Binary {
        left: NodeId<Expression>,
        operator: BinaryOperator,
        right: NodeId<Expression>,
    },
    /// Assignment (e.g., `x = y`).
    Assign {
        left: NodeId<Expression>,
        right: NodeId<Expression>,
    },
    /// Unresolved binary assignment with operator (may be operator-overloaded).
    UnresolvedAssignBinary {
        left: NodeId<Expression>,
        operator: AssignOperator,
        right: NodeId<Expression>,
    },
    /// Assignment with operator (except direct assignment, e.g., `x += y`).
    AssignBinary {
        left: NodeId<Expression>,
        operator: AssignOperator,
        right: NodeId<Expression>,
    },

    /// Member access.
    UnresolvedMember {
        left: NodeId<Expression>,
        name: StringId,
        static_arguments: Option<Vec<NodeId<Argument>>>,
    },
    /// Member access.
    Member {
        left: NodeId<Expression>,
        name: StringId,
        symbol: SymbolId,
        static_arguments: Option<Vec<NodeId<Argument>>>,
    },
    /// Call to a function.
    Call {
        left: NodeId<Expression>,
        static_arguments: Option<Vec<NodeId<Argument>>>,
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
    /// New constructor call.
    New {
        left: NodeId<Expression>,
        static_arguments: Option<Vec<NodeId<Argument>>>,
        dynamic_arguments: Vec<NodeId<Argument>>,
    },
    /// Delete expression.
    Delete { value: NodeId<Expression> },

    /// --------------------------------
    /// Values.
    /// --------------------------------

    /// Unresolved path.
    UnresolvedPath {
        path: Path,
        static_arguments: Option<Vec<NodeId<Argument>>>,
    },
    /// Scalar literal value.
    ScalarLiteral { value: ScalarLiteral },
    /// Template literal value.
    TemplateLiteral { value: TemplateLiteral },
    /// Tagged template literal value.
    TaggedTemplateLiteral {
        tag: NodeId<Expression>,
        value: TemplateLiteral,
    },
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
        properties: Vec<NodeId<Property>>,
    },
    /// Tree creation.
    TreeLiteral {
        left: Option<NodeId<Expression>>,
        arguments: Option<Vec<NodeId<Argument>>>,
        elements: Option<Vec<NodeId<Argument>>>,
    },
    /// Parenthesized expression.
    Parenthesized { expression: NodeId<Expression> },

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
        kind: LoopKind,
        condition: Option<NodeId<Expression>>,
        body: NodeId<Block>,
        scope: ScopeId,
    },
    /// For each loop.
    ForEach {
        asynchrony: Asynchrony,
        kind: ForEachKind,
        pattern: NodeId<Pattern>,
        iterator: NodeId<Expression>,
        body: NodeId<Block>,
        scope: ScopeId,
    },
    /// For three-part loop.
    For {
        initialization: Option<NodeId<Expression>>,
        condition: Option<NodeId<Expression>>,
        increment: Option<NodeId<Expression>>,
        body: NodeId<Block>,
        scope: ScopeId,
    },
    /// Try expression.
    Try {
        try_expression: NodeId<Expression>,
        catch_pattern: Option<NodeId<Pattern>>,
        catch_expression: Option<NodeId<Expression>>,
        finally_expression: Option<NodeId<Expression>>,
        scope: ScopeId,
    },
    /// Match expression.
    Match {
        value: NodeId<Expression>,
        cases: Vec<NodeId<MatchCase>>,
        source: MatchSource,
        scope: ScopeId,
    },
    /// Break expression.
    UnresolvedBreak {
        target: Option<StringId>,
        value: Option<NodeId<Expression>>,
    },
    /// Break expression.
    Break {
        target: ScopeId,
        value: Option<NodeId<Expression>>,
    },
    /// Continue expression.
    UnresolvedContinue { target: Option<StringId> },
    /// Continue expression.
    Continue { target: ScopeId },
    /// Defer expression.
    Defer { expression: NodeId<Expression> },
    /// Throw expression.
    Throw { value: Option<NodeId<Expression>> },
    /// Await expression.
    Await { expression: NodeId<Expression> },
    /// Yield expression.
    Yield {
        cardinality: YieldCardinality,
        value: NodeId<Expression>,
    },
    /// Return expression.
    Return { value: Option<NodeId<Expression>> },

    /// Error expression.
    Error,
}

impl Node for Expression {
    const TYPE: NodeType = NodeType::Expression;
}

impl Expression {
    /// Whether the expression is resolved (ignoring child nodes).
    pub fn is_resolved(&self) -> bool {
        !matches!(
            self,
            Expression::UnresolvedImport { .. }
                | Expression::UnresolvedReExport { .. }
                | Expression::UnresolvedMember { .. }
                | Expression::UnresolvedPath { .. }
                | Expression::UnresolvedUnary { .. }
                | Expression::UnresolvedBinary { .. }
                | Expression::UnresolvedAssignBinary { .. }
                | Expression::UnresolvedBreak { .. }
                | Expression::UnresolvedContinue { .. }
        )
    }

    /// Get the scope of the expression.
    pub fn scope(&self) -> Option<ScopeId> {
        match self {
            Expression::With { scope, .. } => Some(*scope),
            Expression::Loop { scope, .. } => Some(*scope),
            Expression::ForEach { scope, .. } => Some(*scope),
            Expression::For { scope, .. } => Some(*scope),
            Expression::Try { scope, .. } => Some(*scope),
            Expression::Match { scope, .. } => Some(*scope),
            _ => None,
        }
    }

    /// Get the symbol of the expression.
    pub fn symbol(&self) -> Option<SymbolId> {
        match self {
            Expression::Let { symbol, .. } => Some(*symbol),
            Expression::LetType { symbol, .. } => Some(*symbol),
            _ => None,
        }
    }
}

/// The kind of a loop expression.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LoopKind {
    /// No-test loop (like `loop <body>`)
    NoTest,
    /// Pre-test loop (like `while <condition> <body>`)
    PreTest,
    /// Post-test loop (like `do <body> while <condition>`)
    PostTest,
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

/// The kind of a yield expression.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum YieldCardinality {
    /// Generator yield expression.
    Generator,
    /// Scalar yield expression.
    Scalar,
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
