use destack_core::StringId;
use destack_source::AdaptImage;
use serde::{Deserialize, Serialize};

use crate::{
    Ambientness, Argument, AssignOperator, Asynchrony, BinaryOperator, Block, Declaration,
    Declarator, DependencyItem, DependencyKind, ExportMode, GenericArgument, GlobalSymbolId,
    ImportAttributeClause, ImportSource, ImportTarget, LocalNodeId, LocalScopeId, LocalSymbolId,
    LocalTypeId, MatchCase, MatchKind, MatchSource, ModuleTarget, Mutability, Node, NodeTree,
    NodeType, OwnershipCastOperator, OwnershipCastSource, Path, Pattern, Property, ScalarLiteral,
    StaticArgument, StaticProperty, SymbolSpaceOrder, TemplateLiteral, TypeExpression, TypeLiteral,
    UnaryOperator, VarianceBound,
};
use destack_source::NodeSpanType;

/// An Expression is a generic container for all constructs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, AdaptImage)]
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

    /// Unresolved import dependency declaration (like `import "foo"`).
    UnresolvedImport {
        source: ImportSource,
        kind: DependencyKind,
        target: ImportTarget,
        items: Option<Vec<LocalNodeId<DependencyItem>>>,
        attributes: Option<ImportAttributeClause>,
        arguments: Option<Vec<LocalNodeId<Argument>>>,
    },
    /// Unresolved re-export dependency declaration (like `export { bar } from foo`).
    UnresolvedReExport {
        target: StringId,
        kind: DependencyKind,
        items: Vec<LocalNodeId<DependencyItem>>,
        attributes: Option<ImportAttributeClause>,
    },
    /// Import dependency (like `import "foo"` or `import { bar } from "foo"`).
    Import {
        source: ImportSource,
        kind: DependencyKind,
        target: StringId,
        target_module: ModuleTarget,
        items: Option<Vec<LocalNodeId<DependencyItem>>>,
        attributes: Option<ImportAttributeClause>,
        arguments: Option<Vec<LocalNodeId<Argument>>>,
    },
    /// Re-export dependency (like `export { bar } from "foo"` or `export * as foo from "foo"`).
    ReExport {
        target: StringId,
        target_module: ModuleTarget,
        kind: DependencyKind,
        items: Vec<LocalNodeId<DependencyItem>>,
        attributes: Option<ImportAttributeClause>,
    },
    /// Export dependency (like `export { bar }` or `export = foo`).
    Export {
        kind: DependencyKind,
        items: Vec<LocalNodeId<DependencyItem>>,
        attributes: Option<ImportAttributeClause>,
    },
    /// Export the module namespace as a global name (declaration files only).
    ExportNamespace { name: StringId },

    /// Let or var binding for constant or mutable variables (without a value, i.e. not a condition).
    Let {
        export: Option<ExportMode>,
        ambient: Ambientness,
        mutability: Mutability,
        declarators: Vec<LocalNodeId<Declarator>>,
    },
    /// Using binding for explicit resource management.
    Using {
        asynchrony: Asynchrony,
        export: Option<ExportMode>,
        ambient: Ambientness,
        declarators: Vec<LocalNodeId<Declarator>>,
    },

    /// TypeScript-style `as` assertion.
    As {
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

    /// Cast a value expression to a target ownership form.
    OwnershipCast {
        /// The ownership cast operator to apply.
        operator: OwnershipCastOperator,
        /// The origin of the ownership cast in source.
        source: OwnershipCastSource,
        /// The value to cast.
        value: LocalNodeId<Expression>,
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
    /// Pointer type operation (e.g., `*T`).
    PointerOf {
        mutability: Option<Mutability>,
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
        name: Option<StringId>,
        generic_arguments: Vec<LocalNodeId<GenericArgument>>,
    },
    /// Private member access (like `a.#foo` or `a.#foo<T>`).
    PrivateMember {
        left: LocalNodeId<Expression>,
        name: Option<StringId>,
        generic_arguments: Vec<LocalNodeId<GenericArgument>>,
    },
    /// Call to a function.
    Call {
        left: LocalNodeId<Expression>,
        generic_arguments: Vec<LocalNodeId<GenericArgument>>,
        dynamic_arguments: Vec<LocalNodeId<Argument>>,
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
        dynamic_arguments: Vec<LocalNodeId<Argument>>,
    },
    /// Delete expression.
    Delete { value: LocalNodeId<Expression> },

    /// --------------------------------
    /// Values.
    /// --------------------------------

    /// Unresolved path.
    UnresolvedPath {
        path: Path,
        generic_arguments: Vec<LocalNodeId<GenericArgument>>,
        space_order: SymbolSpaceOrder,
    },
    /// Local reference.
    LocalReference {
        path: Path,
        generic_arguments: Vec<LocalNodeId<GenericArgument>>,
        target_symbol: GlobalSymbolId,
    },
    /// Module reference.
    ModuleReference {
        path: Path,
        generic_arguments: Vec<LocalNodeId<GenericArgument>>,
        target_symbol: GlobalSymbolId,
    },
    /// Global reference.
    GlobalReference {
        path: Path,
        generic_arguments: Vec<LocalNodeId<GenericArgument>>,
        target_symbol: GlobalSymbolId,
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
    Type {
        value: LocalNodeId<TypeExpression>,
        resolved_type: LocalTypeId,
    },
    /// Template expression.
    TemplateExpression { value: TemplateLiteral },
    /// Tagged template expression.
    TaggedTemplateExpression {
        tag: LocalNodeId<Expression>,
        value: TemplateLiteral,
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
        ty: Option<LocalNodeId<TypeExpression>>,
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
        kind: IfKind,
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
        kind: ForEachKind,
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
        kind: MatchKind,
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
    /// Await with immediate error propagation (`await? expr`).
    /// Desugared to `Maybe { left: Await { expression } }` after binding.
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
            Expression::Declaration(..) => "declaration",
            Expression::UnresolvedImport { .. } => "unresolved import",
            Expression::UnresolvedReExport { .. } => "unresolved re-export",
            Expression::Import { .. } => "import",
            Expression::ReExport { .. } => "re-export",
            Expression::Export { .. } => "export",
            Expression::ExportNamespace { .. } => "export namespace",

            Expression::Block(..) => "block",
            Expression::Labelled { .. } => "labelled",

            Expression::Let { .. } => "let",
            Expression::Using { .. } => "using",

            Expression::As { .. } => "as",
            Expression::Satisfies { .. } => "satisfies",
            Expression::Is { .. } => "is",
            Expression::InstanceOf { .. } => "instanceof",
            Expression::OwnershipCast { .. } => "ownership cast",
            Expression::Unary { .. } => "unary",
            Expression::ValueOf { .. } => "value of",
            Expression::ReferenceOf { .. } => "reference of",
            Expression::PointerOf { .. } => "pointer of",
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
            Expression::Delete { .. } => "delete",

            Expression::UnresolvedPath { .. } => "unresolved path",
            Expression::LocalReference { .. } => "local reference",
            Expression::ModuleReference { .. } => "module reference",
            Expression::GlobalReference { .. } => "global reference",
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
            Expression::Let { .. } => None,
            Expression::Using { .. } => None,
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

    /// Get the generic arguments attached to the expression, when present.
    pub fn generic_arguments(&self) -> Option<&[LocalNodeId<GenericArgument>]> {
        match self {
            Expression::UnresolvedPath {
                generic_arguments, ..
            }
            | Expression::LocalReference {
                generic_arguments, ..
            }
            | Expression::ModuleReference {
                generic_arguments, ..
            }
            | Expression::GlobalReference {
                generic_arguments, ..
            }
            | Expression::Member {
                generic_arguments, ..
            }
            | Expression::PrivateMember {
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
    pub fn member_source_part(
        tree: &NodeTree,
        expression_id: LocalNodeId<Expression>,
    ) -> NodeSpanType {
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
            Expression::LocalReference { path, .. }
            | Expression::ModuleReference { path, .. }
            | Expression::GlobalReference { path, .. }
                if tree.get_source(current_id.id) == source_id
                    && usize::from(segment_index) < path.segments.len() =>
            {
                NodeSpanType::Segment(segment_index)
            }
            _ => NodeSpanType::Main,
        }
    }
}

/// Static value form of an expression in some static context.
/// Static evaluation supports all constructs, this is for the resulting static value.
/// This is a plain value type, not a tree node so we can pass it around freely.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, AdaptImage)]
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
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, AdaptImage)]
pub enum LoopKind {
    /// No-test loop (like `loop <body>`)
    NoTest,
    /// Pre-test loop (like `while <condition> <body>`)
    PreTest,
    /// Post-test loop (like `do <body> while <condition>`)
    PostTest,
}

/// The style of if expression.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, AdaptImage)]
pub enum IfKind {
    /// If expression.
    If,
    /// If ternary expression.
    Ternary,
}

/// The kind of a let/var/const binding.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, AdaptImage)]
pub enum LetKind {
    /// `let` binding.
    Let,
    /// `var` binding.
    Var,
    /// `const` binding.
    Const,
}

/// The condition for an if expression.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, AdaptImage)]
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
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, AdaptImage)]
pub enum WhileKind {
    /// While expression.
    While,
    /// Do-while expression.
    DoWhile,
}

/// The kind of a for each expression.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, AdaptImage)]
pub enum ForEachKind {
    /// In expression.
    In,
    /// Of expression.
    Of,
}

/// The declaration keyword used by a for each pattern binding.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, AdaptImage)]
pub enum ForEachDeclarationKind {
    /// `var` declaration keyword.
    Var,
    /// `let` declaration keyword.
    Let,
    /// `const` declaration keyword.
    Const,
}

/// The binding of a for each expression.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, AdaptImage)]
pub enum ForEachBinding {
    /// A normal pattern binding.
    Pattern {
        pattern: LocalNodeId<Pattern>,
        declaration_kind: Option<ForEachDeclarationKind>,
    },
    /// A using binding.
    Using {
        asynchrony: Asynchrony,
        pattern: LocalNodeId<Pattern>,
    },
}

/// The kind of a yield expression.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, AdaptImage)]
pub enum YieldCardinality {
    /// Generator yield expression.
    Generator,
    /// Scalar yield expression.
    Scalar,
}

/// A WhereClause is a single clause in a where type declaration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, AdaptImage)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, AdaptImage)]
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
