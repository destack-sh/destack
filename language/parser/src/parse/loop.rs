use destack_dir::{
    Asynchrony, BindingKeyword, BlockContext, Expression, ForEachBinding, ForEachOperator, Keyword,
    LocalNodeId, NodeType, Pattern, TokenType, WhileForm,
};

use crate::parse::context::{
    ExpressionContext, ExpressionStops, FunctionContext, PatternContext, StatementPosition,
};
use crate::{Parser, ParserError, ParserResult};

impl Parser {
    /// Parse one unconditional loop.
    ///
    /// Examples:
    /// ```ds
    /// loop { work(); }
    /// ```
    pub(crate) fn parse_loop(
        &mut self,
        function: FunctionContext,
    ) -> ParserResult<LocalNodeId<Expression>> {
        let start = self.mark_parse_start();

        // loop
        self.eat_keyword(Keyword::Loop)?;

        // { body }
        let body_id = self.parse_block(BlockContext::Statement, function)?;

        // retain the complete loop
        let loop_id = self.insert_node(
            Expression::Loop {
                label: None,
                body: body_id,
            },
            self.range_since(&start),
        );
        Ok(loop_id)
    }

    /// Parse one condition or iteration loop.
    ///
    /// Examples:
    /// ```ds
    /// for (item of items) visit(item);
    /// for (let index = 0; index < count; index++) work(index);
    /// ```
    pub(crate) fn parse_for(
        &mut self,
        function: FunctionContext,
    ) -> ParserResult<LocalNodeId<Expression>> {
        let start = self.mark_parse_start();

        // for
        self.eat_keyword(Keyword::For)?;

        // await
        let asynchrony = if self.peek_is_keyword(Keyword::Await) {
            self.bump();
            Asynchrony::Async
        } else {
            Asynchrony::Sync
        };

        let header_contains_semicolon =
            self.peek_parenthesized_group_contains(TokenType::Semicolon);
        let has_top_level_semicolon = matches!(header_contains_semicolon, Some(true));
        self.eat_token(TokenType::OpenParenthesis)?;

        // for (initialization; condition; increment) body
        if asynchrony == Asynchrony::Sync && has_top_level_semicolon {
            // initialization;
            let initialization_id = if self.peek_is(TokenType::Semicolon) {
                None
            } else {
                Some(self.parse_expression(ExpressionContext {
                    function,
                    statement: StatementPosition::Direct,
                    ..ExpressionContext::default()
                })?)
            };
            self.eat_token(TokenType::Semicolon)?;

            // condition;
            let condition_id = if self.peek_is(TokenType::Semicolon) {
                None
            } else {
                Some(self.parse_expression(ExpressionContext {
                    function,
                    ..ExpressionContext::default()
                })?)
            };
            self.eat_token(TokenType::Semicolon)?;

            // increment)
            let increment_id = if self.peek_is(TokenType::CloseParenthesis) {
                None
            } else {
                Some(self.parse_expression(ExpressionContext {
                    function,
                    ..ExpressionContext::default()
                })?)
            };

            // recover the closing parenthesis before the body
            self.eat_close_token_or_recover_missing_with(
                TokenType::CloseParenthesis,
                NodeType::Expression,
                |parser, token_type| {
                    Self::is_close_delimiter_boundary_token(token_type) || parser.peek_block()
                },
            )?;

            // body
            let body_id = self.parse_block_or_statement(function)?;

            // retain the complete condition loop
            let for_id = self.insert_node(
                Expression::For {
                    label: None,
                    initialization: initialization_id,
                    condition: condition_id,
                    increment: increment_id,
                    body: body_id,
                },
                self.range_since(&start),
            );
            Ok(for_id)
        }
        // for [await] (binding in|of iterator) body
        else {
            // binding
            let binding = self.parse_for_each_binding(function)?;

            // in or of
            let operator_token = self.peek_token_span();
            let operator = match self.eat_keyword_in(&[Keyword::In, Keyword::Of])? {
                Keyword::In => ForEachOperator::In,
                Keyword::Of => ForEachOperator::Of,
                _ => return Err(ParserError::unexpected(operator_token)),
            };

            // iterator)
            let iterator_context = ExpressionContext::for_each(
                function,
                header_contains_semicolon.is_some(),
                self.peek_is(TokenType::OpenBrace),
            );
            let iterator_id = self.parse_expression(iterator_context)?;

            // recover the closing parenthesis before the body
            self.eat_close_token_or_recover_missing_with(
                TokenType::CloseParenthesis,
                NodeType::Expression,
                |parser, token_type| {
                    Self::is_close_delimiter_boundary_token(token_type) || parser.peek_block()
                },
            )?;

            // body
            let body_id = self.parse_block_or_statement(function)?;

            // retain the complete iteration loop
            let for_id = self.insert_node(
                Expression::ForEach {
                    label: None,
                    asynchrony,
                    operator,
                    binding,
                    iterator: iterator_id,
                    body: body_id,
                },
                self.range_since(&start),
            );
            Ok(for_id)
        }
    }

    /// Parse a for-each binding pattern or using declaration.
    pub(crate) fn parse_for_each_binding(
        &mut self,
        function: FunctionContext,
    ) -> ParserResult<ForEachBinding> {
        if let Some(using_asynchrony) = self.peek_for_each_using_asynchrony() {
            if using_asynchrony == Asynchrony::Async {
                self.bump();
            }

            self.bump();
            let pattern = self.parse_pattern(PatternContext {
                function,
                ..PatternContext::default()
            })?;
            Ok(ForEachBinding::Using {
                asynchrony: using_asynchrony,
                pattern,
            })
        } else {
            let keyword = self.peek_for_each_keyword();

            if keyword.is_none() {
                // for each without declarations keeps expression heads as expression patterns
                let start = self.mark_parse_start();
                let expression = self.parse_expression(ExpressionContext {
                    function,
                    stops: ExpressionStops::FOR_EACH.with(ExpressionStops::BODY_BRACE),
                    ..ExpressionContext::default()
                })?;

                let pattern = self.insert_node(
                    Pattern::Expression { value: expression },
                    self.range_since(&start),
                );
                Ok(ForEachBinding::Pattern {
                    pattern,
                    keyword: None,
                })
            } else {
                self.bump();

                // declaration forms keep binding-pattern parsing
                let pattern = self.parse_pattern(PatternContext {
                    function,
                    ..PatternContext::default()
                })?;
                Ok(ForEachBinding::Pattern { pattern, keyword })
            }
        }
    }

    /// Return the using asynchrony when a for each header starts a using binding.
    fn peek_for_each_using_asynchrony(&self) -> Option<Asynchrony> {
        // resolve `using` with optional `await` prefix
        let asynchrony = if self.peek_using(Asynchrony::Async) {
            Asynchrony::Async
        } else if self.peek_using(Asynchrony::Sync) {
            Asynchrony::Sync
        } else {
            return None;
        };

        // keep the binding head on the same line
        let declarator_token_type = self.peek_using_binding_head_token(asynchrony)?;

        // using bindings start with a binding pattern shape
        if !Self::can_start_using_binding_pattern(declarator_token_type) {
            return None;
        }

        // disambiguate identifier starts that should remain expression headers
        if declarator_token_type == TokenType::Identifier {
            let declarator_keyword = self
                .peek_using_binding_head_offset(asynchrony)
                .and_then(|offset| self.peek_keyword_at(offset));

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
    fn peek_for_each_keyword(&self) -> Option<BindingKeyword> {
        match self.peek_keyword()? {
            Keyword::Let => Some(BindingKeyword::Let),
            Keyword::Const | Keyword::Readonly => Some(BindingKeyword::Const),
            _ => None,
        }
    }

    /// Parse one while or do-while loop.
    ///
    /// Examples:
    /// ```ds
    /// while (ready) work();
    /// do { work(); } while (ready)
    /// ```
    pub(crate) fn parse_while(
        &mut self,
        function: FunctionContext,
    ) -> ParserResult<LocalNodeId<Expression>> {
        let start = self.mark_parse_start();

        // do body while (condition)
        if self.peek_is_keyword(Keyword::Do) {
            // do
            self.bump();

            // body
            let body_id = self.parse_block_or_statement(function)?;

            // while
            self.eat_keyword(Keyword::While)?;

            // condition
            let condition_id = self.parse_parenthesized_expression(ExpressionContext {
                function,
                ..ExpressionContext::default()
            })?;

            // retain the complete do-while loop
            let while_id = self.insert_node(
                Expression::While {
                    label: None,
                    form: WhileForm::DoWhile,
                    condition: condition_id,
                    body: body_id,
                },
                self.range_since(&start),
            );
            Ok(while_id)
        }
        // while (condition) body
        else {
            // while
            self.eat_keyword(Keyword::While)?;

            // condition
            let condition_id = self.parse_parenthesized_expression(ExpressionContext {
                function,
                stops: ExpressionStops::BODY_BRACE,
                ..ExpressionContext::default()
            })?;

            // body
            let body_id = self.parse_block_or_statement(function)?;

            // retain the complete while loop
            let while_id = self.insert_node(
                Expression::While {
                    label: None,
                    form: WhileForm::While,
                    condition: condition_id,
                    body: body_id,
                },
                self.range_since(&start),
            );
            Ok(while_id)
        }
    }
}
