use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::{
    Argument, ArrayElement, AssignOperator, AssignPattern, Asynchrony, BinaryOperator, Block,
    Declaration, Identifier, IdentifierName, Literal, LocalNodeId, Node, NodeType, Parameter,
    Pattern, Place, Property, TemplateLiteral, Tree, UnaryOperator, UpdateOperator, UpdatePosition,
};

/// One value-producing JavaScript expression.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub enum Expression {
    /// Declaration expression.
    Declaration {
        declaration: LocalNodeId<Declaration>,
    },

    /// Identifier reference.
    Identifier { identifier: Identifier },
    /// Import meta expression.
    ImportMeta,
    /// This intrinsic value.
    This,
    /// Super intrinsic value.
    Super,
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

    /// Unary operation.
    Unary {
        operator: UnaryOperator,
        right: LocalNodeId<Expression>,
    },
    /// Prefix or postfix update operation.
    Update {
        place: LocalNodeId<Place>,
        operator: UpdateOperator,
        position: UpdatePosition,
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
        left: LocalNodeId<Place>,
        operator: AssignOperator,
        right: LocalNodeId<Expression>,
    },

    /// Private field presence check like `#value in object`.
    PrivateIn {
        identifier: Identifier,
        object: LocalNodeId<Expression>,
    },

    /// Member access.
    Member {
        object: LocalNodeId<Expression>,
        property: IdentifierName,
        is_optional: bool,
    },
    /// Private member access.
    PrivateMember {
        object: LocalNodeId<Expression>,
        property: Identifier,
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
        specifier: LocalNodeId<Expression>,
        options: Option<LocalNodeId<Expression>>,
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
        rest: Option<LocalNodeId<Pattern>>,
        body: ArrowFunctionBody,
    },
    /// If ternary.
    IfTernary {
        condition: LocalNodeId<Expression>,
        then_expression: LocalNodeId<Expression>,
        else_expression: LocalNodeId<Expression>,
    },
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

/// One ECMAScript expression precedence level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
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
    /// Call expressions.
    Call,
    /// Member access and constructed expressions.
    Member,
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
            Self::Postfix => Self::Call,
            Self::Call => Self::Member,
            Self::Member => Self::Primary,
            Self::Primary => Self::Primary,
        }
    }
}

impl Expression {
    /// Return the local precedence for this expression.
    pub(crate) fn precedence(&self) -> Precedence {
        match self {
            Self::SequenceExpression { .. } => Precedence::Lowest,
            Self::Yield { .. } => Precedence::Assignment,
            Self::Assign { .. } | Self::AssignBinary { .. } => Precedence::Assignment,
            Self::IfTernary { .. } => Precedence::Conditional,
            Self::Binary { operator, .. } => operator.precedence(),
            Self::PrivateIn { .. } => Precedence::Compare,
            Self::Await { .. } | Self::Unary { .. } => Precedence::Prefix,
            Self::Update { position, .. } => match position {
                UpdatePosition::Prefix => Precedence::Prefix,
                UpdatePosition::Postfix => Precedence::Postfix,
            },
            Self::Call { .. } | Self::ImportCall { .. } => Precedence::Call,
            Self::Member { .. }
            | Self::PrivateMember { .. }
            | Self::Index { .. }
            | Self::New { .. } => Precedence::Member,
            Self::ArrowFunction { .. } => Precedence::Assignment,
            Self::Declaration { .. }
            | Self::Identifier { .. }
            | Self::ImportMeta
            | Self::This
            | Self::Super
            | Self::TemplateLiteral { .. }
            | Self::ArrayLiteral { .. }
            | Self::ObjectLiteral { .. }
            | Self::Parenthesized { .. } => Precedence::Primary,
            Self::Literal { value } => value.precedence(),
        }
    }

    /// Return whether this expression needs parentheses in an ECMAScript `NoIn` context.
    pub(crate) fn needs_no_in_parentheses(&self, tree: &Tree) -> bool {
        let needs_parentheses = |id: LocalNodeId<Self>| tree.get(id).needs_no_in_parentheses(tree);

        match self {
            Self::Binary {
                left,
                operator,
                right,
            } => {
                *operator == BinaryOperator::In
                    || needs_parentheses(*left)
                    || needs_parentheses(*right)
            }
            Self::Assign { right, .. } => needs_parentheses(*right),
            Self::AssignBinary { right, .. } => needs_parentheses(*right),
            Self::SequenceExpression { expressions } => {
                expressions.iter().copied().any(needs_parentheses)
            }
            Self::IfTernary {
                condition,
                else_expression,
                ..
            } => needs_parentheses(*condition) || needs_parentheses(*else_expression),
            Self::Unary { right, .. } => needs_parentheses(*right),
            Self::Update { .. } => false,
            Self::Await { value } => needs_parentheses(*value),
            Self::Yield { value, .. } => value.is_some_and(needs_parentheses),
            Self::PrivateIn { .. } => true,
            Self::Parenthesized { .. }
            | Self::Declaration { .. }
            | Self::Identifier { .. }
            | Self::ImportMeta
            | Self::This
            | Self::Super
            | Self::Literal { .. }
            | Self::TemplateLiteral { .. }
            | Self::ArrayLiteral { .. }
            | Self::ObjectLiteral { .. }
            | Self::Member { .. }
            | Self::PrivateMember { .. }
            | Self::Index { .. }
            | Self::Call { .. }
            | Self::ImportCall { .. }
            | Self::New { .. }
            | Self::ArrowFunction { .. } => false,
        }
    }

    /// Return whether this expression requires parentheses in statement position.
    pub(crate) fn needs_statement_parentheses(&self, tree: &Tree) -> bool {
        let needs_parentheses =
            |id: LocalNodeId<Self>| tree.get(id).needs_statement_parentheses(tree);

        match self {
            Self::Declaration { .. } | Self::ObjectLiteral { .. } => true,
            Self::TemplateLiteral { value } => value.tag.is_some_and(needs_parentheses),
            Self::SequenceExpression { expressions } => {
                expressions.first().copied().is_some_and(needs_parentheses)
            }
            Self::Binary { left, .. } => needs_parentheses(*left),
            Self::AssignBinary { left, .. } | Self::Update { place: left, .. } => {
                tree.get(*left).needs_statement_parentheses(tree)
            }
            Self::Assign { left, .. } => tree.get(*left).needs_statement_parentheses(tree),
            Self::Member { object, .. } | Self::PrivateMember { object, .. } => {
                needs_parentheses(*object)
            }
            Self::Index { left, .. } | Self::Call { left, .. } => needs_parentheses(*left),
            Self::IfTernary { condition, .. } => needs_parentheses(*condition),
            Self::Parenthesized { .. }
            | Self::Identifier { .. }
            | Self::ImportMeta
            | Self::This
            | Self::Super
            | Self::Literal { .. }
            | Self::ArrayLiteral { .. }
            | Self::Await { .. }
            | Self::Yield { .. }
            | Self::Unary { .. }
            | Self::PrivateIn { .. }
            | Self::ImportCall { .. }
            | Self::New { .. }
            | Self::ArrowFunction { .. } => false,
        }
    }

    /// Return whether this expression needs parentheses as a concise arrow body.
    pub(crate) fn needs_arrow_parentheses(&self, tree: &Tree) -> bool {
        let is_declaration = matches!(self, Self::Declaration { .. });
        let is_wrapped_by_precedence = self.precedence() < Precedence::Assignment;

        !is_declaration && !is_wrapped_by_precedence && self.needs_statement_parentheses(tree)
    }

    /// Return whether this expression is an optional chain.
    pub(crate) fn is_optional_chain(&self, tree: &Tree) -> bool {
        let is_chain = |id: LocalNodeId<Self>| tree.get(id).is_optional_chain(tree);

        match self {
            Self::Member {
                object,
                is_optional,
                ..
            } => *is_optional || is_chain(*object),
            Self::Index {
                left, is_optional, ..
            }
            | Self::Call {
                left, is_optional, ..
            } => *is_optional || is_chain(*left),
            Self::PrivateMember { object, .. } => is_chain(*object),
            _ => false,
        }
    }

    /// Return whether decimal member access requires a second dot.
    pub(crate) fn needs_decimal_member_dot(&self, is_optional: bool) -> bool {
        let Self::Literal {
            value: Literal::Number(value),
        } = self
        else {
            return false;
        };
        if is_optional || !value.is_finite() || value.is_sign_negative() || value.fract() != 0.0 {
            return false;
        }

        let text = value.to_string();

        !text.contains(['.', 'e', 'E'])
    }
}
