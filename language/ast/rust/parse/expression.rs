//! Parse expressions. Mostly defers to other parsers.

use crate::parse::prelude::*;
use crate::{RangeLiteral, ScopedMutability};
use dyst_token::{TokenSpan, TokenType};

use crate::{
    AssignOperator, BinaryOperator, Call, Expression, InfixOperator, Keyword, Mutability, NodeId,
    NodeType, OperatorPrecedence, ParseError, ParseResult, Parser, ParserMark, Runtime,
    TupleLiteral, UnaryOperator, Visibility,
};

impl BinaryOperator {
    /// Get the precedence of the binary operator.
    #[inline]
    pub fn precedence_group(&self) -> OperatorPrecedence {
        match self {
            // multiplication
            BinaryOperator::Multiply => OperatorPrecedence::Multiplication,
            BinaryOperator::WrappingMultiply => OperatorPrecedence::Multiplication,
            BinaryOperator::SaturatingMultiply => OperatorPrecedence::Multiplication,
            BinaryOperator::Divide => OperatorPrecedence::Multiplication,
            BinaryOperator::Remainder => OperatorPrecedence::Multiplication,

            // addition
            BinaryOperator::Add => OperatorPrecedence::Addition,
            BinaryOperator::WrappingAdd => OperatorPrecedence::Addition,
            BinaryOperator::SaturatingAdd => OperatorPrecedence::Addition,
            BinaryOperator::Subtract => OperatorPrecedence::Addition,
            BinaryOperator::WrappingSubtract => OperatorPrecedence::Addition,
            BinaryOperator::SaturatingSubtract => OperatorPrecedence::Addition,

            // shift
            BinaryOperator::ShiftLeft => OperatorPrecedence::Shift,
            BinaryOperator::SaturatingShiftLeft => OperatorPrecedence::Shift,
            BinaryOperator::ShiftRight => OperatorPrecedence::Shift,

            // elementwise
            BinaryOperator::ElementwiseAnd => OperatorPrecedence::Elementwise,
            BinaryOperator::ElementwiseXor => OperatorPrecedence::Elementwise,
            BinaryOperator::ElementwiseOr => OperatorPrecedence::Elementwise,

            // comparison
            BinaryOperator::Equal => OperatorPrecedence::Comparison,
            BinaryOperator::NotEqual => OperatorPrecedence::Comparison,
            BinaryOperator::LessThan => OperatorPrecedence::Comparison,
            BinaryOperator::LessThanOrEqual => OperatorPrecedence::Comparison,
            BinaryOperator::GreaterThan => OperatorPrecedence::Comparison,
            BinaryOperator::GreaterThanOrEqual => OperatorPrecedence::Comparison,

            // logical
            BinaryOperator::And => OperatorPrecedence::Logical,
            BinaryOperator::Or => OperatorPrecedence::Logical,
        }
    }

    /// Get the precedence of the binary operator.
    pub fn precedence(self) -> u8 {
        // just transmute the enum value to an u8
        self as u8
    }

    /// Convert a TokenType to a BinaryOperator (if a direct mapping exists).
    #[inline]
    pub fn from_token_type(token_type: TokenType) -> Option<BinaryOperator> {
        match token_type {
            // multiplication
            TokenType::Multiply => Some(BinaryOperator::Multiply),
            TokenType::WrappingMultiply => Some(BinaryOperator::WrappingMultiply),
            TokenType::SaturatingMultiply => Some(BinaryOperator::SaturatingMultiply),
            TokenType::Divide => Some(BinaryOperator::Divide),
            TokenType::Remainder => Some(BinaryOperator::Remainder),

            // addition
            TokenType::Add => Some(BinaryOperator::Add),
            TokenType::WrappingAdd => Some(BinaryOperator::WrappingAdd),
            TokenType::SaturatingAdd => Some(BinaryOperator::SaturatingAdd),
            TokenType::Subtract => Some(BinaryOperator::Subtract),
            TokenType::WrappingSubtract => Some(BinaryOperator::WrappingSubtract),
            TokenType::SaturatingSubtract => Some(BinaryOperator::SaturatingSubtract),

            // shift
            TokenType::ShiftLeft => Some(BinaryOperator::ShiftLeft),
            TokenType::SaturatingShiftLeft => Some(BinaryOperator::SaturatingShiftLeft),
            TokenType::ShiftRight => Some(BinaryOperator::ShiftRight),

            // elementwise
            TokenType::ElementwiseAnd => Some(BinaryOperator::ElementwiseAnd),
            TokenType::ElementwiseXor => Some(BinaryOperator::ElementwiseXor),
            TokenType::ElementwiseOr => Some(BinaryOperator::ElementwiseOr),

            // comparison
            TokenType::Equal => Some(BinaryOperator::Equal),
            TokenType::NotEqual => Some(BinaryOperator::NotEqual),
            TokenType::LessThan => Some(BinaryOperator::LessThan),
            TokenType::LessThanOrEqual => Some(BinaryOperator::LessThanOrEqual),
            TokenType::GreaterThan => Some(BinaryOperator::GreaterThan),
            TokenType::GreaterThanOrEqual => Some(BinaryOperator::GreaterThanOrEqual),

            // logical
            TokenType::LogicalAnd => Some(BinaryOperator::And),
            TokenType::LogicalOr => Some(BinaryOperator::Or),

            _ => None,
        }
    }

    /// Convert a BinaryOperator to a TokenType (if a direct mapping exists).
    #[inline]
    pub fn as_token_type(&self) -> TokenType {
        match self {
            // multiplication
            BinaryOperator::Multiply => TokenType::Multiply,
            BinaryOperator::WrappingMultiply => TokenType::WrappingMultiply,
            BinaryOperator::SaturatingMultiply => TokenType::SaturatingMultiply,
            BinaryOperator::Divide => TokenType::Divide,
            BinaryOperator::Remainder => TokenType::Remainder,

            // addition
            BinaryOperator::Add => TokenType::Add,
            BinaryOperator::WrappingAdd => TokenType::WrappingAdd,
            BinaryOperator::SaturatingAdd => TokenType::SaturatingAdd,
            BinaryOperator::Subtract => TokenType::Subtract,
            BinaryOperator::WrappingSubtract => TokenType::WrappingSubtract,
            BinaryOperator::SaturatingSubtract => TokenType::SaturatingSubtract,

            // shift
            BinaryOperator::ShiftLeft => TokenType::ShiftLeft,
            BinaryOperator::SaturatingShiftLeft => TokenType::SaturatingShiftLeft,
            BinaryOperator::ShiftRight => TokenType::ShiftRight,

            // elementwise
            BinaryOperator::ElementwiseAnd => TokenType::ElementwiseAnd,
            BinaryOperator::ElementwiseXor => TokenType::ElementwiseXor,
            BinaryOperator::ElementwiseOr => TokenType::ElementwiseOr,

            // comparison
            BinaryOperator::Equal => TokenType::Equal,
            BinaryOperator::NotEqual => TokenType::NotEqual,
            BinaryOperator::LessThan => TokenType::LessThan,
            BinaryOperator::LessThanOrEqual => TokenType::LessThanOrEqual,
            BinaryOperator::GreaterThan => TokenType::GreaterThan,
            BinaryOperator::GreaterThanOrEqual => TokenType::GreaterThanOrEqual,

            // logical
            BinaryOperator::And => TokenType::LogicalAnd,
            BinaryOperator::Or => TokenType::LogicalOr,
        }
    }
}

impl UnaryOperator {
    /// Get the precedence of the unary operator.
    #[inline]
    pub fn precedence_group(&self) -> OperatorPrecedence {
        OperatorPrecedence::Prefix
    }

    /// Get the precedence of the unary operator.
    #[inline]
    pub fn precedence(self) -> u8 {
        // just transmute the enum value to an u8
        self as u8
    }

    /// Covnert a TokenType to a UnaryOperator (if a direct mapping exists).
    #[inline]
    pub fn from_token_type(token_type: TokenType) -> Option<UnaryOperator> {
        match token_type {
            TokenType::Not => Some(UnaryOperator::Not),
            TokenType::Subtract => Some(UnaryOperator::Negate),
            TokenType::WrappingSubtract => Some(UnaryOperator::WrappingNegate),
            TokenType::Multiply => Some(UnaryOperator::Dereference),
            TokenType::ElementwiseNot => Some(UnaryOperator::ElementwiseNot),
            TokenType::Virtual => Some(UnaryOperator::Virtual),
            _ => None,
        }
    }

    /// Convert a UnaryOperator to a TokenType (if a direct mapping exists).
    #[inline]
    pub fn as_token_type(&self) -> TokenType {
        match self {
            UnaryOperator::Not => TokenType::Not,
            UnaryOperator::Negate => TokenType::Subtract,
            UnaryOperator::WrappingNegate => TokenType::WrappingSubtract,
            UnaryOperator::ElementwiseNot => TokenType::ElementwiseNot,
            UnaryOperator::Dereference => TokenType::Multiply,
            UnaryOperator::Virtual => TokenType::Virtual,
        }
    }
}

impl AssignOperator {
    /// Get the precedence of the assignment type.
    #[inline]
    pub fn precedence_group(&self) -> OperatorPrecedence {
        match self {
            // assignment
            AssignOperator::Assign => OperatorPrecedence::Assignment,

            // assignment multiplication
            AssignOperator::MultiplyAssign
            | AssignOperator::WrappingMultiplyAssign
            | AssignOperator::SaturatingMultiplyAssign
            | AssignOperator::DivideAssign
            | AssignOperator::RemainderAssign => OperatorPrecedence::AssignmentMultiplication,

            // assignment addition
            AssignOperator::AddAssign
            | AssignOperator::WrappingAddAssign
            | AssignOperator::SaturatingAddAssign
            | AssignOperator::SubtractAssign
            | AssignOperator::WrappingSubtractAssign
            | AssignOperator::SaturatingSubtractAssign => OperatorPrecedence::AssignmentAddition,

            // assignment shift
            AssignOperator::ShiftLeftAssign
            | AssignOperator::SaturatingShiftLeftAssign
            | AssignOperator::ShiftRightAssign => OperatorPrecedence::AssignmentShift,

            // assignment elementwise
            AssignOperator::ElementwiseAndAssign
            | AssignOperator::ElementwiseXorAssign
            | AssignOperator::ElementwiseOrAssign => OperatorPrecedence::AssignmentElementwise,

            // assignment logical
            AssignOperator::AndAssign | AssignOperator::OrAssign => {
                OperatorPrecedence::AssignmentLogical
            }
        }
    }

    /// Get the precedence of the assignment type.
    #[inline]
    pub fn precedence(self) -> u8 {
        // just transmute the enum value to an u8
        self as u8
    }

    /// Convert a TokenType to an AssignOperator (if a direct mapping exists).
    #[inline]
    pub fn from_token_type(token_type: TokenType) -> Option<AssignOperator> {
        match token_type {
            TokenType::Assign => Some(AssignOperator::Assign),

            // addition
            TokenType::AddAssign => Some(AssignOperator::AddAssign),
            TokenType::WrappingAddAssign => Some(AssignOperator::WrappingAddAssign),
            TokenType::SaturatingAddAssign => Some(AssignOperator::SaturatingAddAssign),
            TokenType::SubtractAssign => Some(AssignOperator::SubtractAssign),
            TokenType::WrappingSubtractAssign => Some(AssignOperator::WrappingSubtractAssign),
            TokenType::SaturatingSubtractAssign => Some(AssignOperator::SaturatingSubtractAssign),

            // multiplication
            TokenType::MultiplyAssign => Some(AssignOperator::MultiplyAssign),
            TokenType::WrappingMultiplyAssign => Some(AssignOperator::WrappingMultiplyAssign),
            TokenType::SaturatingMultiplyAssign => Some(AssignOperator::SaturatingMultiplyAssign),
            TokenType::DivideAssign => Some(AssignOperator::DivideAssign),
            TokenType::RemainderAssign => Some(AssignOperator::RemainderAssign),

            // shift
            TokenType::ShiftLeftAssign => Some(AssignOperator::ShiftLeftAssign),
            TokenType::SaturatingShiftLeftAssign => Some(AssignOperator::SaturatingShiftLeftAssign),
            TokenType::ShiftRightAssign => Some(AssignOperator::ShiftRightAssign),

            // elementwise
            TokenType::ElementwiseAndAssign => Some(AssignOperator::ElementwiseAndAssign),
            TokenType::ElementwiseOrAssign => Some(AssignOperator::ElementwiseOrAssign),
            TokenType::ElementwiseXorAssign => Some(AssignOperator::ElementwiseXorAssign),

            // logical
            TokenType::LogicalAndAssign => Some(AssignOperator::AndAssign),
            TokenType::LogicalOrAssign => Some(AssignOperator::OrAssign),

            _ => None,
        }
    }

    /// Convert an AssignOperator to a TokenType (if a direct mapping exists).
    #[inline]
    pub fn as_token_type(&self) -> TokenType {
        match self {
            AssignOperator::Assign => TokenType::Assign,

            // addition
            AssignOperator::AddAssign => TokenType::AddAssign,
            AssignOperator::WrappingAddAssign => TokenType::WrappingAddAssign,
            AssignOperator::SaturatingAddAssign => TokenType::SaturatingAddAssign,
            AssignOperator::SubtractAssign => TokenType::SubtractAssign,
            AssignOperator::WrappingSubtractAssign => TokenType::WrappingSubtractAssign,
            AssignOperator::SaturatingSubtractAssign => TokenType::SaturatingSubtractAssign,

            // multiplication
            AssignOperator::MultiplyAssign => TokenType::MultiplyAssign,
            AssignOperator::WrappingMultiplyAssign => TokenType::WrappingMultiplyAssign,
            AssignOperator::SaturatingMultiplyAssign => TokenType::SaturatingMultiplyAssign,
            AssignOperator::DivideAssign => TokenType::DivideAssign,
            AssignOperator::RemainderAssign => TokenType::RemainderAssign,

            // shift
            AssignOperator::ShiftLeftAssign => TokenType::ShiftLeftAssign,
            AssignOperator::SaturatingShiftLeftAssign => TokenType::SaturatingShiftLeftAssign,
            AssignOperator::ShiftRightAssign => TokenType::ShiftRightAssign,

            // elementwise
            AssignOperator::ElementwiseAndAssign => TokenType::ElementwiseAndAssign,
            AssignOperator::ElementwiseOrAssign => TokenType::ElementwiseOrAssign,
            AssignOperator::ElementwiseXorAssign => TokenType::ElementwiseXorAssign,

            // logical
            AssignOperator::AndAssign => TokenType::LogicalAndAssign,
            AssignOperator::OrAssign => TokenType::LogicalOrAssign,
        }
    }
}

impl InfixOperator {
    /// Get the precedence of the infix operator.
    #[inline]
    pub fn precedence_group(&self) -> OperatorPrecedence {
        match self {
            InfixOperator::Binary(binary_operator) => binary_operator.precedence_group(),
            InfixOperator::Assign(assign_operator) => assign_operator.precedence_group(),
        }
    }

    /// Get the precedence of the infix operator.
    #[inline]
    pub fn precedence(self) -> u8 {
        match self {
            InfixOperator::Binary(binary_operator) => binary_operator.precedence(),
            InfixOperator::Assign(assign_operator) => assign_operator.precedence(),
        }
    }
}

static IN_STATIC_TYPE_BINARY_OPERATORS: [BinaryOperator; 15] = [
    // multiplication
    BinaryOperator::Multiply,
    BinaryOperator::WrappingMultiply,
    BinaryOperator::SaturatingMultiply,
    BinaryOperator::Divide,
    BinaryOperator::Remainder,
    // addition
    BinaryOperator::Add,
    BinaryOperator::WrappingAdd,
    BinaryOperator::SaturatingAdd,
    BinaryOperator::Subtract,
    BinaryOperator::WrappingSubtract,
    BinaryOperator::SaturatingSubtract,
    // logical
    BinaryOperator::And,
    BinaryOperator::Or,
    // comparison
    BinaryOperator::Equal,
    BinaryOperator::NotEqual,
];

/// Options for parsing an expression.
#[derive(Debug, Copy, Clone, Default)]
pub struct ExpressionParserOptions {
    /// Whether we're in parenthesized expression (directly).
    /// These expressions might be tuple literals if followed by a comma.
    pub is_parenthesized: bool = false,
    /// Whether we're parsing an expression followed by a block (like in if, match, for, while).
    /// We disallow struct literals at the root level in these cases to avoid ambiguity with expr {}.
    pub is_before_block: bool = false,
    /// The left precedence preceding (i.e. before) the expression. 
    /// Determines AST structure.
    pub left_precedence: Option<u8> = None,
    /// The visibility of this expression.
    /// Used when pre-snacking the visibility in an outer parse (like for expressions).
    pub visibility: Option<Visibility> = None,
}

impl<'a> Parser<'a> {
    /// Peek a unary operator.
    #[inline]
    pub fn peek_unary_operator(&self) -> ParseResult<UnaryOperator> {
        let token = self.peek()?;
        UnaryOperator::from_token_type(token.token.r#type).ok_or(ParseError::unexpected(token.span))
    }

    /// Peek a next unary operator.
    #[inline]
    pub fn peek_next_unary_operator(&self) -> ParseResult<UnaryOperator> {
        let token = self.peek_next()?;
        UnaryOperator::from_token_type(token.token.r#type).ok_or(ParseError::unexpected(token.span))
    }
    /// Make a binary operator.
    #[inline]
    fn to_binary_operator(&self, token: &TokenSpan) -> ParseResult<BinaryOperator> {
        if let Some(binary_operator) = BinaryOperator::from_token_type(token.token.r#type) {
            if self.options.in_static_type
                && !IN_STATIC_TYPE_BINARY_OPERATORS.contains(&binary_operator)
            {
                return Err(ParseError::unexpected(token.span));
            }
            Ok(binary_operator)
        } else {
            Err(ParseError::unexpected(token.span))
        }
    }

    /// Make an assign operator.
    #[inline]
    fn to_assign_operator(&self, token: &TokenSpan) -> ParseResult<AssignOperator> {
        if self.options.in_static_type {
            return Err(ParseError::unexpected(token.span));
        }
        AssignOperator::from_token_type(token.token.r#type)
            .ok_or(ParseError::unexpected(token.span))
    }

    /// Peek a binary operator.
    #[inline]
    pub fn peek_binary_operator(&self) -> ParseResult<BinaryOperator> {
        let token = self.peek()?;
        self.to_binary_operator(token)
    }

    /// Peek a next binary operator.
    #[inline]
    pub fn peek_next_binary_operator(&self) -> ParseResult<BinaryOperator> {
        let token = self.peek_next()?;
        self.to_binary_operator(token)
    }

    /// Peek an assign operator.
    #[inline]
    pub fn peek_assign_operator(&self) -> ParseResult<AssignOperator> {
        let token = self.peek()?;
        self.to_assign_operator(token)
    }

    /// Peek a next assign operator.
    #[inline]
    pub fn peek_next_assign_operator(&self) -> ParseResult<AssignOperator> {
        let token = self.peek_next()?;
        self.to_assign_operator(token)
    }

    /// Make an infix operator.
    #[inline]
    fn to_infix_operator(&self, token: &TokenSpan) -> ParseResult<InfixOperator> {
        if let Some(binary_operator) = BinaryOperator::from_token_type(token.token.r#type)
            && (!self.options.in_static_type
                || IN_STATIC_TYPE_BINARY_OPERATORS.contains(&binary_operator))
        {
            Ok(InfixOperator::Binary(binary_operator))
        } else if !self.options.in_static_type
            && let Some(assign_operator) = AssignOperator::from_token_type(token.token.r#type)
        {
            Ok(InfixOperator::Assign(assign_operator))
        } else {
            Err(ParseError::unexpected(token.span))
        }
    }

    /// Peek an infix operator.
    #[inline]
    pub fn peek_infix_operator(&self) -> ParseResult<InfixOperator> {
        let token = self.peek()?;
        self.to_infix_operator(token)
    }

    /// Peek a next infix operator.
    #[inline]
    pub fn peek_next_infix_operator(&self) -> ParseResult<InfixOperator> {
        let token = self.peek_next()?;
        self.to_infix_operator(token)
    }

    /// Make an expression from an infix operator.
    #[inline]
    fn make_infix_expression(
        &self,
        left: NodeId<Expression>,
        operator: InfixOperator,
        right: NodeId<Expression>,
    ) -> Expression {
        match operator {
            InfixOperator::Binary(binary_operator) => Expression::Binary {
                left,
                operator: binary_operator,
                right,
            },
            InfixOperator::Assign(assign_operator) => Expression::Assign {
                left,
                operator: assign_operator,
                right,
            },
        }
    }

    /// Try to eat an expression as a statement (return Expression::Error if error and recovery is possible).
    #[inline]
    pub fn try_eat_expression_as_statement(&mut self) -> ParseResult<NodeId<Expression>> {
        self.try_eat_expression(ExpressionParserOptions::default(), TokenType::Newline)
    }

    /// Try to eat an expression (return Expression::Error if error and recovery is possible).
    #[inline]
    pub fn try_eat_expression(
        &mut self,
        options: ExpressionParserOptions,
        recover: TokenType,
    ) -> ParseResult<NodeId<Expression>> {
        match self.eat_expression(options) {
            Ok(expression_id) => Ok(expression_id),
            Err(err) => {
                let err = err.for_node_type(NodeType::Expression);
                let span = err.leaf_span();
                let start = ParserMark::new(span.start as usize);
                self.try_recover(start, recover, Some(err))?;
                let error_id = self
                    .tree
                    .allocate(Expression::Error, self.get_span_from(start));
                Ok(error_id)
            }
        }
    }

    /// Peek a member access of the given token type.
    /// Returns the total distance to eat (including the newlines, dot, and token).
    #[inline]
    fn peek_member(&self, token_type: TokenType) -> ParseResult<u8> {
        // immediate member access
        if self.peek_token(TokenType::Dot).is_ok() && self.peek_next_token(token_type).is_ok() {
            Ok(2)
        }
        // member access across newline
        else if self.peek_token(TokenType::Newline).is_ok()
            && self.peek_next_token(TokenType::Dot).is_ok()
            && self.peek_next_next_token(token_type).is_ok()
        {
            Ok(3)
        }
        // nothing
        else {
            Err(ParseError::unexpected(self.peek()?.span))
        }
    }

    /// Eat an expression.
    pub fn eat_expression(
        &mut self,
        options: ExpressionParserOptions,
    ) -> ParseResult<NodeId<Expression>> {
        let start = self.mark();

        // visibility
        let visibility = if options.visibility.is_some() {
            options.visibility
        } else {
            let visibility = self.peek_visibility()?;
            if visibility.is_some() {
                self.bump(); // eat visibility
            }
            visibility
        };

        // runtime
        let mut runtime = if self.peek_token(TokenType::At).is_ok() {
            self.bump(); // eat @
            Some(Runtime::Static)
        } else {
            None
        };

        let mut left_expression_id: NodeId<Expression> = {
            let token = self.peek()?;
            let keyword = self.peek_any_keyword().ok();

            //
            // ------------------------------------------------------------
            // Grouping
            // ------------------------------------------------------------
            //

            // parenthesis
            if token.token.r#type == TokenType::OpenParenthesis {
                self.bump(); // eat open paranthesis
                // parse inner expressions in their own context
                let expression_id = self.eat_expression(ExpressionParserOptions {
                    is_parenthesized: true,
                    ..ExpressionParserOptions::default()
                })?;
                self.eat_token(TokenType::CloseParenthesis)?;
                self.tree.set_span(expression_id, self.get_span_from(start));
                expression_id
            }
            //
            // ------------------------------------------------------------
            // Unary operations (prefix, right associative)
            // ------------------------------------------------------------
            //

            // unary operations
            else if let Ok(unary_operator) = self.peek_unary_operator() {
                let right_precedence = unary_operator.precedence();
                self.bump(); // eat unary operator (always because right associative)
                let right = self.eat_expression(ExpressionParserOptions {
                    left_precedence: Some(right_precedence),
                    ..options
                })?;
                let expression = Expression::Unary {
                    operator: unary_operator,
                    right,
                };
                self.tree.allocate(expression, self.get_span_from(start))
            }
            // reference (`&` or `&var` or `&const`)
            else if self.peek_token(TokenType::ElementwiseAnd).is_ok() {
                self.bump(); // eat &
                let mutability = if self.peek_keyword(Keyword::Var).is_ok()
                    || self.peek_keyword(Keyword::Const).is_ok()
                {
                    self.eat_scoped_mutability()
                        .for_node_type(NodeType::Expression)?
                } else {
                    ScopedMutability::Unscoped {
                        mutability: Mutability::Immutable,
                    }
                };
                let right = self.eat_expression(options)?;
                let expression = Expression::Reference { mutability, right };
                self.tree.allocate(expression, self.get_span_from(start))
            }
            //
            // ------------------------------------------------------------
            // Declarations
            // ------------------------------------------------------------
            //

            // module
            else if keyword == Some(Keyword::Module) {
                let module_id = self
                    .eat_module(visibility)
                    .for_node_type(NodeType::Module)?;
                let expression = Expression::Module(module_id);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            // struct
            else if keyword == Some(Keyword::Struct) {
                let struct_id = self
                    .eat_struct(visibility)
                    .for_node_type(NodeType::Struct)?;
                let expression = Expression::Struct(struct_id);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            // enum
            else if keyword == Some(Keyword::Enum) {
                let enum_id = self.eat_enum(visibility).for_node_type(NodeType::Enum)?;
                let expression = Expression::Enum(enum_id);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            // union
            else if keyword == Some(Keyword::Union) {
                let union_id = self.eat_union(visibility).for_node_type(NodeType::Union)?;
                let expression = Expression::Union(union_id);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            // trait
            else if keyword == Some(Keyword::Trait) {
                let trait_id = self.eat_trait(visibility).for_node_type(NodeType::Trait)?;
                let expression = Expression::Trait(trait_id);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            // implement
            else if keyword == Some(Keyword::Implement) {
                let implement_id = self.eat_implement().for_node_type(NodeType::Implement)?;
                let expression = Expression::Implement(implement_id);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            // function
            else if keyword == Some(Keyword::Function) {
                let function_id = self
                    .eat_function(visibility)
                    .for_node_type(NodeType::Function)?;
                let expression = Expression::Function(function_id);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            // block
            else if self.peek_block().is_ok() {
                let block_id = self.eat_block().for_node_type(NodeType::Block)?;
                let expression = Expression::Block(block_id);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            //
            // ------------------------------------------------------------
            // Control flow
            // ------------------------------------------------------------
            //
            // with
            else if keyword == Some(Keyword::With) {
                let with_id = self.eat_with().for_node_type(NodeType::With)?;
                let expression = Expression::With(with_id);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            // use
            else if keyword == Some(Keyword::Use) {
                let use_id = self.eat_use(visibility).for_node_type(NodeType::Use)?;
                let expression = Expression::Use(use_id);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            // let
            else if keyword == Some(Keyword::Let)
                || keyword == Some(Keyword::Var)
                || keyword == Some(Keyword::Const)
            {
                let let_id = self.eat_let(visibility).for_node_type(NodeType::Let)?;
                let expression = Expression::Let(let_id);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            // if
            else if keyword == Some(Keyword::If) {
                let if_id = self.eat_if(runtime).for_node_type(NodeType::If)?;
                let expression = Expression::If(if_id);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            // while
            else if keyword == Some(Keyword::While) {
                let while_id = self.eat_while(runtime).for_node_type(NodeType::While)?;
                let expression = Expression::While(while_id);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            // for
            else if keyword == Some(Keyword::For) {
                let for_id = self.eat_for(runtime).for_node_type(NodeType::For)?;
                let expression = Expression::For(for_id);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            // loop
            else if keyword == Some(Keyword::Loop) {
                let loop_id = self.eat_loop(runtime).for_node_type(NodeType::Loop)?;
                let expression = Expression::Loop(loop_id);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            // try
            else if keyword == Some(Keyword::Try) {
                let try_id = self.eat_try_catch().for_node_type(NodeType::Try)?;
                let expression = Expression::Try(try_id);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            // match
            else if keyword == Some(Keyword::Match) {
                let match_id = self.eat_match().for_node_type(NodeType::Match)?;
                let expression = Expression::Match(match_id);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            // break
            else if keyword == Some(Keyword::Break) {
                let break_id = self.eat_break().for_node_type(NodeType::Break)?;
                let expression = Expression::Break(break_id);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            // continue
            else if keyword == Some(Keyword::Continue) {
                let continue_id = self.eat_continue().for_node_type(NodeType::Continue)?;
                let expression = Expression::Continue(continue_id);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            // defer
            else if keyword == Some(Keyword::Defer) {
                let defer_id = self.eat_defer().for_node_type(NodeType::Defer)?;
                let expression = Expression::Defer(defer_id);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            // return
            else if keyword == Some(Keyword::Return) {
                let return_id = self.eat_return().for_node_type(NodeType::Return)?;
                let expression = Expression::Return(return_id);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            //
            // ------------------------------------------------------------
            // Bindings / Literals / Aliases
            // ------------------------------------------------------------
            //
            // let
            else if keyword == Some(Keyword::Let)
                || keyword == Some(Keyword::Var)
                || keyword == Some(Keyword::Const)
            {
                let let_id = self.eat_let(visibility).for_node_type(NodeType::Let)?;
                let expression = Expression::Let(let_id);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            // array
            else if token.token.r#type == TokenType::OpenBracket {
                let array_literal = self
                    .eat_array_literal()
                    .for_node_type(NodeType::ArrayLiteral)?;
                let expression = Expression::ArrayLiteral(array_literal);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            // scalar
            else if self.peek_scalar_literal().is_ok() {
                let scalar_literal = self
                    .eat_scalar_literal()
                    .for_node_type(NodeType::ScalarLiteral)?;
                let expression = Expression::ScalarLiteral(scalar_literal);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            // struct
            else if !options.is_before_block && self.peek_struct_literal().is_ok() {
                let struct_literal = self
                    .eat_struct_literal()
                    .for_node_type(NodeType::StructLiteral)?;
                let expression = Expression::StructLiteral(struct_literal);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            // alias / path
            else if token.token.r#type == TokenType::Identifier {
                let path_id = self.eat_path().for_node_type(NodeType::Expression)?;
                // TODO! nocheckin: parse static arguments (for Types as values, but also literals)
                //  (also see peek_path and peek_struct_literal)
                let expression = Expression::Path {
                    path: path_id,
                    static_arguments: None,
                };
                self.tree.allocate(expression, self.get_span_from(start))
            }
            //
            // ------------------------------------------------------------
            // Error
            // ------------------------------------------------------------
            //
            else {
                return Err(ParseError::unexpected(self.peek()?.span));
            }
        };

        //
        // ------------------------------------------------------------
        // Postfix operations
        // ------------------------------------------------------------
        //

        // implicitly call static functions without arguments (e.g., `#entity`)
        let left_expression = self.tree.get(left_expression_id);
        if runtime.is_some()
            && let Expression::Path { .. } = left_expression
            && self.peek_token(TokenType::OpenParenthesis).is_err()
        {
            let call_id = self.tree.allocate(
                Call {
                    runtime,
                    receiver: left_expression_id,
                    static_arguments: None,
                    dynamic_arguments: vec![],
                },
                self.get_span_from(start),
            );
            left_expression_id = self
                .tree
                .allocate(Expression::Call(call_id), self.get_span_from(start));
            runtime = None;
        }

        // eat all postfix operations
        loop {
            // range (implicit with `..`)
            if self.peek_token(TokenType::Range).is_ok()
                || self.peek_token(TokenType::RangeWide).is_ok()
            {
                self.bump(); // eat ..
                let right_expression_id = self
                    .eat_expression(options)
                    .for_node_type(NodeType::RangeLiteral)?;
                let literal_id = self.tree.allocate(
                    RangeLiteral {
                        start: left_expression_id,
                        end: right_expression_id,
                        is_inclusive: true,
                    },
                    self.get_span_from(start),
                );
                let expression = Expression::RangeLiteral(literal_id);
                left_expression_id = self.tree.allocate(expression, self.get_span_from(start));
            }
            // member (also works across newline)
            else if let Ok(distance) = self.peek_member(TokenType::Identifier) {
                self.bump_by(distance - 1); // keep the identifier
                let path_id = self.eat_path().for_node_type(NodeType::Expression)?;
                let expression = Expression::Member {
                    receiver: left_expression_id,
                    path: path_id,
                };
                left_expression_id = self.tree.allocate(expression, self.get_span_from(start));
            }
            // dereference (postfix with `.*`)
            else if let Ok(distance) = self.peek_member(TokenType::Multiply) {
                self.bump_by(distance); // eat dereference
                let expression = Expression::Unary {
                    operator: UnaryOperator::Dereference,
                    right: left_expression_id,
                };
                left_expression_id = self.tree.allocate(expression, self.get_span_from(start));
            }
            // reference (postfix with `&`)
            else if let Ok(distance) = self.peek_member(TokenType::ElementwiseAnd) {
                self.bump_by(distance); // eat &
                let mutability = self.eat_scoped_mutability()?;
                let expression = Expression::Reference {
                    mutability,
                    right: left_expression_id,
                };
                left_expression_id = self.tree.allocate(expression, self.get_span_from(start));
            }
            // index (implicit with `.0`)
            else if let Ok(distance) = self.peek_member(TokenType::Literal) {
                self.bump_by(distance - 2); // eat only newlines
                let index_id = self
                    .eat_index_postfix_implicit(left_expression_id)
                    .for_node_type(NodeType::Index)?;
                let expression = Expression::Index(index_id);
                left_expression_id = self.tree.allocate(expression, self.get_span_from(start));
            }
            // index (explicit with `[]`)
            else if self.peek_token(TokenType::OpenBracket).is_ok() {
                let index_id = self
                    .eat_index_postfix_explicit(left_expression_id)
                    .for_node_type(NodeType::Index)?;
                let expression = Expression::Index(index_id);
                left_expression_id = self.tree.allocate(expression, self.get_span_from(start));
            }
            // call
            else if self.peek_token(TokenType::OpenParenthesis).is_ok() {
                let call_id = self
                    .eat_call_postfix(left_expression_id, runtime)
                    .for_node_type(NodeType::Call)?;
                let expression = Expression::Call(call_id);
                left_expression_id = self.tree.allocate(expression, self.get_span_from(start));
            }
            // cast
            else if self.peek_keyword(Keyword::As).is_ok() {
                let cast_id = self
                    .eat_as_postfix(left_expression_id)
                    .for_node_type(NodeType::Cast)?;
                let expression = Expression::Cast(cast_id);
                left_expression_id = self.tree.allocate(expression, self.get_span_from(start));
            }
            // unwrap
            else if self.peek_token(TokenType::Maybe).is_ok() {
                self.bump(); // eat ?
                let expression = Expression::Maybe(left_expression_id);
                left_expression_id = self.tree.allocate(expression, self.get_span_from(start));
            }
            // force unwrap
            else if self.peek_token(TokenType::Not).is_ok() {
                self.bump(); // eat !
                let expression = Expression::Must(left_expression_id);
                left_expression_id = self.tree.allocate(expression, self.get_span_from(start));
            }
            // coalesce
            else if self.peek_token(TokenType::Coalesce).is_ok() {
                let coalesce_id = self
                    .eat_coalesce_postfix(left_expression_id)
                    .for_node_type(NodeType::Coalesce)?;
                let expression = Expression::Coalesce(coalesce_id);
                left_expression_id = self.tree.allocate(expression, self.get_span_from(start));
            }
            // tuple
            else if options.is_parenthesized && self.peek_token(TokenType::Comma).is_ok() {
                self.bump(); // eat comma
                self.eat_newlines_maybe()?;
                let tuple_elements = self
                    .eat_tuple_literal_body(left_expression_id)
                    .for_node_type(NodeType::TupleLiteral)?;
                let tuple_literal_id = self.tree.allocate(
                    TupleLiteral {
                        elements: tuple_elements,
                    },
                    self.get_span_from(start),
                );
                let expression = Expression::TupleLiteral(tuple_literal_id);
                left_expression_id = self.tree.allocate(expression, self.get_span_from(start));
            }
            // done
            else {
                break;
            }
        }

        //
        // ------------------------------------------------------------
        // Infix operations (binary and assign, left associative)
        // ------------------------------------------------------------
        //

        // eat infix expressions while left precedence is weaker than right precedence
        loop {
            let right_operator = {
                // infix operator on same line
                if let Ok(right_operator) = self.peek_infix_operator()
                    && (options.left_precedence.is_none()
                        || options.left_precedence.unwrap() < right_operator.precedence())
                {
                    right_operator
                }
                // infix operator on next line
                else if self.peek_token(TokenType::Newline).is_ok()
                    && let Ok(right_operator) = self.peek_next_infix_operator()
                    && (options.left_precedence.is_none()
                        || options.left_precedence.unwrap() < right_operator.precedence())
                {
                    right_operator
                }
                // no infix operator, break
                else {
                    break;
                }
            };
            if self.peek_token(TokenType::Newline).is_ok() {
                self.bump(); // eat newline
            }
            self.bump(); // eat infix operator
            self.eat_newline_maybe()?; // allow one newline

            // eat right expression
            let right_expression_id = self.eat_expression(ExpressionParserOptions {
                left_precedence: Some(right_operator.precedence()),
                ..options
            })?;

            // combine into new left expression
            let left_expression =
                self.make_infix_expression(left_expression_id, right_operator, right_expression_id);
            left_expression_id = self
                .tree
                .allocate(left_expression, self.get_span_from(start))
        }

        Ok(left_expression_id)
    }
}

#[cfg(test)]
mod tests {
    use crate::parse::expression::ExpressionParserOptions;
    use crate::parse::tests::TestParser;
    use crate::{
        BinaryOperator, Call, Coalesce, Expression, FieldLiteral, Let, Mutability, Pattern,
        RangeLiteral, Runtime, ScalarLiteral, ScopedMutability, StructLiteral, TupleLiteral, Type,
        UnaryOperator, assert_expr_path, assert_int, assert_lit_int, assert_node, assert_path,
        assert_string,
    };

    /// Tuple literals are disambiguated.
    /// (1, 2)
    #[test]
    fn test_parse_tuple_literal() {
        let mut test = TestParser::new("(1, 2)");
        let mut parser = test.prepare();
        let expr_id = parser
            .eat_expression(ExpressionParserOptions::default())
            .unwrap();
        // (1, 2)
        assert_node!(
            parser.tree,
            expr_id,
            Expression::TupleLiteral(tuple_literal_id) => {
                assert_node!(
                    parser.tree,
                    *tuple_literal_id,
                    TupleLiteral { elements } => {
                        assert_eq!(elements.len(), 2);
                        // 1
                        assert_node!(
                            parser.tree,
                            elements[0],
                            Expression::ScalarLiteral(scalar_literal_id) => {
                                assert_int!(parser.tree, *scalar_literal_id, 1);
                            }
                        );
                        // 2
                        assert_node!(
                            parser.tree,
                            elements[1],
                            Expression::ScalarLiteral(scalar_literal_id) => {
                                assert_int!(parser.tree, *scalar_literal_id, 2);
                            }
                        );
                    }
                );
            }
        );
    }

    /// Range literals are disambiguated.
    /// 1..3
    #[test]
    fn test_parse_range_literal() {
        let mut test = TestParser::new("1..3");
        let mut parser = test.prepare();
        let expr_id = parser
            .eat_expression(ExpressionParserOptions::default())
            .unwrap();

        // 1..3
        assert_node!(parser.tree, expr_id, Expression::RangeLiteral(range_literal_id) => {
            assert_node!(parser.tree, *range_literal_id, RangeLiteral { start, end, .. } => {
                assert_node!(parser.tree, *start, Expression::ScalarLiteral(scalar_literal_id) => {
                    assert_lit_int!(parser.session, parser.tree.get(*scalar_literal_id), 1);
                });
                assert_node!(parser.tree, *end, Expression::ScalarLiteral(scalar_literal_id) => {
                    assert_lit_int!(parser.session, parser.tree.get(*scalar_literal_id), 3);
                });
            });
        });
    }

    /// Struct literals are disambiguated.
    /// geom.Vector2 { x: 1, y }
    #[test]
    fn test_parse_struct_literal_path() {
        let mut test = TestParser::new("geom.Vector2 { x: 1, y }");
        let mut parser = test.prepare();

        let expr_id = parser
            .eat_expression(ExpressionParserOptions::default())
            .unwrap();

        // geom.Vector2 { x: 1, y }
        assert_node!(
            parser.tree,
            expr_id,
            Expression::StructLiteral(struct_literal_id) => {
                assert_node!(
                    parser.tree,
                    *struct_literal_id,
                    StructLiteral { r#type, fields } => {
                        // geom.Vector2
                        assert_node!(
                            parser.tree,
                            *r#type,
                            Type::Path { path, static_arguments: _ } => {
                                assert_path!(parser.session, *path, "geom.Vector2");
                            }
                        );
                        // fields
                        assert_eq!(fields.len(), 2);
                        // x: 1
                        assert_node!(
                            parser.tree,
                            fields[0],
                            FieldLiteral::Named { name, value } => {
                                // x
                                assert_string!(parser.session, *name, "x");
                                // 1
                                assert_node!(
                                    parser.tree,
                                    *value,
                                    Expression::ScalarLiteral(scalar_id) => {
                                        assert_node!(
                                            parser.tree,
                                            *scalar_id,
                                            ScalarLiteral::Integer(1, _)
                                        );
                                    }
                                );
                            }
                        );
                        // y
                        assert_node!(
                            parser.tree,
                            fields[1],
                            FieldLiteral::NamedShorthand { name } => {
                                // y
                                assert_string!(parser.session, *name, "y");
                            }
                        );
                    }
                );
            }
        );
    }

    /// Struct literals with static parameters are disambiguated.
    /// geom.Mesh<2, Dims: 4> {
    ///     vertices: [1, 2]
    ///     y  
    /// }
    #[test]
    fn test_parse_struct_literal_path_with_static_parameters() {
        let mut test = TestParser::new(
            r##"
geom.Mesh<2, Dims: 4> { 
    vertices: [1, 2]
    y
}"##,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let expr_id = parser
            .eat_expression(ExpressionParserOptions::default())
            .unwrap();

        assert_node!(
            parser.tree,
            expr_id,
            Expression::StructLiteral(struct_literal_id) => {
                let struct_literal = parser.tree.get(*struct_literal_id);
                // geom.Mesh<2, Dims: 4>
                assert_node!(
                    parser.tree,
                    struct_literal.r#type,
                    Type::Path { path, static_arguments } => {
                        // geom.Mesh
                        assert_path!(parser.session, *path, "geom.Mesh");
                        // <2, Dims: 4>
                        assert!(static_arguments.is_some());
                        let params = static_arguments.as_ref().unwrap();
                        assert_eq!(params.len(), 2);
                    }
                );
                let fields = &struct_literal.fields;
                assert_eq!(fields.len(), 2);
                // vertices: [1, 2]
                assert_node!(
                    parser.tree,
                    fields[0],
                    FieldLiteral::Named { name, value } => {
                        assert_string!(parser.session, *name, "vertices");
                        assert_node!(
                            parser.tree,
                            *value,
                            Expression::ArrayLiteral(_)
                        );
                    }
                );
                // y
                assert_node!(
                    parser.tree,
                    fields[1],
                    FieldLiteral::NamedShorthand { name } => {
                        assert_string!(parser.session, *name, "y");
                    }
                );
            }
        );
    }

    /// Dereference variable.
    /// *x
    #[test]
    fn test_parse_dereference_variable() {
        let mut test = TestParser::new("*x");
        let mut parser = test.prepare();

        let expr_id = parser
            .eat_expression(ExpressionParserOptions::default())
            .unwrap();

        // *x
        assert_node!(parser.tree, expr_id, Expression::Unary { operator, right } => {
            assert_eq!(*operator, UnaryOperator::Dereference);
            assert_expr_path!(parser.session, parser.tree.get(*right), "x");
        });
    }

    /// Dereference variable postfix.
    /// x.*
    #[test]
    fn test_parse_dereference_variable_postfix() {
        let mut test = TestParser::new("x.*");
        let mut parser = test.prepare();
        let expr_id = parser
            .eat_expression(ExpressionParserOptions::default())
            .unwrap();

        // x.*
        assert_node!(parser.tree, expr_id, Expression::Unary { operator, right } => {
            assert_eq!(*operator, UnaryOperator::Dereference);
            assert_expr_path!(parser.session, parser.tree.get(*right), "x");
        });
    }

    /// Reference operator on variable.
    /// &x
    #[test]
    fn test_parse_reference_variable() {
        let mut test = TestParser::new("&x");
        let mut parser = test.prepare();
        let expr_id = parser
            .eat_expression(ExpressionParserOptions::default())
            .unwrap();

        // &x
        assert_node!(
            parser.tree,
            expr_id,
            Expression::Reference { mutability, right } => {
                // &
                assert_eq!(*mutability, ScopedMutability::Unscoped { mutability: Mutability::Immutable });
                // x
                assert_expr_path!(parser.session, parser.tree.get(*right), "x");
            }
        );
    }

    /// Reference operator postfix.
    /// &var x
    #[test]
    fn test_parse_reference_variable_postfix() {
        let mut test = TestParser::new("x.&");
        let mut parser = test.prepare();
        let expr_id = parser
            .eat_expression(ExpressionParserOptions::default())
            .unwrap();

        // x.&
        assert_node!(parser.tree, expr_id, Expression::Reference { mutability, right } => {
            assert_eq!(*mutability, ScopedMutability::Unscoped { mutability: Mutability::Immutable });
            assert_expr_path!(parser.session, parser.tree.get(*right), "x");
        });
    }

    /// Reference operator postfix with scoped mutability.
    /// &var x
    #[test]
    fn test_parse_reference_variable_postfix_with_scoped_mutability() {
        let mut test = TestParser::new("pos.&var(y)");
        let mut parser = test.prepare();
        let expr_id = parser
            .eat_expression(ExpressionParserOptions::default())
            .unwrap();

        // pos.&var(y)
        assert_node!(parser.tree, expr_id, Expression::Reference { mutability, right } => {
            match mutability {
                ScopedMutability::Scoped { mutability, scopes } => {
                    assert_eq!(*mutability, Mutability::Mutable);
                    assert_eq!(scopes.len(), 1);
                    assert_path!(parser.session, scopes[0], "y");
                }
                _ => panic!("expected ScopedMutability::Scoped"),
            }
            assert_expr_path!(parser.session, parser.tree.get(*right), "pos");
        });
    }

    /// Reference operator on member access with method call.
    /// &var self.foo()
    #[test]
    fn test_parse_reference_member_call() {
        let mut test = TestParser::new("&var self.foo()");
        let mut parser = test.prepare();
        let expr_id = parser
            .eat_expression(ExpressionParserOptions::default())
            .unwrap();

        // &var self.foo()
        assert_node!(
            parser.tree,
            expr_id,
            Expression::Reference { mutability, right } => {
                // &var
                assert_eq!(*mutability, ScopedMutability::Unscoped { mutability: Mutability::Mutable });
                // self.foo()
                assert_node!(
                    parser.tree,
                    *right,
                    Expression::Call(call_id) => {
                        assert_node!(
                            parser.tree,
                            *call_id,
                            Call { runtime, receiver, static_arguments: _, dynamic_arguments: _ } => {
                                assert_eq!(*runtime, None);
                                // self.foo
                                assert_node!(
                                    parser.tree,
                                    *receiver,
                                    Expression::Path { path, static_arguments: _ } => {
                                        assert_path!(parser.session, *path, "self.foo");
                                    }
                                );
                            }
                        );
                    }
                );
            }
        );
    }

    /// Test parse mult-line let with multi-linx infix.
    /// let x =
    ///     foo.parse()
    ///         + 2
    ///         + (x / 4)
    #[test]
    fn test_parse_let_multiline_infix() {
        let mut test = TestParser::new(
            r"
let x = 
    foo.parse()
        + 2 
        + (x / 4)
",
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let expr_id = parser
            .eat_expression(ExpressionParserOptions::default())
            .unwrap();

        // let x = foo.parse() + 2 + (x / 4)
        assert_node!(
            parser.tree,
            expr_id,
            Expression::Let(let_id) => {
                assert_node!(
                    parser.tree,
                    *let_id,
                    Let { mutability, pattern, r#type: _, value, visibility: _, .. } => {
                        assert_eq!(*mutability, ScopedMutability::Unscoped { mutability: Mutability::Immutable });
                        // x
                        assert_node!(
                            parser.tree,
                            *pattern,
                            Pattern::Binding { name, .. } => {
                                assert_eq!(parser.session.get_string(*name), "x");
                            }
                        );

                        // foo.parse() + 2 + (x / 4)
                        assert_node!(
                            parser.tree,
                            value.unwrap(),
                            Expression::Binary { left, operator, right } => {
                                assert_eq!(*operator, BinaryOperator::Add);
                                // foo.parse() + 2
                                assert_node!(
                                    parser.tree,
                                    *left,
                                    Expression::Binary { left, operator, right } => {
                                        assert_eq!(*operator, BinaryOperator::Add);
                                        // foo.parse()
                                        assert_node!(
                                            parser.tree,
                                            *left,
                                            Expression::Call(call_id) => {
                                                assert_node!(
                                                    parser.tree,
                                                    *call_id,
                                                    Call { runtime, receiver, static_arguments: _, dynamic_arguments: _ } => {
                                                        assert_eq!(*runtime, None);
                                                        // foo.parse
                                                        assert_node!(
                                                            parser.tree,
                                                            *receiver,
                                                            Expression::Path { path, static_arguments: _ } => {
                                                                assert_path!(parser.session, *path, "foo.parse");
                                                            }
                                                        );
                                                    }
                                                );
                                            }
                                        );
                                        // 2
                                        assert_node!(
                                            parser.tree,
                                            *right,
                                            Expression::ScalarLiteral(scalar_id) => {
                                                assert_node!(
                                                    parser.tree,
                                                    *scalar_id,
                                                    ScalarLiteral::Integer(value, _) => {
                                                        assert_eq!(*value, 2);
                                                    }
                                                );
                                            }
                                        );
                                    }
                                );
                                // (x / 4)
                                assert_node!(
                                    parser.tree,
                                    *right,
                                    Expression::Binary { left, operator, right } => {
                                        assert_eq!(*operator, BinaryOperator::Divide);
                                        // x
                                        assert_expr_path!(parser.session, parser.tree.get(*left), "x");
                                        // 4
                                        assert_node!(
                                            parser.tree,
                                            *right,
                                            Expression::ScalarLiteral(scalar_id) => {
                                                assert_node!(
                                                    parser.tree,
                                                    *scalar_id,
                                                    ScalarLiteral::Integer(value, _) => {
                                                        assert_eq!(*value, 4);
                                                    }
                                                );
                                            }
                                        );
                                    }
                                );
                            }
                        );
                    }
                );
            }
        );
    }

    /// Reference operator on member access with method call.
    /// self
    ///    .foo()
    ///    .baz()
    #[test]
    fn test_parse_member_access_multiline() {
        let mut test = TestParser::new(
            r"
self
    .foo()
    .baz()
",
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let expr_id = parser
            .eat_expression(ExpressionParserOptions::default())
            .unwrap();

        // self.foo().baz()
        assert_node!(
            parser.tree,
            expr_id,
            Expression::Call(call_id) => {
                assert_node!(
                    parser.tree,
                    *call_id,
                    Call { runtime, receiver, static_arguments: _, dynamic_arguments: _ } => {
                        assert_eq!(*runtime, None);
                        // self.foo().baz
                        assert_node!(
                            parser.tree,
                            *receiver,
                            Expression::Member { receiver, path } => {
                                // self.foo()
                                assert_node!(
                                    parser.tree,
                                    *receiver,
                                    Expression::Call(call_id) => {
                                        assert_node!(
                                            parser.tree,
                                            *call_id,
                                            Call { runtime, receiver, static_arguments: _, dynamic_arguments: _ } => {
                                                assert_eq!(*runtime, None);
                                                // self.foo
                                                assert_node!(
                                                    parser.tree,
                                                    *receiver,
                                                    Expression::Member { receiver, path } => {
                                                        // self
                                                        assert_expr_path!(parser.session, parser.tree.get(*receiver), "self");
                                                        // foo
                                                        assert_path!(parser.session, *path, "foo");
                                                    }
                                                );
                                            }
                                        );
                                    }
                                );
                                // baz
                                assert_path!(parser.session, *path, "baz");
                            }
                        );
                    }
                );
            }
        );
    }

    /// Addition is left associative.
    /// a + b + c
    /// => ((a + b) + c)
    #[test]
    fn test_parse_precedence_addition_left_associative() {
        let mut test = TestParser::new("a + b + c");
        let mut parser = test.prepare();
        let expr_id = parser
            .eat_expression(ExpressionParserOptions::default())
            .unwrap();

        assert_node!(
            parser.tree,
            expr_id,
            // ((a + b) + c)
            Expression::Binary { left, operator, right } => {
                assert_eq!(*operator, BinaryOperator::Add);
                assert_node!(
                    parser.tree,
                    *left,
                    // (a + b)
                    Expression::Binary { left, operator, right } => {
                        // a
                        assert_expr_path!(parser.session, parser.tree.get(*left), "a");
                        // +
                        assert_eq!(*operator, BinaryOperator::Add);
                        // b
                        assert_expr_path!(parser.session, parser.tree.get(*right), "b");
                    }
                );
                // c
                assert_expr_path!(parser.session, parser.tree.get(*right), "c");
            }
        );
    }

    /// Infix operators work across lines.
    /// a +
    /// b +
    /// c
    /// => ((a + b) + c)
    #[test]
    fn test_parse_precedence_addition_across_lines() {
        let mut test = TestParser::new("a +\n b +\n c");
        let mut parser = test.prepare();
        let expr_id = parser
            .eat_expression(ExpressionParserOptions::default())
            .unwrap();

        assert_node!(
            parser.tree,
            expr_id,
            // ((a + b) + c)
            Expression::Binary { left, operator, right } => {
                assert_eq!(*operator, BinaryOperator::Add);
                assert_node!(
                    parser.tree,
                    *left,
                    // (a + b)
                    Expression::Binary { left, operator, right } => {
                        // a
                        assert_expr_path!(parser.session, parser.tree.get(*left), "a");
                        // +
                        assert_eq!(*operator, BinaryOperator::Add);
                        // b
                        assert_expr_path!(parser.session, parser.tree.get(*right), "b");
                    }
                );
                // c
                assert_expr_path!(parser.session, parser.tree.get(*right), "c");
            }
        );
    }

    /// Multiplication has higher precedence than addition.
    /// a + b * c
    /// => (a + (b * c))
    #[test]
    fn test_parse_precedence_multiply_before_addition() {
        let mut test = TestParser::new("a + b * c");
        let mut parser = test.prepare();
        let expr_id = parser
            .eat_expression(ExpressionParserOptions::default())
            .unwrap();

        assert_node!(
            parser.tree,
            expr_id,
            // (a + (b * c))
            Expression::Binary { left, operator, right } => {
                // +
                assert_eq!(*operator, BinaryOperator::Add);
                // a
                assert_expr_path!(parser.session, parser.tree.get(*left), "a");
                assert_node!(
                    parser.tree,
                    *right,
                    // (b * c)
                    Expression::Binary { left, operator, right } => {
                        // *
                        assert_eq!(*operator, BinaryOperator::Multiply);
                        // b
                        assert_expr_path!(parser.session, parser.tree.get(*left), "b");
                        // c
                        assert_expr_path!(parser.session, parser.tree.get(*right), "c");
                    }
                );
            }
        );
    }

    /// Parentheses override operator precedence.
    /// (a + b) * c
    /// => ((a + b) * c)
    #[test]
    fn test_parse_precedence_parentheses_override() {
        let mut test = TestParser::new("(a + b) * c");
        let mut parser = test.prepare();
        let expr_id = parser
            .eat_expression(ExpressionParserOptions::default())
            .unwrap();

        assert_node!(
            parser.tree,
            expr_id,
            // ((a + b) * c)
            Expression::Binary { left, operator, right } => {
                // *
                assert_eq!(*operator, BinaryOperator::Multiply);
                assert_node!(
                    parser.tree,
                    *left,
                    // (a + b)
                    Expression::Binary { left, operator, right } => {
                        // +
                        assert_eq!(*operator, BinaryOperator::Add);
                        // a
                        assert_expr_path!(parser.session, parser.tree.get(*left), "a");
                        // b
                        assert_expr_path!(parser.session, parser.tree.get(*right), "b");
                    }
                );
                // c
                assert_expr_path!(parser.session, parser.tree.get(*right), "c");
            }
        );
    }

    /// Mixed precedence chain with addition and multiplication.
    /// a + b * c + d
    /// => ((a + (b * c)) + d)
    #[test]
    fn test_parse_precedence_chain_mixed() {
        let mut test = TestParser::new("a + b * c + d");
        let mut parser = test.prepare();
        let expr_id = parser
            .eat_expression(ExpressionParserOptions::default())
            .unwrap();

        assert_node!(
            parser.tree,
            expr_id,
            // ((a + (b * c)) + d)
            Expression::Binary { left, operator, right } => {
                // +
                assert_eq!(*operator, BinaryOperator::Add);
                assert_node!(
                    parser.tree,
                    *left,
                    // (a + (b * c))
                    Expression::Binary { left, operator, right } => {
                        // +
                        assert_eq!(*operator, BinaryOperator::Add);
                        // a
                        assert_expr_path!(parser.session, parser.tree.get(*left), "a");
                        assert_node!(
                            parser.tree,
                            *right,
                            // (b * c)
                            Expression::Binary { left, operator, right } => {
                                // *
                                assert_eq!(*operator, BinaryOperator::Multiply);
                                // b
                                assert_expr_path!(parser.session, parser.tree.get(*left), "b");
                                // c
                                assert_expr_path!(parser.session, parser.tree.get(*right), "c");
                            }
                        );
                    }
                );
                // d
                assert_expr_path!(parser.session, parser.tree.get(*right), "d");
            }
        );
    }

    /// Addition has higher precedence than elementwise or.
    /// a + b | c + d
    /// => ((a + b) | (c + d))
    #[test]
    fn test_parse_precedence_elementwise_vs_addition() {
        let mut test = TestParser::new("a + b | c + d");
        let mut parser = test.prepare();
        let expr_id = parser
            .eat_expression(ExpressionParserOptions::default())
            .unwrap();

        assert_node!(
            parser.tree,
            expr_id,
            // ((a + b) | (c + d))
            Expression::Binary { left, operator, right } => {
                // |
                assert_eq!(*operator, BinaryOperator::ElementwiseOr);
                assert_node!(
                    parser.tree,
                    *left,
                    // (a + b)
                    Expression::Binary { left, operator, right } => {
                        // +
                        assert_eq!(*operator, BinaryOperator::Add);
                        // a
                        assert_expr_path!(parser.session, parser.tree.get(*left), "a");
                        // b
                        assert_expr_path!(parser.session, parser.tree.get(*right), "b");
                    }
                );
                assert_node!(
                    parser.tree,
                    *right,
                    // (c + d)
                    Expression::Binary { left, operator, right } => {
                        // +
                        assert_eq!(*operator, BinaryOperator::Add);
                        // c
                        assert_expr_path!(parser.session, parser.tree.get(*left), "c");
                        // d
                        assert_expr_path!(parser.session, parser.tree.get(*right), "d");
                    }
                );
            }
        );
    }

    /// Comparison has higher precedence than logical and.
    /// a == b && c == d
    /// => ((a == b) && (c == d))
    #[test]
    fn test_parse_precedence_comparison_vs_logical() {
        let mut test = TestParser::new("a == b && c == d");
        let mut parser = test.prepare();
        let expr_id = parser
            .eat_expression(ExpressionParserOptions::default())
            .unwrap();

        assert_node!(
            parser.tree,
            expr_id,
            // ((a == b) && (c == d))
            Expression::Binary { left, operator, right } => {
                // &&
                assert_eq!(*operator, BinaryOperator::And);
                assert_node!(
                    parser.tree,
                    *left,
                    // (a == b)
                    Expression::Binary { left, operator, right } => {
                        // ==
                        assert_eq!(*operator, BinaryOperator::Equal);
                        // a
                        assert_expr_path!(parser.session, parser.tree.get(*left), "a");
                        // b
                        assert_expr_path!(parser.session, parser.tree.get(*right), "b");
                    }
                );
                assert_node!(
                    parser.tree,
                    *right,
                    // (c == d)
                    Expression::Binary { left, operator, right } => {
                        // ==
                        assert_eq!(*operator, BinaryOperator::Equal);
                        // c
                        assert_expr_path!(parser.session, parser.tree.get(*left), "c");
                        // d
                        assert_expr_path!(parser.session, parser.tree.get(*right), "d");
                    }
                );
            }
        );
    }

    /// Unary prefix has higher precedence than multiplication.
    /// -a * b
    /// => ((-a) * b)
    #[test]
    fn test_parse_precedence_unary_before_multiply() {
        let mut test = TestParser::new("-a * b");
        let mut parser = test.prepare();
        let expr_id = parser
            .eat_expression(ExpressionParserOptions::default())
            .unwrap();

        assert_node!(
            parser.tree,
            expr_id,
            // ((-a) * b)
            Expression::Binary { left, operator, right } => {
                // *
                assert_eq!(*operator, BinaryOperator::Multiply);
                assert_node!(
                    parser.tree,
                    *left,
                    // (-a)
                    Expression::Unary { operator: _, right } => {
                        // a
                        assert_expr_path!(parser.session, parser.tree.get(*right), "a");
                    }
                );
                // b
                assert_expr_path!(parser.session, parser.tree.get(*right), "b");
            }
        );
    }

    /// Postfix call has higher precedence than addition.
    /// Static calls are right associative.
    /// a() + @b() / c
    /// => ((a()) + ((@b()) / c))
    #[test]
    fn test_parse_precedence_postfix_call_before_add() {
        let mut test = TestParser::new("a() + @b() / c");
        let mut parser = test.prepare();
        let expr_id = parser
            .eat_expression(ExpressionParserOptions::default())
            .unwrap();

        assert_node!(
            parser.tree,
            expr_id,
            // ((a()) + ((@b()) / b))
            Expression::Binary { left, operator, right } => {
                // +
                assert_eq!(*operator, BinaryOperator::Add);
                // (a())
                assert_node!(
                    parser.tree,
                    *left,
                    // a()
                    Expression::Call(call_id) => {
                        assert_node!(
                            parser.tree,
                            *call_id,
                            Call { runtime, receiver, static_arguments: _, dynamic_arguments: _ } => {
                                assert_eq!(*runtime, None);
                                // a
                                assert_expr_path!(parser.session, parser.tree.get(*receiver), "a");
                            }
                        );
                    }
                );
                // ((@b()) / c)
                assert_node!(
                    parser.tree,
                    *right,
                    Expression::Binary { left, operator, right } => {
                        // /
                        assert_eq!(*operator, BinaryOperator::Divide);
                        // (@b())
                        assert_node!(
                            parser.tree,
                            *left,
                            Expression::Call(call_id) => {
                                assert_node!(
                                    parser.tree,
                                    *call_id,
                                    Call { runtime, receiver, static_arguments: _, dynamic_arguments: _ } => {
                                        assert_eq!(*runtime, Some(Runtime::Static));
                                        // b
                                        assert_expr_path!(parser.session, parser.tree.get(*receiver), "b");
                                    }
                                );
                            }
                        );
                        // c
                        assert_expr_path!(parser.session, parser.tree.get(*right), "c");
                    }
                );
            }
        );
    }

    /// Combine postfix member access and call with coalesce.
    /// (y * y).sqrt() ?? 0
    /// => (((y * y).sqrt()) ?? 0)
    #[test]
    fn test_parse_precedence_postfix_call_before_coalesce() {
        let mut test = TestParser::new("(y * y).sqrt() ?? 0");
        let mut parser = test.prepare();
        let expr_id = parser
            .eat_expression(ExpressionParserOptions::default())
            .unwrap();

        assert_node!(
            parser.tree,
            expr_id,
            // (((y * y).sqrt()) ?? 0)
            Expression::Coalesce(coalesce_id) => {
                assert_node!(
                    parser.tree,
                    *coalesce_id,
                    Coalesce { receiver, default } => {
                        // ((y * y).sqrt())
                        assert_node!(parser.tree, *receiver, Expression::Call(call_id) => {
                            assert_node!(parser.tree, *call_id, Call { receiver, .. } => {
                                // ((y * y).sqrt())
                                assert_node!(parser.tree, *receiver, Expression::Member { receiver, path } => {
                                    // sqrt
                                    assert_path!(parser.session, *path, "sqrt");
                                    // (y * y)
                                    assert_node!(parser.tree, *receiver, Expression::Binary { left, operator, right } => {
                                        // *
                                        assert_eq!(*operator, BinaryOperator::Multiply);
                                        // y
                                        assert_expr_path!(parser.session, parser.tree.get(*left), "y");
                                        // y
                                        assert_expr_path!(parser.session, parser.tree.get(*right), "y");
                                    });
                                });
                            });
                        });

                        // 0
                        assert_node!(
                            parser.tree,
                            *default,
                            Expression::ScalarLiteral(scalar_id) => {
                                assert_node!(
                                    parser.tree,
                                    *scalar_id,
                                    ScalarLiteral::Integer(0, _)
                                );
                            }
                        );
                    }
                );
            }
        );
    }
}
