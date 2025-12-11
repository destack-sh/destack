use crate::{
    Argument, AssignOperator, BinaryOperator, Declaration, FunctionSignature, LocalNodeId, Node,
    NodeType, Path, Property, ScalarLiteral, StringId, TemplateLiteral, TypeBinaryOperator,
    TypeUnaryOperator, UnaryOperator,
};

/// The position of a postfix expression.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PostfixPosition {
    // Regular postfix (just `x?`)
    Direct,
    // Dot postfix (like `x.?`)
    Indirect,
}

/// An Expression is value-producing JS form.
#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    /// Declaration expression.
    Declaration {
        declaration: LocalNodeId<Declaration>,
    },

    /// Path.
    Path {
        path: Path,
        static_arguments: Option<Vec<LocalNodeId<Argument>>>,
    },
    /// Scalar literal.
    ScalarLiteral { value: ScalarLiteral },
    /// Template literal.
    TemplateLiteral { value: TemplateLiteral },
    /// Array literal.
    ArrayLiteral {
        elements: Vec<LocalNodeId<Expression>>,
    },
    /// Sequence expression (JS comma operator).
    SequenceExpression {
        expressions: Vec<LocalNodeId<Expression>>,
    },
    /// Object literal.
    ObjectLiteral {
        properties: Vec<LocalNodeId<Property>>,
    },

    /// Parenthesized expression.
    Parenthesized { expression: LocalNodeId<Expression> },

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
    /// Unary operation.
    Unary {
        operator: UnaryOperator,
        right: LocalNodeId<Expression>,
    },
    /// Binary operation.
    Binary {
        left: LocalNodeId<Expression>,
        operator: BinaryOperator,
        right: LocalNodeId<Expression>,
    },
    /// Assignment operation.
    Assign {
        left: LocalNodeId<Expression>,
        right: LocalNodeId<Expression>,
    },
    /// Assignment binary operation.
    AssignBinary {
        left: LocalNodeId<Expression>,
        operator: AssignOperator,
        right: LocalNodeId<Expression>,
    },

    /// Maybe unwrap an expression with `?`.
    Maybe {
        position: PostfixPosition,
        left: LocalNodeId<Expression>,
    },
    /// Force unwrap an expression with `!`.
    Must {
        position: PostfixPosition,
        left: LocalNodeId<Expression>,
    },
    /// Member access.
    Member {
        left: LocalNodeId<Expression>,
        name: StringId,
        static_arguments: Option<Vec<LocalNodeId<Argument>>>,
    },
    /// Index.
    Index {
        position: PostfixPosition,
        left: LocalNodeId<Expression>,
        right: LocalNodeId<Expression>,
    },
    /// Call.
    Call {
        position: PostfixPosition,
        left: LocalNodeId<Expression>,
        static_arguments: Option<Vec<LocalNodeId<Argument>>>,
        dynamic_arguments: Vec<LocalNodeId<Argument>>,
    },
    /// New.
    New {
        left: LocalNodeId<Expression>,
        static_arguments: Option<Vec<LocalNodeId<Argument>>>,
        dynamic_arguments: Vec<LocalNodeId<Argument>>,
    },
    /// Arrow function expression.
    ArrowFunction {
        signature: FunctionSignature,
        body: LocalNodeId<Expression>,
    },
    /// If ternary.
    IfTernary {
        condition: LocalNodeId<Expression>,
        then_expression: LocalNodeId<Expression>,
        else_expression: Option<LocalNodeId<Expression>>,
    },

    /// Stub placeholder for annotation-only files.
    Stub,

    /// Error placeholder.
    Error,
}

impl Node for Expression {
    const TYPE: NodeType = NodeType::Expression;
}
