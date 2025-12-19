use destack_base::StringId;
use destack_source::ModuleId;

use crate::{
    Argument, AssignOperator, Asynchrony, BinaryOperator, Block, Declaration,
    DeclarationDescriptor, Declarator, DependencyItem, DependencyKind, GlobalSymbolId, LocalNodeId,
    LocalScopeId, LocalSymbolId, LocalTypeId, MatchCase, MatchSource, Mutability, Node, NodeType,
    Path, Pattern, Property, ScalarLiteral, StaticArgument, StaticProperty, TemplateLiteral,
    TypeBinaryOperator, TypeLiteral, TypeUnaryOperator, UnaryOperator, VarianceBound,
};

/// An Expression is a generic container for all constructs.
#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    /// Declaration as a value (with a name or anonymous).
    Declaration {
        declaration: LocalNodeId<Declaration>,
    },

    /// Block of "statements" (inside `{}` usually).
    Block { block: LocalNodeId<Block> },

    /// Statement expression (explicit statement with a `;` terminator).
    Statement { statement: LocalNodeId<Expression> },

    /// Labelled statement (like `label: stmt` in JavaScript).
    Labelled {
        label: StringId,
        body: LocalNodeId<Expression>,
        symbol: LocalSymbolId,
    },

    /// Unresolved import dependency declaration (like `import "foo"`).
    UnresolvedImport {
        kind: DependencyKind,
        target: StringId,
        items: Vec<LocalNodeId<DependencyItem>>,
        arguments: Option<Vec<LocalNodeId<Argument>>>,
    },
    /// Unresolved re-export dependency declaration (like `export { bar } from foo`).
    UnresolvedReExport {
        target: StringId,
        kind: DependencyKind,
        items: Vec<LocalNodeId<DependencyItem>>,
    },
    /// Import dependency (like `import "foo"` or `import { bar } from "foo"`).
    Import {
        kind: DependencyKind,
        target: StringId,
        target_module: ModuleId,
        items: Vec<LocalNodeId<DependencyItem>>,
        arguments: Option<Vec<LocalNodeId<Argument>>>,
    },
    /// Re-export dependency (like `export { bar } from "foo"` or `export * as foo from "foo"`).
    ReExport {
        target: StringId,
        target_module: ModuleId,
        kind: DependencyKind,
        items: Vec<LocalNodeId<DependencyItem>>,
    },
    /// Export dependency (like `export { bar }` or `export = foo`).
    Export {
        kind: DependencyKind,
        items: Vec<LocalNodeId<DependencyItem>>,
    },

    /// Let or var binding for constant or mutable variables (without a value, i.e. not a condition).
    Let {
        descriptor: DeclarationDescriptor,
        mutability: Mutability,
        declarators: Vec<LocalNodeId<Declarator>>,
    },

    /// Type unary operation.
    TypeUnary {
        operator: TypeUnaryOperator,
        right: LocalNodeId<Expression>,
    },
    /// Type binary operation.
    TypeBinary {
        left: LocalNodeId<Expression>,
        operator: TypeBinaryOperator,
        right: LocalNodeId<Expression>,
    },
    /// Unary operation (except reference/dereference, e.g., `-x`).
    Unary {
        operator: UnaryOperator,
        right: LocalNodeId<Expression>,
    },
    /// Value operation (e.g., `^x`).
    ValueOf {
        mutability: Option<Mutability>,
        variance: Option<VarianceBound>,
        right: LocalNodeId<Expression>,
    },
    /// Reference of operation (e.g., `&x`).
    ReferenceOf {
        mutability: Option<Mutability>,
        variance: Option<VarianceBound>,
        right: LocalNodeId<Expression>,
    },
    /// Binary operation.
    Binary {
        left: LocalNodeId<Expression>,
        operator: BinaryOperator,
        right: LocalNodeId<Expression>,
    },
    /// Assignment (e.g., `x = y`).
    Assign {
        left: LocalNodeId<Expression>,
        right: LocalNodeId<Expression>,
    },
    /// Assignment with operator (except direct assignment, e.g., `x += y`).
    AssignBinary {
        left: LocalNodeId<Expression>,
        operator: AssignOperator,
        right: LocalNodeId<Expression>,
    },

    /// Member access (like `a.foo` or `a.foo<T>`).
    Member {
        left: LocalNodeId<Expression>,
        name: StringId,
        static_arguments: Option<Vec<LocalNodeId<Argument>>>,
    },
    /// Call to a function.
    Call {
        left: LocalNodeId<Expression>,
        static_arguments: Option<Vec<LocalNodeId<Argument>>>,
        dynamic_arguments: Vec<LocalNodeId<Argument>>,
    },
    /// Index into an array or slice.
    Index {
        left: LocalNodeId<Expression>,
        right: Option<LocalNodeId<Expression>>,
    },
    /// Maybe unwrap an expression with `?` and propagate.
    Maybe { left: LocalNodeId<Expression> },
    /// Force unwrap an expression with `!` and propagate.
    Must { left: LocalNodeId<Expression> },
    /// New constructor call.
    New {
        left: LocalNodeId<Expression>,
        static_arguments: Option<Vec<LocalNodeId<Argument>>>,
        dynamic_arguments: Vec<LocalNodeId<Argument>>,
    },
    /// Delete expression.
    Delete { value: LocalNodeId<Expression> },

    /// --------------------------------
    /// Values.
    /// --------------------------------

    // TODO #Incomplete: support ImportMeta in DIR (as Expression variant? also get it from.. Module?)

    /// Unresolved path.
    UnresolvedPath {
        path: Path,
        static_arguments: Option<Vec<LocalNodeId<Argument>>>,
    },
    /// Local reference.
    LocalReference {
        path: Path,
        static_arguments: Option<Vec<LocalNodeId<Argument>>>,
        target_symbol: GlobalSymbolId,
    },
    /// Module reference.
    ModuleReference {
        path: Path,
        static_arguments: Option<Vec<LocalNodeId<Argument>>>,
        target_symbol: GlobalSymbolId,
    },
    /// Global reference.
    GlobalReference {
        path: Path,
        static_arguments: Option<Vec<LocalNodeId<Argument>>>,
        target_symbol: GlobalSymbolId,
    },

    /// Scalar literal value.
    ScalarLiteral { value: ScalarLiteral },
    /// Type literal value.
    TypeLiteral { value: TypeLiteral },

    /// Type as a value.
    Type { value: LocalTypeId },
    /// Template expression.
    TemplateExpression { value: TemplateLiteral },
    /// Tagged template expression.
    TaggedTemplateExpression {
        tag: LocalNodeId<Expression>,
        value: TemplateLiteral,
    },
    /// Range expression.
    RangeExpression {
        start: LocalNodeId<Expression>,
        end: LocalNodeId<Expression>,
        is_inclusive: bool,
    },
    /// Array expression (anonymous).
    ArrayExpression {
        elements: Vec<LocalNodeId<Argument>>,
    },
    /// Tuple expression (anonymous).
    TupleExpression {
        elements: Vec<LocalNodeId<Argument>>,
    },
    /// Sequence expression (JS/TS comma operator).
    SequenceExpression {
        expressions: Vec<LocalNodeId<Expression>>,
    },
    /// Object expression (anonymous).
    ObjectExpression {
        properties: Vec<LocalNodeId<Property>>,
    },
    /// Tree expression.
    TreeExpression {
        left: Option<LocalNodeId<Expression>>,
        arguments: Option<Vec<LocalNodeId<Argument>>>,
        elements: Option<Vec<LocalNodeId<Argument>>>,
    },
    /// Tagged scalar expression for newtype construction (e.g., `UserId(20)`).
    TaggedScalarExpression {
        ty: LocalNodeId<Expression>,
        value: LocalNodeId<Expression>,
    },
    /// Tagged tuple expression for newtype construction (e.g., `Point(1, 2)`).
    TaggedTupleExpression {
        ty: LocalNodeId<Expression>,
        elements: Vec<LocalNodeId<Argument>>,
    },
    /// Tagged object expression for nominal struct construction (e.g., `Vector3 { x: 1, y: 2 }`).
    TaggedObjectExpression {
        ty: LocalNodeId<Expression>,
        properties: Vec<LocalNodeId<Property>>,
    },
    /// Parenthesized expression.
    Parenthesized { expression: LocalNodeId<Expression> },

    /// --------------------------------
    /// Control analyze.
    /// --------------------------------

    /// If expression.
    If {
        kind: IfKind,
        condition: LocalNodeId<Expression>,
        then_expression: LocalNodeId<Expression>,
        else_expression: Option<LocalNodeId<Expression>>,
    },
    /// Loop expression.
    Loop {
        kind: LoopKind,
        condition: Option<LocalNodeId<Expression>>,
        body: LocalNodeId<Block>,
        scope: LocalScopeId,
        symbol: LocalSymbolId,
    },
    /// For each loop.
    ForEach {
        asynchrony: Asynchrony,
        kind: ForEachKind,
        pattern: LocalNodeId<Pattern>,
        iterator: LocalNodeId<Expression>,
        body: LocalNodeId<Block>,
        scope: LocalScopeId,
        symbol: LocalSymbolId,
    },
    /// For three-part loop.
    For {
        initialization: Option<LocalNodeId<Expression>>,
        condition: Option<LocalNodeId<Expression>>,
        increment: Option<LocalNodeId<Expression>>,
        body: LocalNodeId<Block>,
        scope: LocalScopeId,
        symbol: LocalSymbolId,
    },
    /// Try expression.
    Try {
        try_expression: LocalNodeId<Expression>,
        catch_pattern: Option<LocalNodeId<Pattern>>,
        catch_expression: Option<LocalNodeId<Expression>>,
        finally_expression: Option<LocalNodeId<Expression>>,
        scope: LocalScopeId,
        symbol: LocalSymbolId,
    },
    /// Match expression.
    Match {
        value: LocalNodeId<Expression>,
        cases: Vec<LocalNodeId<MatchCase>>,
        source: MatchSource,
        scope: LocalScopeId,
        symbol: LocalSymbolId,
    },
    /// Break expression.
    UnresolvedBreak {
        target: StringId,
        value: Option<LocalNodeId<Expression>>,
    },
    /// Break expression.
    Break {
        target: Option<StringId>,
        target_symbol: Option<GlobalSymbolId>,
        value: Option<LocalNodeId<Expression>>,
    },
    /// Continue expression.
    UnresolvedContinue { target: StringId },
    /// Continue expression.
    Continue {
        target: Option<StringId>,
        target_symbol: Option<GlobalSymbolId>,
    },
    /// Throw expression.
    Throw { value: LocalNodeId<Expression> },
    /// Await expression.
    Await { expression: LocalNodeId<Expression> },
    /// Yield expression.
    Yield {
        cardinality: YieldCardinality,
        value: Option<LocalNodeId<Expression>>,
    },
    /// Return expression.
    Return {
        value: Option<LocalNodeId<Expression>>,
    },

    /// Debugger statement.
    Debugger,

    /// Stub placeholder for annotation-only files.
    Stub,

    /// Error expression.
    Error,
}

impl Node for Expression {
    const TYPE: NodeType = NodeType::Expression;

    fn is_resolved(&self) -> bool {
        !matches!(
            self,
            Expression::UnresolvedImport { .. }
                | Expression::UnresolvedReExport { .. }
                | Expression::UnresolvedPath { .. }
                | Expression::UnresolvedBreak { .. }
                | Expression::UnresolvedContinue { .. }
        )
    }
}

impl Expression {
    /// Get the name of this kind of expression.
    pub fn kind_name(&self) -> &'static str {
        match self {
            Expression::Declaration { .. } => "declaration",
            Expression::UnresolvedImport { .. } => "unresolved import",
            Expression::UnresolvedReExport { .. } => "unresolved re-export",
            Expression::Import { .. } => "import",
            Expression::ReExport { .. } => "re-export",
            Expression::Export { .. } => "export",

            Expression::Block { .. } => "block",
            Expression::Statement { .. } => "statement",
            Expression::Labelled { .. } => "labelled",

            Expression::Let { .. } => "let",

            Expression::TypeUnary { .. } => "type unary",
            Expression::TypeBinary { .. } => "type binary",

            Expression::Unary { .. } => "unary",
            Expression::ValueOf { .. } => "value of",
            Expression::ReferenceOf { .. } => "reference of",
            Expression::Binary { .. } => "binary",
            Expression::Assign { .. } => "assign",
            Expression::AssignBinary { .. } => "assign binary",
            Expression::Member { .. } => "member",
            Expression::Call { .. } => "call",
            Expression::Index { .. } => "index",
            Expression::Maybe { .. } => "maybe",
            Expression::Must { .. } => "must",
            Expression::New { .. } => "new",
            Expression::Delete { .. } => "delete",

            Expression::UnresolvedPath { .. } => "unresolved path",
            Expression::LocalReference { .. } => "local reference",
            Expression::ModuleReference { .. } => "module reference",
            Expression::GlobalReference { .. } => "global reference",

            Expression::Type { .. } => "type",
            Expression::ScalarLiteral { .. } => "scalar literal",
            Expression::TemplateExpression { .. } => "template expression",
            Expression::TaggedTemplateExpression { .. } => "tagged template expression",
            Expression::TypeLiteral { .. } => "type literal",
            Expression::RangeExpression { .. } => "range expression",
            Expression::ArrayExpression { .. } => "array expression",
            Expression::TupleExpression { .. } => "tuple expression",
            Expression::SequenceExpression { .. } => "sequence expression",
            Expression::ObjectExpression { .. } => "object expression",
            Expression::TreeExpression { .. } => "tree expression",
            Expression::TaggedScalarExpression { .. } => "tagged scalar expression",
            Expression::TaggedTupleExpression { .. } => "tagged tuple expression",
            Expression::TaggedObjectExpression { .. } => "tagged object expression",
            Expression::Parenthesized { .. } => "parenthesized",

            Expression::If { .. } => "if",
            Expression::Loop { .. } => "loop",
            Expression::ForEach { .. } => "for each",
            Expression::For { .. } => "for",
            Expression::Try { .. } => "try",
            Expression::Match { .. } => "match",
            Expression::UnresolvedBreak { .. } => "unresolved break",
            Expression::Break { .. } => "break",
            Expression::UnresolvedContinue { .. } => "unresolved continue",
            Expression::Continue { .. } => "continue",
            Expression::Throw { .. } => "throw",
            Expression::Await { .. } => "await",
            Expression::Yield { .. } => "yield",
            Expression::Return { .. } => "return",

            Expression::Debugger => "debugger",

            Expression::Stub => "stub",
            Expression::Error => "error",
        }
    }

    /// Get the scope of the expression.
    pub fn scope(&self) -> Option<LocalScopeId> {
        match self {
            Expression::Loop { scope, .. } => Some(*scope),
            Expression::ForEach { scope, .. } => Some(*scope),
            Expression::For { scope, .. } => Some(*scope),
            Expression::Try { scope, .. } => Some(*scope),
            Expression::Match { scope, .. } => Some(*scope),
            _ => None,
        }
    }

    /// Get the symbol of the expression.
    pub fn symbol(&self) -> Option<LocalSymbolId> {
        match self {
            Expression::Let { descriptor, .. } => Some(descriptor.symbol),
            _ => None,
        }
    }

    /// Get the target symbol of the expression.
    pub fn target_symbol(&self) -> Option<GlobalSymbolId> {
        match self {
            Expression::LocalReference { target_symbol, .. } => Some(*target_symbol),
            Expression::ModuleReference { target_symbol, .. } => Some(*target_symbol),
            Expression::GlobalReference { target_symbol, .. } => Some(*target_symbol),
            _ => None,
        }
    }
}

/// Static value form of an expression in some static context.
/// Static evaluation supports all constructs, this is for the resulting static value.
/// This is a plain value type, not a tree node so we can pass it around freely.
#[derive(Debug, Clone, PartialEq)]
pub enum StaticExpression {
    /// Unevaluated expression (needs compile-time evaluation).
    Unevaluated { node: LocalNodeId<Expression> },

    /// Scalar literal.
    ScalarLiteral { value: ScalarLiteral },
    /// Type literal.
    TypeLiteral { value: TypeLiteral },

    /// Declaration reference with optional static arguments.
    Declaration {
        declaration: LocalNodeId<Declaration>,
        static_arguments: Option<Vec<StaticArgument>>,
    },
    /// Type.
    Type { ty: LocalTypeId },
    /// Range expression.
    RangeExpression {
        start: Box<StaticExpression>,
        end: Box<StaticExpression>,
        is_inclusive: bool,
    },
    /// Array expression.
    ArrayExpression { elements: Vec<StaticExpression> },
    /// Tuple expression.
    TupleExpression { elements: Vec<StaticExpression> },
    /// Object expression.
    ObjectExpression { properties: Vec<StaticProperty> },
}

impl StaticExpression {
    /// Check if the static expression and all its children have been evaluated.
    pub fn is_evaluated(&self) -> bool {
        match self {
            StaticExpression::Unevaluated { .. } => false,
            StaticExpression::ScalarLiteral { .. } => true,
            StaticExpression::TypeLiteral { .. } => true,
            StaticExpression::Type { .. } => true,
            StaticExpression::Declaration {
                static_arguments, ..
            } => static_arguments
                .as_ref()
                .map(|args| args.iter().all(StaticArgument::is_evaluated))
                .unwrap_or(true),
            StaticExpression::RangeExpression { start, end, .. } => {
                start.is_evaluated() && end.is_evaluated()
            }
            StaticExpression::ArrayExpression { elements } => {
                elements.iter().all(StaticExpression::is_evaluated)
            }
            StaticExpression::TupleExpression { elements } => {
                elements.iter().all(StaticExpression::is_evaluated)
            }
            StaticExpression::ObjectExpression { properties } => {
                properties.iter().all(StaticProperty::is_evaluated)
            }
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

/// A WhereClause is a single clause in a where type declaration.
#[derive(Debug, Clone, PartialEq)]
pub enum WhereClause {
    /// Where assertion (like `T: int32`).
    Assertion {
        /// The target to assert (like `T` in `T: int32`)
        left: StringId,
        /// The assertion type (like `int32` in `T: int32`)
        right: LocalNodeId<Expression>,
    },
    /// Where guard (like `T > Y`).
    Guard {
        /// The guard (like `T > Y` in `with T > Y`)
        guard: LocalNodeId<Expression>,
    },
}

impl Node for WhereClause {
    const TYPE: NodeType = NodeType::WhereClause;
}
