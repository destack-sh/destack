use crate::{ParseError, ParseResult, Parser, ParserMark};

use destack_ast::{
    Argument, Expression, Keyword, LocalNodeId, TokenType, TypeBinaryOperator, TypeUnaryOperator,
};

impl Parser {
    pub fn can_follow_type_arguments_in_expression(&mut self) -> bool {
        let cursor = self.peek_scanner_cursor();

        // a line break terminates the current expression statement
        if cursor.has_line_break_before {
            return true;
        }

        self.can_follow_type_arguments_at_index(cursor.index)
    }

    /// Check whether a static argument list can be followed by a specific token.
    pub(super) fn can_follow_type_arguments_at_index(&mut self, index: usize) -> bool {
        let token_type = self.token_type_at(index);

        // allow end and static closers
        if token_type == TokenType::End {
            return true;
        }
        if self.options.is_in_static() && token_type == TokenType::GreaterThan {
            return true;
        }

        // allow ternary and arrow continuations
        if self.options.is_in_ternary_condition() && token_type == TokenType::Colon {
            return true;
        }
        if self.options.is_in_type()
            && matches!(token_type, TokenType::Arrow | TokenType::ArrowWide)
        {
            return true;
        }

        // allow statement-start keywords after static args in new receivers
        if self.options.is_in_new_receiver()
            && token_type == TokenType::Identifier
            && self.keyword_for_index(index).is_some()
        {
            return true;
        }

        // allow stops and delimiters
        if matches!(
            token_type,
            TokenType::Comma | TokenType::Semicolon | TokenType::Newline | TokenType::End
        ) {
            return true;
        }
        if matches!(
            token_type,
            TokenType::CloseParenthesis | TokenType::CloseBracket | TokenType::CloseBrace
        ) {
            return true;
        }
        if matches!(
            token_type,
            TokenType::OpenParenthesis | TokenType::OpenBracket | TokenType::Dot
        ) {
            return true;
        }
        if token_type == TokenType::Maybe {
            return true;
        }
        if matches!(
            token_type,
            TokenType::TemplateString | TokenType::TemplateStringStart
        ) {
            return true;
        }

        // allow heritage terminators after static arguments
        if self.options.is_in_super_type() {
            if token_type == TokenType::OpenBrace {
                return true;
            }
            if token_type == TokenType::Identifier
                && matches!(
                    self.keyword_for_index(index),
                    Some(Keyword::Implements | Keyword::With | Keyword::Where)
                )
            {
                return true;
            }
        }

        if self.has_infix_or_assign_operator_at_index(index) {
            return true;
        }

        false
    }

    /// Check whether static arguments can be followed by an object literal.
    #[inline]
    pub(super) fn can_follow_type_arguments_in_object_literal(&mut self) -> bool {
        self.peek_is(TokenType::OpenBrace)
    }

    /// Check whether static arguments can be followed by a statement-start keyword.
    #[inline]
    pub(super) fn can_follow_type_arguments_with_statement_keyword(&mut self) -> bool {
        let cursor = self.peek_scanner_cursor();
        cursor.token_type == TokenType::Identifier && self.keyword_for_index(cursor.index).is_some()
    }

    /// Speculatively eat static arguments and validate a compatible follow token.
    pub(crate) fn eat_static_arguments_with_follow_maybe(
        &mut self,
        allow_object_literal: bool,
        allow_statement_keyword: bool,
        allow_newline_prefix: bool,
    ) -> Option<Vec<LocalNodeId<Argument>>> {
        // javascript modes do not support static arguments
        if self.language.is_javascript()
            && !self.options.is_in_type()
            && !self.options.is_in_decorator()
        {
            return None;
        }

        // static argument start
        let has_static_argument_start =
            if self.peek_is(TokenType::LessThan) || self.peek_is(TokenType::ShiftLeft) {
                true
            } else if allow_newline_prefix {
                let cursor = self.peek_scanner_cursor();
                cursor.has_line_break_before
                    && matches!(
                        cursor.token_type,
                        TokenType::LessThan | TokenType::ShiftLeft
                    )
            } else {
                false
            };
        if !has_static_argument_start {
            return None;
        }

        // parse static arguments speculatively
        let speculative_start = self.mark();
        let speculative_start_idx = self.tree.next_id();

        // normalize optional line break prefix before `<...>`
        if allow_newline_prefix {
            self.sync_to_scanner_cursor();
        }

        match self.eat_static_arguments() {
            Ok(static_arguments) => {
                // in type or decorator context, type arguments are always valid
                if self.options.is_in_type() || self.options.is_in_decorator() {
                    return Some(static_arguments);
                }

                // validate that a follow token makes sense for a type argument list
                let mut can_follow = self.can_follow_type_arguments_in_expression();
                if allow_object_literal && self.can_follow_type_arguments_in_object_literal() {
                    can_follow = true;
                }
                if allow_statement_keyword
                    && self.can_follow_type_arguments_with_statement_keyword()
                {
                    can_follow = true;
                }
                if can_follow {
                    Some(static_arguments)
                } else {
                    self.restore(speculative_start, speculative_start_idx);
                    None
                }
            }
            Err(_err) => {
                self.restore(speculative_start, speculative_start_idx);
                None
            }
        }
    }

    /// Eat static arguments in expression position if the follow token allows it.
    pub(super) fn eat_static_arguments_in_expression(
        &mut self,
        allow_object_literal: bool,
    ) -> Option<Vec<LocalNodeId<Argument>>> {
        // new receivers parse static arguments in `eat_new` with dedicated follow validation
        if self.options.is_in_new_receiver() {
            return None;
        }

        // in typescript value expressions, defer static arguments to postfix parsing
        // this keeps `f<T>` and `obj.method<T>` as instantiation or call forms
        if self.language.is_typescript()
            && !self.options.is_in_type()
            && !self.options.is_in_decorator()
        {
            return None;
        }

        self.eat_static_arguments_with_follow_maybe(allow_object_literal, false, false)
    }

    /// Eat a TypeScript angle bracket type assertion.
    pub(super) fn eat_type_assertion_expression(
        &mut self,
        start: &ParserMark,
    ) -> ParseResult<LocalNodeId<Expression>> {
        let operator_start = self.mark_span();

        // parse `<const>` with dedicated ts assertion behavior
        let is_const_assertion = {
            let const_mark = self.mark();
            let const_tree_start = self.tree.next_id();
            let parse_const_assertion = (|| -> ParseResult<()> {
                self.eat_token(TokenType::LessThan)?;
                self.eat_newlines_maybe()?;
                self.eat_keyword(Keyword::Const)?;
                self.eat_newlines_maybe()?;
                self.eat_type_angle_close()?;
                Ok(())
            })();

            match parse_const_assertion {
                Ok(()) => true,
                Err(_) => {
                    self.restore(const_mark, const_tree_start);
                    false
                }
            }
        };

        // parse standard `<Type>` assertions
        let asserted_type = if is_const_assertion {
            None
        } else {
            let static_arguments = self.eat_static_arguments()?;

            // type assertions require exactly one positional type argument
            if static_arguments.len() != 1 {
                let operator_span = self.get_span_from(&operator_start);
                let unexpected_span = static_arguments
                    .get(1)
                    .map(|argument_id| self.tree.get_span(*argument_id))
                    .unwrap_or(operator_span);
                return Err(ParseError::unexpected(unexpected_span));
            }

            // extract the asserted type expression
            match self.tree.get(static_arguments[0]) {
                Argument::Positional {
                    modifiers: None,
                    value,
                } => Some(*value),
                _ => {
                    return Err(ParseError::unexpected(
                        self.tree.get_span(static_arguments[0]),
                    ));
                }
            }
        };

        let operator_span = self.get_span_from(&operator_start);

        // parse the asserted value expression
        let right_options = self
            .options
            .not_in_position()
            .in_left_precedence(TypeBinaryOperator::Cast.precedence());
        let asserted_value = self.eat_expression(right_options)?;

        // in typescript, angle assertions require a real expression value: `<T>()` is invalid
        if self.language.is_typescript()
            && matches!(
                self.tree.get(asserted_value),
                Expression::TupleExpression { elements, .. } if elements.is_empty()
            )
        {
            return Err(ParseError::unexpected(self.tree.get_span(asserted_value)));
        }
        if self.language.is_typescript()
            && matches!(
                self.tree.get(asserted_value),
                Expression::SequenceExpression { expressions } if expressions.is_empty()
            )
        {
            return Err(ParseError::unexpected(self.tree.get_span(asserted_value)));
        }

        // lower const assertions to the same unary node used by `as const`
        let expression_id = if is_const_assertion {
            self.tree.insert(
                Expression::TypeUnary {
                    operator: TypeUnaryOperator::AsConst,
                    right: asserted_value,
                },
                self.get_span_from(start),
            )
        }
        // lower type assertions to the same cast node used by `as`
        else {
            let Some(asserted_type) = asserted_type else {
                return Err(ParseError::unexpected(operator_span));
            };

            self.tree.insert(
                Expression::TypeBinary {
                    left: asserted_value,
                    operator: TypeBinaryOperator::Cast,
                    right: asserted_type,
                },
                self.get_span_from(start),
            )
        };
        self.tree.set_main_span(expression_id, operator_span);
        Ok(expression_id)
    }
}
