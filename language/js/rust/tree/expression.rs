use dyst_source::StringId;

use crate::{
    Argument, BinaryOperator, Node, NodeId, NodeType, Parameter, Path, ScalarLiteral,
    TemplateLiteral, Type, TypeBinaryOperator, TypeUnaryOperator, UnaryOperator,
};

#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    /// This.
    This,
    /// Super.
    Super,
    /// Import meta (`import.meta`).
    ImportMeta,

    /// Class expression.
    Class {},
    /// Function expression.
    Function {},
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
        expression: NodeId<Expression>,
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
        expression: NodeId<Expression>,
    },
    /// Binary operation.
    Binary {
        left: NodeId<Expression>,
        operator: BinaryOperator,
        right: NodeId<Expression>,
    },

    /// Member access.
    Member {
        left: NodeId<Expression>,
        path: Path,
        is_maybe: bool,
        static_arguments: Option<Vec<NodeId<Argument>>>,
    },
    /// Index.
    Index {
        left: NodeId<Expression>,
        is_maybe: bool,
        index: Option<NodeId<Expression>>,
    },
    /// Call.
    Call {
        left: NodeId<Expression>,
        is_maybe: bool,
        dynamic_arguments: Vec<NodeId<Argument>>,
    },
    /// Import call.
    ImportCall { source: StringId },
    /// New.
    New {
        left: Path,
        static_arguments: Option<Vec<NodeId<Argument>>>,
        dynamic_arguments: Vec<NodeId<Argument>>,
    },
}

impl Node for Expression {
    const TYPE: NodeType = NodeType::Expression;
}
