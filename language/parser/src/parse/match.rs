use crate::parse::PendingDecorators;
use crate::parse::flags::ParserFlags;
use crate::parse::prelude::*;
use crate::{ParseResult, Parser};

use destack_dir::{
    Block, BlockContext, BlockForm, Expression, Keyword, LocalNodeId, MatchCase, MatchForm,
    MatchSelector, NodeType, Pattern, TokenType,
};
use destack_source::{NodeSpanRegion, NodeSpanType, Span};

impl Parser {
    /// Eat a match statement.
    ///
    /// Examples:
    /// ```
    /// match (<expr>) {
    ///     (x, y, ..) => {
    ///         ...
    ///     }
    ///     (x, y, z) => {
    ///         ...
    ///     }
    ///     _ = ohNoes()
    /// }
    /// ```
    pub fn eat_match(&mut self) -> ParseResult<LocalNodeId<Expression>> {
        // keyword
        let keyword = self.eat_keyword_in(&[Keyword::Match, Keyword::Switch])?;
        let form = if keyword == Keyword::Switch {
            MatchForm::Switch
        } else {
            MatchForm::Match
        };

        // body
        self.eat_match_body(form)
    }

    /// Eat a match body (without the match keyword)
    pub fn eat_match_body(&mut self, form: MatchForm) -> ParseResult<LocalNodeId<Expression>> {
        let start = self.span_start();

        // value
        let value_id = self.with_flags(self.match_value_flags(), |parser| {
            parser.eat_parenthesized_expression()
        })?;

        // cases
        self.try_eat_token(TokenType::OpenBrace, TokenType::CloseBrace)
            .for_node_type(NodeType::MatchCase)?;
        let cases_id = self.eat_match_cases(form)?;
        self.eat_close_token_or_recover_missing(TokenType::CloseBrace, NodeType::MatchCase)?;

        // match
        let match_id = self.insert_node(
            Expression::Match {
                form,
                value: value_id,
                cases: cases_id,
            },
            self.get_span_from(&start),
        );
        Ok(match_id)
    }

    /// Return parser flags for a match value expression.
    #[inline]
    fn match_value_flags(&self) -> ParserFlags {
        let ambient_context = self.flags.with_before_block(true);
        let expression_context = self.flags;

        self.flags
            .with_ambient_context(ambient_context)
            .with_expression_context(expression_context)
    }

    /// Return parser flags for a match-case pattern.
    #[inline]
    fn match_pattern_flags(&self) -> ParserFlags {
        let ambient_context = self.flags.with_match_case(true);
        let expression_context = self.flags;

        self.flags
            .with_ambient_context(ambient_context)
            .with_expression_context(expression_context)
    }

    /// Return parser flags for a match-case guard.
    #[inline]
    fn match_guard_flags(&self) -> ParserFlags {
        let ambient_context = self.flags.with_match_case(true).with_before_block(true);
        let expression_context = ParserFlags::default();

        self.flags
            .with_ambient_context(ambient_context)
            .with_expression_context(expression_context)
    }

    /// Eat a match-case guard when present.
    fn eat_match_guard(
        &mut self,
        guard_clause_span: &mut Option<Span>,
    ) -> ParseResult<Option<LocalNodeId<Expression>>> {
        if !self.is_keyword(Keyword::If) {
            return Ok(None);
        }

        let guard_start = self.span_start();
        self.eat_keyword(Keyword::If)?;
        let guard = self.with_flags(self.match_guard_flags(), |parser| {
            parser.eat_parenthesized_expression()
        })?;
        *guard_clause_span = Some(self.get_span_from(&guard_start));

        Ok(Some(guard))
    }

    /// Eat multiple match cases separated as statements (without the `{` and `}`).
    ///
    /// Examples:
    /// ```
    /// 2 => parse_int(2)
    /// (x, y) => {
    ///     ...
    /// }
    /// ```
    pub(crate) fn eat_match_cases(
        &mut self,
        form: MatchForm,
    ) -> ParseResult<Vec<LocalNodeId<MatchCase>>> {
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
                    && self.is_keyword(Keyword::Default)
                {
                    return Err(ParseError::unexpected(self.peek()?.span));
                }

                let case = self
                    .eat_match_case(form)
                    .for_node_type(NodeType::MatchCase)?;

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

    /// Eat a match case.
    ///
    /// Examples:
    /// ```
    /// 2 => parse_int(2)
    ///
    /// (x, y) if (x > y) => {
    ///     ...
    /// }
    /// ```
    fn eat_match_case(&mut self, form: MatchForm) -> ParseResult<LocalNodeId<MatchCase>> {
        // decorators before match arms
        let mut pending_case_decorators = if self.peek_is(TokenType::At) {
            self.eat_decorators_maybe()?
        } else {
            smallvec::SmallVec::new()
        };

        let start = self.span_start();
        let mut guard_clause_span = None;

        let selector = match form {
            MatchForm::Switch => {
                // default case
                if self.is_keyword(Keyword::Default) {
                    self.bump();
                    self.eat_colon()?;
                    MatchSelector::Default
                }
                // regular case
                else {
                    self.eat_keyword(Keyword::Case)?;
                    let pattern_start = self.span_start();
                    // allow a wildcard here so switch cases do not bind `_`
                    let pattern = if self.peek_identifier_str_is("_") {
                        self.bump();
                        self.tree
                            .insert(Pattern::Wildcard, self.get_span_from(&pattern_start))
                    } else {
                        let value = self.eat_expression(self.match_guard_flags())?;
                        self.insert_node(
                            Pattern::Expression { value },
                            self.get_span_from(&pattern_start),
                        )
                    };

                    // guard
                    let guard = self.eat_match_guard(&mut guard_clause_span)?;

                    self.eat_colon()?;
                    MatchSelector::Pattern { pattern, guard }
                }
            }
            // match form
            MatchForm::Match => {
                // pattern
                let pattern =
                    self.with_flags(self.match_pattern_flags(), |parser| parser.eat_pattern())?;

                // guard
                let guard = self.eat_match_guard(&mut guard_clause_span)?;

                // "arrow"
                self.eat_arrow()?;

                MatchSelector::Pattern { pattern, guard }
            }
        };

        // switch case body: consume statements until break or next case boundary
        if form == MatchForm::Switch {
            // eat expressions until we hit a break (inclusive) or case / default (exclusive)

            // empty case body before the next case, default, or closing brace
            let is_empty_case = self.is_keyword(Keyword::Case)
                || self.is_keyword(Keyword::Default)
                || self.peek_is(TokenType::CloseBrace);
            if is_empty_case {
                let block_id = self.insert_node(
                    Block {
                        context: BlockContext::Statement,
                        form: BlockForm::Implicit,
                        leading_expressions: Vec::new(),
                        tail_expression: None,
                    },
                    self.get_span_from(&start),
                );
                let match_case_id = self.insert_node(
                    MatchCase::Block {
                        selector,
                        body: block_id,
                    },
                    self.get_span_from(&start),
                );
                self.finish_match_case(
                    match_case_id,
                    guard_clause_span,
                    &mut pending_case_decorators,
                );

                return Ok(match_case_id);
            }
            let mut expressions: Vec<LocalNodeId<Expression>> = Vec::new();
            while self.has_more_tokens() {
                // consume empty statements between switch body statements
                if self.peek_is(TokenType::Semicolon) {
                    self.eat_statement_stop()?;
                    continue;
                }

                // stop at the next case boundary
                if self.is_keyword(Keyword::Case)
                    || self.is_keyword(Keyword::Default)
                    || self.peek_is(TokenType::CloseBrace)
                {
                    break;
                }
                let statement_start = self.span_start();
                let expression_id = self
                    .with_statement_recovery(
                        &statement_start,
                        |parser| parser.try_eat_statement_expression().map(Some),
                        None,
                    )
                    .unwrap_or_else(|| {
                        self.tree
                            .insert(Expression::Error, self.get_span_from(&statement_start))
                    });
                expressions.push(expression_id);

                // consume real separators without treating eof as progress
                if matches!(
                    self.peek_token_type(),
                    TokenType::Comma | TokenType::Semicolon
                ) {
                    self.eat_any_stop()?;
                }
            }
            // single expression case
            let match_case_id = if expressions.len() == 1 {
                self.insert_node(
                    MatchCase::Expression {
                        selector,
                        body: expressions[0],
                    },
                    self.get_span_from(&start),
                )
            }
            // multiple expression block
            else {
                let block_id = self.insert_node(
                    Block {
                        context: BlockContext::Statement,
                        form: BlockForm::Implicit,
                        leading_expressions: expressions,
                        tail_expression: None,
                    },
                    self.get_span_from(&start),
                );
                self.insert_node(
                    MatchCase::Block {
                        selector,
                        body: block_id,
                    },
                    self.get_span_from(&start),
                )
            };
            self.finish_match_case(
                match_case_id,
                guard_clause_span,
                &mut pending_case_decorators,
            );
            Ok(match_case_id)
        }
        // block body
        else if self.is_block_start() {
            let block_id = self.eat_block(BlockContext::Expression)?;
            let match_case_id = self.insert_node(
                MatchCase::Block {
                    selector,
                    body: block_id,
                },
                self.get_span_from(&start),
            );
            self.finish_match_case(
                match_case_id,
                guard_clause_span,
                &mut pending_case_decorators,
            );
            Ok(match_case_id)
        }
        // single expression
        else {
            let expression_id = self.with_flags(self.flags.in_match_case_body(), |parser| {
                parser.eat_expression(parser.flags.not_in_sequence_expression())
            })?;
            let match_case_id = self.insert_node(
                MatchCase::Expression {
                    selector,
                    body: expression_id,
                },
                self.get_span_from(&start),
            );
            self.finish_match_case(
                match_case_id,
                guard_clause_span,
                &mut pending_case_decorators,
            );
            Ok(match_case_id)
        }
    }

    /// Finish spans and decorators for one match case.
    fn finish_match_case(
        &mut self,
        match_case_id: LocalNodeId<MatchCase>,
        guard_clause_span: Option<Span>,
        pending_case_decorators: &mut PendingDecorators,
    ) {
        self.record_match_case_guard_clause(match_case_id, guard_clause_span);

        if !pending_case_decorators.is_empty() {
            self.attach_decorators(match_case_id.id, std::mem::take(pending_case_decorators));
        }
    }

    /// Record the guard clause span for one match case.
    fn record_match_case_guard_clause(
        &mut self,
        match_case_id: LocalNodeId<MatchCase>,
        guard_clause_span: Option<Span>,
    ) {
        if let Some(guard_clause_span) = guard_clause_span {
            self.tree.set_side_span(
                match_case_id,
                NodeSpanType::Region(NodeSpanRegion::Clause),
                guard_clause_span,
            );
        }
    }
}
