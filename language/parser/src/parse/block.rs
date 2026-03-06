use destack_ast::{
    Block, BlockContext, BlockFormat, Declaration, Expression, FunctionKind, Keyword, LetKind,
    LocalNodeId, NodeType, TokenType, YieldCardinality,
};

use crate::parse::parser::ParserOptions;
use crate::parse::prelude::*;
use crate::{ParseError, ParseResult, Parser, ParserMark};

/// The recursion interval for stack growth checks in statement parsing.
const STATEMENT_STACK_GROW_CHECK_INTERVAL: u32 = if cfg!(debug_assertions) { 1 } else { 256 };

impl Parser {
    /// Return parser contexts for statement-position parsing.
    #[inline]
    pub(crate) fn statement_position_contexts(&self) -> (ParserOptions, ParserOptions) {
        let ambient_context = self.options.nested().with_statement_context(true);
        let expression_context = self.options.nested().with_statement_position(true);
        (ambient_context, expression_context)
    }

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

    /// Eat an expression in a non-position context.
    #[inline]
    fn eat_expression_not_in_position(&mut self) -> ParseResult<LocalNodeId<Expression>> {
        self.eat_expression_with_context_unchecked(self.options.not_in_position())
    }

    /// Return true when a token ends the current block body.
    #[inline]
    fn is_block_body_terminator_token(&self, token_type: TokenType, format: BlockFormat) -> bool {
        token_type == TokenType::End
            || (token_type == TokenType::CloseBrace && format != BlockFormat::Implicit)
    }

    /// Push a non-tail expression into a block body as a statement wrapper.
    #[inline]
    fn push_block_body_non_tail_expression(
        &mut self,
        statements: &mut Vec<LocalNodeId<Expression>>,
        expression_id: LocalNodeId<Expression>,
    ) {
        // non-tail expressions in block bodies are always statement items
        let statement_id =
            self.wrap_statement_expression(expression_id, self.tree.get_span(expression_id));
        statements.push(statement_id);
    }

    /// Try to parse a labelled statement before generic statement keyword dispatch.
    fn try_parse_labelled_statement_expression(
        &mut self,
        start: &ParserMark,
    ) -> ParseResult<Option<LocalNodeId<Expression>>> {
        // labels only start on `identifier:`
        if !self.peek_next_is(TokenType::Colon) {
            return Ok(None);
        }

        // inspect the label target to determine whether label parsing is allowed here
        let colon_index = self.index_for_next();
        let label_target_index = self.next_non_newline_index_from(colon_index + 1);
        let label_target_token = self.token_at(label_target_index);
        let label_target_keyword = label_target_token
            .filter(|token| token.token.ty == TokenType::Identifier)
            .and_then(|_| self.keyword_for_index(label_target_index));

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
        let is_in_statement_position = self.options.is_in_statement_position();
        let can_parse_label = if is_in_statement_position && !self.language.is_destack() {
            true
        } else {
            is_labelled_expression || (is_in_statement_position && is_labelled_block)
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
                    context: BlockContext::Statement,
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

    /// Try to parse a statement expression that starts with an identifier.
    #[inline]
    fn try_dispatch_identifier_statement_expression(
        &mut self,
        start: &ParserMark,
    ) -> ParseResult<Option<LocalNodeId<Expression>>> {
        if let Some(speculation_stats) = self.speculation_stats.as_mut() {
            speculation_stats.statement_keyword_dispatch_calls += 1;
        }

        // parse labelled statements before keyword and expression dispatch
        if let Some(expression_id) = self.try_parse_labelled_statement_expression(start)? {
            return Ok(Some(expression_id));
        }

        // direct keyword dispatch in statement position
        if let Some(keyword) = self.keyword_for_index(self.pos_index()) {
            let next_raw_index = self.index_for_next();
            let next_raw_token_type = self.token_type_at(next_raw_index);
            let next_cursor = self.scanner_cursor_from(next_raw_index);
            if let Some(expression_id) = self.try_eat_direct_statement_keyword_expression(
                start,
                keyword,
                next_raw_token_type,
                next_cursor,
            )? {
                if let Some(speculation_stats) = self.speculation_stats.as_mut() {
                    speculation_stats.statement_keyword_dispatch_direct_hits += 1;
                }

                let expression = self.tree.get(expression_id);
                let is_terminal_statement = matches!(expression, Expression::Statement(_))
                    || expression.is_top_level_statement();
                if is_terminal_statement {
                    return Ok(Some(expression_id));
                }

                let continuation_id = self.eat_expression_continuation(start, expression_id)?;
                return Ok(Some(continuation_id));
            }

            if let Some(speculation_stats) = self.speculation_stats.as_mut() {
                speculation_stats.statement_keyword_dispatch_direct_misses += 1;
            }

            return Ok(None);
        }

        if let Some(speculation_stats) = self.speculation_stats.as_mut() {
            speculation_stats.statement_keyword_dispatch_keyword_rejects += 1;
        }

        // parse plain identifier paths without re-running generic identifier entry checks
        let pos_index = self.pos_index();
        let next_raw_index = self.index_for_next();
        let next_raw_token_type = self.token_type_at(next_raw_index);
        if let Some(expression_id) = self.try_parse_plain_identifier_expression_from_identifier(
            start,
            pos_index,
            next_raw_index,
            next_raw_token_type,
        )? {
            return Ok(Some(expression_id));
        }

        Ok(None)
    }

    /// Try to dispatch a statement expression.
    #[inline]
    fn try_dispatch_statement_expression(
        &mut self,
        start: &ParserMark,
        token_type: TokenType,
    ) -> ParseResult<Option<LocalNodeId<Expression>>> {
        // block statements stay in the statement dispatch
        if token_type == TokenType::OpenBrace {
            let block_id = self.eat_block(BlockContext::Expression)?;
            let expression_id = self
                .tree
                .insert(Expression::Block(block_id), self.get_span_from(start));
            return Ok(Some(expression_id));
        }

        // identifier starts use a dedicated statement dispatch
        if token_type == TokenType::Identifier {
            return self.try_dispatch_identifier_statement_expression(start);
        }

        if let Some(speculation_stats) = self.speculation_stats.as_mut() {
            speculation_stats.statement_keyword_dispatch_prefilter_rejects += 1;
        }

        Ok(None)
    }

    /// Eat one statement expression in the current parser options.
    #[inline]
    pub(crate) fn eat_statement_expression(&mut self) -> ParseResult<LocalNodeId<Expression>> {
        // normalize to the next non-newline token once per dispatch
        self.eat_newlines_maybe()?;
        let token_type = self.peek_token_type();
        self.eat_statement_expression_from_token_kind(token_type)
    }

    /// Eat one statement expression when the parser cursor is already normalized.
    /// Applies stack growth checks for recursive statement parsing.
    #[inline]
    fn eat_statement_expression_from_token_kind(
        &mut self,
        token_type: TokenType,
    ) -> ParseResult<LocalNodeId<Expression>> {
        let depth = self.statement_stack_depth;
        self.statement_stack_depth = depth + 1;
        let should_check_stack =
            depth != 0 && (depth & (STATEMENT_STACK_GROW_CHECK_INTERVAL - 1)) == 0;
        let result = if should_check_stack {
            destack_base::ensure_sufficient_stack(|| {
                self.eat_statement_expression_from_token_kind_inner(token_type)
            })
        } else {
            self.eat_statement_expression_from_token_kind_inner(token_type)
        };
        self.statement_stack_depth = depth;
        result
    }

    /// Eat one statement expression when the parser cursor is already normalized.
    #[inline]
    fn eat_statement_expression_from_token_kind_inner(
        &mut self,
        token_type: TokenType,
    ) -> ParseResult<LocalNodeId<Expression>> {
        // statement dispatch
        let start = self.mark_span();
        if let Some(expression_id) = self.try_dispatch_statement_expression(&start, token_type)? {
            return Ok(expression_id);
        }

        // parenthesized lambda heads keep statement mode
        if token_type == TokenType::OpenParenthesis && self.options.is_in_statement_position() {
            let open_index = self.pos_index();
            if let Some(close_index) = self.matching_pair_or_lex(open_index) {
                let follow_index = self.next_non_newline_index_from(close_index + 1);
                let follow_token_type = self.token_type_at(follow_index);
                if matches!(
                    follow_token_type,
                    TokenType::Arrow | TokenType::ArrowWide | TokenType::Colon
                ) {
                    return self.eat_expression_in_scope();
                }
            }
        }

        // non identifier starts parse outside statement mode
        if token_type != TokenType::Identifier {
            return self.eat_expression_outside_statement_position();
        }

        // plain identifiers parse outside statement mode
        if self.keyword_for_index(self.pos_index()).is_none() {
            return self.eat_expression_outside_statement_position();
        }

        // keyword fallbacks keep statement mode
        self.eat_expression_after_statement_keyword_dispatch()
    }

    /// Eat a block or a single statement wrapped in a block.
    pub fn eat_block_or_statement(&mut self) -> ParseResult<LocalNodeId<Block>> {
        self.eat_newlines_maybe()?;

        // if it's a block, just eat it
        if self.is_block_start() {
            return self.eat_block(BlockContext::Statement);
        }

        let start = self.mark_span();

        // empty statement (just semicolon, e.g., `for (x of y);`)
        if self.peek_is(TokenType::Semicolon) {
            self.bump();
            let block_id = self.tree.insert(
                Block {
                    context: BlockContext::Statement,
                    format: BlockFormat::Implicit,
                    expressions: vec![],
                },
                self.get_span_from(&start),
            );
            return Ok(block_id);
        }

        // otherwise, eat a single statement and wrap it in a block
        let (ambient_context, expression_context) = self.statement_position_contexts();
        let expression_id = self.with_options(
            self.options
                .with_ambient_context(ambient_context)
                .with_expression_context(expression_context),
            |parser| parser.eat_statement_expression(),
        )?;

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
                context: BlockContext::Statement,
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
            Expression::Declaration(declaration_id) => !matches!(
                self.tree.get(*declaration_id),
                Declaration::Function { signature, .. }
                    if signature.kind == FunctionKind::Lambda
            ),
            Expression::Using { .. }
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
    pub fn eat_block(&mut self, block_context: BlockContext) -> ParseResult<LocalNodeId<Block>> {
        let start = self.mark_span();

        // `do` prefix
        if self.language.is_destack() && self.is_keyword(Keyword::Do) {
            self.bump(); // eat keyword
        }

        // body
        self.try_eat_token(TokenType::OpenBrace, TokenType::CloseBrace)
            .for_node_type(NodeType::Block)?;
        let expressions = self
            .eat_block_body_with_context(BlockFormat::Explicit, block_context)
            .for_node_type(NodeType::Block)?;
        self.eat_token(TokenType::CloseBrace)?;

        // block
        let block_id = self.tree.insert(
            Block {
                context: block_context,
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
        let block_context = if format == BlockFormat::Explicit {
            BlockContext::Expression
        } else {
            BlockContext::Statement
        };
        self.eat_block_body_with_context(format, block_context)
    }

    /// Eat a block body with an explicit block context.
    pub fn eat_block_body_with_context(
        &mut self,
        format: BlockFormat,
        block_context: BlockContext,
    ) -> ParseResult<Vec<LocalNodeId<Expression>>> {
        let _timing = self.timing_scope(tags::PARSE_BLOCK_BODY);

        // keep statement options for the whole body to avoid per statement option churn
        let (ambient_context, expression_context) = self.statement_position_contexts();
        if self.options == ambient_context && self.options == expression_context {
            return self.eat_block_body_in_statement_position(format, block_context);
        }

        if let Some(speculation_stats) = self.speculation_stats.as_mut() {
            speculation_stats.with_options_calls += 1;
        }
        self.with_options(
            self.options
                .with_ambient_context(ambient_context)
                .with_expression_context(expression_context),
            |parser| parser.eat_block_body_in_statement_position(format, block_context),
        )
    }

    /// Eat a block body while already in statement position.
    fn eat_block_body_in_statement_position(
        &mut self,
        format: BlockFormat,
        block_context: BlockContext,
    ) -> ParseResult<Vec<LocalNodeId<Expression>>> {
        // parse all statement items and keep at most one tail expression
        let mut statements: Vec<LocalNodeId<Expression>> = Vec::new();
        let mut pending_tail_expression: Option<LocalNodeId<Expression>> = None;

        loop {
            // normalize block body cursor once per iteration
            self.eat_newlines_maybe()?;
            let token_type = self.peek_token_type();

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

            // previous tail expressions are no longer block tails once a new item starts
            if let Some(pending_id) = pending_tail_expression.take() {
                self.push_block_body_non_tail_expression(&mut statements, pending_id);
            }

            // parse and recover one statement item
            let start = self.mark_span();
            let (expression_id, is_statement) =
                match self.eat_statement_expression_from_token_kind(token_type) {
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

            // keep at most one tail candidate, emit statements directly
            if is_statement {
                statements.push(expression_id);
            } else {
                pending_tail_expression = Some(expression_id);
            }
        }

        // finalize the remaining tail expression
        if let Some(expression_id) = pending_tail_expression {
            // explicit blocks in destack preserve expression tails for implicit returns
            if format == BlockFormat::Explicit
                && self.language.is_destack()
                && block_context == BlockContext::Expression
            {
                statements.push(expression_id);
            } else {
                self.push_block_body_non_tail_expression(&mut statements, expression_id);
            }
        }

        Ok(statements)
    }

    /// Try to eat a statement expression (return Expression::Error if error and recovery is possible).
    /// Returns whether the expression should be treated as a statement.
    pub fn try_eat_statement_expression_with_flag(
        &mut self,
    ) -> ParseResult<(LocalNodeId<Expression>, bool)> {
        let (ambient_context, expression_context) = self.statement_position_contexts();
        self.with_options(
            self.options
                .with_ambient_context(ambient_context)
                .with_expression_context(expression_context),
            |parser| parser.try_eat_statement_expression_with_flag_in_statement_position(),
        )
    }

    /// Try to eat a statement expression while already in statement position.
    /// Returns whether the expression should be treated as a statement.
    #[inline]
    fn try_eat_statement_expression_with_flag_in_statement_position(
        &mut self,
    ) -> ParseResult<(LocalNodeId<Expression>, bool)> {
        let start = self.mark_span();

        match self.eat_statement_expression() {
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

        // detect expression kinds that should stay statement-shaped
        let expression = self.tree.get(expression_id);
        let is_statement =
            matches!(expression, Expression::Statement(_)) || expression.is_top_level_statement();
        let separator_cursor = self.scanner_cursor_from(self.pos_index());
        let has_separator = separator_cursor.starts_after_statement_boundary();

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

    /// Return true when the token after the current identifier ends a JS style label.
    #[inline]
    fn next_token_ends_label_statement(&mut self) -> bool {
        let next_index = self.index_for_next();
        let next_cursor = self.scanner_cursor_from(next_index);
        next_cursor.index != next_index || next_cursor.starts_after_statement_boundary()
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
            let value_id = if self.language.is_destack()
                && self.has_more_tokens()
                && !self.is_statement_stop()
            {
                let value_id = self.eat_expression_not_in_position()?;
                Some(value_id)
            } else {
                None
            };
            (Some(label), Some(label_span), value_id)
        }
        // JS style: break label (identifier followed by statement stop)
        else if self.peek_is(TokenType::Identifier) && self.next_token_ends_label_statement() {
            let (label, label_span) = self.eat_identifier_with_span()?;
            (Some(label), Some(label_span), None)
        }
        // Destack extension: break value (no label)
        else if self.language.is_destack() && self.has_more_tokens() && !self.is_statement_stop()
        {
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
        } else if self.peek_is(TokenType::Identifier) && self.next_token_ends_label_statement() {
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

        // allow multiline await operands
        self.eat_newlines_maybe()?;

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
        let ambient_context = self.options.with_comptime(true);
        let expression_context = self.options.not_in_position();
        let body_id = self.with_options(
            self.options
                .with_ambient_context(ambient_context)
                .with_expression_context(expression_context),
            |parser| parser.eat_statement_expression(),
        )?;

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
        let mut operand_is_omitted = self.yield_operand_is_omitted();
        if operand_is_omitted {
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
            operand_is_omitted = self.yield_operand_is_omitted();
            YieldCardinality::Generator
        } else {
            YieldCardinality::Scalar
        };

        // value (optional, like return/throw)
        // yield without value is valid: `function* a() { yield }`
        let value_id = if self.has_more_tokens() && !operand_is_omitted {
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
        let cursor = self.scanner_cursor_from(self.pos_index());
        cursor.omits_restricted_operand()
    }

    /// Return true when trivia before the current token contains a line terminator.
    pub(crate) fn has_line_terminator_before_current_token(&mut self) -> bool {
        self.scanner_cursor_from(self.pos_index())
            .has_line_break_before
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
        let cursor = self.scanner_cursor_from(self.pos_index());
        if cursor.starts_after_statement_boundary() {
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
        let cursor = self.scanner_cursor_from(self.pos_index());
        let value_id = if !cursor.starts_after_statement_boundary() {
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
        BlockContext, CommentStyle, Declaration, Expression, IfKind, LetKind, ScalarLiteral,
        TokenType, TypeBinaryOperator, YieldCardinality,
    };
    use destack_source::LanguageType;

    use crate::{TestParser, assert_expression_path, assert_node, assert_path, assert_string};

    #[test]
    fn test_parse_empty_block() {
        let mut test = TestParser::new("{}");
        let mut parser = test.prepare();
        let block_id = parser.eat_block(BlockContext::Expression).unwrap();
        let block = parser.tree.get(block_id);
        assert!(block.expressions.is_empty());
    }

    #[test]
    fn test_parse_root_unmatched_close_brace_recovery() {
        let mut test = TestParser::new("}\nnextValue");
        let mut parser = test.prepare();
        let expressions = parser.parse();

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
        let expressions = parser.parse();

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
        let block_id = parser.eat_block(BlockContext::Expression).unwrap();
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

        let return_expression_id = match parser.tree.get(block.expressions[1]) {
            Expression::Statement(expression_id) => *expression_id,
            _ => block.expressions[1],
        };
        assert_node!(parser.tree, return_expression_id, Expression::Return { value } => {
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
    fn test_parse_block_statement_before_close_brace_without_semicolon_javascript() {
        let mut test = TestParser::new_with_options("{ process.exit(1)}", LanguageType::JavaScript);
        let mut parser = test.prepare();
        let block_id = parser.eat_block(BlockContext::Expression).unwrap();
        let block = parser.tree.get(block_id);

        // block should contain one statement expression
        assert_eq!(block.expressions.len(), 1);
        assert_node!(parser.tree, block.expressions[0], Expression::Statement(statement_id) => {
            assert_node!(parser.tree, *statement_id, Expression::Call { .. });
        });
    }

    /// Parse Destack if-body block tails as value expressions.
    #[test]
    fn test_parse_destack_if_block_keeps_tail_expression_value() {
        let mut test = TestParser::new("if (x) { foo() }");
        let mut parser = test.prepare();
        let expressions = parser.parse();

        // if (x) { foo() }
        assert_eq!(expressions.len(), 1);
        let if_expression_id = parser.unwrap_statement_expression(expressions[0]);
        assert_node!(parser.tree, if_expression_id, Expression::If { then_expression, .. } => {
            assert_node!(parser.tree, *then_expression, Expression::Block(block_id) => {
                let block = parser.tree.get(*block_id);
                assert_eq!(block.expressions.len(), 1);
                assert_node!(parser.tree, block.expressions[0], Expression::Call { left, .. } => {
                    assert_expression_path!(parser, parser.tree.get(*left), "foo");
                });
            });
        });
    }

    /// Parse Destack function body tails as value expressions.
    #[test]
    fn test_parse_destack_function_body_keeps_tail_expression_value() {
        let mut test = TestParser::new("function run() { foo() }");
        let mut parser = test.prepare();
        let expressions = parser.parse();

        // function run() { foo() }
        assert_eq!(expressions.len(), 1);
        let function_expression_id = parser.unwrap_statement_expression(expressions[0]);
        assert_node!(parser.tree, function_expression_id, Expression::Declaration(function_id) => {
            assert_node!(parser.tree, *function_id, Declaration::Function { body: Some(body_id), .. } => {
                assert_node!(parser.tree, *body_id, Expression::Block(block_id) => {
                    let block = parser.tree.get(*block_id);
                    assert_eq!(block.expressions.len(), 1);
                    assert_node!(parser.tree, block.expressions[0], Expression::Call { left, .. } => {
                        assert_expression_path!(parser, parser.tree.get(*left), "foo");
                    });
                });
            });
        });
    }

    /// Parse a function declaration followed by a call on the same line in JavaScript.
    #[test]
    fn test_parse_function_declaration_followed_by_call_without_newline_javascript() {
        let mut test = TestParser::new_with_options(
            "function main(){return 1}main().catch((function(error){console.error(error);process.exit(1)}));",
            LanguageType::JavaScript,
        );
        let mut parser = test.prepare();
        let expressions = parser.parse();

        // function main(){...} main().catch(...)
        assert_eq!(expressions.len(), 2);

        // first expression: function declaration
        let declaration_id = parser.unwrap_statement_expression(expressions[0]);
        assert_node!(parser.tree, declaration_id, Expression::Declaration(function_id) => {
            assert_node!(parser.tree, *function_id, Declaration::Function { .. });
        });

        // second expression: call expression on `main().catch`
        let call_id = parser.unwrap_statement_expression(expressions[1]);
        assert_node!(parser.tree, call_id, Expression::Call { left, .. } => {
            assert_node!(parser.tree, *left, Expression::Member { left, name, .. } => {
                assert_string!(parser, *name, "catch");
                assert_node!(parser.tree, *left, Expression::Call { left, .. } => {
                    assert_expression_path!(parser, parser.tree.get(*left), "main");
                });
            });
        });
    }

    #[test]
    fn test_parse_javascript_block_sequence_statement_with_newlines_after_commas() {
        let mut test = TestParser::new_with_options(
            r#"{
  callA(),
  callB(),
  callC()
}"#,
            LanguageType::JavaScript,
        );
        let mut parser = test.prepare();
        let block_id = parser.eat_block(BlockContext::Expression).unwrap();
        let block = parser.tree.get(block_id);

        // block should contain one statement expression
        assert_eq!(block.expressions.len(), 1);

        // statement should wrap one sequence expression
        assert_node!(parser.tree, block.expressions[0], Expression::Statement(statement_id) => {
            assert_node!(parser.tree, *statement_id, Expression::SequenceExpression { expressions } => {
                assert_eq!(expressions.len(), 3);
                assert_node!(parser.tree, expressions[0], Expression::Call { left, .. } => {
                    assert_expression_path!(parser, parser.tree.get(*left), "callA");
                });
                assert_node!(parser.tree, expressions[1], Expression::Call { left, .. } => {
                    assert_expression_path!(parser, parser.tree.get(*left), "callB");
                });
                assert_node!(parser.tree, expressions[2], Expression::Call { left, .. } => {
                    assert_expression_path!(parser, parser.tree.get(*left), "callC");
                });
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

    #[test]
    fn test_parse_throw_trailing_comment_on_statement_wrapper_owner() {
        let mut test =
            TestParser::new_with_options("throw error // throw-tail", LanguageType::TypeScript);
        let mut parser = test.prepare();
        let expressions = parser.parse();

        assert!(
            parser.errors.is_empty(),
            "unexpected parser errors: {:?}",
            parser.errors
        );
        assert_eq!(expressions.len(), 1);

        let statement_id = expressions[0];
        assert_node!(parser.tree, statement_id, Expression::Statement(throw_id) => {
            assert_node!(parser.tree, *throw_id, Expression::Throw { .. } => {});
            let throw_annotations = parser.tree.get_annotations(throw_id.id);
            assert_eq!(throw_annotations.len(), 0);
        });

        let annotations = parser.tree.get_annotations(statement_id.id);
        assert!(annotations.is_empty());
        assert_eq!(parser.tree.comment_trivia().len(), 1);
        crate::assert_comment_trivia!(parser, 0, CommentStyle::Slash, "throw-tail");
    }

    #[test]
    fn test_parse_throw_semicolon_trailing_comment_on_statement_wrapper_owner() {
        let mut test =
            TestParser::new_with_options("throw error; // throw-tail", LanguageType::TypeScript);
        let mut parser = test.prepare();
        let expressions = parser.parse();

        assert!(
            parser.errors.is_empty(),
            "unexpected parser errors: {:?}",
            parser.errors
        );
        assert_eq!(expressions.len(), 1);

        let statement_id = expressions[0];
        assert_node!(parser.tree, statement_id, Expression::Statement(throw_id) => {
            assert_node!(parser.tree, *throw_id, Expression::Throw { .. } => {});
            let throw_annotations = parser.tree.get_annotations(throw_id.id);
            assert_eq!(throw_annotations.len(), 0);
        });

        let annotations = parser.tree.get_annotations(statement_id.id);
        assert!(annotations.is_empty());
        assert_eq!(parser.tree.comment_trivia().len(), 1);
        crate::assert_comment_trivia!(parser, 0, CommentStyle::Slash, "throw-tail");
    }

    #[test]
    fn test_parse_return_semicolon_trailing_comment_on_statement_wrapper_owner() {
        let mut test =
            TestParser::new_with_options("return value; // return-tail", LanguageType::TypeScript);
        let mut parser = test.prepare();
        let expressions = parser.parse();

        assert!(
            parser.errors.is_empty(),
            "unexpected parser errors: {:?}",
            parser.errors
        );
        assert_eq!(expressions.len(), 1);

        let statement_id = expressions[0];
        assert_node!(parser.tree, statement_id, Expression::Statement(return_id) => {
            assert_node!(parser.tree, *return_id, Expression::Return { value } => {
                assert!(value.is_some());
            });
            let return_annotations = parser.tree.get_annotations(return_id.id);
            assert_eq!(return_annotations.len(), 0);
        });

        let annotations = parser.tree.get_annotations(statement_id.id);
        assert!(annotations.is_empty());
        assert_eq!(parser.tree.comment_trivia().len(), 1);
        crate::assert_comment_trivia!(parser, 0, CommentStyle::Slash, "return-tail");
    }

    #[test]
    fn test_parse_function_throw_semicolon_trailing_comment_on_statement_wrapper_owner() {
        let mut test = TestParser::new_with_options(
            "function fail() {\n    throw error; // throw-tail\n}",
            LanguageType::TypeScript,
        );
        let mut parser = test.prepare();
        let expressions = parser.parse();

        assert!(
            parser.errors.is_empty(),
            "unexpected parser errors: {:?}",
            parser.errors
        );
        assert_eq!(expressions.len(), 1);

        let function_expression_id = parser.unwrap_statement_expression(expressions[0]);
        assert_node!(parser.tree, function_expression_id, Expression::Declaration(function_id) => {
            assert_node!(parser.tree, *function_id, Declaration::Function { body, .. } => {
                let body_id = body.expect("expected function body");
                assert_node!(parser.tree, body_id, Expression::Block(block_id) => {
                    let block = parser.tree.get(*block_id);
                    assert_eq!(block.expressions.len(), 1);

                    let statement_id = block.expressions[0];
                    assert_node!(parser.tree, statement_id, Expression::Statement(throw_id) => {
                        assert_node!(parser.tree, *throw_id, Expression::Throw { .. } => {});
                        let throw_annotations = parser.tree.get_annotations(throw_id.id);
                        assert_eq!(throw_annotations.len(), 0);
                    });

                    let annotations = parser.tree.get_annotations(statement_id.id);
                    assert!(annotations.is_empty());
                    assert_eq!(parser.tree.comment_trivia().len(), 1);
                    crate::assert_comment_trivia!(parser, 0, CommentStyle::Slash, "throw-tail");
                });
            });
        });
    }
    #[test]
    fn test_parse_return_tree_literal_with_close_paren_text_in_ternary_typescript_xml() {
        let mut test = TestParser::new_with_options(
            "function render(isEnabled) {
  return (
    <div>
      {isEnabled ? (
        <div>)</div>
      ) : null}
    </div>
  )
}",
            LanguageType::TypeScriptXml,
        );
        let mut parser = test.prepare();
        let expressions = parser.parse();

        assert!(
            parser.errors.is_empty(),
            "unexpected parser errors: {:?}",
            parser.errors
        );
        assert_eq!(expressions.len(), 1);

        let function_expression_id = parser.unwrap_statement_expression(expressions[0]);
        assert_node!(parser.tree, function_expression_id, Expression::Declaration(function_id) => {
            assert_node!(parser.tree, *function_id, Declaration::Function { body: Some(body), .. } => {
                assert_node!(parser.tree, *body, Expression::Block(block_id) => {
                    let block = parser.tree.get(*block_id);
                    assert_eq!(block.expressions.len(), 1);

                    let return_id = parser.unwrap_statement_expression(block.expressions[0]);
                    assert_node!(parser.tree, return_id, Expression::Return { value: Some(value) } => {
                        assert_node!(parser.tree, *value, Expression::Parenthesized { expression } => {
                            assert_node!(parser.tree, *expression, Expression::TreeExpression { .. });
                        });
                    });
                });
            });
        });
    }

    #[test]
    fn test_parse_statement_separator_comment_before_semicolon_attaches_to_previous_statement() {
        let mut test = TestParser::new_with_options(
            "declare const PAGE_PATH: string\n  //<- keep-marker\n;(()=>{})()",
            LanguageType::TypeScript,
        );
        let mut parser = test.prepare();
        let expressions = parser.parse();

        assert!(
            parser.errors.is_empty(),
            "unexpected parser errors: {:?}",
            parser.errors
        );
        assert_eq!(expressions.len(), 2);

        let first_annotations = parser.tree.get_annotations(expressions[0].id);
        assert!(first_annotations.is_empty());
        assert_eq!(parser.tree.comment_trivia().len(), 1);
        crate::assert_comment_trivia!(parser, 0, CommentStyle::Slash, "<- keep-marker");
    }

    /// Parse deeply nested JavaScript if statements without overflowing the parser stack.
    #[test]
    fn test_parse_deeply_nested_if_statement() {
        let depth = 512;
        let mut source = String::new();

        // open nested if blocks
        for _ in 0..depth {
            source.push_str("if (true) {");
        }

        // terminal block expression
        source.push('0');

        // close nested if blocks
        for _ in 0..depth {
            source.push('}');
        }

        let mut test = TestParser::new_with_options(&source, LanguageType::JavaScript);
        let mut parser = test.prepare();
        let expressions = parser.parse();

        assert_eq!(expressions.len(), 1);
        assert!(
            parser.errors.is_empty(),
            "unexpected parser errors: {:?}",
            parser.errors
        );
    }
}
