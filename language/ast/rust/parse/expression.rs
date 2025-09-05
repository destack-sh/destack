//! Parse expressions. Mostly defers to other parsers.

use destack_language_token::TokenType;

use crate::{
    AssignOperator, BinaryOperator, Expression, InfixOperator, Keyword, NodeId, OperatorPrecedence,
    ParseError, ParseResult, Parser, UnaryOperator,
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
    pub fn as_token_type(&self) -> Option<TokenType> {
        match self {
            // multiplication
            BinaryOperator::Multiply => Some(TokenType::Multiply),
            BinaryOperator::WrappingMultiply => Some(TokenType::WrappingMultiply),
            BinaryOperator::SaturatingMultiply => Some(TokenType::SaturatingMultiply),
            BinaryOperator::Divide => Some(TokenType::Divide),
            BinaryOperator::Remainder => Some(TokenType::Remainder),

            // addition
            BinaryOperator::Add => Some(TokenType::Add),
            BinaryOperator::WrappingAdd => Some(TokenType::WrappingAdd),
            BinaryOperator::SaturatingAdd => Some(TokenType::SaturatingAdd),
            BinaryOperator::Subtract => Some(TokenType::Subtract),
            BinaryOperator::WrappingSubtract => Some(TokenType::WrappingSubtract),
            BinaryOperator::SaturatingSubtract => Some(TokenType::SaturatingSubtract),

            // shift
            BinaryOperator::ShiftLeft => Some(TokenType::ShiftLeft),
            BinaryOperator::SaturatingShiftLeft => Some(TokenType::SaturatingShiftLeft),
            BinaryOperator::ShiftRight => Some(TokenType::ShiftRight),

            // bitwise
            BinaryOperator::BitwiseAnd => Some(TokenType::BitwiseAnd),
            BinaryOperator::BitwiseXor => Some(TokenType::BitwiseXor),
            BinaryOperator::BitwiseOr => Some(TokenType::BitwiseOr),

            // comparison
            BinaryOperator::Equal => Some(TokenType::Equal),
            BinaryOperator::NotEqual => Some(TokenType::NotEqual),
            BinaryOperator::LessThan => Some(TokenType::LessThan),
            BinaryOperator::LessThanOrEqual => Some(TokenType::LessThanOrEqual),
            BinaryOperator::GreaterThan => Some(TokenType::GreaterThan),
            BinaryOperator::GreaterThanOrEqual => Some(TokenType::GreaterThanOrEqual),

            // logical
            BinaryOperator::LogicalAnd => Some(TokenType::LogicalAnd),
            BinaryOperator::LogicalOr => Some(TokenType::LogicalOr),
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
            TokenType::Bang => Some(UnaryOperator::LogicalNot),
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
    pub fn as_token_type(&self) -> Option<TokenType> {
        match self {
            UnaryOperator::LogicalNot => Some(TokenType::Bang),
            UnaryOperator::Negate => Some(TokenType::Subtract),
            UnaryOperator::WrappingNegate => Some(TokenType::WrappingSubtract),
            UnaryOperator::Dereference => Some(TokenType::Multiply),
            UnaryOperator::Reference => Some(TokenType::BitwiseAnd),
            UnaryOperator::BitwiseNot => Some(TokenType::BitwiseNot),
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
    pub fn as_token_type(&self) -> Option<TokenType> {
        match self {
            AssignOperator::Assign => Some(TokenType::Assign),

            // addition
            AssignOperator::AddAssign => Some(TokenType::AddAssign),
            AssignOperator::WrappingAddAssign => Some(TokenType::WrappingAddAssign),
            AssignOperator::SaturatingAddAssign => Some(TokenType::SaturatingAddAssign),
            AssignOperator::SubtractAssign => Some(TokenType::SubtractAssign),
            AssignOperator::WrappingSubtractAssign => Some(TokenType::WrappingSubtractAssign),
            AssignOperator::SaturatingSubtractAssign => Some(TokenType::SaturatingSubtractAssign),

            // multiplication
            AssignOperator::MultiplyAssign => Some(TokenType::MultiplyAssign),
            AssignOperator::WrappingMultiplyAssign => Some(TokenType::WrappingMultiplyAssign),
            AssignOperator::SaturatingMultiplyAssign => Some(TokenType::SaturatingMultiplyAssign),
            AssignOperator::DivideAssign => Some(TokenType::DivideAssign),
            AssignOperator::RemainderAssign => Some(TokenType::RemainderAssign),

            // shift
            AssignOperator::ShiftLeftAssign => Some(TokenType::ShiftLeftAssign),
            AssignOperator::SaturatingShiftLeftAssign => Some(TokenType::SaturatingShiftLeftAssign),
            AssignOperator::ShiftRightAssign => Some(TokenType::ShiftRightAssign),

            // bitwise
            AssignOperator::BitwiseAndAssign => Some(TokenType::BitwiseAndAssign),
            AssignOperator::BitwiseOrAssign => Some(TokenType::BitwiseOrAssign),
            AssignOperator::BitwiseXorAssign => Some(TokenType::BitwiseXorAssign),

            // logical
            AssignOperator::LogicalAndAssign => Some(TokenType::LogicalAndAssign),
            AssignOperator::LogicalOrAssign => Some(TokenType::LogicalOrAssign),
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

impl<'a> Parser<'a> {
    /// Peek a unary operator.
    #[inline]
    pub fn peek_unary_operator(&self) -> ParseResult<UnaryOperator> {
        let token = self.peek()?;
        UnaryOperator::from_token_type(token.token.r#type)
            .ok_or(ParseError::UnexpectedToken(token.span))
    }

    /// Peek a binary operator.
    #[inline]
    pub fn peek_binary_operator(&self) -> ParseResult<BinaryOperator> {
        let token = self.peek()?;
        BinaryOperator::from_token_type(token.token.r#type)
            .ok_or(ParseError::UnexpectedToken(token.span))
    }

    /// Peek an assign operator.
    #[inline]
    pub fn peek_assign_operator(&self) -> ParseResult<AssignOperator> {
        let token = self.peek()?;
        AssignOperator::from_token_type(token.token.r#type)
            .ok_or(ParseError::UnexpectedToken(token.span))
    }

    /// Peek an infix operator.
    #[inline]
    pub fn peek_infix_operator(&self) -> ParseResult<InfixOperator> {
        let token = self.peek()?;
        if let Some(binary_operator) = BinaryOperator::from_token_type(token.token.r#type) {
            Ok(InfixOperator::Binary(binary_operator))
        } else if let Some(assign_operator) = AssignOperator::from_token_type(token.token.r#type) {
            Ok(InfixOperator::Assign(assign_operator))
        } else {
            Err(ParseError::UnexpectedToken(token.span))
        }
    }

    /// Make an expression from an infix operator.
    #[inline]
    fn make_infix_expression(
        &self,
        lhs: NodeId<Expression>,
        operator: InfixOperator,
        rhs: NodeId<Expression>,
    ) -> Expression {
        match operator {
            InfixOperator::Binary(binary_operator) => Expression::Binary {
                lhs,
                operator: binary_operator,
                rhs,
            },
            InfixOperator::Assign(assign_operator) => Expression::Assign {
                lhs,
                operator: assign_operator,
                rhs,
            },
        }
    }

    /// Eat an expression.
    pub fn eat_expression(
        &mut self,
        left_precedence: Option<u8>,
    ) -> ParseResult<NodeId<Expression>> {
        let start = self.mark();

        let mut left_expression_id: NodeId<Expression> = {
            //
            // ------------------------------------------------------------
            // Grouping
            // ------------------------------------------------------------
            //

            // parenthesis
            if self.peek_token(TokenType::OpenParenthesis).is_ok() {
                self.bump(); // eat open paranthesis
                // parse inner expression without outer precedence
                let expression_id = self.eat_expression(None)?;
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
                let rhs = self.eat_expression(Some(right_precedence))?;
                let expression = Expression::Unary {
                    operator: unary_operator,
                    rhs,
                };
                self.tree.allocate(expression, self.get_span_from(start))
            }
            //
            // ------------------------------------------------------------
            // Declarations
            // ------------------------------------------------------------
            //

            // module
            else if self.peek_keyword(Keyword::Module).is_ok() {
                let module_id = self.eat_module()?;
                let expression = Expression::Module(module_id);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            // struct
            else if self.peek_keyword(Keyword::Struct).is_ok() {
                let struct_id = self.eat_struct()?;
                let expression = Expression::Struct(struct_id);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            // enum
            else if self.peek_keyword(Keyword::Enum).is_ok() {
                let enum_id = self.eat_enum()?;
                let expression = Expression::Enum(enum_id);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            // union
            else if self.peek_keyword(Keyword::Union).is_ok() {
                let union_id = self.eat_union()?;
                let expression = Expression::Union(union_id);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            // trait
            else if self.peek_keyword(Keyword::Trait).is_ok() {
                let trait_id = self.eat_trait()?;
                let expression = Expression::Trait(trait_id);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            // implement
            else if self.peek_keyword(Keyword::Implement).is_ok() {
                let implement_id = self.eat_implement()?;
                let expression = Expression::Implement(implement_id);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            // function
            else if self.peek_keyword(Keyword::Function).is_ok() {
                let function_id = self.eat_function()?;
                let expression = Expression::Function(function_id);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            //
            // ------------------------------------------------------------
            // Control flow
            // ------------------------------------------------------------
            //
            // if
            else if self.peek_keyword(Keyword::If).is_ok() {
                let if_id = self.eat_if()?;
                let expression = Expression::If(if_id);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            // while
            else if self.peek_keyword(Keyword::While).is_ok() {
                let while_id = self.eat_while()?;
                let expression = Expression::While(while_id);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            // for
            else if self.peek_keyword(Keyword::For).is_ok() {
                let for_id = self.eat_for()?;
                let expression = Expression::For(for_id);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            // loop
            else if self.peek_keyword(Keyword::Loop).is_ok() {
                let loop_id = self.eat_loop()?;
                let expression = Expression::Loop(loop_id);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            // break
            else if self.peek_keyword(Keyword::Break).is_ok() {
                let break_id = self.eat_break()?;
                let expression = Expression::Break(break_id);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            // continue
            else if self.peek_keyword(Keyword::Continue).is_ok() {
                let continue_id = self.eat_continue()?;
                let expression = Expression::Continue(continue_id);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            // defer
            else if self.peek_keyword(Keyword::Defer).is_ok() {
                let defer_id = self.eat_defer()?;
                let expression = Expression::Defer(defer_id);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            // return
            else if self.peek_keyword(Keyword::Return).is_ok() {
                let return_id = self.eat_return()?;
                let expression = Expression::Return(return_id);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            // try
            else if self.peek_keyword(Keyword::Try).is_ok() {
                let try_id = self.eat_try_catch()?;
                let expression = Expression::Try(try_id);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            // match
            else if self.peek_keyword(Keyword::Match).is_ok() {
                let match_id = self.eat_match()?;
                let expression = Expression::Match(match_id);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            //
            // ------------------------------------------------------------
            // Literals / aliases
            // ------------------------------------------------------------
            //
            // array
            else if self.peek_token(TokenType::OpenBracket).is_ok() {
                let array_literal = self.eat_array_literal()?;
                let expression = Expression::ArrayLiteral(array_literal);
                self.tree.allocate(expression, self.get_span_from(start))
            // tuple
            } else if self.peek_token(TokenType::OpenParenthesis).is_ok() {
                let tuple_literal = self.eat_tuple_literal()?;
                let expression = Expression::TupleLiteral(tuple_literal);
                self.tree.allocate(expression, self.get_span_from(start))
            // todo!: parse struct literals (postfix to avoid unbounded lookahead?)
            //  (also for patterns?)
            // scalar
            } else if self.peek_scalar_literal().is_ok() {
                let scalar_literal = self.eat_scalar_literal()?;
                let expression = Expression::ScalarLiteral(scalar_literal);
                self.tree.allocate(expression, self.get_span_from(start))
            // alias / path
            } else if self.peek_identifier().is_ok() {
                let path_id = self.eat_path()?;
                let expression = Expression::Path { path: path_id };
                self.tree.allocate(expression, self.get_span_from(start))
            // _
            } else {
                let expression = Expression::Error;
                self.tree.allocate(expression, self.get_span_from(start))
            }
        };

        //
        // ------------------------------------------------------------
        // Postfix operations
        // ------------------------------------------------------------
        //

        // eat all postfix operations
        loop {
            // index
            if self.peek_token(TokenType::OpenBracket).is_ok() {
                let index_id = self.eat_index_postfix(left_expression_id)?;
                let expression = Expression::Index(index_id);
                left_expression_id = self.tree.allocate(expression, self.get_span_from(start));
            }
            // call
            else if self.peek_token(TokenType::OpenParenthesis).is_ok() {
                let call_id = self.eat_call_postfix(left_expression_id)?;
                let expression = Expression::Call(call_id);
                left_expression_id = self.tree.allocate(expression, self.get_span_from(start));
            }
            // as
            else if self.peek_keyword(Keyword::As).is_ok() {
                let cast_id = self.eat_as_postfix(left_expression_id)?;
                let expression = Expression::Cast(cast_id);
                left_expression_id = self.tree.allocate(expression, self.get_span_from(start));
            }
            // done
            else {
                break;
            }
        }

        //
        // ------------------------------------------------------------
        // Binary and assignment operations (infix, left associative)
        // ------------------------------------------------------------
        //

        // infix binary operations
        while let Ok(right_operator) = self.peek_infix_operator() {
            let right_precedence = right_operator.precedence();

            // left_precedence is set and >= right_precedence
            //  => leave to outer expression (left associative)
            if let Some(left_precedence) = left_precedence
                && left_precedence >= right_precedence
            {
                break;
            }
            // left_precedence is unset or < right_precedence
            //  => consume operator + rhs
            else {
                self.bump(); // eat infix operator
                let right_expression_id = self.eat_expression(Some(right_precedence))?;
                let left_expression = self.make_infix_expression(
                    left_expression_id,
                    right_operator,
                    right_expression_id,
                );
                left_expression_id = self
                    .tree
                    .allocate(left_expression, self.get_span_from(start))
            }
        }

        Ok(left_expression_id)
    }
}

#[cfg(test)]
mod tests {
    use destack_language_token::{SourceFile, tokenize_semantic};

    use crate::{BinaryOperator, Call, Expression, Parser, StringId};

    // assert an Expression::Path with a single-segment name id
    fn assert_path_eq(
        parser: &crate::Parser<'_>,
        expr_id: crate::NodeId<Expression>,
        expected: StringId,
    ) {
        match parser.tree.get(expr_id) {
            &Expression::Path { path } => {
                let p = parser.paths.get(path);
                assert_eq!(p.segments.len(), 1);
                assert_eq!(p.segments[0], expected);
            }
            other => panic!("expected path {expected:?}, got {other:?}"),
        }
    }

    /// Addition is left associative.
    /// a + b + c
    /// => ((a + b) + c)
    #[test]
    fn test_precedence_addition_left_associative() {
        let input = "a + b + c";
        let tokens = tokenize_semantic(input);
        let mut parser = Parser::new(SourceFile::new(0, input, input.len() as u32), &tokens);
        let expr_id = parser.eat_expression(None).unwrap();
        let a = parser.strings.intern("a");
        let b = parser.strings.intern("b");
        let c = parser.strings.intern("c");
        match parser.tree.get(expr_id) {
            Expression::Binary { lhs, operator, rhs } => {
                assert_eq!(*operator, BinaryOperator::Add);
                match parser.tree.get(*lhs) {
                    Expression::Binary { lhs, operator, rhs } => {
                        assert_eq!(*operator, BinaryOperator::Add);
                        assert_path_eq(&parser, *lhs, a);
                        assert_path_eq(&parser, *rhs, b);
                    }
                    other => panic!("expected binary add, got {other:?}"),
                }
                assert_path_eq(&parser, *rhs, c);
            }
            other => panic!("expected binary add, got {other:?}"),
        }
    }

    /// Multiplication has higher precedence than addition.
    /// a + b * c
    /// => (a + (b * c))
    #[test]
    fn test_precedence_multiply_before_addition() {
        let input = "a + b * c";
        let tokens = tokenize_semantic(input);
        let mut parser = Parser::new(SourceFile::new(0, input, input.len() as u32), &tokens);
        let expr_id = parser.eat_expression(None).unwrap();
        let a = parser.strings.intern("a");
        let b = parser.strings.intern("b");
        let c = parser.strings.intern("c");
        match parser.tree.get(expr_id) {
            Expression::Binary { lhs, operator, rhs } => {
                assert_eq!(*operator, BinaryOperator::Add);
                assert_path_eq(&parser, *lhs, a);
                match parser.tree.get(*rhs) {
                    Expression::Binary { lhs, operator, rhs } => {
                        assert_eq!(*operator, BinaryOperator::Multiply);
                        assert_path_eq(&parser, *lhs, b);
                        assert_path_eq(&parser, *rhs, c);
                    }
                    other => panic!("expected binary multiply, got {other:?}"),
                }
            }
            other => panic!("expected binary add, got {other:?}"),
        }
    }

    /// Parentheses override operator precedence.
    /// (a + b) * c
    /// => ((a + b) * c)
    #[test]
    fn test_precedence_parentheses_override() {
        let input = "(a + b) * c";
        let tokens = tokenize_semantic(input);
        let mut parser = Parser::new(SourceFile::new(0, input, input.len() as u32), &tokens);
        let expr_id = parser.eat_expression(None).unwrap();
        let a = parser.strings.intern("a");
        let b = parser.strings.intern("b");
        let c = parser.strings.intern("c");
        match parser.tree.get(expr_id) {
            Expression::Binary { lhs, operator, rhs } => {
                assert_eq!(*operator, BinaryOperator::Multiply);
                match parser.tree.get(*lhs) {
                    Expression::Binary { lhs, operator, rhs } => {
                        assert_eq!(*operator, BinaryOperator::Add);
                        assert_path_eq(&parser, *lhs, a);
                        assert_path_eq(&parser, *rhs, b);
                    }
                    other => panic!("expected binary add, got {other:?}"),
                }
                assert_path_eq(&parser, *rhs, c);
            }
            other => panic!("expected binary multiply, got {other:?}"),
        }
    }

    /// Mixed precedence chain with addition and multiplication.
    /// a + b * c + d
    /// => ((a + (b * c)) + d)
    #[test]
    fn test_precedence_chain_mixed() {
        let input = "a + b * c + d";
        let tokens = tokenize_semantic(input);
        let mut parser = Parser::new(SourceFile::new(0, input, input.len() as u32), &tokens);
        let expr_id = parser.eat_expression(None).unwrap();
        let a = parser.strings.intern("a");
        let b = parser.strings.intern("b");
        let c = parser.strings.intern("c");
        let d = parser.strings.intern("d");
        match parser.tree.get(expr_id) {
            Expression::Binary { lhs, operator, rhs } => {
                assert_eq!(*operator, BinaryOperator::Add);
                match parser.tree.get(*lhs) {
                    Expression::Binary { lhs, operator, rhs } => {
                        assert_eq!(*operator, BinaryOperator::Add);
                        assert_path_eq(&parser, *lhs, a);
                        match parser.tree.get(*rhs) {
                            Expression::Binary { lhs, operator, rhs } => {
                                assert_eq!(*operator, BinaryOperator::Multiply);
                                assert_path_eq(&parser, *lhs, b);
                                assert_path_eq(&parser, *rhs, c);
                            }
                            other => panic!("expected binary multiply, got {other:?}"),
                        }
                    }
                    other => panic!("expected binary add, got {other:?}"),
                }
                assert_path_eq(&parser, *rhs, d);
            }
            other => panic!("expected binary add, got {other:?}"),
        }
    }

    /// Addition has higher precedence than bitwise or.
    /// a + b | c + d
    /// => ((a + b) | (c + d))
    #[test]
    fn test_precedence_bitwise_vs_addition() {
        let input = "a + b | c + d";
        let tokens = tokenize_semantic(input);
        let mut parser = Parser::new(SourceFile::new(0, input, input.len() as u32), &tokens);
        let expr_id = parser.eat_expression(None).unwrap();
        let a = parser.strings.intern("a");
        let b = parser.strings.intern("b");
        let c = parser.strings.intern("c");
        let d = parser.strings.intern("d");
        match parser.tree.get(expr_id) {
            Expression::Binary { lhs, operator, rhs } => {
                assert_eq!(*operator, BinaryOperator::BitwiseOr);
                match parser.tree.get(*lhs) {
                    Expression::Binary { lhs, operator, rhs } => {
                        assert_eq!(*operator, BinaryOperator::Add);
                        assert_path_eq(&parser, *lhs, a);
                        assert_path_eq(&parser, *rhs, b);
                    }
                    other => panic!("expected binary add, got {other:?}"),
                }
                match parser.tree.get(*rhs) {
                    Expression::Binary { lhs, operator, rhs } => {
                        assert_eq!(*operator, BinaryOperator::Add);
                        assert_path_eq(&parser, *lhs, c);
                        assert_path_eq(&parser, *rhs, d);
                    }
                    other => panic!("expected binary add, got {other:?}"),
                }
            }
            other => panic!("expected binary bitwise or, got {other:?}"),
        }
    }

    /// Comparison has higher precedence than logical and.
    /// a == b && c == d
    /// => ((a == b) && (c == d))
    #[test]
    fn test_precedence_comparison_vs_logical() {
        let input = "a == b && c == d";
        let tokens = tokenize_semantic(input);
        let mut parser = Parser::new(SourceFile::new(0, input, input.len() as u32), &tokens);
        let expr_id = parser.eat_expression(None).unwrap();
        let a = parser.strings.intern("a");
        let b = parser.strings.intern("b");
        let c = parser.strings.intern("c");
        let d = parser.strings.intern("d");
        match parser.tree.get(expr_id) {
            Expression::Binary { lhs, operator, rhs } => {
                assert_eq!(*operator, BinaryOperator::LogicalAnd);
                match parser.tree.get(*lhs) {
                    Expression::Binary { lhs, operator, rhs } => {
                        assert_eq!(*operator, BinaryOperator::Equal);
                        assert_path_eq(&parser, *lhs, a);
                        assert_path_eq(&parser, *rhs, b);
                    }
                    other => panic!("expected binary equal, got {other:?}"),
                }
                match parser.tree.get(*rhs) {
                    Expression::Binary { lhs, operator, rhs } => {
                        assert_eq!(*operator, BinaryOperator::Equal);
                        assert_path_eq(&parser, *lhs, c);
                        assert_path_eq(&parser, *rhs, d);
                    }
                    other => panic!("expected binary equal, got {other:?}"),
                }
            }
            other => panic!("expected binary logical and, got {other:?}"),
        }
    }

    /// Unary prefix has higher precedence than multiplication.
    /// -a * b
    /// => ((-a) * b)
    #[test]
    fn test_precedence_unary_before_multiply() {
        let input = "-a * b";
        let tokens = tokenize_semantic(input);
        let mut parser = Parser::new(SourceFile::new(0, input, input.len() as u32), &tokens);
        let expr_id = parser.eat_expression(None).unwrap();
        let a = parser.strings.intern("a");
        let b = parser.strings.intern("b");
        match parser.tree.get(expr_id) {
            Expression::Binary { lhs, operator, rhs } => {
                assert_eq!(*operator, BinaryOperator::Multiply);
                match parser.tree.get(*lhs) {
                    Expression::Unary { operator: _, rhs } => {
                        assert_path_eq(&parser, *rhs, a);
                    }
                    other => panic!("expected unary on lhs, got {other:?}"),
                }
                assert_path_eq(&parser, *rhs, b);
            }
            other => panic!("expected binary multiply, got {other:?}"),
        }
    }

    /// Postfix call has higher precedence than addition.
    /// a() + b
    /// => (a() + b)
    #[test]
    fn test_precedence_postfix_call_before_add() {
        let input = "a() + b";
        let tokens = tokenize_semantic(input);
        let mut parser = Parser::new(SourceFile::new(0, input, input.len() as u32), &tokens);
        let expr_id = parser.eat_expression(None).unwrap();
        let a = parser.strings.intern("a");
        let b = parser.strings.intern("b");
        match parser.tree.get(expr_id) {
            Expression::Binary { lhs, operator, rhs } => {
                assert_eq!(*operator, BinaryOperator::Add);
                match parser.tree.get(*lhs) {
                    &Expression::Call(call_id) => {
                        let call = parser.tree.get(call_id);
                        let Call {
                            receiver,
                            static_arguments,
                            dynamic_arguments,
                            ..
                        } = call;
                        assert!(static_arguments.is_none());
                        assert!(dynamic_arguments.is_empty());
                        assert_path_eq(&parser, *receiver, a);
                    }
                    other => panic!("expected call on lhs, got {other:?}"),
                }
                assert_path_eq(&parser, *rhs, b);
            }
            other => panic!("expected binary add, got {other:?}"),
        }
    }
}
