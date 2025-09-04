//! Parse expressions. Mostly defers to other parsers.

use destack_language_token::TokenType;

use crate::{
    BinaryOperator, Expression, Keyword, NodeId, OperatorPrecedence, ParseResult, Parser,
    UnaryOperator,
};

impl BinaryOperator {
    /// Get the precedence of the binary operator.
    pub fn precedence(&self) -> OperatorPrecedence {
        match self {
            // arithmetic - multiplication
            BinaryOperator::Multiply => OperatorPrecedence::Multiplication,
            BinaryOperator::WrappingMultiply => OperatorPrecedence::Multiplication,
            BinaryOperator::SaturatingMultiply => OperatorPrecedence::Multiplication,
            BinaryOperator::Divide => OperatorPrecedence::Multiplication,
            BinaryOperator::Remainder => OperatorPrecedence::Multiplication,

            // arithmetic - addition
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
            BinaryOperator::BitwiseOr => OperatorPrecedence::Bitwise,
            BinaryOperator::BitwiseXor => OperatorPrecedence::Bitwise,

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

    /// Convert a TokenType to a BinaryOperator (if a direct mapping exists).
    pub fn from_token_type(token_type: TokenType) -> Option<BinaryOperator> {
        match token_type {
            // arithmetic
            TokenType::Add => Some(BinaryOperator::Add),
            TokenType::WrappingAdd => Some(BinaryOperator::WrappingAdd),
            TokenType::SaturatingAdd => Some(BinaryOperator::SaturatingAdd),
            TokenType::Subtract => Some(BinaryOperator::Subtract),
            TokenType::WrappingSubtract => Some(BinaryOperator::WrappingSubtract),
            TokenType::SaturatingSubtract => Some(BinaryOperator::SaturatingSubtract),
            TokenType::Multiply => Some(BinaryOperator::Multiply),
            TokenType::WrappingMultiply => Some(BinaryOperator::WrappingMultiply),
            TokenType::SaturatingMultiply => Some(BinaryOperator::SaturatingMultiply),
            TokenType::Divide => Some(BinaryOperator::Divide),
            TokenType::Remainder => Some(BinaryOperator::Remainder),

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

            // bitwise
            TokenType::BitwiseAnd => Some(BinaryOperator::BitwiseAnd),
            TokenType::BitwiseOr => Some(BinaryOperator::BitwiseOr),
            TokenType::BitwiseXor => Some(BinaryOperator::BitwiseXor),
            TokenType::ShiftLeft => Some(BinaryOperator::ShiftLeft),
            TokenType::SaturatingShiftLeft => Some(BinaryOperator::SaturatingShiftLeft),
            TokenType::ShiftRight => Some(BinaryOperator::ShiftRight),

            _ => None,
        }
    }

    /// Convert a BinaryOperator to a TokenType (if a direct mapping exists).
    pub fn as_token_type(&self) -> Option<TokenType> {
        match self {
            // arithmetic
            BinaryOperator::Add => Some(TokenType::Add),
            BinaryOperator::WrappingAdd => Some(TokenType::WrappingAdd),
            BinaryOperator::SaturatingAdd => Some(TokenType::SaturatingAdd),
            BinaryOperator::Subtract => Some(TokenType::Subtract),
            BinaryOperator::WrappingSubtract => Some(TokenType::WrappingSubtract),
            BinaryOperator::SaturatingSubtract => Some(TokenType::SaturatingSubtract),
            BinaryOperator::Multiply => Some(TokenType::Multiply),
            BinaryOperator::WrappingMultiply => Some(TokenType::WrappingMultiply),
            BinaryOperator::SaturatingMultiply => Some(TokenType::SaturatingMultiply),
            BinaryOperator::Divide => Some(TokenType::Divide),
            BinaryOperator::Remainder => Some(TokenType::Remainder),

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

            // bitwise
            BinaryOperator::BitwiseAnd => Some(TokenType::BitwiseAnd),
            BinaryOperator::BitwiseOr => Some(TokenType::BitwiseOr),
            BinaryOperator::BitwiseXor => Some(TokenType::BitwiseXor),
            BinaryOperator::ShiftLeft => Some(TokenType::ShiftLeft),
            BinaryOperator::SaturatingShiftLeft => Some(TokenType::SaturatingShiftLeft),
            BinaryOperator::ShiftRight => Some(TokenType::ShiftRight),
        }
    }
}

impl<'a> Parser<'a> {
    /// Eat an expression.
    pub fn eat_expression(&mut self) -> ParseResult<NodeId<Expression>> {
        let start = self.mark();

        // grouping parentheses
        if self.peek_token(TokenType::OpenParenthesis).is_ok() {
            self.bump();
            let expression_id = self.eat_expression()?;
            self.eat_token(TokenType::CloseParenthesis)?;
            self.tree.set_span(expression_id, self.get_span_from(start));
            return Ok(expression_id);
        }

        //
        // ------------------------------------------------------------
        // Unary operations (prefix)
        // ------------------------------------------------------------
        //

        let unary_operator: Option<UnaryOperator> = {
            if self.peek_token(TokenType::Bang).is_ok() {
                Some(UnaryOperator::LogicalNot)
            } else if self.peek_token(TokenType::Subtract).is_ok() {
                Some(UnaryOperator::Negate)
            } else if self.peek_token(TokenType::Tilde).is_ok() {
                Some(UnaryOperator::BitwiseNot)
            } else if self.peek_token(TokenType::BitwiseAnd).is_ok() {
                Some(UnaryOperator::Reference)
            } else if self.peek_token(TokenType::Multiply).is_ok() {
                Some(UnaryOperator::Dereference)
            } else if self.peek_token(TokenType::At).is_ok() {
                todo!("mark function or call as static?")
            } else {
                None
            }
        };
        if let Some(unary_operator) = unary_operator {
            let rhs = self.eat_expression()?;
            let expression = Expression::UnaryOperation {
                operator: unary_operator,
                rhs,
            };
            let expression_id = self.tree.allocate(expression, self.get_span_from(start));
            return Ok(expression_id);
        }

        let expression = {
            //
            // ------------------------------------------------------------
            // Declarations
            // ------------------------------------------------------------
            //
            // module
            if self.peek_keyword(Keyword::Module).is_ok() {
                let module_id = self.eat_module()?;
                Expression::Module(module_id)
            }
            // struct
            else if self.peek_keyword(Keyword::Struct).is_ok() {
                let struct_id = self.eat_struct()?;
                Expression::Struct(struct_id)
            }
            // enum
            else if self.peek_keyword(Keyword::Enum).is_ok() {
                let enum_id = self.eat_enum()?;
                Expression::Enum(enum_id)
            }
            // union
            else if self.peek_keyword(Keyword::Union).is_ok() {
                let union_id = self.eat_union()?;
                Expression::Union(union_id)
            }
            // trait
            else if self.peek_keyword(Keyword::Trait).is_ok() {
                let trait_id = self.eat_trait()?;
                Expression::Trait(trait_id)
            }
            // implement
            else if self.peek_keyword(Keyword::Implement).is_ok() {
                let implement_id = self.eat_implement()?;
                Expression::Implement(implement_id)
            }
            // function
            else if self.peek_keyword(Keyword::Function).is_ok() {
                let function_id = self.eat_function()?;
                Expression::Function(function_id)
            }
            //
            // ------------------------------------------------------------
            // Control flow
            // ------------------------------------------------------------
            //
            // if
            else if self.peek_keyword(Keyword::If).is_ok() {
                let if_id = self.eat_if()?;
                Expression::If(if_id)
            }
            // while
            else if self.peek_keyword(Keyword::While).is_ok() {
                let while_id = self.eat_while()?;
                Expression::While(while_id)
            }
            // for
            else if self.peek_keyword(Keyword::For).is_ok() {
                let for_id = self.eat_for()?;
                Expression::For(for_id)
            }
            // loop
            else if self.peek_keyword(Keyword::Loop).is_ok() {
                let loop_id = self.eat_loop()?;
                Expression::Loop(loop_id)
            }
            // break
            else if self.peek_keyword(Keyword::Break).is_ok() {
                let break_id = self.eat_break()?;
                Expression::Break(break_id)
            }
            // continue
            else if self.peek_keyword(Keyword::Continue).is_ok() {
                let continue_id = self.eat_continue()?;
                Expression::Continue(continue_id)
            }
            // defer
            else if self.peek_keyword(Keyword::Defer).is_ok() {
                let defer_id = self.eat_defer()?;
                Expression::Defer(defer_id)
            }
            // return
            else if self.peek_keyword(Keyword::Return).is_ok() {
                let return_id = self.eat_return()?;
                Expression::Return(return_id)
            }
            // try
            else if self.peek_keyword(Keyword::Try).is_ok() {
                let try_id = self.eat_try_catch()?;
                Expression::Try(try_id)
            }
            // match
            else if self.peek_keyword(Keyword::Match).is_ok() {
                let match_id = self.eat_match()?;
                Expression::Match(match_id)
            }
            //
            // ------------------------------------------------------------
            // Literals / aliases
            // ------------------------------------------------------------
            //
            // array
            else if self.peek_token(TokenType::OpenBracket).is_ok() {
                let array_literal = self.eat_array_literal()?;
                Expression::ArrayLiteral(array_literal)
            // tuple
            } else if self.peek_token(TokenType::OpenParenthesis).is_ok() {
                let tuple_literal = self.eat_tuple_literal()?;
                Expression::TupleLiteral(tuple_literal)
            // TODO: parse struct literals (postfix to avoid unbounded lookahead)
            // scalar
            } else if self.peek_scalar_literal().is_ok() {
                let scalar_literal = self.eat_scalar_literal()?;
                Expression::ScalarLiteral(scalar_literal)
            // alias / path
            } else if self.peek_identifier().is_ok() {
                let path_id = self.eat_path()?;
                Expression::Path { path: path_id }
            // _
            } else {
                Expression::Error
            }
        };
        let mut expression_id = self.tree.allocate(expression, self.get_span_from(start));

        //
        // ------------------------------------------------------------
        // Postfix operations
        // ------------------------------------------------------------
        //

        // eat all postfix operations
        loop {
            // index
            if self.peek_token(TokenType::OpenBracket).is_ok() {
                let index_id = self.eat_index_postfix(expression_id)?;
                let expression = Expression::Index(index_id);
                expression_id = self.tree.allocate(expression, self.get_span_from(start));
            }
            // call
            else if self.peek_token(TokenType::OpenParenthesis).is_ok() {
                let call_id = self.eat_call_postfix(expression_id)?;
                let expression = Expression::Call(call_id);
                expression_id = self.tree.allocate(expression, self.get_span_from(start));
            }
            // as
            else if self.peek_keyword(Keyword::As).is_ok() {
                let cast_id = self.eat_as_postfix(expression_id)?;
                let expression = Expression::Cast(cast_id);
                expression_id = self.tree.allocate(expression, self.get_span_from(start));
            }
            // done
            else {
                break;
            }
        }

        //
        // ------------------------------------------------------------
        // Infix operations
        // ------------------------------------------------------------
        //

        // nocheckin: infix binary operations

        Ok(expression_id)
    }
}
