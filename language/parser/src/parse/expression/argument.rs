use crate::{ParseError, ParseResult, Parser, ParserMark};

use destack_ast::{
    Argument, Expression, Keyword, LocalNodeId, Path, TokenType, TypeBinaryOperator,
};

impl Parser {
    pub fn can_follow_type_arguments_in_expression(&mut self) -> bool {
        // a newline terminates the current expression statement
        if self.peek_is(TokenType::Newline) {
            return true;
        }

        let index = self.pos_index();
        self.can_follow_type_arguments_at_index(index)
    }

    /// Check whether a static argument list can be followed by a specific token.
    pub(super) fn can_follow_type_arguments_at_index(&mut self, index: usize) -> bool {
        let token_type = self.token_type_at(index);

        // allow end and static closers
        if token_type == TokenType::End {
            return true;
        }
        if self.options.in_static && token_type == TokenType::GreaterThan {
            return true;
        }

        // allow ternary and arrow continuations
        if self.options.in_ternary_condition && token_type == TokenType::Colon {
            return true;
        }
        if self.options.in_type && matches!(token_type, TokenType::Arrow | TokenType::ArrowWide) {
            return true;
        }

        // allow statement-start keywords after static args in new receivers
        if self.options.in_new_receiver
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
        if self.options.in_super_type {
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
        let index = if self.peek_is(TokenType::Newline) {
            self.next_non_newline_index_from(self.pos_index())
        } else {
            self.pos_index()
        };
        self.token_type_at(index) == TokenType::Identifier
            && self.keyword_for_index(index).is_some()
    }

    /// Speculatively eat static arguments and validate a compatible follow token.
    pub(crate) fn eat_static_arguments_with_follow_maybe(
        &mut self,
        allow_object_literal: bool,
        allow_statement_keyword: bool,
        allow_newline_prefix: bool,
    ) -> Option<Vec<LocalNodeId<Argument>>> {
        // static argument start
        let has_static_argument_start = self.peek_is(TokenType::LessThan)
            || self.peek_is(TokenType::ShiftLeft)
            || allow_newline_prefix
                && self.peek_is(TokenType::Newline)
                && (self.peek_next_is(TokenType::LessThan)
                    || self.peek_next_is(TokenType::ShiftLeft));
        if !has_static_argument_start {
            return None;
        }

        // parse static arguments speculatively
        let speculative_start = self.mark();
        let speculative_start_idx = self.tree.next_id();

        // normalize optional newline prefix before `<...>`
        if allow_newline_prefix {
            while self.peek_is(TokenType::Newline) {
                self.bump();
            }
        }

        match self.eat_static_arguments() {
            Ok(static_arguments) => {
                // in type or decorator context, type arguments are always valid
                if self.options.in_type || self.options.in_decorator {
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
        if self.options.in_new_receiver {
            return None;
        }

        // in typescript value expressions, defer static arguments to postfix parsing
        // this keeps `f<T>` and `obj.method<T>` as instantiation or call forms
        if self.language.is_typescript() && !self.options.in_type && !self.options.in_decorator {
            return None;
        }

        self.eat_static_arguments_with_follow_maybe(allow_object_literal, false, false)
    }

    /// Eat a TypeScript angle bracket type assertion.
    pub(super) fn eat_type_assertion_expression(
        &mut self,
        start: &ParserMark,
    ) -> ParseResult<LocalNodeId<Expression>> {
        let operator_start = self.mark();

        // parse `<const>` with dedicated ts assertion behavior
        let const_assertion = {
            let const_mark = self.mark();
            let const_tree_start = self.tree.next_id();
            let parse_const_assertion = (|| -> ParseResult<LocalNodeId<Expression>> {
                self.eat_token(TokenType::LessThan)?;
                self.eat_newlines_maybe()?;
                let const_span = self.peek()?.span;
                self.eat_keyword(Keyword::Const)?;
                self.eat_newlines_maybe()?;
                self.eat_type_angle_close()?;

                let const_id = self.strings.intern("const");
                let const_path = Path {
                    segments: smallvec::smallvec![const_id],
                };
                let asserted_type = self.tree.insert(
                    Expression::Path {
                        path: const_path,
                        static_arguments: None,
                    },
                    const_span,
                );
                Ok(asserted_type)
            })();

            match parse_const_assertion {
                Ok(asserted_type) => Some(asserted_type),
                Err(_) => {
                    self.restore(const_mark, const_tree_start);
                    None
                }
            }
        };

        // parse standard `<Type>` assertions
        let asserted_type = if let Some(asserted_type) = const_assertion {
            asserted_type
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
                } => *value,
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
        let asserted_value = self.with_options(right_options, |parser| parser.eat_expression())?;

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

        // lower to the same cast node used by `as`
        let expression_id = self.tree.insert(
            Expression::TypeBinary {
                left: asserted_value,
                operator: TypeBinaryOperator::Cast,
                right: asserted_type,
            },
            self.get_span_from(start),
        );
        self.tree.set_main_span(expression_id, operator_span);
        Ok(expression_id)
    }
}
