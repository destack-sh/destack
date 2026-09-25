use tspp_dir::{
    Asynchrony, BindingKeyword, BlockContext, Condition, Expression, ForEachBinding, Keyword,
    LocalNodeId, NodeType, Pattern, TokenType, WhileForm,
};
use tspp_source::{NodeSpanRegion, NodeSpanType};

use crate::parse::{ExpressionPosition, ExpressionStop};
use crate::{Parser, ParserResult};

impl Parser {
    /// Parse one unconditional loop.
    ///
    /// Examples:
    /// ```tspp
    /// loop { work(); }
    /// ```
    pub(crate) fn parse_loop(&mut self) -> ParserResult<LocalNodeId<Expression>> {
        let start = self.mark_parse_start();

        // loop
        let keyword_range = self.eat_keyword(Keyword::Loop)?.range();

        // { body }
        let body_id = self.parse_block(BlockContext::Statement)?;

        // retain the complete loop
        let loop_id = self.insert_node(
            Expression::Loop {
                label: None,
                body: body_id,
            },
            self.range_since(&start),
        );
        self.set_node_keyword_range(loop_id, keyword_range);

        Ok(loop_id)
    }

    /// Parse one condition or iteration loop.
    ///
    /// Examples:
    /// ```tspp
    /// for (item of items) visit(item);
    /// for (let index = 0; index < count; index++) work(index);
    /// ```
    pub(crate) fn parse_for(&mut self) -> ParserResult<LocalNodeId<Expression>> {
        let start = self.mark_parse_start();

        // for
        let keyword_range = self.eat_keyword(Keyword::For)?.range();

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
            let initialization_id =
                if self.peek_is(TokenType::Semicolon) {
                    None
                } else {
                    Some(self.parse_expression(
                        ExpressionPosition::Statement,
                        ExpressionStop::default(),
                    )?)
                };
            self.eat_token(TokenType::Semicolon)?;

            // condition;
            let condition_id = if self.peek_is(TokenType::Semicolon) {
                None
            } else {
                Some(self.parse_expression(ExpressionPosition::Value, ExpressionStop::default())?)
            };
            self.eat_token(TokenType::Semicolon)?;

            // increment)
            let increment_id = if self.peek_is(TokenType::CloseParenthesis) {
                None
            } else {
                Some(self.parse_expression(ExpressionPosition::Value, ExpressionStop::default())?)
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
            let body_id = self.parse_block_or_statement()?;

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
            self.set_node_keyword_range(for_id, keyword_range);

            Ok(for_id)
        }
        // for [await] (binding of iterator) body
        else {
            // retain the binding declaration keyword
            let binding_keyword_range = self
                .peek_for_each_keyword()
                .map(|_| self.peek_token_span().token.range());

            // binding
            let binding = self.parse_for_each_binding()?;

            // of
            self.eat_keyword(Keyword::Of)?;

            // iterator)
            let is_body_brace_ambiguous =
                header_contains_semicolon.is_none() && !self.peek_is(TokenType::OpenBrace);
            let stops = if is_body_brace_ambiguous {
                ExpressionStop::BODY_BRACE
            } else {
                ExpressionStop::default()
            };
            let iterator_id = self.parse_expression(ExpressionPosition::Value, stops)?;

            // recover the closing parenthesis before the body
            self.eat_close_token_or_recover_missing_with(
                TokenType::CloseParenthesis,
                NodeType::Expression,
                |parser, token_type| {
                    Self::is_close_delimiter_boundary_token(token_type) || parser.peek_block()
                },
            )?;

            // body
            let body_id = self.parse_block_or_statement()?;

            // retain the complete iteration loop
            let for_id = self.insert_node(
                Expression::ForEach {
                    label: None,
                    asynchrony,
                    binding,
                    iterator: iterator_id,
                    body: body_id,
                },
                self.range_since(&start),
            );
            self.set_node_keyword_range(for_id, keyword_range);
            if let Some(binding_keyword_range) = binding_keyword_range {
                self.tree.set_side_range(
                    for_id,
                    NodeSpanType::Region(NodeSpanRegion::BindingKeyword),
                    binding_keyword_range,
                );
            }

            Ok(for_id)
        }
    }

    /// Parse a for-each binding pattern or using declaration.
    pub(crate) fn parse_for_each_binding(&mut self) -> ParserResult<ForEachBinding> {
        if let Some(using_asynchrony) = self.peek_for_each_using_asynchrony() {
            if using_asynchrony == Asynchrony::Async {
                self.bump();
            }

            self.bump();
            let pattern = self.parse_pattern()?;
            Ok(ForEachBinding::Using {
                asynchrony: using_asynchrony,
                pattern,
            })
        } else {
            let keyword = self.peek_for_each_keyword();

            if keyword.is_none() {
                // for each without declarations keeps expression heads as expression patterns
                let start = self.mark_parse_start();
                let stop = ExpressionStop::FOR_EACH.add(ExpressionStop::BODY_BRACE);
                let expression = self.parse_expression(ExpressionPosition::Value, stop)?;

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
                let pattern = self.parse_pattern()?;
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
    /// ```tspp
    /// while (ready) work();
    /// do { work(); } while (ready)
    /// ```
    pub(crate) fn parse_while(&mut self) -> ParserResult<LocalNodeId<Expression>> {
        let start = self.mark_parse_start();

        // do body while (condition)
        if self.peek_is_keyword(Keyword::Do) {
            // do
            let keyword_range = self.eat_keyword(Keyword::Do)?.range();

            // body
            let body_id = self.parse_block_or_statement()?;

            // while
            self.eat_keyword(Keyword::While)?;

            // condition
            let condition = self.parse_parenthesized_expression(ExpressionPosition::Value)?;
            let condition = Condition::expression(condition);

            // retain the complete do-while loop
            let while_id = self.insert_node(
                Expression::While {
                    label: None,
                    form: WhileForm::DoWhile,
                    condition,
                    body: body_id,
                },
                self.range_since(&start),
            );
            self.set_node_keyword_range(while_id, keyword_range);

            Ok(while_id)
        }
        // while (condition) body
        else {
            // while
            let keyword_range = self.eat_keyword(Keyword::While)?.range();

            // condition
            let condition = self.parse_parenthesized_condition(
                ExpressionPosition::Value,
                ExpressionStop::BODY_BRACE,
            )?;

            // body
            let body_id = self.parse_block_or_statement()?;

            // retain the complete while loop
            let while_id = self.insert_node(
                Expression::While {
                    label: None,
                    form: WhileForm::While,
                    condition,
                    body: body_id,
                },
                self.range_since(&start),
            );
            self.set_node_keyword_range(while_id, keyword_range);

            Ok(while_id)
        }
    }
}
