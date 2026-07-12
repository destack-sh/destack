use crate::parse::context::{ExpressionContext, ExpressionStops, FunctionContext, PatternContext};
use crate::parse::error::ParserResultExt;
use crate::{ParseStart, Parser, ParserError, ParserResult};

use destack_dir::{
    Block, BlockContext, BlockForm, Expression, Keyword, LocalNodeId, MatchCase, MatchForm,
    MatchSelector, NodeType, Pattern, TokenType,
};
use destack_source::{ByteRange, NodeSpanRegion, NodeSpanType};

/// One match guard and its clause range.
struct MatchGuard {
    /// The guard expression.
    expression: LocalNodeId<Expression>,
    /// The complete guard clause range.
    range: ByteRange,
}

impl Parser {
    /// Parse one match or switch expression.
    ///
    /// Examples:
    /// ```ds
    /// match (value) { Some(value): value; none: 0 }
    /// ```
    pub(crate) fn parse_match(
        &mut self,
        function: FunctionContext,
    ) -> ParserResult<LocalNodeId<Expression>> {
        // match or switch
        let keyword = self.eat_keyword_in(&[Keyword::Match, Keyword::Switch])?;
        let form = if keyword == Keyword::Switch {
            MatchForm::Switch
        } else {
            MatchForm::Match
        };

        // (value) { cases }
        self.parse_match_body(form, function)
    }

    /// Parse one match body after its keyword.
    pub(crate) fn parse_match_body(
        &mut self,
        form: MatchForm,
        function: FunctionContext,
    ) -> ParserResult<LocalNodeId<Expression>> {
        let start = self.mark_parse_start();

        // (value)
        let value_id = self.parse_parenthesized_expression(ExpressionContext {
            function,
            ..ExpressionContext::default()
        })?;

        // { cases }
        self.eat_token_before(TokenType::OpenBrace, TokenType::CloseBrace)
            .in_node(NodeType::MatchCase)?;
        let cases_id = self.parse_match_cases(form, function)?;
        self.eat_close_token_or_recover_missing(TokenType::CloseBrace, NodeType::MatchCase)?;

        // retain the complete expression
        let match_id = self.insert_node(
            Expression::Match {
                form,
                value: value_id,
                cases: cases_id,
            },
            self.range_since(&start),
        );
        Ok(match_id)
    }

    /// Parse a match-case guard when present.
    fn parse_match_guard(&mut self, function: FunctionContext) -> ParserResult<Option<MatchGuard>> {
        if !self.peek_is_keyword(Keyword::If) {
            return Ok(None);
        }

        let guard_start = self.mark_parse_start();
        self.eat_keyword(Keyword::If)?;
        let guard = self.parse_parenthesized_expression(ExpressionContext {
            function,
            ..ExpressionContext::default()
        })?;
        Ok(Some(MatchGuard {
            expression: guard,
            range: self.range_since(&guard_start),
        }))
    }

    /// Parse match cases until the closing brace.
    pub(crate) fn parse_match_cases(
        &mut self,
        form: MatchForm,
        function: FunctionContext,
    ) -> ParserResult<Vec<LocalNodeId<MatchCase>>> {
        let mut cases: Vec<LocalNodeId<MatchCase>> = Vec::new();
        let mut has_default_case = false;
        while self.has_more_tokens() {
            // read the current token once per iteration
            let token_type = self.peek_token_type();

            // stop on closing brace
            if token_type == TokenType::CloseBrace {
                break;
            }
            // allow statement separators between cases (newline/semicolon)
            else if Self::is_statement_stop_token(token_type) {
                self.eat_statement_stop()?;
            }
            // case
            else {
                // reject duplicate default selectors in switch blocks
                if form == MatchForm::Switch
                    && has_default_case
                    && self.peek_is_keyword(Keyword::Default)
                {
                    return Err(ParserError::unexpected(self.peek_token_span()));
                }

                let case = self
                    .parse_match_case(form, function)
                    .in_node(NodeType::MatchCase)?;

                // track default selectors for duplicate checks
                if form == MatchForm::Switch {
                    let selector = match self.tree.get(case) {
                        MatchCase::Expression { selector, .. }
                        | MatchCase::Block { selector, .. } => selector,
                    };
                    if matches!(selector, MatchSelector::Default) {
                        has_default_case = true;
                    }
                }

                cases.push(case);
            }
        }
        Ok(cases)
    }

    /// Parse one match case.
    fn parse_match_case(
        &mut self,
        form: MatchForm,
        function: FunctionContext,
    ) -> ParserResult<LocalNodeId<MatchCase>> {
        // decorators before match arms
        let decorators = if self.peek_is(TokenType::At) {
            self.parse_decorators(function)
        } else {
            smallvec::SmallVec::new()
        };

        let start = self.mark_parse_start();

        let (selector, guard_range) = match form {
            MatchForm::Switch => {
                // default case
                if self.peek_is_keyword(Keyword::Default) {
                    self.bump();
                    self.eat_token(TokenType::Colon)?;
                    (MatchSelector::Default, None)
                }
                // regular case
                else {
                    self.eat_keyword(Keyword::Case)?;
                    let pattern_start = self.mark_parse_start();
                    // allow a wildcard here so switch cases do not bind `_`
                    let pattern = if self.peek_identifier_is("_") {
                        self.bump();
                        self.insert_node(Pattern::Wildcard, self.range_since(&pattern_start))
                    } else {
                        let value = self.parse_expression(ExpressionContext {
                            function,
                            stops: ExpressionStops::MATCH_COLON,
                            ..ExpressionContext::default()
                        })?;
                        self.insert_node(
                            Pattern::Expression { value },
                            self.range_since(&pattern_start),
                        )
                    };

                    // guard
                    let guard = self.parse_match_guard(function)?;

                    self.eat_token(TokenType::Colon)?;
                    let guard_expression = guard.as_ref().map(|guard| guard.expression);
                    let guard_range = guard.map(|guard| guard.range);

                    (
                        MatchSelector::Pattern {
                            pattern,
                            guard: guard_expression,
                        },
                        guard_range,
                    )
                }
            }
            // match form
            MatchForm::Match => {
                // pattern
                let pattern = self.parse_pattern(PatternContext {
                    function,
                    is_match_case: true,
                    ..PatternContext::default()
                })?;

                // guard
                let guard = self.parse_match_guard(function)?;

                // "arrow"
                self.eat_token(TokenType::ArrowWide)?;

                let guard_expression = guard.as_ref().map(|guard| guard.expression);
                let guard_range = guard.map(|guard| guard.range);

                (
                    MatchSelector::Pattern {
                        pattern,
                        guard: guard_expression,
                    },
                    guard_range,
                )
            }
        };

        // parse the case body in its source form
        let case = if form == MatchForm::Switch {
            self.parse_switch_case_body(&start, selector, function)?
        } else if self.peek_block() {
            let block = self.parse_block(BlockContext::Expression, function)?;
            self.insert_node(
                MatchCase::Block {
                    selector,
                    body: block,
                },
                self.range_since(&start),
            )
        } else {
            let expression = self.parse_expression(ExpressionContext {
                function,
                stops: ExpressionStops::MATCH_COLON.with(ExpressionStops::MATCH_LINE),
                ..ExpressionContext::default()
            })?;
            self.insert_node(
                MatchCase::Expression {
                    selector,
                    body: expression,
                },
                self.range_since(&start),
            )
        };

        // retain the optional guard clause
        if let Some(guard_range) = guard_range {
            self.tree.set_side_range(
                case,
                NodeSpanType::Region(NodeSpanRegion::Guard),
                guard_range,
            );
        }

        // attach case decorators in source order
        if !decorators.is_empty() {
            self.attach_decorators(case.id, decorators);
        }

        Ok(case)
    }

    /// Parse one switch case body until the next case boundary.
    fn parse_switch_case_body(
        &mut self,
        start: &ParseStart,
        selector: MatchSelector,
        function: FunctionContext,
    ) -> ParserResult<LocalNodeId<MatchCase>> {
        let mut expressions = Vec::new();

        while self.has_more_tokens() {
            // consume empty statements between body expressions
            if self.peek_is(TokenType::Semicolon) {
                self.eat_statement_stop()?;
                continue;
            }

            // stop before the next case
            if self.peek_is_keyword(Keyword::Case)
                || self.peek_is_keyword(Keyword::Default)
                || self.peek_is(TokenType::CloseBrace)
            {
                break;
            }

            // parse one body statement
            let expression = self.parse_statement(function);
            expressions.push(expression);

            // consume an explicit separator without treating EOF as progress
            if matches!(
                self.peek_token_type(),
                TokenType::Comma | TokenType::Semicolon
            ) {
                self.eat_any_stop()?;
            }
        }

        // retain a lone expression without an implicit block
        if let [expression] = expressions.as_slice() {
            return Ok(self.insert_node(
                MatchCase::Expression {
                    selector,
                    body: *expression,
                },
                self.range_since(start),
            ));
        }

        // represent empty and multi-statement cases as implicit blocks
        let block = self.insert_node(
            Block {
                context: BlockContext::Statement,
                form: BlockForm::Implicit,
                leading_expressions: expressions,
                tail_expression: None,
            },
            self.range_since(start),
        );

        Ok(self.insert_node(
            MatchCase::Block {
                selector,
                body: block,
            },
            self.range_since(start),
        ))
    }
}
