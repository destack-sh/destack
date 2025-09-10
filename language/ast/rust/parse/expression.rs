//! Parse expressions. Mostly defers to other parsers.

use dyst_language_token::TokenType;

use crate::{
    AssignOperator, BinaryOperator, Expression, InfixOperator, Keyword, NodeId, OperatorPrecedence,
    ParseError, ParseResult, Parser, ParserMark, Runtime, TupleLiteral, UnaryOperator, Visibility,
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

            // bitwise
            BinaryOperator::BitwiseAnd => OperatorPrecedence::Bitwise,
            BinaryOperator::BitwiseXor => OperatorPrecedence::Bitwise,
            BinaryOperator::BitwiseOr => OperatorPrecedence::Bitwise,

            // comparison
            BinaryOperator::Equal => OperatorPrecedence::Comparison,
            BinaryOperator::NotEqual => OperatorPrecedence::Comparison,
            BinaryOperator::LessThan => OperatorPrecedence::Comparison,
            BinaryOperator::LessThanOrEqual => OperatorPrecedence::Comparison,
            BinaryOperator::GreaterThan => OperatorPrecedence::Comparison,
            BinaryOperator::GreaterThanOrEqual => OperatorPrecedence::Comparison,

            // logical
            BinaryOperator::LogicalAnd => OperatorPrecedence::Logical,
            BinaryOperator::LogicalOr => OperatorPrecedence::Logical,
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

            // bitwise
            TokenType::BitwiseAnd => Some(BinaryOperator::BitwiseAnd),
            TokenType::BitwiseXor => Some(BinaryOperator::BitwiseXor),
            TokenType::BitwiseOr => Some(BinaryOperator::BitwiseOr),

            // comparison
            TokenType::Equal => Some(BinaryOperator::Equal),
            TokenType::NotEqual => Some(BinaryOperator::NotEqual),
            TokenType::LessThan => Some(BinaryOperator::LessThan),
            TokenType::LessThanOrEqual => Some(BinaryOperator::LessThanOrEqual),
            TokenType::GreaterThan => Some(BinaryOperator::GreaterThan),
            TokenType::GreaterThanOrEqual => Some(BinaryOperator::GreaterThanOrEqual),

            // logical
            TokenType::LogicalAnd => Some(BinaryOperator::LogicalAnd),
            TokenType::LogicalOr => Some(BinaryOperator::LogicalOr),

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

            // bitwise
            BinaryOperator::BitwiseAnd => TokenType::BitwiseAnd,
            BinaryOperator::BitwiseXor => TokenType::BitwiseXor,
            BinaryOperator::BitwiseOr => TokenType::BitwiseOr,

            // comparison
            BinaryOperator::Equal => TokenType::Equal,
            BinaryOperator::NotEqual => TokenType::NotEqual,
            BinaryOperator::LessThan => TokenType::LessThan,
            BinaryOperator::LessThanOrEqual => TokenType::LessThanOrEqual,
            BinaryOperator::GreaterThan => TokenType::GreaterThan,
            BinaryOperator::GreaterThanOrEqual => TokenType::GreaterThanOrEqual,

            // logical
            BinaryOperator::LogicalAnd => TokenType::LogicalAnd,
            BinaryOperator::LogicalOr => TokenType::LogicalOr,
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

    /// Convert a TokenType to a UnaryOperator (if a direct mapping exists).
    #[inline]
    pub fn from_token_type(token_type: TokenType) -> Option<UnaryOperator> {
        match token_type {
            TokenType::Not => Some(UnaryOperator::LogicalNot),
            TokenType::Subtract => Some(UnaryOperator::Negate),
            TokenType::WrappingSubtract => Some(UnaryOperator::WrappingNegate),
            TokenType::Multiply => Some(UnaryOperator::Dereference),
            TokenType::BitwiseAnd => Some(UnaryOperator::Reference),
            TokenType::BitwiseNot => Some(UnaryOperator::BitwiseNot),
            _ => None,
        }
    }

    /// Convert a UnaryOperator to a TokenType (if a direct mapping exists).
    #[inline]
    pub fn as_token_type(&self) -> TokenType {
        match self {
            UnaryOperator::LogicalNot => TokenType::Not,
            UnaryOperator::Negate => TokenType::Subtract,
            UnaryOperator::WrappingNegate => TokenType::WrappingSubtract,
            UnaryOperator::Dereference => TokenType::Multiply,
            UnaryOperator::Reference => TokenType::BitwiseAnd,
            UnaryOperator::BitwiseNot => TokenType::BitwiseNot,
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

            // assignment bitwise
            AssignOperator::BitwiseAndAssign
            | AssignOperator::BitwiseXorAssign
            | AssignOperator::BitwiseOrAssign => OperatorPrecedence::AssignmentBitwise,

            // assignment logical
            AssignOperator::LogicalAndAssign | AssignOperator::LogicalOrAssign => {
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

            // bitwise
            TokenType::BitwiseAndAssign => Some(AssignOperator::BitwiseAndAssign),
            TokenType::BitwiseOrAssign => Some(AssignOperator::BitwiseOrAssign),
            TokenType::BitwiseXorAssign => Some(AssignOperator::BitwiseXorAssign),

            // logical
            TokenType::LogicalAndAssign => Some(AssignOperator::LogicalAndAssign),
            TokenType::LogicalOrAssign => Some(AssignOperator::LogicalOrAssign),

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

            // bitwise
            AssignOperator::BitwiseAndAssign => TokenType::BitwiseAndAssign,
            AssignOperator::BitwiseOrAssign => TokenType::BitwiseOrAssign,
            AssignOperator::BitwiseXorAssign => TokenType::BitwiseXorAssign,

            // logical
            AssignOperator::LogicalAndAssign => TokenType::LogicalAndAssign,
            AssignOperator::LogicalOrAssign => TokenType::LogicalOrAssign,
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
    BinaryOperator::LogicalAnd,
    BinaryOperator::LogicalOr,
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
    /// Whether we're parsing an expression followed by a block (like in if, match).
    /// We disallow struct literals at the root level in these cases to avoid ambiguity with expr {}.
    pub is_before_block: bool = false,
    /// The left precedence preceding (i.e. before) the expression. 
    /// Determines AST structure.
    pub left_precedence: Option<u8> = None,
    /// The visibility of this expression.
    /// Used when pre-snacking the visibility in an outer parse (like for statements).
    pub visibility: Option<Visibility> = None,
}

impl<'a> Parser<'a> {
    /// Peek a unary operator.
    #[inline]
    pub fn peek_unary_operator(&self) -> ParseResult<UnaryOperator> {
        let token = self.peek()?;
        UnaryOperator::from_token_type(token.token.r#type).ok_or(ParseError::expected_token(
            token.span,
            TokenType::Identifier,
        ))
    }

    /// Peek a binary operator.
    #[inline]
    pub fn peek_binary_operator(&self) -> ParseResult<BinaryOperator> {
        if self.options.in_static_type
            && !IN_STATIC_TYPE_BINARY_OPERATORS
                .contains(&BinaryOperator::from_token_type(self.peek()?.token.r#type).unwrap())
        {
            return Err(ParseError::unexpected(self.peek()?.span));
        }
        let token = self.peek()?;
        BinaryOperator::from_token_type(token.token.r#type)
            .ok_or(ParseError::unexpected(token.span))
    }

    /// Peek an assign operator.
    #[inline]
    pub fn peek_assign_operator(&self) -> ParseResult<AssignOperator> {
        if self.options.in_static_type {
            return Err(ParseError::unexpected(self.peek()?.span));
        }
        let token = self.peek()?;
        AssignOperator::from_token_type(token.token.r#type)
            .ok_or(ParseError::unexpected(token.span))
    }

    /// Peek an infix operator.
    #[inline]
    pub fn peek_infix_operator(&self) -> ParseResult<InfixOperator> {
        let token = self.peek()?;
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
                let start = ParserMark::new(err.span.start as usize);
                self.try_recover(start, recover)?;
                let error_id = self
                    .tree
                    .allocate(Expression::Error(err), self.get_span_from(start));
                Ok(error_id)
            }
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
        let runtime = if self.peek_token(TokenType::At).is_ok() {
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
            //
            // ------------------------------------------------------------
            // Declarations
            // ------------------------------------------------------------
            //

            // module
            else if keyword == Some(Keyword::Module) {
                let module_id = self.eat_module(visibility)?;
                let expression = Expression::Module(module_id);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            // struct
            else if keyword == Some(Keyword::Struct) {
                let struct_id = self.eat_struct(visibility)?;
                let expression = Expression::Struct(struct_id);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            // enum
            else if keyword == Some(Keyword::Enum) {
                let enum_id = self.eat_enum(visibility)?;
                let expression = Expression::Enum(enum_id);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            // union
            else if keyword == Some(Keyword::Union) {
                let union_id = self.eat_union(visibility)?;
                let expression = Expression::Union(union_id);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            // trait
            else if keyword == Some(Keyword::Trait) {
                let trait_id = self.eat_trait(visibility)?;
                let expression = Expression::Trait(trait_id);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            // implement
            else if keyword == Some(Keyword::Implement) {
                let implement_id = self.eat_implement()?;
                let expression = Expression::Implement(implement_id);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            // function
            else if keyword == Some(Keyword::Function) {
                let function_id = self.eat_function(visibility)?;
                let expression = Expression::Function(function_id);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            //
            // ------------------------------------------------------------
            // Control flow
            // ------------------------------------------------------------
            //
            // if
            else if keyword == Some(Keyword::If) {
                let if_id = self.eat_if()?;
                let expression = Expression::If(if_id);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            // while
            else if keyword == Some(Keyword::While) {
                let while_id = self.eat_while()?;
                let expression = Expression::While(while_id);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            // for
            else if keyword == Some(Keyword::For) {
                let for_id = self.eat_for()?;
                let expression = Expression::For(for_id);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            // loop
            else if keyword == Some(Keyword::Loop) {
                let loop_id = self.eat_loop()?;
                let expression = Expression::Loop(loop_id);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            // break
            else if keyword == Some(Keyword::Break) {
                let break_id = self.eat_break()?;
                let expression = Expression::Break(break_id);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            // continue
            else if keyword == Some(Keyword::Continue) {
                let continue_id = self.eat_continue()?;
                let expression = Expression::Continue(continue_id);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            // defer
            else if keyword == Some(Keyword::Defer) {
                let defer_id = self.eat_defer()?;
                let expression = Expression::Defer(defer_id);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            // return
            else if keyword == Some(Keyword::Return) {
                let return_id = self.eat_return()?;
                let expression = Expression::Return(return_id);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            // try
            else if keyword == Some(Keyword::Try) {
                let try_id = self.eat_try_catch()?;
                let expression = Expression::Try(try_id);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            // match
            else if keyword == Some(Keyword::Match) {
                let match_id = self.eat_match()?;
                let expression = Expression::Match(match_id);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            //
            // ------------------------------------------------------------
            // Bindings / Literals / Aliases
            // ------------------------------------------------------------
            //
            // let
            else if keyword == Some(Keyword::Let) || keyword == Some(Keyword::Var) {
                let let_id = self.eat_let_or_var(visibility)?;
                let expression = Expression::Let(let_id);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            // array
            else if token.token.r#type == TokenType::OpenBracket {
                let array_literal = self.eat_array_literal()?;
                let expression = Expression::ArrayLiteral(array_literal);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            // scalar
            else if self.peek_scalar_literal().is_ok() {
                let scalar_literal = self.eat_scalar_literal()?;
                let expression = Expression::ScalarLiteral(scalar_literal);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            // struct
            else if !options.is_before_block && self.peek_struct_literal().is_ok() {
                let struct_literal = self.eat_struct_literal()?;
                let expression = Expression::StructLiteral(struct_literal);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            // alias / path
            else if token.token.r#type == TokenType::Identifier {
                let path_id = self.eat_path()?;
                let expression = Expression::Path(path_id);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            //
            // ------------------------------------------------------------
            // Documentation
            // ------------------------------------------------------------
            //
            // doc
            else if token.token.r#type == TokenType::DocLineComment
                || token.token.r#type == TokenType::DocBlockComment
            {
                let doc_id = self.eat_doc()?;
                let expression = Expression::Doc(doc_id);
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

        // eat all postfix operations
        loop {
            // member
            if self.peek_token(TokenType::Dot).is_ok() {
                self.bump(); // eat dot
                let path_id = self.eat_path()?;
                let expression = Expression::Member {
                    receiver: left_expression_id,
                    path: path_id,
                };
                left_expression_id = self.tree.allocate(expression, self.get_span_from(start));
            }
            // index
            else if self.peek_token(TokenType::OpenBracket).is_ok() {
                let index_id = self.eat_index_postfix(left_expression_id)?;
                let expression = Expression::Index(index_id);
                left_expression_id = self.tree.allocate(expression, self.get_span_from(start));
            }
            // call
            else if self.peek_token(TokenType::OpenParenthesis).is_ok() {
                let call_id =
                    self.eat_call_postfix(left_expression_id, runtime.unwrap_or(Runtime::Dynamic))?;
                let expression = Expression::Call(call_id);
                left_expression_id = self.tree.allocate(expression, self.get_span_from(start));
            }
            // cast
            else if self.peek_keyword(Keyword::As).is_ok() {
                let cast_id = self.eat_as_postfix(left_expression_id)?;
                let expression = Expression::Cast(cast_id);
                left_expression_id = self.tree.allocate(expression, self.get_span_from(start));
            }
            // coalesce
            else if self.peek_token(TokenType::Coalesce).is_ok() {
                let coalesce_id = self.eat_coalesce_postfix(left_expression_id)?;
                let expression = Expression::Coalesce(coalesce_id);
                left_expression_id = self.tree.allocate(expression, self.get_span_from(start));
            }
            // unwrap
            else if self.peek_token(TokenType::Maybe).is_ok() {
                self.bump(); // eat question
                let expression = Expression::Unwrap(left_expression_id);
                left_expression_id = self.tree.allocate(expression, self.get_span_from(start));
            }
            // tuple (maybe)
            else if options.is_parenthesized && self.peek_token(TokenType::Comma).is_ok() {
                self.bump(); // eat comma
                self.eat_newlines_maybe()?;
                let tuple_elements = self.eat_tuple_literal_body(left_expression_id)?;
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
        while let Ok(right_operator) = self.peek_infix_operator()
            && (options.left_precedence.is_none()
                || options.left_precedence.unwrap() < right_operator.precedence())
        {
            self.bump(); // eat infix operator
            let right_expression_id = self.eat_expression(ExpressionParserOptions {
                left_precedence: Some(right_operator.precedence()),
                ..options
            })?;
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
        BinaryOperator, Call, Coalesce, Expression, FieldLiteral, Runtime, ScalarLiteral,
        StructLiteral, TupleLiteral, Type, assert_int, assert_node, assert_path,
    };

    /// Tuple literals are disambiguated.
    /// (1, 2)
    #[test]
    fn test_parse_tuple_literal() {
        let test = TestParser::new("(1, 2)");
        let mut parser = test.parser();
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

    /// Struct literals are disambiguated.
    /// geom.Vector2 { x: 1, y }
    #[test]
    fn test_parse_struct_literal_path() {
        let test = TestParser::new("geom.Vector2 { x: 1, y }");
        let mut parser = test.parser();

        let expr_id = parser
            .eat_expression(ExpressionParserOptions::default())
            .unwrap();
        let geom = parser.strings.intern("geom");
        let vector2 = parser.strings.intern("Vector2");
        let x = parser.strings.intern("x");
        let y = parser.strings.intern("y");

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
                                let p = parser.paths.get(*path);
                                assert_eq!(p.segments.len(), 2);
                                assert_eq!(p.segments[0], geom);
                                assert_eq!(p.segments[1], vector2);
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
                                assert_eq!(*name, x);
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
                                assert_eq!(*name, y);
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
        let test = TestParser::new(
            r##"
geom.Mesh<2, Dims: 4> { 
    vertices: [1, 2]
    y
}"##,
        );
        let mut parser = test.parser();
        parser.eat_newline().unwrap();

        let geom = parser.strings.intern("geom");
        let mesh = parser.strings.intern("Mesh");
        let vertices = parser.strings.intern("vertices");
        let y = parser.strings.intern("y");

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
                        assert_eq!(*path, parser.paths.intern(vec![geom, mesh]));
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
                        assert_eq!(*name, vertices);
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
                        assert_eq!(*name, y);
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
        let test = TestParser::new("a + b + c");
        let mut parser = test.parser();
        let expr_id = parser
            .eat_expression(ExpressionParserOptions::default())
            .unwrap();

        let a = parser.strings.intern("a");
        let b = parser.strings.intern("b");
        let c = parser.strings.intern("c");

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
                        assert_path!(parser.tree, *left, a, using |path_id| {
                            let p = parser.paths.get(path_id);
                            assert_eq!(p.segments.len(), 1);
                            p.segments[0]
                        });
                        // +
                        assert_eq!(*operator, BinaryOperator::Add);
                        // b
                        assert_path!(parser.tree, *right, b, using |path_id| {
                            let p = parser.paths.get(path_id);
                            assert_eq!(p.segments.len(), 1);
                            p.segments[0]
                        });
                    }
                );
                // c
                assert_path!(parser.tree, *right, c, using |path_id| {
                    let p = parser.paths.get(path_id);
                    assert_eq!(p.segments.len(), 1);
                    p.segments[0]
                });
            }
        );
    }

    /// Multiplication has higher precedence than addition.
    /// a + b * c
    /// => (a + (b * c))
    #[test]
    fn test_parse_precedence_multiply_before_addition() {
        let test = TestParser::new("a + b * c");
        let mut parser = test.parser();
        let expr_id = parser
            .eat_expression(ExpressionParserOptions::default())
            .unwrap();

        let a = parser.strings.intern("a");
        let b = parser.strings.intern("b");
        let c = parser.strings.intern("c");

        assert_node!(
            parser.tree,
            expr_id,
            // (a + (b * c))
            Expression::Binary { left, operator, right } => {
                // +
                assert_eq!(*operator, BinaryOperator::Add);
                // a
                assert_path!(parser.tree, *left, a, using |path_id| {
                    let p = parser.paths.get(path_id);
                    assert_eq!(p.segments.len(), 1);
                    p.segments[0]
                });
                assert_node!(
                    parser.tree,
                    *right,
                    // (b * c)
                    Expression::Binary { left, operator, right } => {
                        // *
                        assert_eq!(*operator, BinaryOperator::Multiply);
                        // b
                        assert_path!(parser.tree, *left, b, using |path_id| {
                            let p = parser.paths.get(path_id);
                            assert_eq!(p.segments.len(), 1);
                            p.segments[0]
                        });
                        // c
                        assert_path!(parser.tree, *right, c, using |path_id| {
                            let p = parser.paths.get(path_id);
                            assert_eq!(p.segments.len(), 1);
                            p.segments[0]
                        });
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
        let test = TestParser::new("(a + b) * c");
        let mut parser = test.parser();
        let expr_id = parser
            .eat_expression(ExpressionParserOptions::default())
            .unwrap();

        let a = parser.strings.intern("a");
        let b = parser.strings.intern("b");
        let c = parser.strings.intern("c");

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
                        assert_path!(parser.tree, *left, a, using |path_id| {
                            let p = parser.paths.get(path_id);
                            assert_eq!(p.segments.len(), 1);
                            p.segments[0]
                        });
                        // b
                        assert_path!(parser.tree, *right, b, using |path_id| {
                            let p = parser.paths.get(path_id);
                            assert_eq!(p.segments.len(), 1);
                            p.segments[0]
                        });
                    }
                );
                // c
                assert_path!(parser.tree, *right, c, using |path_id| {
                    let p = parser.paths.get(path_id);
                    assert_eq!(p.segments.len(), 1);
                    p.segments[0]
                });
            }
        );
    }

    /// Mixed precedence chain with addition and multiplication.
    /// a + b * c + d
    /// => ((a + (b * c)) + d)
    #[test]
    fn test_parse_precedence_chain_mixed() {
        let test = TestParser::new("a + b * c + d");
        let mut parser = test.parser();
        let expr_id = parser
            .eat_expression(ExpressionParserOptions::default())
            .unwrap();

        let a = parser.strings.intern("a");
        let b = parser.strings.intern("b");
        let c = parser.strings.intern("c");
        let d = parser.strings.intern("d");

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
                        assert_path!(parser.tree, *left, a, using |path_id| {
                            let p = parser.paths.get(path_id);
                            assert_eq!(p.segments.len(), 1);
                            p.segments[0]
                        });
                        assert_node!(
                            parser.tree,
                            *right,
                            // (b * c)
                            Expression::Binary { left, operator, right } => {
                                // *
                                assert_eq!(*operator, BinaryOperator::Multiply);
                                // b
                                assert_path!(parser.tree, *left, b, using |path_id| {
                                    let p = parser.paths.get(path_id);
                                    assert_eq!(p.segments.len(), 1);
                                    p.segments[0]
                                });
                                // c
                                assert_path!(parser.tree, *right, c, using |path_id| {
                                    let p = parser.paths.get(path_id);
                                    assert_eq!(p.segments.len(), 1);
                                    p.segments[0]
                                });
                            }
                        );
                    }
                );
                // d
                assert_path!(parser.tree, *right, d, using |path_id| {
                    let p = parser.paths.get(path_id);
                    assert_eq!(p.segments.len(), 1);
                    p.segments[0]
                });
            }
        );
    }

    /// Addition has higher precedence than bitwise or.
    /// a + b | c + d
    /// => ((a + b) | (c + d))
    #[test]
    fn test_parse_precedence_bitwise_vs_addition() {
        let test = TestParser::new("a + b | c + d");
        let mut parser = test.parser();
        let expr_id = parser
            .eat_expression(ExpressionParserOptions::default())
            .unwrap();

        let a = parser.strings.intern("a");
        let b = parser.strings.intern("b");
        let c = parser.strings.intern("c");
        let d = parser.strings.intern("d");

        assert_node!(
            parser.tree,
            expr_id,
            // ((a + b) | (c + d))
            Expression::Binary { left, operator, right } => {
                // |
                assert_eq!(*operator, BinaryOperator::BitwiseOr);
                assert_node!(
                    parser.tree,
                    *left,
                    // (a + b)
                    Expression::Binary { left, operator, right } => {
                        // +
                        assert_eq!(*operator, BinaryOperator::Add);
                        // a
                        assert_path!(parser.tree, *left, a, using |path_id| {
                            let p = parser.paths.get(path_id);
                            assert_eq!(p.segments.len(), 1);
                            p.segments[0]
                        });
                        // b
                        assert_path!(parser.tree, *right, b, using |path_id| {
                            let p = parser.paths.get(path_id);
                            assert_eq!(p.segments.len(), 1);
                            p.segments[0]
                        });
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
                        assert_path!(parser.tree, *left, c, using |path_id| {
                            let p = parser.paths.get(path_id);
                            assert_eq!(p.segments.len(), 1);
                            p.segments[0]
                        });
                        // d
                        assert_path!(parser.tree, *right, d, using |path_id| {
                            let p = parser.paths.get(path_id);
                            assert_eq!(p.segments.len(), 1);
                            p.segments[0]
                        });
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
        let test = TestParser::new("a == b && c == d");
        let mut parser = test.parser();
        let expr_id = parser
            .eat_expression(ExpressionParserOptions::default())
            .unwrap();

        let a = parser.strings.intern("a");
        let b = parser.strings.intern("b");
        let c = parser.strings.intern("c");
        let d = parser.strings.intern("d");

        assert_node!(
            parser.tree,
            expr_id,
            // ((a == b) && (c == d))
            Expression::Binary { left, operator, right } => {
                // &&
                assert_eq!(*operator, BinaryOperator::LogicalAnd);
                assert_node!(
                    parser.tree,
                    *left,
                    // (a == b)
                    Expression::Binary { left, operator, right } => {
                        // ==
                        assert_eq!(*operator, BinaryOperator::Equal);
                        // a
                        assert_path!(parser.tree, *left, a, using |path_id| {
                            let p = parser.paths.get(path_id);
                            assert_eq!(p.segments.len(), 1);
                            p.segments[0]
                        });
                        // b
                        assert_path!(parser.tree, *right, b, using |path_id| {
                            let p = parser.paths.get(path_id);
                            assert_eq!(p.segments.len(), 1);
                            p.segments[0]
                        });
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
                        assert_path!(parser.tree, *left, c, using |path_id| {
                            let p = parser.paths.get(path_id);
                            assert_eq!(p.segments.len(), 1);
                            p.segments[0]
                        });
                        // d
                        assert_path!(parser.tree, *right, d, using |path_id| {
                            let p = parser.paths.get(path_id);
                            assert_eq!(p.segments.len(), 1);
                            p.segments[0]
                        });
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
        let test = TestParser::new("-a * b");
        let mut parser = test.parser();
        let expr_id = parser
            .eat_expression(ExpressionParserOptions::default())
            .unwrap();

        let a = parser.strings.intern("a");
        let b = parser.strings.intern("b");

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
                        assert_path!(parser.tree, *right, a, using |path_id| {
                            let p = parser.paths.get(path_id);
                            assert_eq!(p.segments.len(), 1);
                            p.segments[0]
                        });
                    }
                );
                // b
                assert_path!(parser.tree, *right, b, using |path_id| {
                    let p = parser.paths.get(path_id);
                    assert_eq!(p.segments.len(), 1);
                    p.segments[0]
                });
            }
        );
    }

    /// Postfix call has higher precedence than addition.
    /// Static calls are right associative.
    /// a() + @b() / c
    /// => ((a()) + ((@b()) / c))
    #[test]
    fn test_parse_precedence_postfix_call_before_add() {
        let test = TestParser::new("a() + @b() / c");
        let mut parser = test.parser();
        let expr_id = parser
            .eat_expression(ExpressionParserOptions::default())
            .unwrap();

        let a = parser.strings.intern("a");
        let b = parser.strings.intern("b");
        let c = parser.strings.intern("c");

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
                                assert_eq!(*runtime, Runtime::Dynamic);
                                // a
                                assert_path!(parser.tree, *receiver, a, using |path_id| {
                                    let path = parser.paths.get(path_id);
                                    assert_eq!(path.segments.len(), 1);
                                    path.segments[0]
                                });
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
                                        assert_eq!(*runtime, Runtime::Static);
                                        // b
                                        assert_path!(parser.tree, *receiver, b, using |path_id| {
                                            let path = parser.paths.get(path_id);
                                            assert_eq!(path.segments.len(), 1);
                                            path.segments[0]
                                        });
                                    }
                                );
                            }
                        );
                        // c
                        assert_path!(parser.tree, *right, c, using |path_id| {
                            let path = parser.paths.get(path_id);
                            assert_eq!(path.segments.len(), 1);
                            path.segments[0]
                        });
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
        let test = TestParser::new("(y * y).sqrt() ?? 0");
        let mut parser = test.parser();
        let expr_id = parser
            .eat_expression(ExpressionParserOptions::default())
            .unwrap();

        let y = parser.strings.intern("y");
        let sqrt = parser.strings.intern("sqrt");

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
                                    assert_eq!(*path, parser.paths.intern(vec![sqrt]));
                                    // (y * y)
                                    assert_node!(parser.tree, *receiver, Expression::Binary { left, operator, right } => {
                                        // *
                                        assert_eq!(*operator, BinaryOperator::Multiply);
                                        // y
                                        assert_path!(parser.tree, *left, y, using |path_id| {
                                            let path = parser.paths.get(path_id);
                                            assert_eq!(path.segments.len(), 1);
                                            path.segments[0]
                                        });
                                        // y
                                        assert_path!(parser.tree, *right, y, using |path_id| {
                                            let path = parser.paths.get(path_id);
                                            assert_eq!(path.segments.len(), 1);
                                            path.segments[0]
                                        });
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
