//! Parse expressions. Mostly defers to other parsers.

use destack_language_token::TokenType;

use crate::{Expression, Keyword, NodeId, ParseResult, Parser, UnaryOperator};

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
            // struct
            } else if self.peek_token(TokenType::OpenBrace).is_ok() {
                let struct_literal = self.eat_struct_literal()?;
                Expression::StructLiteral(struct_literal)
            // scalar
            } else if self.peek_scalar_literal().is_ok() {
                let scalar_literal = self.eat_scalar_literal()?;
                Expression::ScalarLiteral(scalar_literal)
            // alias / path
            } else if self.peek_identifier().is_ok() {
                let path_id = self.eat_path()?;
                Expression::Alias { path: path_id }
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
            // no more postfix operations
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
