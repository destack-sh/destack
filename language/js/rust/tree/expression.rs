use crate::{
    Argument, AssignOperator, BinaryOperator, Node, NodeId, NodeType, Path, ScalarLiteral, TemplateLiteral, TypeBinaryOperator, TypeUnaryOperator, UnaryOperator
};

#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    /// Import.
    Import {},
    /// Export.
    Export {},

    /// Let.
    Let {},
    /// Let type.
    LetType {},

    /// If.
    If {
		condition: NodeId<Expression>,
		then_block: NodeId<Block>,
		else_block: Option<NodeId<Block>>,
	},
	/// If ternary.
	IfTernary {
		condition: NodeId<Expression>,
		then_expression: NodeId<Expression>,
		else_expression: Option<NodeId<Expression>>,
	},
    /// While.
    While {
		condition: NodeId<Expression>,
		body: NodeId<Block>,
	},
    /// For each.
    ForIn {
		pattern: Option<NodeId<Pattern>>,
		iterator: NodeId<Expression>,
		body: NodeId<Block>,
	},
    /// For of.
    ForOf {
		pattern: Option<NodeId<Pattern>>,
		iterator: NodeId<Expression>,
		body: NodeId<Block>,
	},
    /// For condition.
    ForCondition {
		initialization: Option<NodeId<Expression>>,
		condition: NodeId<Expression>,
		increment: Option<NodeId<Expression>>,
		body: NodeId<Block>,
	},
    
	/// Try.
    Try {},
    /// Await.
    Await {},
    /// Yield.
    Yield {},
    /// Throw.
    Throw {},
    /// Return.
    Return {},

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
    ObjectLiteral {
		fields: Vec<NodeId<Argument>>,
	},
    /// Tree literal.
    TreeLiteral {
		path: Option<Path>,
		arguments: Option<Vec<NodeId<Argument>>>,
		elements: Option<Vec<NodeId<Expression>>>,
	},

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
	/// Assignment.
	Assign {
		left: NodeId<Expression>,
		operator: AssignOperator,
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
