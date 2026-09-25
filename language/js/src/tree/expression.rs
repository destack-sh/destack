use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

use crate::{
    Argument, AssignOperator, AssignPattern, Asynchrony, BinaryOperator, Block, Declaration,
    Literal, LocalNodeId, Node, NodeType, Parameter, Path, Property, StringId, TemplateLiteral,
    Tree, UnaryOperator,
};
use tspp_source::ModuleId;

/// An Expression is value-producing JS form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub enum Expression {
    /// Declaration expression.
    Declaration {
        declaration: LocalNodeId<Declaration>,
    },

    /// Path.
    Path { path: Path },
    /// Import meta expression.
    ImportMeta,
    /// This intrinsic value.
    This,
    /// Super intrinsic value.
    Super,
    /// Private identifier.
    PrivateIdentifier { name: StringId },
    /// Scalar literal.
    Literal { value: Literal },
    /// Template literal.
    TemplateLiteral { value: TemplateLiteral },
    /// Array literal.
    ArrayLiteral {
        elements: Vec<LocalNodeId<ArrayElement>>,
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

    /// Runtime constructor guard.
    InstanceOf {
        value: LocalNodeId<Expression>,
        target: LocalNodeId<Expression>,
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
        left: LocalNodeId<AssignPattern>,
        right: LocalNodeId<Expression>,
    },
    /// Assignment binary operation.
    AssignBinary {
        left: LocalNodeId<Expression>,
        operator: AssignOperator,
        right: LocalNodeId<Expression>,
    },

    /// Member access.
    Member {
        left: LocalNodeId<Expression>,
        name: StringId,
        is_optional: bool,
    },
    /// Private member access.
    PrivateMember {
        left: LocalNodeId<Expression>,
        name: StringId,
    },
    /// Index.
    Index {
        left: LocalNodeId<Expression>,
        right: LocalNodeId<Expression>,
        is_optional: bool,
    },
    /// Call.
    Call {
        left: LocalNodeId<Expression>,
        arguments: Vec<LocalNodeId<Argument>>,
        is_optional: bool,
    },
    /// Dynamic import call.
    ImportCall {
        target: LocalNodeId<Expression>,
        target_module: Option<ModuleId>,
        arguments: Vec<LocalNodeId<Argument>>,
    },
    /// Await expression.
    Await { value: LocalNodeId<Expression> },
    /// Yield expression.
    Yield {
        is_delegate: bool,
        value: Option<LocalNodeId<Expression>>,
    },
    /// New.
    New {
        left: LocalNodeId<Expression>,
        arguments: Vec<LocalNodeId<Argument>>,
    },
    /// Arrow function expression.
    ArrowFunction {
        asynchrony: Asynchrony,
        parameters: Vec<LocalNodeId<Parameter>>,
        body: ArrowFunctionBody,
    },
    /// If ternary.
    IfTernary {
        condition: LocalNodeId<Expression>,
        then_expression: LocalNodeId<Expression>,
        else_expression: LocalNodeId<Expression>,
    },

    /// Error placeholder.
    Error,
}

impl Node for Expression {
    const TYPE: NodeType = NodeType::Expression;
}

/// One arrow function body.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub enum ArrowFunctionBody {
    /// Expression body.
    Expression(LocalNodeId<Expression>),
    /// Block body.
    Block(LocalNodeId<Block>),
}

/// One element in an array literal.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub enum ArrayElement {
    /// One positional array element.
    Expression { value: LocalNodeId<Expression> },
    /// One spread array element.
    Spread { value: LocalNodeId<Expression> },
    /// One elided array slot.
    Elision,
}

impl Node for ArrayElement {
    const TYPE: NodeType = NodeType::ArrayElement;
}

/// One local expression precedence level for JS printing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Reflect)]
pub(crate) enum Precedence {
    /// Comma and lowest-precedence expressions.
    Lowest,
    /// Assignment expressions.
    Assignment,
    /// Conditional expressions.
    Conditional,
    /// Nullish coalescing expressions.
    Coalesce,
    /// Logical or expressions.
    LogicalOr,
    /// Logical and expressions.
    LogicalAnd,
    /// Bitwise or expressions.
    BitwiseOr,
    /// Bitwise xor expressions.
    BitwiseXor,
    /// Bitwise and expressions.
    BitwiseAnd,
    /// Equality expressions.
    Equality,
    /// Relational expressions.
    Compare,
    /// Shift expressions.
    Shift,
    /// Additive expressions.
    Add,
    /// Multiplicative expressions.
    Multiply,
    /// Exponent expressions.
    Exponent,
    /// Prefix expressions.
    Prefix,
    /// Postfix expressions.
    Postfix,
    /// Primary expressions.
    Primary,
}

impl Precedence {
    /// Return the next tighter precedence level.
    pub(crate) fn tighter(self) -> Self {
        match self {
            Self::Lowest => Self::Assignment,
            Self::Assignment => Self::Conditional,
            Self::Conditional => Self::Coalesce,
            Self::Coalesce => Self::LogicalOr,
            Self::LogicalOr => Self::LogicalAnd,
            Self::LogicalAnd => Self::BitwiseOr,
            Self::BitwiseOr => Self::BitwiseXor,
            Self::BitwiseXor => Self::BitwiseAnd,
            Self::BitwiseAnd => Self::Equality,
            Self::Equality => Self::Compare,
            Self::Compare => Self::Shift,
            Self::Shift => Self::Add,
            Self::Add => Self::Multiply,
            Self::Multiply => Self::Exponent,
            Self::Exponent => Self::Prefix,
            Self::Prefix => Self::Postfix,
            Self::Postfix => Self::Primary,
            Self::Primary => Self::Primary,
        }
    }
}

impl Expression {
    /// Return this expression without redundant explicit parentheses.
    pub(crate) fn without_parentheses<'a>(tree: &'a Tree, expression: &'a Self) -> &'a Self {
        let mut expression = expression;

        while let Self::Parenthesized {
            expression: inner_expression_id,
        } = expression
        {
            expression = tree.get(*inner_expression_id);
        }

        expression
    }

    /// Return the local precedence for this expression.
    pub(crate) fn precedence(&self) -> Precedence {
        match self {
            Self::SequenceExpression { .. } => Precedence::Lowest,
            Self::Yield { .. } => Precedence::Assignment,
            Self::Assign { .. } | Self::AssignBinary { .. } => Precedence::Assignment,
            Self::IfTernary { .. } => Precedence::Conditional,
            Self::Binary { operator, .. } => operator.precedence(),
            Self::InstanceOf { .. } => Precedence::Compare,
            Self::Await { .. } | Self::Unary { .. } => Precedence::Prefix,
            Self::Member { .. }
            | Self::PrivateMember { .. }
            | Self::Index { .. }
            | Self::Call { .. }
            | Self::ImportCall { .. }
            | Self::New { .. } => Precedence::Postfix,
            Self::ArrowFunction { .. } => Precedence::Conditional,
            Self::Declaration { .. }
            | Self::Path { .. }
            | Self::ImportMeta
            | Self::This
            | Self::Super
            | Self::PrivateIdentifier { .. }
            | Self::Literal { .. }
            | Self::TemplateLiteral { .. }
            | Self::ArrayLiteral { .. }
            | Self::ObjectLiteral { .. }
            | Self::Parenthesized { .. }
            | Self::Error => Precedence::Primary,
        }
    }
}

impl BinaryOperator {
    /// Return the local precedence for this binary operator.
    pub(crate) fn precedence(self) -> Precedence {
        match self {
            Self::Multiply | Self::Divide | Self::Remainder => Precedence::Multiply,
            Self::Exponent => Precedence::Exponent,
            Self::Add | Self::Subtract => Precedence::Add,
            Self::ShiftLeft | Self::ShiftRight | Self::UnsignedShiftRight => Precedence::Shift,
            Self::ElementwiseAnd => Precedence::BitwiseAnd,
            Self::ElementwiseXor => Precedence::BitwiseXor,
            Self::ElementwiseOr => Precedence::BitwiseOr,
            Self::Equal | Self::NotEqual | Self::EqualStrict | Self::NotEqualStrict => {
                Precedence::Equality
            }
            Self::LessThan
            | Self::LessThanOrEqual
            | Self::GreaterThan
            | Self::GreaterThanOrEqual
            | Self::In
            | Self::InstanceOf => Precedence::Compare,
            Self::And => Precedence::LogicalAnd,
            Self::Or => Precedence::LogicalOr,
            Self::Coalesce => Precedence::Coalesce,
        }
    }
}
