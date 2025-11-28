use dyst_ast::StringId;

use crate::{
    Argument, AssignOperator, Asynchrony, BinaryOperator, Block, Declaration,
    DeclarationDescriptor, DependencyItem, DependencyKind, GlobalSymbolId, LocalNodeId,
    LocalScopeId, LocalSymbolId, MatchCase, MatchSource, ModuleId, Mutability, Node, NodeType,
    Parameter, Path, Pattern, Property, ScalarLiteral, TemplateLiteral, TypeBinaryOperator,
    TypeKind, TypeLiteral, TypeUnaryOperator, UnaryOperator, VarianceBound,
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

    /// With context declaration (like `with Foo, Bar` for `with Foo.Bar`).
    With {
        clauses: Vec<LocalNodeId<WithClause>>,
        scope: LocalScopeId,
        symbol: LocalSymbolId,
        body: Option<LocalNodeId<Block>>,
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
        pattern: LocalNodeId<Pattern>,
        value: Option<LocalNodeId<Expression>>,
    },
    /// Type alias binding.
    LetType {
        descriptor: DeclarationDescriptor,
        kind: TypeKind,
        mutability: Option<Mutability>,
        static_parameters: Option<Vec<LocalNodeId<Parameter>>>,
        value: LocalNodeId<Expression>,
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

    /// Member access.
    UnresolvedMember {
        left: LocalNodeId<Expression>,
        name: StringId,
        static_arguments: Option<Vec<LocalNodeId<Argument>>>,
    },
    /// Member access.
    Member {
        left: LocalNodeId<Expression>,
        name: StringId,
        symbol: LocalSymbolId,
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

    /// Unresolved absolute path.
    UnresolvedAbsolutePath {
        path: Path,
        static_arguments: Option<Vec<LocalNodeId<Argument>>>,
    },
    /// Unresolved relative path to a remote symbol.
    UnresolvedRelativePath {
        path: Path,
        target_symbol: LocalSymbolId,
        remaining_path: Path,
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

    /// Type as a value.
    Type { symbol: LocalSymbolId },
    /// Scalar literal value.
    ScalarLiteral { value: ScalarLiteral },
    /// Template literal value.
    TemplateLiteral { value: TemplateLiteral },
    /// Tagged template literal value.
    TaggedTemplateLiteral {
        tag: LocalNodeId<Expression>,
        value: TemplateLiteral,
    },
    /// Type literal value.
    TypeLiteral { value: TypeLiteral },
    /// Range literal value.
    RangeLiteral {
        start: LocalNodeId<Expression>,
        end: LocalNodeId<Expression>,
        is_inclusive: bool,
    },
    /// Array creation.
    ArrayLiteral {
        elements: Vec<LocalNodeId<Argument>>,
    },
    /// Tuple creation.
    TupleLiteral {
        ty: Option<LocalNodeId<Expression>>,
        elements: Vec<LocalNodeId<Argument>>,
    },
    /// Struct creation.
    StructLiteral {
        ty: Option<LocalNodeId<Expression>>,
        properties: Vec<LocalNodeId<Property>>,
    },
    /// Tree creation.
    TreeLiteral {
        left: Option<LocalNodeId<Expression>>,
        arguments: Option<Vec<LocalNodeId<Argument>>>,
        elements: Option<Vec<LocalNodeId<Argument>>>,
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
        target: Option<StringId>,
        value: Option<LocalNodeId<Expression>>,
    },
    /// Break expression.
    Break {
        target: LocalScopeId,
        value: Option<LocalNodeId<Expression>>,
    },
    /// Continue expression.
    UnresolvedContinue { target: Option<StringId> },
    /// Continue expression.
    Continue { target: LocalScopeId },
    /// Defer expression.
    Defer { expression: LocalNodeId<Expression> },
    /// Throw expression.
    Throw {
        value: Option<LocalNodeId<Expression>>,
    },
    /// Await expression.
    Await { expression: LocalNodeId<Expression> },
    /// Yield expression.
    Yield {
        cardinality: YieldCardinality,
        value: LocalNodeId<Expression>,
    },
    /// Return expression.
    Return {
        value: Option<LocalNodeId<Expression>>,
    },

    /// Error expression.
    Error,
}

impl Node for Expression {
    const TYPE: NodeType = NodeType::Expression;

    fn is_resolved(&self) -> bool {
        true
    }
}

impl Expression {
    /// Get the name of this kind of expression.
    pub fn kind_name(&self) -> &'static str {
        match self {
            Expression::Declaration { .. } => "declaration",
            Expression::With { .. } => "with",
            Expression::UnresolvedImport { .. } => "unresolved import",
            Expression::UnresolvedReExport { .. } => "unresolved re-export",
            Expression::Import { .. } => "import",
            Expression::ReExport { .. } => "re-export",
            Expression::Export { .. } => "export",

            Expression::Block { .. } => "block",
            Expression::Statement { .. } => "statement",

            Expression::Let { .. } => "let",
            Expression::LetType { .. } => "let type",

            Expression::TypeUnary { .. } => "type unary",
            Expression::TypeBinary { .. } => "type binary",

            Expression::Unary { .. } => "unary",
            Expression::ValueOf { .. } => "value of",
            Expression::ReferenceOf { .. } => "reference of",
            Expression::Binary { .. } => "binary",
            Expression::Assign { .. } => "assign",
            Expression::AssignBinary { .. } => "assign binary",
            Expression::UnresolvedMember { .. } => "unresolved member",
            Expression::Member { .. } => "member",
            Expression::Call { .. } => "call",
            Expression::Index { .. } => "index",
            Expression::Maybe { .. } => "maybe",
            Expression::Must { .. } => "must",
            Expression::New { .. } => "new",
            Expression::Delete { .. } => "delete",

            Expression::UnresolvedAbsolutePath { .. } => "unresolved path",
            Expression::UnresolvedRelativePath { .. } => "unresolved relative path",
            Expression::LocalReference { .. } => "local reference",
            Expression::ModuleReference { .. } => "module reference",
            Expression::GlobalReference { .. } => "global reference",

            Expression::Type { .. } => "type",
            Expression::ScalarLiteral { .. } => "scalar literal",
            Expression::TemplateLiteral { .. } => "template literal",
            Expression::TaggedTemplateLiteral { .. } => "tagged template literal",
            Expression::TypeLiteral { .. } => "type literal",
            Expression::RangeLiteral { .. } => "range literal",
            Expression::ArrayLiteral { .. } => "array literal",
            Expression::TupleLiteral { .. } => "tuple literal",
            Expression::StructLiteral { .. } => "struct literal",
            Expression::TreeLiteral { .. } => "tree literal",
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
            Expression::Defer { .. } => "defer",
            Expression::Throw { .. } => "throw",
            Expression::Await { .. } => "await",
            Expression::Yield { .. } => "yield",
            Expression::Return { .. } => "return",

            Expression::Error => "error",
        }
    }

    /// Whether the expression is resolved (ignoring child nodes).
    pub fn is_resolved(&self) -> bool {
        !matches!(
            self,
            Expression::UnresolvedImport { .. }
                | Expression::UnresolvedReExport { .. }
                | Expression::UnresolvedMember { .. }
                | Expression::UnresolvedAbsolutePath { .. }
                | Expression::UnresolvedRelativePath { .. }
                | Expression::UnresolvedBreak { .. }
                | Expression::UnresolvedContinue { .. }
        )
    }

    /// Get the scope of the expression.
    pub fn scope(&self) -> Option<LocalScopeId> {
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
    pub fn symbol(&self) -> Option<LocalSymbolId> {
        match self {
            Expression::Let { descriptor, .. } => Some(descriptor.symbol),
            Expression::LetType { descriptor, .. } => Some(descriptor.symbol),
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

/// A WithClause is a single clause in a with Context declaration or declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct WithClause {
    /// The name of the declaration (the `T` in `T: Foo`).
    pub alias: Option<StringId>,
    /// The type of the declaration (the `Foo` in `T: Foo` or `!Foo`).
    pub right: LocalNodeId<Expression>,
}

impl Node for WithClause {
    const TYPE: NodeType = NodeType::WithClause;

    fn is_resolved(&self) -> bool {
        true
    }
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

    fn is_resolved(&self) -> bool {
        true
    }
}
