use destack_ast::{
    Block, BlockFormat, Decorator, Expression, Keyword, LetKind, LocalNodeId, NodeType, TokenType,
    YieldCardinality,
};

use crate::parse::prelude::*;
use crate::{ParseError, ParseResult, Parser, ParserMark};

impl Parser {
    /// Return true when the current token sequence starts a block.
    #[inline]
    pub(crate) fn is_block_start(&mut self) -> bool {
        self.peek_is(TokenType::OpenBrace)
            || self.language.is_destack()
                && self.is_keyword(Keyword::Do)
                && self.peek_next_is(TokenType::OpenBrace)
    }

    /// Return true when the next token sequence starts a block.
    #[inline]
    pub(crate) fn is_next_block_start(&mut self) -> bool {
        self.peek_next_is(TokenType::OpenBrace)
            || self.language.is_destack()
                && self.is_next_keyword(Keyword::Do)
                && self.token_type_at(self.index_for_next_next()) == TokenType::OpenBrace
    }

    /// Eat an expression in statement position with explicit parser options.
    #[inline]
    pub(crate) fn eat_statement_expression(
        &mut self,
        options: ParserOptions,
    ) -> ParseResult<LocalNodeId<Expression>> {
        let options = options.in_statement_position();
        let old_options = self.options;
        self.options = options;
        let result = self.eat_statement_expression_in_current_options();
        self.options = old_options;
        result
    }

    /// Eat an expression in a non-position context.
    #[inline]
    fn eat_expression_not_in_position(&mut self) -> ParseResult<LocalNodeId<Expression>> {
        self.eat_expression(self.options.not_in_position())
    }

    /// Return true when a token ends the current block body.
    #[inline]
    fn is_block_body_terminator_token(&self, token_type: TokenType, format: BlockFormat) -> bool {
        token_type == TokenType::End
            || (token_type == TokenType::CloseBrace && format != BlockFormat::Implicit)
    }

    /// Push an expression into a statement list, wrapping when statement coercion is required.
    #[inline]
    fn push_block_body_expression(
        &mut self,
        statements: &mut Vec<LocalNodeId<Expression>>,
        expression_id: LocalNodeId<Expression>,
        is_statement: bool,
        force_statement: bool,
    ) {
        if is_statement || !force_statement {
            statements.push(expression_id);
            return;
        }

        let statement_id =
            self.wrap_statement_expression(expression_id, self.tree.get_span(expression_id));
        statements.push(statement_id);
    }

    /// Try to parse a labelled statement before generic statement keyword dispatch.
    fn try_eat_labelled_statement_expression_fast(
        &mut self,
        start: &ParserMark,
    ) -> ParseResult<Option<LocalNodeId<Expression>>> {
        // labels only start on `identifier:`
        if !self.peek_is(TokenType::Identifier) || !self.peek_next_is(TokenType::Colon) {
            return Ok(None);
        }

        // inspect the label target to determine whether label parsing is allowed here
        let colon_index = self.index_for_next();
        let label_target_index = self.next_non_newline_index_from(colon_index + 1);
        let label_target_token = self.token_at(label_target_index);
        let label_target_keyword = label_target_token
            .filter(|token| token.token.ty == TokenType::Identifier)
            .and_then(|_| self.keyword_for_index_maybe_fast(label_target_index));

        // label targets that are always expression statements
        let is_labelled_expression = matches!(
            label_target_keyword,
            Some(
                Keyword::While
                    | Keyword::Do
                    | Keyword::For
                    | Keyword::Loop
                    | Keyword::If
                    | Keyword::Switch
                    | Keyword::Try
                    | Keyword::With
            )
        );

        // labelled blocks are only allowed in statement position
        let is_labelled_block =
            label_target_token.is_some_and(|token| token.token.ty == TokenType::OpenBrace);
        let can_parse_label = if self.options.in_statement_position && !self.language.is_destack() {
            true
        } else {
            is_labelled_expression || (self.options.in_statement_position && is_labelled_block)
        };

        if !can_parse_label {
            return Ok(None);
        }

        // parse label prefix
        let (label, label_span) = self.eat_identifier_with_span()?;
        self.eat_colon()?;
        self.eat_newlines_maybe()?;

        // allow empty labelled statements (`label:;`)
        let body = if self.peek_is(TokenType::Semicolon) {
            let body_start = self.mark_span();
            self.bump(); // eat semicolon
            let block_id = self.tree.insert(
                Block {
                    format: BlockFormat::Implicit,
                    expressions: Vec::new(),
                },
                self.get_span_from(&body_start),
            );
            self.tree
                .insert(Expression::Block(block_id), self.get_span_from(&body_start))
        } else {
            self.eat_expression(self.options)?
        };

        // reject labelled declarations in JS and TS
        if !self.language.is_destack() && self.is_single_statement_declaration(body) {
            return Err(ParseError::unexpected(self.tree.get_span(body)));
        }

        // build labelled expression
        let labelled_id = self.tree.insert(
            Expression::Labelled { label, body },
            self.get_span_from(start),
        );
        self.tree.set_main_span(labelled_id, label_span);

        Ok(Some(labelled_id))
    }

    /// Try to dispatch a statement expression from a scanner-style cursor.
    #[inline]
    fn try_eat_statement_expression_fast_dispatch(
        &mut self,
        start: &ParserMark,
        cursor: NonNewlineTokenCursor,
    ) -> ParseResult<Option<LocalNodeId<Expression>>> {
        // direct block statement dispatch avoids generic expression entry
        if cursor.token_type == TokenType::OpenBrace {
            let block_id = self.eat_block()?;
            let expression_id = self
                .tree
                .insert(Expression::Block(block_id), self.get_span_from(start));
            return Ok(Some(expression_id));
        }

        // identifier led dispatch handles labels, statement keywords, and plain identifiers
        if cursor.token_type != TokenType::Identifier {
            return Ok(None);
        }

        if let Some(expression_id) = self.try_eat_labelled_statement_expression_fast(start)? {
            return Ok(Some(expression_id));
        }

        if let Some(expression_id) = self.try_eat_statement_keyword_expression_fast(start)? {
            let expression = self.tree.get(expression_id);
            let is_terminal_statement = matches!(expression, Expression::Statement(_))
                || expression.is_top_level_statement();
            if is_terminal_statement {
                return Ok(Some(expression_id));
            }

            let continuation_id = self.eat_expression_continuation(start, expression_id)?;
            return Ok(Some(continuation_id));
        }

        if let Some(expression_id) = self.try_eat_plain_identifier_expression_fast(start)? {
            return Ok(Some(expression_id));
        }

        Ok(None)
    }

    /// Eat one statement expression in the current parser options.
    #[inline]
    pub(crate) fn eat_statement_expression_in_current_options(
        &mut self,
    ) -> ParseResult<LocalNodeId<Expression>> {
        // normalize to the next non-newline token once per dispatch
        let cursor = self.normalize_to_scanner_cursor();

        let start = self.mark_span();
        if let Some(expression_id) =
            self.try_eat_statement_expression_fast_dispatch(&start, cursor)?
        {
            return Ok(expression_id);
        }

        // non identifier statements can parse through expression mode without statement flags
        if cursor.token_type != TokenType::Identifier {
            let old_options = self.swap_options(self.options.not_in_statement_position());
            let expression_id = self.eat_expression_without_statement_keyword_fast();
            self.restore_options(old_options);
            return expression_id;
        }

        // fallback: parse through the full expression parser
        self.eat_expression_without_statement_keyword_fast()
    }

    /// Eat a block or a single statement wrapped in a block.
    pub fn eat_block_or_statement(&mut self) -> ParseResult<LocalNodeId<Block>> {
        self.eat_newlines_maybe()?;

        // if it's a block, just eat it
        if self.is_block_start() {
            return self.eat_block();
        }

        let start = self.mark_span();

        // empty statement (just semicolon, e.g., `for (x of y);`)
        if self.peek_is(TokenType::Semicolon) {
            self.bump();
            let block_id = self.tree.insert(
                Block {
                    format: BlockFormat::Implicit,
                    expressions: vec![],
                },
                self.get_span_from(&start),
            );
            return Ok(block_id);
        }

        // otherwise, eat a single statement and wrap it in a block
        let statement_options = self.options.nested().in_statement_position();
        let old_options = self.options;
        self.options = statement_options;
        let expression_id = self.eat_statement_expression_in_current_options();
        self.options = old_options;
        let expression_id = expression_id?;

        // reject declaration statements in single statement contexts
        if !self.language.is_destack() && self.is_single_statement_declaration(expression_id) {
            return Err(ParseError::unexpected(self.tree.get_span(expression_id)));
        }

        // consume trailing semicolon if present (e.g., `do x; while (true)`)
        if self.peek_is(TokenType::Semicolon) {
            self.bump();
        }

        // wrap in a block
        let block_id = self.tree.insert(
            Block {
                format: BlockFormat::Implicit,
                expressions: vec![expression_id],
            },
            self.get_span_from(&start),
        );
        Ok(block_id)
    }

    /// Check whether a statement expression is a declaration in a single statement context.
    pub(crate) fn is_single_statement_declaration(
        &self,
        expression_id: LocalNodeId<Expression>,
    ) -> bool {
        // unwrap statement and label layers
        let current = self.unwrap_statement_expression(expression_id);

        // detect declaration expressions that are invalid in single statement contexts
        match self.tree.get(current) {
            Expression::Declaration(_)
            | Expression::Using { .. }
            | Expression::Import { .. }
            | Expression::Export { .. }
            | Expression::ExportNamespace { .. } => true,
            Expression::Let { kind, .. } => *kind != LetKind::Var,
            _ => false,
        }
    }

    /// Peek a block. Optional `do` prefix for disambiguation.
    #[inline]
    pub fn peek_block(&mut self) -> ParseResult<()> {
        if self.is_block_start() {
            Ok(())
        } else {
            Err(ParseError::expected(self.eof_span(), TokenType::OpenBrace))
        }
    }

    /// Peek a next block. Optional `do` prefix for disambiguation.
    #[inline]
    pub fn peek_next_block(&mut self) -> ParseResult<()> {
        if self.is_next_block_start() {
            Ok(())
        } else {
            Err(ParseError::expected(self.eof_span(), TokenType::OpenBrace))
        }
    }

    /// Eat a block (including the label, `{`, and `}`). Optional `do` prefix for disambiguation.
    ///
    /// Examples:
    /// ```
    /// { ... }
    /// block: { ... }
    pub fn eat_block(&mut self) -> ParseResult<LocalNodeId<Block>> {
        let start = self.mark_span();

        // `do` prefix
        if self.language.is_destack() && self.is_keyword(Keyword::Do) {
            self.bump(); // eat keyword
        }

        // body
        self.try_eat_token(TokenType::OpenBrace, TokenType::CloseBrace)
            .for_node_type(NodeType::Block)?;
        let expressions = self
            .eat_block_body(BlockFormat::Explicit)
            .for_node_type(NodeType::Block)?;
        self.eat_token(TokenType::CloseBrace)?;

        // block
        let block_id = self.tree.insert(
            Block {
                format: BlockFormat::Explicit,
                expressions,
            },
            self.get_span_from(&start),
        );
        Ok(block_id)
    }

    /// Eat a block of expressions (without the label, `{`, and `}`).
    /// ASI rules apply such that expressions are automatically coerced into statements in relevant positions.
    pub fn eat_block_body(
        &mut self,
        format: BlockFormat,
    ) -> ParseResult<Vec<LocalNodeId<Expression>>> {
        let _timing = self.timing_scope(tags::PARSE_BLOCK_BODY);

        // keep statement options for the whole body to avoid per statement option churn
        let statement_options = self.options.nested().in_statement_position();
        let old_options = self.options;
        self.options = statement_options;

        // parse all statement items and keep at most one tail expression
        let result = (|| {
            let mut statements: Vec<LocalNodeId<Expression>> = Vec::new();
            let mut pending_tail_expression: Option<LocalNodeId<Expression>> = None;
            let mut pending_statement_decorators: Vec<LocalNodeId<Decorator>> = Vec::new();

            loop {
                // normalize block body cursor once per iteration
                let cursor = self.peek_cursor();
                if cursor.index != self.pos_index() {
                    self.advance_to(cursor.index);
                }
                let token_type = cursor.token_type;

                // stop at block terminators
                // NOTE #Cleanup: recover block parse more explicitly?
                if self.is_block_body_terminator_token(token_type, format) {
                    break;
                }

                // consume statement separators
                if token_type == TokenType::Semicolon {
                    self.bump(); // eat semicolon
                    continue;
                }

                // consume decorator prefixes for the next statement item
                if token_type == TokenType::At {
                    let mut decorators = self.eat_decorators_prefix_collect_maybe()?;
                    pending_statement_decorators.append(&mut decorators);
                    continue;
                }

                // previous tail expressions are no longer block tails once a new item starts
                if let Some(pending_id) = pending_tail_expression.take() {
                    self.push_block_body_expression(&mut statements, pending_id, false, true);
                }

                // parse and recover one statement item
                let start = self.mark_span();
                let (expression_id, is_statement) =
                    match self.eat_statement_expression_in_current_options() {
                        Ok(expression_id) => {
                            self.finalize_statement_expression_with_flag(&start, expression_id)?
                        }
                        Err(err) => {
                            let err = err.for_node_type(NodeType::Expression);
                            let span = err.leaf_span();
                            let start = ParserMark::from_span(span);
                            self.try_recover(&start, TokenType::Newline, Some(err))?;
                            let error_id = self
                                .tree
                                .insert(Expression::Error, self.get_span_from(&start));
                            (error_id, true)
                        }
                    };

                // attach any pending decorators to this statement item
                if !pending_statement_decorators.is_empty() {
                    self.attach_decorators_to_target(
                        std::mem::take(&mut pending_statement_decorators),
                        expression_id.id,
                    );
                }

                // keep at most one tail candidate, emit statements directly
                if is_statement {
                    statements.push(expression_id);
                } else {
                    pending_tail_expression = Some(expression_id);
                }
            }

            // finalize the remaining tail expression
            if let Some(expression_id) = pending_tail_expression {
                let force_statement = format == BlockFormat::Implicit;
                self.push_block_body_expression(
                    &mut statements,
                    expression_id,
                    false,
                    force_statement,
                );
            }

            Ok(statements)
        })();

        self.options = old_options;
        result
    }

    /// Try to eat a statement expression (return Expression::Error if error and recovery is possible).
    /// Returns whether the expression should be treated as a statement.
    pub fn try_eat_statement_expression_with_flag(
        &mut self,
    ) -> ParseResult<(LocalNodeId<Expression>, bool)> {
        let statement_options = self.options.nested().in_statement_position();
        let old_options = self.options;
        self.options = statement_options;
        let result = self.try_eat_statement_expression_with_flag_in_statement_position();
        self.options = old_options;
        result
    }

    /// Try to eat a statement expression while already in statement position.
    /// Returns whether the expression should be treated as a statement.
    #[inline]
    fn try_eat_statement_expression_with_flag_in_statement_position(
        &mut self,
    ) -> ParseResult<(LocalNodeId<Expression>, bool)> {
        let start = self.mark_span();

        match self.eat_statement_expression_in_current_options() {
            Ok(expression_id) => {
                self.finalize_statement_expression_with_flag(&start, expression_id)
            }
            Err(err) => {
                let err = err.for_node_type(NodeType::Expression);
                let span = err.leaf_span();
                let start = ParserMark::from_span(span);
                self.try_recover(&start, TokenType::Newline, Some(err))?;
                let error_id = self
                    .tree
                    .insert(Expression::Error, self.get_span_from(&start));
                Ok((error_id, true))
            }
        }
    }

    /// Finalize statement parsing with separator checks and statement coercion.
    #[inline]
    fn finalize_statement_expression_with_flag(
        &mut self,
        start: &ParserMark,
        expression_id: LocalNodeId<Expression>,
    ) -> ParseResult<(LocalNodeId<Expression>, bool)> {
        // semicolon terminated expressions always become statement expressions
        if self.peek_token_type() == TokenType::Semicolon {
            self.bump(); // eat semicolon
            let expression_id =
                self.wrap_statement_expression(expression_id, self.get_span_from(start));
            return Ok((expression_id, true));
        }

        // detect expression kinds that are already statements
        let expression = self.tree.get(expression_id);
        let is_statement =
            matches!(expression, Expression::Statement(_)) || expression.is_top_level_statement();
        let separator_cursor = self.peek_cursor();
        let has_separator = matches!(
            separator_cursor.token_type,
            TokenType::Semicolon | TokenType::CloseBrace | TokenType::End
        ) || separator_cursor.has_line_break_before;

        // require statement separators after expressions to avoid token glue
        if !is_statement && !has_separator {
            return Err(ParseError::unexpected(self.peek()?.span));
        }

        Ok((expression_id, is_statement))
    }

    /// Try to eat a statement expression (return Expression::Error if error and recovery is possible).
    /// Wraps semicolon expressions in a Statement expression, otherwise just returns the expression.
    pub fn try_eat_statement_expression(&mut self) -> ParseResult<LocalNodeId<Expression>> {
        let (expression_id, _is_statement) = self.try_eat_statement_expression_with_flag()?;
        Ok(expression_id)
    }

    /// Eat a break expression.
    ///
    /// Examples:
    /// ```
    /// break
    /// break :label
    /// break label      // JS-style (no colon)
    /// break :label 15  // Destack extension: label + value
    /// ```
    pub fn eat_break(&mut self) -> ParseResult<LocalNodeId<Expression>> {
        let start = self.mark_span();
        self.eat_keyword(Keyword::Break)?;

        // label and value:
        // 1. `:identifier` → Destack style label, optionally followed by value
        // 2. `identifier` at statement stop → JS style label (no value)
        // 3. Otherwise → value expression (Destack extension, no label)
        let (label, label_span, value_id) = if self.peek_is(TokenType::Colon) {
            // Destack style: break :label [value]
            self.bump(); // eat colon
            let (label, label_span) = self.eat_identifier_with_span()?;
            let value_id = if self.has_more_tokens() && !self.is_statement_stop() {
                let value_id = self.eat_expression_not_in_position()?;
                Some(value_id)
            } else {
                None
            };
            (Some(label), Some(label_span), value_id)
        } else if self.peek_is(TokenType::Identifier)
            && matches!(
                self.peek_next_token_type(),
                TokenType::Newline | TokenType::Semicolon | TokenType::End | TokenType::CloseBrace
            )
        {
            // JS style: break label (identifier followed by statement stop)
            let (label, label_span) = self.eat_identifier_with_span()?;
            (Some(label), Some(label_span), None)
        } else if self.has_more_tokens() && !self.is_statement_stop() {
            // Destack extension: break value (no label)
            let value_id = self.eat_expression_not_in_position()?;
            (None, None, Some(value_id))
        } else {
            (None, None, None)
        };

        // break
        let break_id = self.tree.insert(
            Expression::Break {
                label,
                value: value_id,
            },
            self.get_span_from(&start),
        );
        if let Some(label_span) = label_span {
            self.tree.set_main_span(break_id, label_span);
        }
        Ok(break_id)
    }

    /// Eat a continue expression.
    ///
    /// Examples:
    /// ```
    /// continue
    /// continue :label  // Destack style
    /// continue label   // JS style (no colon)
    /// ```
    pub fn eat_continue(&mut self) -> ParseResult<LocalNodeId<Expression>> {
        let start = self.mark_span();
        self.eat_keyword(Keyword::Continue)?;

        // label parsing:
        // 1. `:identifier` → Destack style label
        // 2. `identifier` at statement stop → JS style label
        let (label, label_span) = if self.peek_is(TokenType::Colon) {
            // Destack style: continue :label
            self.bump(); // eat colon
            let (label, label_span) = self.eat_identifier_with_span()?;
            (Some(label), Some(label_span))
        } else if self.peek_is(TokenType::Identifier)
            && matches!(
                self.peek_next_token_type(),
                TokenType::Newline | TokenType::Semicolon | TokenType::End | TokenType::CloseBrace
            )
        {
            // JS style: continue label
            let (label, label_span) = self.eat_identifier_with_span()?;
            (Some(label), Some(label_span))
        } else {
            (None, None)
        };

        // continue
        let continue_id = self
            .tree
            .insert(Expression::Continue { label }, self.get_span_from(&start));
        if let Some(label_span) = label_span {
            self.tree.set_main_span(continue_id, label_span);
        }
        Ok(continue_id)
    }

    /// Eat an await expression.
    ///
    /// Examples:
    /// ```
    /// await someFunction()
    /// await? someFallibleAsync()
    /// ```
    pub fn eat_await(&mut self) -> ParseResult<LocalNodeId<Expression>> {
        let start = self.mark_span();

        // keyword
        self.eat_keyword(Keyword::Await)?;

        // check for await? (sugar for (await expr)?)
        let is_maybe = self.peek_is(TokenType::Maybe);
        if is_maybe {
            self.bump(); // eat ?
        }

        // expression
        let expression_id = self.eat_expression_not_in_position()?;

        // await or await?
        let expression = if is_maybe {
            Expression::AwaitMaybe {
                expression: expression_id,
            }
        } else {
            Expression::Await {
                expression: expression_id,
            }
        };
        let await_id = self.tree.insert(expression, self.get_span_from(&start));
        Ok(await_id)
    }

    /// Eat a comptime expression.
    ///
    /// Examples:
    /// ```
    /// comptime 1 + 2
    /// comptime factorial(10)
    /// comptime { generateLookupTable() }
    /// ```
    pub fn eat_comptime(&mut self) -> ParseResult<LocalNodeId<Expression>> {
        let start = self.mark_span();

        // keyword
        self.eat_keyword(Keyword::Comptime)?;

        // body expression
        self.eat_newlines_maybe()?;
        // parse body with comptime statement options
        let comptime_options = self.options.not_in_position().in_comptime();
        let old_options = self.options;
        self.options = comptime_options;
        let body_id = self.eat_statement_expression_in_current_options();
        self.options = old_options;
        let body_id = body_id?;

        // comptime
        let comptime_id = self.tree.insert(
            Expression::Comptime { body: body_id },
            self.get_span_from(&start),
        );
        Ok(comptime_id)
    }

    /// Eat a yield expression.
    ///
    /// Examples:
    /// ```
    /// yield
    /// yield someValue
    /// yield* someIterator
    /// yield *a  // same as yield* a (only if no newline after yield)
    /// ```
    pub fn eat_yield(&mut self) -> ParseResult<LocalNodeId<Expression>> {
        let start = self.mark_span();

        // keyword
        self.eat_keyword(Keyword::Yield)?;

        // stop when yield has no explicit operand in this position
        if self.yield_operand_is_omitted() {
            let yield_id = self.tree.insert(
                Expression::Yield {
                    cardinality: YieldCardinality::Scalar,
                    value: None,
                },
                self.get_span_from(&start),
            );
            return Ok(yield_id);
        }

        // cardinality: `yield*` or `yield *` (space before *, but no newline)
        let cardinality = if self.peek_is(TokenType::Multiply) {
            self.bump(); // eat *
            YieldCardinality::Generator
        } else {
            YieldCardinality::Scalar
        };

        // value (optional, like return/throw)
        // yield without value is valid: `function* a() { yield }`
        let value_id = if self.has_more_tokens() && !self.yield_operand_is_omitted() {
            let value_id = self.eat_expression_not_in_position()?;
            Some(value_id)
        } else {
            None
        };

        // `yield*` always requires an operand
        if cardinality == YieldCardinality::Generator && value_id.is_none() {
            return Err(ParseError::unexpected_for(
                self.get_span_from(&start),
                NodeType::Expression,
            ));
        }

        // yield
        let yield_id = self.tree.insert(
            Expression::Yield {
                cardinality,
                value: value_id,
            },
            self.get_span_from(&start),
        );
        Ok(yield_id)
    }

    /// Return true when yield has no explicit operand in this context.
    #[inline]
    fn yield_operand_is_omitted(&mut self) -> bool {
        let cursor = self.peek_cursor();

        // line breaks and statement delimiters terminate bare yield
        if cursor.has_line_break_before
            || matches!(
                cursor.token_type,
                TokenType::Semicolon | TokenType::End | TokenType::CloseBrace
            )
        {
            return true;
        }

        // punctuation that closes the surrounding expression also terminates bare yield
        matches!(
            cursor.token_type,
            TokenType::CloseParenthesis
                | TokenType::CloseBracket
                | TokenType::CloseBrace
                | TokenType::Comma
                | TokenType::Colon
        )
    }

    /// Return true when trivia before the current token contains a line terminator.
    pub(crate) fn has_line_terminator_before_current_token(&mut self) -> bool {
        self.peek_cursor().has_line_break_before
    }

    /// Eat a throw expression.
    ///
    /// `throw` is a restricted production: a newline after `throw` triggers ASI,
    /// but unlike `return`, `throw` REQUIRES an expression, so `throw;` is invalid.
    ///
    /// Examples:
    /// ```
    /// throw someError
    /// throw anyOldExpression()
    /// ```
    pub fn eat_throw(&mut self) -> ParseResult<LocalNodeId<Expression>> {
        let start = self.mark_span();
        self.eat_keyword(Keyword::Throw)?;

        // value
        let cursor = self.peek_cursor();
        if cursor.has_line_break_before
            || matches!(
                cursor.token_type,
                TokenType::Semicolon | TokenType::End | TokenType::CloseBrace
            )
        {
            return Err(ParseError::unexpected_for(
                self.get_span_from(&start),
                NodeType::Expression,
            ));
        }
        let value_id = self.eat_expression_not_in_position()?;

        // throw
        let throw_id = self.tree.insert(
            Expression::Throw { value: value_id },
            self.get_span_from(&start),
        );
        Ok(throw_id)
    }

    /// Eat a return expression.
    ///
    /// Examples:
    /// ```
    /// return
    /// return 17
    /// ```
    pub fn eat_return(&mut self) -> ParseResult<LocalNodeId<Expression>> {
        let start = self.mark_span();
        self.eat_keyword(Keyword::Return)?;

        // value
        let cursor = self.peek_cursor();
        let value_id = if !cursor.has_line_break_before
            && !matches!(
                cursor.token_type,
                TokenType::Semicolon | TokenType::End | TokenType::CloseBrace
            ) {
            let value_id = self
                .eat_expression_not_in_position()
                .for_node_type(NodeType::Expression)?;
            Some(value_id)
        } else {
            None
        };
        // return
        let return_id = self.tree.insert(
            Expression::Return { value: value_id },
            self.get_span_from(&start),
        );
        Ok(return_id)
    }
}

#[cfg(test)]
mod tests {
    use destack_ast::{
        Expression, IfKind, LetKind, ScalarLiteral, TokenType, TypeBinaryOperator, YieldCardinality,
    };
    use destack_source::LanguageType;

    use crate::{TestParser, assert_expression_path, assert_node, assert_path, assert_string};

    #[test]
    fn test_parse_empty_block() {
        let mut test = TestParser::new("{}");
        let mut parser = test.prepare();
        let block_id = parser.eat_block().unwrap();
        let block = parser.tree.get(block_id);
        assert!(block.expressions.is_empty());
    }

    #[test]
    fn test_parse_root_unmatched_close_brace_recovery() {
        let mut test = TestParser::new("}\nnextValue");
        let mut parser = test.prepare();
        let expressions = parser.parse_without_finish();

        assert_eq!(expressions.len(), 2);
        assert_node!(parser.tree, expressions[0], Expression::Error);
        assert_node!(parser.tree, expressions[1], Expression::Statement(expression_id) => {
            assert_expression_path!(parser, parser.tree.get(*expression_id), "nextValue");
        });
    }

    #[test]
    fn test_parse_root_unmatched_close_parenthesis_recovery() {
        let mut test = TestParser::new(")\nnextValue");
        let mut parser = test.prepare();
        let expressions = parser.parse_without_finish();

        assert_eq!(expressions.len(), 2);
        assert_node!(parser.tree, expressions[0], Expression::Error);
        assert_node!(parser.tree, expressions[1], Expression::Statement(expression_id) => {
            assert_expression_path!(parser, parser.tree.get(*expression_id), "nextValue");
        });
    }

    #[test]
    fn test_statement_expression_separator_with_comment_newline() {
        let mut test = TestParser::new_with_options(
            "'use strict' /**/ \n nextValue",
            LanguageType::TypeScript,
        );
        let mut parser = test.prepare();
        let (directive_id, is_statement) = parser.try_eat_statement_expression_with_flag().unwrap();
        assert!(!is_statement);
        assert_node!(parser.tree, directive_id, Expression::ScalarLiteral(ScalarLiteral::String(string_id)) => {
            assert_string!(parser, *string_id, "use strict");
        });

        let next_id = parser.try_eat_statement_expression().unwrap();
        assert_expression_path!(parser, parser.tree.get(next_id), "nextValue");
    }

    #[test]
    fn test_break_no_label_no_value() {
        let mut test = TestParser::new("break");
        let mut parser = test.prepare();
        let break_id = parser.eat_break().unwrap();
        assert_node!(parser.tree, break_id, Expression::Break { label: None, value } => {
            assert!(value.is_none());
        });
    }

    #[test]
    fn test_break_with_label() {
        let mut test = TestParser::new("break :label");
        let mut parser = test.prepare();
        let break_id = parser.eat_break().unwrap();
        assert_node!(parser.tree, break_id, Expression::Break { label, value } => {
            assert_string!(parser, label.unwrap(), "label");
            assert!(value.is_none());
        });

        let main_span = parser
            .tree
            .get_main_span(break_id)
            .expect("expected break label span");
        assert_eq!(parser.get_span_str(main_span), "label");
    }

    #[test]
    fn test_break_with_label_and_value() {
        let mut test = TestParser::new("break :label 17");
        let mut parser = test.prepare();
        let break_id = parser.eat_break().unwrap();
        assert_node!(parser.tree, break_id, Expression::Break { label, value } => {
            assert_string!(parser, label.unwrap(), "label");
            assert!(value.is_some());
            assert_node!(parser.tree, value.unwrap(), Expression::ScalarLiteral(ScalarLiteral::Integer(17)));
        });
    }

    #[test]
    fn test_break_with_value() {
        let mut test = TestParser::new("break 15");
        let mut parser = test.prepare();
        let break_id = parser.eat_break().unwrap();
        assert_node!(parser.tree, break_id, Expression::Break { label: None, value } => {
            assert!(value.is_some());
            assert_node!(parser.tree, value.unwrap(), Expression::ScalarLiteral(ScalarLiteral::Integer(15)));
        });
    }

    #[test]
    fn test_continue_no_label() {
        let mut test = TestParser::new("continue");
        let mut parser = test.prepare();
        let continue_id = parser.eat_continue().unwrap();
        assert_node!(parser.tree, continue_id, Expression::Continue { label: None } => {
        });
    }

    #[test]
    fn test_continue_with_label() {
        let mut test = TestParser::new("continue :label");
        let mut parser = test.prepare();
        let continue_id = parser.eat_continue().unwrap();
        assert_node!(parser.tree, continue_id, Expression::Continue { label } => {
            assert_string!(parser, label.unwrap(), "label");
        });

        let main_span = parser
            .tree
            .get_main_span(continue_id)
            .expect("expected continue label span");
        assert_eq!(parser.get_span_str(main_span), "label");
    }

    #[test]
    fn test_break_js_style_label() {
        let mut test = TestParser::new("break foo;");
        let mut parser = test.prepare();
        let break_id = parser.eat_break().unwrap();
        assert_node!(parser.tree, break_id, Expression::Break { label, value } => {
            assert_string!(parser, label.unwrap(), "foo");
            assert!(value.is_none());
        });
    }

    #[test]
    fn test_break_js_style_label_newline() {
        let mut test = TestParser::new("break foo\n");
        let mut parser = test.prepare();
        let break_id = parser.eat_break().unwrap();
        assert_node!(parser.tree, break_id, Expression::Break { label, value } => {
            assert_string!(parser, label.unwrap(), "foo");
            assert!(value.is_none());
        });
    }

    #[test]
    fn test_continue_js_style_label() {
        let mut test = TestParser::new("continue foo;");
        let mut parser = test.prepare();
        let continue_id = parser.eat_continue().unwrap();
        assert_node!(parser.tree, continue_id, Expression::Continue { label } => {
            assert_string!(parser, label.unwrap(), "foo");
        });
    }

    #[test]
    fn test_continue_js_style_label_newline() {
        let mut test = TestParser::new("continue foo\n");
        let mut parser = test.prepare();
        let continue_id = parser.eat_continue().unwrap();
        assert_node!(parser.tree, continue_id, Expression::Continue { label } => {
            assert_string!(parser, label.unwrap(), "foo");
        });
    }

    #[test]
    fn test_await_expression() {
        let mut test = TestParser::new("await someFunction()");
        let mut parser = test.prepare();
        let await_id = parser.eat_await().unwrap();
        // await someFunction()
        assert_node!(parser.tree, await_id, Expression::Await { expression } => {
            // someFunction()
            assert_node!(parser.tree, *expression, Expression::Call { position: _, left, static_arguments: None, dynamic_arguments } => {
                assert_expression_path!(parser, parser.tree.get(*left), "someFunction");
                assert!(dynamic_arguments.is_empty());
            });
        });
    }

    #[test]
    fn test_await_maybe_expression() {
        let mut test = TestParser::new("await? someFunction()");
        let mut parser = test.prepare();
        let await_id = parser.eat_await().unwrap();
        // await? someFunction()
        assert_node!(parser.tree, await_id, Expression::AwaitMaybe { expression } => {
            // someFunction()
            assert_node!(parser.tree, *expression, Expression::Call { position: _, left, static_arguments: None, dynamic_arguments } => {
                assert_expression_path!(parser, parser.tree.get(*left), "someFunction");
                assert!(dynamic_arguments.is_empty());
            });
        });
    }

    #[test]
    fn test_comptime_expression() {
        let mut test = TestParser::new("comptime factorial(10)");
        let mut parser = test.prepare();
        let comptime_id = parser.eat_comptime().unwrap();
        // comptime factorial(10)
        assert_node!(parser.tree, comptime_id, Expression::Comptime { body } => {
            // factorial(10)
            assert_node!(parser.tree, *body, Expression::Call { position: _, left, static_arguments: None, dynamic_arguments } => {
                assert_expression_path!(parser, parser.tree.get(*left), "factorial");
                assert_eq!(dynamic_arguments.len(), 1);
            });
        });
    }

    #[test]
    fn test_comptime_expression_simple() {
        // comptime 1 + 2
        let mut test = TestParser::new("comptime 1 + 2");
        let mut parser = test.prepare();
        let comptime_id = parser.eat_comptime().unwrap();
        // comptime 1 + 2
        assert_node!(parser.tree, comptime_id, Expression::Comptime { body } => {
            // 1 + 2
            assert_node!(parser.tree, *body, Expression::Binary { .. } => {
                // binary addition
            });
        });
    }

    #[test]
    fn test_comptime_block_expression() {
        let mut test = TestParser::new("comptime { let x = 1; x + 2 }");
        let mut parser = test.prepare();
        let comptime_id = parser.eat_comptime().unwrap();
        assert_node!(parser.tree, comptime_id, Expression::Comptime { body } => {
            assert_node!(parser.tree, *body, Expression::Block(_));
        });
    }

    #[test]
    fn test_yield_expression() {
        let mut test = TestParser::new("yield someFunction()");
        let mut parser = test.prepare();
        let yield_id = parser.eat_yield().unwrap();
        // yield someFunction()
        assert_node!(parser.tree, yield_id, Expression::Yield { cardinality, value } => {
            assert_eq!(*cardinality, YieldCardinality::Scalar);
            // someFunction()
            assert_node!(parser.tree, value.unwrap(), Expression::Call { position: _, left, static_arguments: None, dynamic_arguments } => {
                assert_expression_path!(parser, parser.tree.get(*left), "someFunction");
                assert!(dynamic_arguments.is_empty());
            });
        });
    }

    #[test]
    fn test_yield_expression_no_value() {
        let mut test = TestParser::new("yield");
        let mut parser = test.prepare();
        let yield_id = parser.eat_yield().unwrap();
        assert_node!(parser.tree, yield_id, Expression::Yield { cardinality, value } => {
            assert_eq!(*cardinality, YieldCardinality::Scalar);
            assert!(value.is_none());
        });
    }

    #[test]
    fn test_yield_expression_no_value_before_close_parenthesis() {
        // source: yield)
        let mut test = TestParser::new("yield)");
        let mut parser = test.prepare();
        let yield_id = parser.eat_yield().unwrap();
        assert_node!(parser.tree, yield_id, Expression::Yield { cardinality, value } => {
            assert_eq!(*cardinality, YieldCardinality::Scalar);
            assert!(value.is_none());
        });
        assert!(parser.peek_is(TokenType::CloseParenthesis));
    }

    #[test]
    fn test_yield_expression_no_value_before_close_bracket() {
        // source: yield]
        let mut test = TestParser::new("yield]");
        let mut parser = test.prepare();
        let yield_id = parser.eat_yield().unwrap();
        assert_node!(parser.tree, yield_id, Expression::Yield { cardinality, value } => {
            assert_eq!(*cardinality, YieldCardinality::Scalar);
            assert!(value.is_none());
        });
        assert!(parser.peek_is(TokenType::CloseBracket));
    }

    #[test]
    fn test_yield_expression_generator() {
        let mut test = TestParser::new("yield* someFunction()");
        let mut parser = test.prepare();
        let yield_id = parser.eat_yield().unwrap();
        assert_node!(parser.tree, yield_id, Expression::Yield { cardinality, value } => {
            assert_eq!(*cardinality, YieldCardinality::Generator);
            // someFunction()
            assert_node!(parser.tree, value.unwrap(), Expression::Call { position: _, left, static_arguments: None, dynamic_arguments } => {
                assert_expression_path!(parser, parser.tree.get(*left), "someFunction");
                assert!(dynamic_arguments.is_empty());
            });
        });
    }

    #[test]
    fn test_yield_expression_generator_with_space() {
        let mut test = TestParser::new("yield *a");
        let mut parser = test.prepare();
        let yield_id = parser.eat_yield().unwrap();
        assert_node!(parser.tree, yield_id, Expression::Yield { cardinality, value } => {
            assert_eq!(*cardinality, YieldCardinality::Generator);
            assert!(value.is_some());
        });
    }

    #[test]
    fn test_reject_yield_star_without_operand() {
        // source: yield*
        let mut test = TestParser::new("yield*");
        let mut parser = test.prepare();

        let error = parser.eat_yield().unwrap_err();

        // yield*
        assert_eq!(parser.get_span_str(error.leaf_span()), "yield*");
    }

    /// `yield\n*a` should NOT be parsed as `yield* a` due to ASI restricted production.
    #[test]
    fn test_yield_asi_with_newline() {
        let mut test = TestParser::new("yield\n*a");
        let mut parser = test.prepare();
        let yield_id = parser.eat_yield().unwrap();
        assert_node!(parser.tree, yield_id, Expression::Yield { cardinality, value } => {
            assert_eq!(*cardinality, YieldCardinality::Scalar);
            assert!(value.is_none()); // ASI applied, no value
        });
    }

    /// `yield\n*a` should NOT be parsed as `yield* a` due to ASI restricted production.
    #[test]
    fn test_yield_asi_with_newline_js_mode() {
        let options = LanguageType::JavaScript;
        let mut test = TestParser::new_with_options("yield\n*a", options);
        let mut parser = test.prepare();

        // yield parses fine with ASI
        let yield_id = parser.eat_yield().unwrap();
        assert_node!(parser.tree, yield_id, Expression::Yield { cardinality, value } => {
            assert_eq!(*cardinality, YieldCardinality::Scalar);
            assert!(value.is_none()); // ASI applied, no value
        });

        // try to parse *a as next statement - should fail in JS mode
        // (because * is not valid as unary prefix in JS)
        let error = parser.eat_expression(parser.options).unwrap_err();

        // \n
        assert_eq!(parser.get_span_str(error.leaf_span()), "\n");

        // consume newline and reject following *
        parser.eat_newline().unwrap();
        let error = parser.eat_expression(parser.options).unwrap_err();

        // *
        assert_eq!(parser.get_span_str(error.leaf_span()), "*");
    }

    /// `yield/*\n*/*a` should not be parsed as `yield* a`.
    #[test]
    fn test_yield_asi_with_block_comment_newline_js_mode() {
        // source: yield/*\n*/*a
        let mut test = TestParser::new_with_options("yield/*\n*/*a", LanguageType::JavaScript);
        let mut parser = test.prepare();

        let yield_id = parser.eat_yield().unwrap();
        assert_node!(parser.tree, yield_id, Expression::Yield { cardinality, value } => {
            assert_eq!(*cardinality, YieldCardinality::Scalar);
            assert!(value.is_none());
        });

        let error = parser.eat_expression(parser.options).unwrap_err();

        // *
        assert_eq!(parser.get_span_str(error.leaf_span()), "*");
    }

    #[test]
    fn test_throw_expression_with_value() {
        let mut test = TestParser::new("throw 17");
        let mut parser = test.prepare();
        let throw_id = parser.eat_throw().unwrap();
        assert_node!(parser.tree, throw_id, Expression::Throw { value } => {
            assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(17)));
        });
    }

    /// Reject throw expressions split by a block comment newline.
    #[test]
    fn test_reject_throw_expression_with_block_comment_newline() {
        // source: throw /*\n*/ e
        let mut test = TestParser::new_with_options("throw /*\n*/ e", LanguageType::JavaScript);
        let mut parser = test.prepare();
        let error = parser.eat_throw().unwrap_err();

        // throw ... (restricted production failure span starts at throw)
        assert_eq!(error.leaf_span().start, 0);

        // e
        assert!(parser.peek_is(TokenType::Identifier));
    }

    /// Reject throw expressions split by unicode line separator comments.
    #[test]
    fn test_reject_throw_expression_with_line_separator_comment() {
        // source: throw /* \u{2028} */ e
        let mut test =
            TestParser::new_with_options("throw /* \u{2028} */ e", LanguageType::JavaScript);
        let mut parser = test.prepare();
        let error = parser.eat_throw().unwrap_err();

        // throw ... (restricted production failure span starts at throw)
        assert_eq!(error.leaf_span().start, 0);

        // e
        assert!(parser.peek_is(TokenType::Identifier));
    }

    #[test]
    fn test_return_no_value() {
        let mut test = TestParser::new("return");
        let mut parser = test.prepare();
        let return_id = parser.eat_return().unwrap();
        assert_node!(parser.tree, return_id, Expression::Return { value } => {
            assert!(value.is_none());
        });
    }

    #[test]
    fn test_return_with_value() {
        let mut test = TestParser::new("return 42");
        let mut parser = test.prepare();
        let return_id = parser.eat_return().unwrap();
        assert_node!(parser.tree, return_id, Expression::Return { value } => {
            assert!(value.is_some());
            assert_node!(parser.tree, value.unwrap(), Expression::ScalarLiteral(ScalarLiteral::Integer(42)));
        });
    }

    #[test]
    fn test_parse_block_const_then_return_cast_typescript() {
        let mut test = TestParser::new_with_options(
            "{\n  const result = CreateRecord(IntegerKey, value)\n  return result as never\n}",
            LanguageType::TypeScript,
        );
        let mut parser = test.prepare();
        let block_id = parser.eat_block().unwrap();
        let block = parser.tree.get(block_id);
        assert_eq!(block.expressions.len(), 2);

        let let_expression_id = match parser.tree.get(block.expressions[0]) {
            Expression::Statement(expression_id) => *expression_id,
            _ => block.expressions[0],
        };
        assert_node!(parser.tree, let_expression_id, Expression::Let { kind, declarators, .. } => {
            assert_eq!(*kind, LetKind::Const);
            assert_eq!(declarators.len(), 1);
        });

        assert_node!(parser.tree, block.expressions[1], Expression::Return { value } => {
            let value = value.expect("expected return value");
            assert_node!(parser.tree, value, Expression::TypeBinary { operator, .. } => {
                assert_eq!(*operator, TypeBinaryOperator::Cast);
            });
        });
    }

    #[test]
    fn test_parse_return_ternary_with_newline_before_question() {
        let mut test = TestParser::new_with_options(
            "return Result.IsExtendsTrueLike(check)\n  ? TryInferResults(tail, right, [...result, head])\n  : undefined",
            LanguageType::TypeScript,
        );
        let mut parser = test.prepare();
        let return_id = parser.eat_return().unwrap();

        assert_node!(parser.tree, return_id, Expression::Return { value } => {
            let value = value.expect("expected return value");
            assert_node!(parser.tree, value, Expression::If { kind, .. } => {
                assert_eq!(*kind, IfKind::Ternary);
            });
        });
    }

    #[test]
    fn test_return_no_value_before_close_brace() {
        let mut test = TestParser::new("return }");
        let mut parser = test.prepare();
        let return_id = parser.eat_return().unwrap();

        assert_node!(parser.tree, return_id, Expression::Return { value } => {
            assert!(value.is_none());
        });
        assert!(parser.peek_is(TokenType::CloseBrace));
    }

    /// `return/*\n*/value` should omit the operand due to line terminator trivia.
    #[test]
    fn test_return_asi_with_block_comment_newline_js_mode() {
        let mut test = TestParser::new_with_options("return/*\n*/value", LanguageType::JavaScript);
        let mut parser = test.prepare();
        let return_id = parser.eat_return().unwrap();

        assert_node!(parser.tree, return_id, Expression::Return { value } => {
            assert!(value.is_none());
        });
        let value_id = parser.eat_expression(parser.options).unwrap();
        assert_expression_path!(parser, parser.tree.get(value_id), "value");
    }
}
