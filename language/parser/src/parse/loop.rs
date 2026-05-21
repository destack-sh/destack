use destack_dir::{
    Asynchrony, BindingKeyword, BlockContext, Expression, ForEachBinding, ForEachOperator, Keyword,
    LocalNodeId, NodeType, Pattern, TokenType, WhileForm,
};

use crate::parse::flags::ParserFlags;
use crate::{ParseError, ParseResult, Parser};

impl Parser {
    /// Eat a loop (e.g., `loop { ... }`).
    ///
    /// Examples:
    /// ```
    /// loop {
    ///     y = getNext()
    ///     if (y < 0) {
    ///         break
    ///     }
    /// }
    /// ```
    pub fn eat_loop(&mut self) -> ParseResult<LocalNodeId<Expression>> {
        let start = self.span_start();

        // keyword
        self.eat_keyword(Keyword::Loop)?;

        // body
        let body_id = self.eat_block(BlockContext::Statement)?;

        // loop
        let loop_id = self.insert_node(
            Expression::Loop { body: body_id },
            self.get_span_from(&start),
        );
        Ok(loop_id)
    }

    /// Eat a for each or for condition loop (including keyword and header).
    ///
    /// Examples:
    /// ```
    /// for (const item in items) {
    ///     item
    /// }
    ///
    /// for (const x in zeds.iter()) a: {
    ///     if (y > 5) {
    ///         continue a
    ///     }
    ///     y = 2
    /// }
    ///
    /// for (let x = 0; x < 10; x++) {
    ///     y = 2
    /// }
    /// ```
    pub fn eat_for(&mut self) -> ParseResult<LocalNodeId<Expression>> {
        let start = self.span_start();

        // keyword
        self.eat_keyword(Keyword::For)?;

        // asynchrony
        let asynchrony = if self.is_keyword(Keyword::Await) {
            self.bump(); // eat await keyword
            Asynchrony::Async
        } else {
            Asynchrony::Sync
        };

        self.eat_token(TokenType::OpenParenthesis)?;

        let close_span = self.find_matching_close_after_open_maybe(
            TokenType::OpenParenthesis,
            TokenType::CloseParenthesis,
        );
        let has_top_level_semicolon = if let Some(close_span) = close_span {
            self.has_token_before_matching_close_after_open(
                close_span,
                TokenType::Semicolon,
                false,
            )?
        } else {
            false
        };

        // for condition loop
        if asynchrony == Asynchrony::Sync && has_top_level_semicolon {
            // c style for clauses always allow comma operator expressions
            let mut clause_flags = self.flags.nested();
            clause_flags.set_allow_sequence_expression(true);

            // initialization
            let initialization_id = if self.peek_is(TokenType::Semicolon) {
                None
            } else {
                Some(self.eat_expression(clause_flags)?)
            };
            self.eat_token(TokenType::Semicolon)?;

            // condition
            let condition_id = if self.peek_is(TokenType::Semicolon) {
                None
            } else {
                Some(self.eat_expression(clause_flags)?)
            };
            self.eat_token(TokenType::Semicolon)?;

            // increment
            let increment_id = if self.peek_is(TokenType::CloseParenthesis) {
                None
            } else {
                Some(self.eat_expression(clause_flags)?)
            };

            // close parenthesis
            self.eat_close_token_or_recover_missing_with(
                TokenType::CloseParenthesis,
                NodeType::Expression,
                |parser, token_type| {
                    Self::is_close_delimiter_boundary_token(token_type) || parser.is_block_start()
                },
            )?;

            // body
            let body_id = self.eat_block_or_statement()?;

            // for
            let for_id = self.insert_node(
                Expression::For {
                    initialization: initialization_id,
                    condition: condition_id,
                    increment: increment_id,
                    body: body_id,
                },
                self.get_span_from(&start),
            );
            Ok(for_id)
        }
        // explicit pattern for loop
        else {
            // binding
            let binding = self.eat_for_each_binding()?;

            // in
            let operator = match self.eat_keyword_in(&[Keyword::In, Keyword::Of])? {
                Keyword::In => ForEachOperator::In,
                Keyword::Of => ForEachOperator::Of,
                _ => unreachable!(),
            };

            // reject using bindings in semicolon statement `for ... in`
            // block-value mode allows them
            if !self.language.is_destack()
                && operator == ForEachOperator::In
                && matches!(binding, ForEachBinding::Using { .. })
            {
                return Err(ParseError::unexpected(self.peek()?.span));
            }

            // iterator
            let iterator_flags = self.for_each_value_flags(close_span.is_some());
            let iterator_id =
                self.with_flags(iterator_flags, |parser| parser.eat_expression(parser.flags))?;

            // close parenthesis
            self.eat_close_token_or_recover_missing_with(
                TokenType::CloseParenthesis,
                NodeType::Expression,
                |parser, token_type| {
                    Self::is_close_delimiter_boundary_token(token_type) || parser.is_block_start()
                },
            )?;

            // body
            let body_id = self.eat_block_or_statement()?;

            // for
            let for_id = self.insert_node(
                Expression::ForEach {
                    asynchrony,
                    operator,
                    binding,
                    iterator: iterator_id,
                    body: body_id,
                },
                self.get_span_from(&start),
            );
            Ok(for_id)
        }
    }

    /// Eat a for each binding (pattern or using).
    pub(crate) fn eat_for_each_binding(&mut self) -> ParseResult<ForEachBinding> {
        let using_asynchrony = self.for_each_using_binding_asynchrony();

        if let Some(using_asynchrony) = using_asynchrony {
            if using_asynchrony == Asynchrony::Async {
                self.bump(); // eat await
            }

            self.bump(); // eat using
            let pattern_flags = self.flags.not_in_position().in_for_each();
            let pattern = self.with_flags(pattern_flags, |parser| parser.eat_pattern())?;
            Ok(ForEachBinding::Using {
                asynchrony: using_asynchrony,
                pattern,
            })
        } else {
            let keyword = self.peek_for_each_keyword();

            if keyword.is_none() {
                // for each without declarations keeps expression heads as expression patterns
                let start = self.span_start();
                let expression = self
                    .eat_expression(self.flags.not_in_position().in_for_each().in_before_block())?;

                let pattern = self.insert_node(
                    Pattern::Expression { value: expression },
                    self.get_span_from(&start),
                );
                Ok(ForEachBinding::Pattern {
                    pattern,
                    keyword: None,
                })
            } else {
                self.bump();

                // declaration forms keep binding-pattern parsing
                let pattern_flags = self.flags.not_in_position().in_for_each();
                let pattern = self.with_flags(pattern_flags, |parser| parser.eat_pattern())?;
                Ok(ForEachBinding::Pattern { pattern, keyword })
            }
        }
    }

    /// Return expression flags for a for-each iterator expression.
    fn for_each_value_flags(&mut self, has_header_close: bool) -> ParserFlags {
        let flags = self.flags.nested().not_in_position();

        // preserve object literal iterators in valid headers
        if has_header_close || self.peek_is(TokenType::OpenBrace) {
            return flags;
        }

        // recover missing `)` before a body block
        flags.in_for_each().in_before_block()
    }

    /// Return the using asynchrony when a for each header starts a using binding.
    fn for_each_using_binding_asynchrony(&mut self) -> Option<Asynchrony> {
        // resolve `using` with optional `await` prefix
        let asynchrony = if self.using_keyword_is(Asynchrony::Async) {
            Asynchrony::Async
        } else if self.using_keyword_is(Asynchrony::Sync) {
            Asynchrony::Sync
        } else {
            return None;
        };

        // keep the binding head on the same line
        let declarator_token_type = self.using_binding_head_token(asynchrony)?;

        // using bindings start with a binding pattern shape
        if !self.token_can_start_using_binding_pattern(declarator_token_type) {
            return None;
        }

        // disambiguate identifier starts that should remain expression headers
        if declarator_token_type == TokenType::Identifier {
            let declarator_keyword = self
                .using_binding_head_offset(asynchrony)
                .and_then(|offset| self.keyword_at_offset(offset));

            // `for (using in ...)` should parse as identifier `using`
            if declarator_keyword == Some(Keyword::In) {
                return None;
            }

            // `for (using of of)` should parse as identifier `using` in semicolon statement forms
            if declarator_keyword == Some(Keyword::Of) && asynchrony == Asynchrony::Sync {
                return None;
            }
        }

        Some(asynchrony)
    }
    /// Return the declaration keyword for a for each pattern binding.
    fn peek_for_each_keyword(&mut self) -> Option<BindingKeyword> {
        let keyword = self.peek_any_keyword().ok()?;
        match keyword {
            Keyword::Let => Some(BindingKeyword::Let),
            Keyword::Const | Keyword::Readonly => Some(BindingKeyword::Const),
            _ => None,
        }
    }

    /// Eat a while or do-while loop (including keyword and header).
    ///
    /// Examples:
    /// ```
    /// while (x > 1) {
    ///     y = 2
    /// }
    ///
    /// l: while (y < 10) {
    ///     y = 2
    ///     break l
    /// }
    ///
    /// do {
    ///     y = 2
    /// } while (x > 1)
    ///
    /// do console.log("test"); while (true)
    /// ```
    pub fn eat_while(&mut self) -> ParseResult<LocalNodeId<Expression>> {
        let start = self.span_start();

        // do-while loop
        if self.is_keyword(Keyword::Do) {
            // do keyword
            self.bump(); // eat do keyword

            // body
            let body_id = self.eat_block_or_statement()?;

            // while keyword
            self.eat_keyword(Keyword::While)?;

            // condition
            let condition_flags = self.flags.not_in_position();
            let condition_id = self.with_flags(condition_flags, |parser| {
                parser.eat_parenthesized_expression()
            })?;

            // while
            let while_id = self.insert_node(
                Expression::While {
                    form: WhileForm::DoWhile,
                    condition: condition_id,
                    body: body_id,
                },
                self.get_span_from(&start),
            );
            Ok(while_id)
        }
        // while loop
        else {
            // while keyword
            self.eat_keyword(Keyword::While)?;

            // condition
            let condition_flags = self.flags.not_in_position().in_before_block();
            let condition_id = self.with_flags(condition_flags, |parser| {
                parser.eat_parenthesized_expression()
            })?;

            // body
            let body_id = self.eat_block_or_statement()?;

            // while
            let while_id = self.insert_node(
                Expression::While {
                    form: WhileForm::While,
                    condition: condition_id,
                    body: body_id,
                },
                self.get_span_from(&start),
            );
            Ok(while_id)
        }
    }
}
