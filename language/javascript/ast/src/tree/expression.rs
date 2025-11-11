use crate::{
    Argument, BinaryOperator, Definition, Node, NodeId, NodeType, Parameter, Path, ScalarLiteral,
    TemplateLiteral, Type, TypeBinaryOperator, TypeUnaryOperator, UnaryOperator,
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
    /// Arrow function expression.
    ArrowFunction {
        dynamic_parameters: Vec<NodeId<Parameter>>,
        return_type: Option<NodeId<Type>>,
        body: NodeId<Expression>,
    },

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
    ObjectLiteral { fields: Vec<NodeId<Argument>> },

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
        dynamic_arguments: Vec<NodeId<Argument>>,
    },
    /// New.
    New {
        left: Path,
        static_arguments: Option<Vec<NodeId<Argument>>>,
        dynamic_arguments: Vec<NodeId<Argument>>,
    },

    /// If ternary.
    IfTernary {
        condition: NodeId<Expression>,
        then_expression: NodeId<Expression>,
        else_expression: Option<NodeId<Expression>>,
    },
}

impl Node for Expression {
    const TYPE: NodeType = NodeType::Expression;
}
