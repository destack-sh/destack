use crate::{
    Argument, AssignOperator, BinaryOperator, Definition, FunctionSignature, Node, NodeId,
    NodeType, Path, Property, ScalarLiteral, TemplateLiteral, TypeBinaryOperator,
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
    /// Definition expression.
    Definition { definition: NodeId<Definition> },
    
    /// Path.
    Path {
        path: Path,
        static_arguments: Option<Vec<NodeId<Argument>>>,
    },
    /// Scalar literal.
    ScalarLiteral { value: ScalarLiteral },
    /// Template literal.
    TemplateLiteral { value: TemplateLiteral },
    /// Array literal.
    ArrayLiteral { elements: Vec<NodeId<Expression>> },
    /// Object literal.
    ObjectLiteral { properties: Vec<NodeId<Property>> },

    /// Parenthesized expression.
    Parenthesized { expression: NodeId<Expression> },

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
    /// Unary operation.
    Unary {
        operator: UnaryOperator,
        right: NodeId<Expression>,
    },
    /// Binary operation.
    Binary {
        left: NodeId<Expression>,
        operator: BinaryOperator,
        right: NodeId<Expression>,
    },
    /// Assignment operation.
    Assign {
        left: NodeId<Expression>,
        right: NodeId<Expression>,
    },
    /// Assignment binary operation.
    AssignBinary {
        left: NodeId<Expression>,
        operator: AssignOperator,
        right: NodeId<Expression>,
    },

    /// Maybe unwrap an expression with `?`.
    Maybe {
        position: PostfixPosition,
        left: NodeId<Expression>,
    },
    /// Force unwrap an expression with `!`.
    Must {
        position: PostfixPosition,
        left: NodeId<Expression>,
    },
    /// Member access.
    Member {
        left: NodeId<Expression>,
        path: Path,
        static_arguments: Option<Vec<NodeId<Argument>>>,
    },
    /// Index.
    Index {
        position: PostfixPosition,
        left: NodeId<Expression>,
        right: NodeId<Expression>,
    },
    /// Call.
    Call {
        position: PostfixPosition,
        left: NodeId<Expression>,
        static_arguments: Option<Vec<NodeId<Argument>>>,
        dynamic_arguments: Vec<NodeId<Argument>>,
    },
    /// New.
    New {
        left: Path,
        static_arguments: Option<Vec<NodeId<Argument>>>,
        dynamic_arguments: Vec<NodeId<Argument>>,
    },
    /// Arrow function expression.
    ArrowFunction {
        signature: FunctionSignature,
        body: NodeId<Expression>,
    },
    /// If ternary.
    IfTernary {
        condition: NodeId<Expression>,
        then_expression: NodeId<Expression>,
        else_expression: Option<NodeId<Expression>>,
    },

    /// Error placeholder.
    Error,
}

impl Node for Expression {
    const TYPE: NodeType = NodeType::Expression;
}
