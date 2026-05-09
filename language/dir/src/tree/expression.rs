use destack_core::StringId;
use serde::{Deserialize, Serialize};

use crate::{
    Argument, AssignOperator, AssignPattern, Asynchrony, BinaryOperator, Block, CastOperator,
    CastOrigin, Declaration, Declarator, DependencyItem, DependencySpace, ExportKind,
    GenericArgument, ImportAttributeClause, LocalNodeId, LocalScopeId, LocalSymbolId, LocalTypeId,
    MatchCase, MatchForm, MatchOrigin, Mutability, Node, NodeType, Path, Pattern, Property,
    ScalarLiteral, StaticArgument, StaticProperty, TemplateLiteral, Tree, TypeExpression,
    TypeLiteral, UnaryOperator, VarianceBound,
};
use destack_source::{NodeSpanList, NodeSpanType};

/// An Expression is a generic container for all constructs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Expression {
    /// Declaration as a value (with a name or anonymous).
    Declaration(LocalNodeId<Declaration>),

    /// Block of "statements" (inside `{}` usually).
    Block(LocalNodeId<Block>),

    /// Labelled statement (like `label: stmt` in JavaScript).
    Labelled {
        label: StringId,
        body: LocalNodeId<Expression>,
        symbol: LocalSymbolId,
    },

    /// Import dependency declaration.
    Import {
        space: DependencySpace,
        target: StringId,
        items: Option<Vec<LocalNodeId<DependencyItem>>>,
        attributes: Option<ImportAttributeClause>,
    },
    /// Re-export dependency declaration.
    ReExport {
        target: StringId,
        space: DependencySpace,
        items: Vec<LocalNodeId<DependencyItem>>,
        attributes: Option<ImportAttributeClause>,
    },
    /// Export dependency.
    Export {
        space: DependencySpace,
        items: Vec<LocalNodeId<DependencyItem>>,
        attributes: Option<ImportAttributeClause>,
    },

    /// Let binding for mutable and immutable variables.
    Let {
        export: Option<ExportKind>,
        mutability: Mutability,
        declarators: Vec<LocalNodeId<Declarator>>,
        is_ambient: bool,
    },
    /// Let-else binding with an early-exit branch.
    /// The else branch is currently an explicit block.
    LetElse {
        kind: LetKind,
        mutability: Mutability,
        declarator: LocalNodeId<Declarator>,
        else_branch: LocalNodeId<Expression>,
    },
    /// Using binding for explicit resource management.
    Using {
        asynchrony: Asynchrony,
        export: Option<ExportKind>,
        declarators: Vec<LocalNodeId<Declarator>>,
        is_ambient: bool,
    },

    /// TypeScript-style `as` assertion.
    As {
        /// The resolved cast operator after elaborate.
        operator: Option<CastOperator>,
        /// Whether the cast was written in source or inserted during reify.
        source: CastOrigin,
        /// The source expression.
        expression: LocalNodeId<Expression>,
        /// The target type.
        target_type: LocalNodeId<TypeExpression>,
    },

    /// Check a value expression against a target type without changing its type.
    Satisfies {
        /// The source expression.
        expression: LocalNodeId<Expression>,
        /// The target type.
        target_type: LocalNodeId<TypeExpression>,
    },

    /// Runtime type guard.
    Is {
        value: LocalNodeId<Expression>,
        target_type: LocalNodeId<TypeExpression>,
    },

    /// Runtime constructor guard.
    InstanceOf {
        value: LocalNodeId<Expression>,
        target: LocalNodeId<Expression>,
    },

    /// Unary operation (e.g., `-x`, `!x`, `*x`).
    Unary {
        operator: UnaryOperator,
        right: LocalNodeId<Expression>,
    },
    /// Move operation (e.g., `^x`).
    MoveOf {
        mutability: Option<Mutability>,
        variance: Option<VarianceBound>,
        right: LocalNodeId<Expression>,
    },
    /// Borrow operation (e.g., `&x`).
    BorrowOf {
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
        left: LocalNodeId<AssignPattern>,
        right: LocalNodeId<Expression>,
    },
    /// Assignment with operator (except direct assignment, e.g., `x += y`).
    AssignBinary {
        left: LocalNodeId<Expression>,
        operator: AssignOperator,
        right: LocalNodeId<Expression>,
    },

    /// Member access (like `a.foo`).
    Member {
        left: LocalNodeId<Expression>,
        name: Option<StringId>,
    },
    /// Private member access (like `a.#foo`).
    PrivateMember {
        left: LocalNodeId<Expression>,
        name: Option<StringId>,
    },
    /// Call to a function.
    Call {
        left: LocalNodeId<Expression>,
        generic_arguments: Vec<LocalNodeId<GenericArgument>>,
        arguments: Vec<LocalNodeId<Argument>>,
    },
    /// Index into an array or slice.
    Index {
        left: LocalNodeId<Expression>,
        right: Option<LocalNodeId<Expression>>,
    },
    /// Instantiation expression (TypeScript).
    Instantiation {
        left: LocalNodeId<Expression>,
        generic_arguments: Vec<LocalNodeId<GenericArgument>>,
    },
    /// Maybe unwrap an expression with `?` and propagate.
    Maybe { left: LocalNodeId<Expression> },
    /// Force unwrap an expression with `!` and propagate.
    Must { left: LocalNodeId<Expression> },
    /// New constructor call.
    New {
        left: LocalNodeId<Expression>,
        generic_arguments: Vec<LocalNodeId<GenericArgument>>,
        arguments: Vec<LocalNodeId<Argument>>,
    },
    /// --------------------------------
    /// Values.
    /// --------------------------------

    /// Qualified value reference with optional generic arguments.
    Path {
        path: Path,
        generic_arguments: Vec<LocalNodeId<GenericArgument>>,
    },
    /// Private identifier.
    PrivateIdentifier { name: StringId },

    /// Import meta intrinsic value.
    ImportMeta,
    /// New target intrinsic value.
    NewTarget,
    /// This intrinsic value.
    This,
    /// Super intrinsic value.
    Super,

    /// Scalar literal value.
    ScalarLiteral { value: ScalarLiteral },
    /// Type literal value.
    TypeLiteral { value: TypeLiteral },

    /// Type as a value.
    Type { value: LocalNodeId<TypeExpression> },
    /// Template expression.
    TemplateExpression { value: TemplateLiteral },
    /// Tagged template expression.
    TaggedTemplateExpression {
        tag: LocalNodeId<Expression>,
        generic_arguments: Vec<LocalNodeId<GenericArgument>>,
        value: TemplateLiteral,
    },
    /// Array expression (anonymous).
    ArrayExpression {
        elements: Vec<LocalNodeId<Argument>>,
    },
    /// Fixed array repeat expression.
    FixedArrayExpression {
        value: LocalNodeId<Expression>,
        length: LocalNodeId<Expression>,
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
        ty: Option<LocalNodeId<TypeExpression>>,
        properties: Vec<LocalNodeId<Property>>,
    },
    /// Tree expression.
    TreeExpression {
        left: Option<LocalNodeId<Expression>>,
        generic_arguments: Vec<LocalNodeId<GenericArgument>>,
        arguments: Option<Vec<LocalNodeId<Argument>>>,
        elements: Option<Vec<LocalNodeId<Argument>>>,
    },
    /// Tagged scalar expression for newtype construction (e.g., `UserId(20)`).
    TaggedScalarExpression {
        ty: LocalNodeId<TypeExpression>,
        value: LocalNodeId<Expression>,
    },
    /// Tagged tuple expression for newtype construction (e.g., `Point(1, 2)`).
    TaggedTupleExpression {
        ty: LocalNodeId<TypeExpression>,
        elements: Vec<LocalNodeId<Argument>>,
    },
    /// Tagged object expression for nominal struct construction (e.g., `Vector3 { x: 1, y: 2 }`).
    TaggedObjectExpression {
        ty: LocalNodeId<TypeExpression>,
        properties: Vec<LocalNodeId<Property>>,
    },
    /// Parenthesized expression.
    Parenthesized { expression: LocalNodeId<Expression> },

    /// --------------------------------
    /// Control analyze.
    /// --------------------------------

    /// If expression.
    If {
        form: IfForm,
        condition: IfCondition,
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
        operator: ForEachOperator,
        binding: ForEachBinding,
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
        catch_ty: Option<LocalNodeId<TypeExpression>>,
        catch_expression: Option<LocalNodeId<Expression>>,
        finally_expression: Option<LocalNodeId<Expression>>,
        scope: LocalScopeId,
        symbol: LocalSymbolId,
    },
    /// Match expression.
    Match {
        form: MatchForm,
        value: LocalNodeId<Expression>,
        cases: Vec<LocalNodeId<MatchCase>>,
        source: MatchOrigin,
        scope: LocalScopeId,
        symbol: LocalSymbolId,
    },
    /// Break expression.
    Break {
        target: Option<StringId>,
        value: Option<LocalNodeId<Expression>>,
    },
    /// Continue expression.
    Continue { target: Option<StringId> },
    /// Throw expression.
    Throw { value: LocalNodeId<Expression> },
    /// Await expression.
    Await { expression: LocalNodeId<Expression> },
    /// Await with immediate error propagation (`await? expr`).
    /// Normalized to `Maybe { left: Await { expression } }` after binding.
    AwaitMaybe { expression: LocalNodeId<Expression> },
    /// Compile-time evaluated expression.
    Comptime { body: LocalNodeId<Expression> },
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

    /// Missing expression child.
    Missing,

    /// Stub placeholder.
    Stub,

    /// Error expression.
    Error,
}

impl Node for Expression {
    const TYPE: NodeType = NodeType::Expression;
}

impl Expression {
    /// Get the name of this kind of expression.
    pub fn kind_name(&self) -> &'static str {
        match self {
            Expression::Declaration(..) => "declaration",
            Expression::Import { .. } => "import",
            Expression::ReExport { .. } => "re-export",
            Expression::Export { .. } => "export",

            Expression::Block(..) => "block",
            Expression::Labelled { .. } => "labelled",

            Expression::Let { .. } => "let",
            Expression::LetElse { .. } => "let else",
            Expression::Using { .. } => "using",

            Expression::As { .. } => "as",
            Expression::Satisfies { .. } => "satisfies",
            Expression::Is { .. } => "is",
            Expression::InstanceOf { .. } => "instanceof",
            Expression::Unary { .. } => "unary",
            Expression::MoveOf { .. } => "move of",
            Expression::BorrowOf { .. } => "borrow of",
            Expression::Binary { .. } => "binary",
            Expression::Assign { .. } => "assign",
            Expression::AssignBinary { .. } => "assign binary",
            Expression::Member { .. } => "member",
            Expression::PrivateMember { .. } => "private member",
            Expression::Call { .. } => "call",
            Expression::Index { .. } => "index",
            Expression::Instantiation { .. } => "instantiation",
            Expression::Maybe { .. } => "maybe",
            Expression::Must { .. } => "must",
            Expression::New { .. } => "new",

            Expression::Path { .. } => "path",
            Expression::PrivateIdentifier { .. } => "private identifier",
            Expression::ImportMeta => "import meta",
            Expression::NewTarget => "new target",
            Expression::This => "this",
            Expression::Super => "super",

            Expression::Type { .. } => "type",
            Expression::ScalarLiteral { .. } => "scalar literal",
            Expression::TemplateExpression { .. } => "template expression",
            Expression::TaggedTemplateExpression { .. } => "tagged template expression",
            Expression::TypeLiteral { .. } => "type literal",
            Expression::ArrayExpression { .. } => "array expression",
            Expression::FixedArrayExpression { .. } => "fixed array expression",
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
            Expression::Break { .. } => "break",
            Expression::Continue { .. } => "continue",
            Expression::Throw { .. } => "throw",
            Expression::Await { .. } => "await",
            Expression::AwaitMaybe { .. } => "await?",
            Expression::Comptime { .. } => "comptime",
            Expression::Yield { .. } => "yield",
            Expression::Return { .. } => "return",

            Expression::Debugger => "debugger",

            Expression::Missing => "missing",
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
            Expression::Labelled { symbol, .. }
            | Expression::Loop { symbol, .. }
            | Expression::ForEach { symbol, .. }
            | Expression::For { symbol, .. }
            | Expression::Try { symbol, .. }
            | Expression::Match { symbol, .. } => Some(*symbol),
            _ => None,
        }
    }

    /// Get the generic arguments attached to the expression, when present.
    pub fn generic_arguments(&self) -> Option<&[LocalNodeId<GenericArgument>]> {
        match self {
            Expression::Path {
                generic_arguments, ..
            }
            | Expression::TaggedTemplateExpression {
                generic_arguments, ..
            }
            | Expression::TreeExpression {
                generic_arguments, ..
            }
            | Expression::Call {
                generic_arguments, ..
            }
            | Expression::New {
                generic_arguments, ..
            } => Some(generic_arguments.as_slice()),
            Expression::Instantiation {
                generic_arguments, ..
            } => Some(generic_arguments.as_slice()),
            _ => None,
        }
    }

    /// Resolve the source span kind that identifies this member name token.
    pub fn member_source_part(tree: &Tree, expression_id: LocalNodeId<Expression>) -> NodeSpanType {
        let source_id = tree.get_source(expression_id.id);
        let mut segment_index = 0u16;
        let mut current_id = expression_id;

        // walk left through one lowered member chain
        loop {
            let current_expression = tree.get::<Expression>(current_id);

            // count each synthetic member hop that still belongs to the same source node
            if let Expression::Member { left, .. } = current_expression
                && tree.get_source(left.id) == source_id
            {
                current_id = *left;
                segment_index = segment_index
                    .checked_add(1)
                    .expect("member source part segment index overflow");
                continue;
            }

            break;
        }

        // use indexed path segments for lowered qualified paths
        match tree.get::<Expression>(current_id) {
            Expression::Path { path, .. }
                if tree.get_source(current_id.id) == source_id
                    && usize::from(segment_index) < path.segments.len() =>
            {
                NodeSpanType::ListItem(NodeSpanList::Segment, segment_index)
            }
            _ => NodeSpanType::Main,
        }
    }
}

/// Static value form of an expression in some static context.
/// Static evaluation supports all constructs, this is for the resulting static value.
/// This is a plain value type, not a tree node so we can pass it around freely.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
        generic_arguments: Option<Vec<StaticArgument>>,
    },
    /// Type.
    Type { ty: LocalTypeId },
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
                generic_arguments, ..
            } => generic_arguments
                .as_ref()
                .map(|args| args.iter().all(StaticArgument::is_evaluated))
                .unwrap_or(true),
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
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum LoopKind {
    /// No-test loop (like `loop <body>`)
    NoTest,
    /// Pre-test loop (like `while <condition> <body>`)
    PreTest,
    /// Post-test loop (like `do <body> while <condition>`)
    PostTest,
}

/// The style of if expression.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum IfForm {
    /// If expression.
    If,
    /// If ternary expression.
    Ternary,
}

/// The kind of a let or const binding.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum LetKind {
    /// `let` binding.
    Let,
    /// `const` binding.
    Const,
}

/// The condition for an if expression.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum IfCondition {
    /// A regular condition expression.
    Expression { condition: LocalNodeId<Expression> },
    /// A let binding condition.
    Let {
        /// The keyword used for the let binding.
        kind: LetKind,
        /// The mutability derived from the binding keyword.
        mutability: Mutability,
        /// The declarator for the binding.
        declarator: LocalNodeId<Declarator>,
    },
}

/// The kind of a while expression.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum WhileForm {
    /// While expression.
    While,
    /// Do-while expression.
    DoWhile,
}

/// The kind of a for each expression.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ForEachOperator {
    /// In expression.
    In,
    /// Of expression.
    Of,
}

/// The declaration keyword used by a for each pattern binding.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum BindingKeyword {
    /// `let` declaration keyword.
    Let,
    /// `const` declaration keyword.
    Const,
}

/// The binding of a for each expression.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ForEachBinding {
    /// A normal pattern binding.
    Pattern {
        pattern: LocalNodeId<Pattern>,
        keyword: Option<BindingKeyword>,
    },
    /// A using binding.
    Using {
        asynchrony: Asynchrony,
        pattern: LocalNodeId<Pattern>,
    },
}

/// The kind of a yield expression.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum YieldCardinality {
    /// Generator yield expression.
    Generator,
    /// Scalar yield expression.
    Scalar,
}

/// A WhereClause is a single clause in a where type declaration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WhereClause {
    /// The target to constrain (like `T` in `T: int32`).
    pub left: StringId,
    /// The constraint type (like `int32` in `T: int32`).
    pub right: LocalNodeId<TypeExpression>,
}

impl Node for WhereClause {
    const TYPE: NodeType = NodeType::WhereClause;
}

/// The addressability of an expression.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Addressability {
    /// A place expression that refers to storage.
    Place,
    /// A value expression that does not refer to storage.
    Value,
}

impl Addressability {
    /// Return true when the expression is a place.
    pub fn is_place(self) -> bool {
        matches!(self, Addressability::Place)
    }
}
