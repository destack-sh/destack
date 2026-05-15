use destack_core::StringId;
use destack_dir::{
    Block, BlockContext, BlockForm, Declaration, Expression, FunctionDeclaration, FunctionForm,
    Keyword, LocalNodeId, NodeType, TokenType, YieldCardinality,
};
use destack_source::{NodeSpanRegion, NodeSpanType, Span};

use crate::parse::parser::ParserFlags;
use crate::parse::prelude::*;
use crate::{ParseError, ParseResult, Parser, ParserSpanStart};

/// The recursion interval for stack growth checks in statement parsing.
#[cfg(not(debug_assertions))]
const STATEMENT_STACK_GROW_CHECK_INTERVAL: u32 = 256;

impl Parser {
    /// Return true when a token can start a recovered statement item.
    #[inline]
    fn token_can_start_recovered_statement_item(token_type: TokenType) -> bool {
        matches!(
            token_type,
            TokenType::Identifier
                | TokenType::OpenBrace
                | TokenType::OpenParenthesis
                | TokenType::OpenBracket
                | TokenType::LessThan
                | TokenType::Literal
                | TokenType::At
                | TokenType::Spread
                | TokenType::Multiply
                | TokenType::ElementwiseAnd
                | TokenType::ElementwiseOr
                | TokenType::ElementwiseXor
                | TokenType::Not
                | TokenType::Hash
                | TokenType::Divide
                | TokenType::DivideAssign
        )
    }

    /// Return parser contexts for statement-position parsing.
    #[inline]
    pub(crate) fn statement_position_contexts(&self) -> (ParserFlags, ParserFlags) {
        let ambient_context = self.flags.nested().with_statement_context(true);
        let expression_context = self.flags.nested().with_statement_position(true);
        (ambient_context, expression_context)
    }

    /// Return true when the current token sequence starts a block.
    #[inline]
    pub(crate) fn is_block_start(&mut self) -> bool {
        self.peek_is(TokenType::OpenBrace)
            || self.language.is_destack()
                && self.is_keyword(Keyword::Do)
                && self.lookahead(|parser| {
                    parser.bump();
                    parser.peek_is(TokenType::OpenBrace)
                })
    }

    /// Return true when the next token sequence starts a block.
    #[inline]
    pub(crate) fn is_next_block_start(&mut self) -> bool {
        self.lookahead(|parser| {
            parser.bump();
            parser.peek_is(TokenType::OpenBrace)
        }) || self.language.is_destack()
            && self.is_next_keyword(Keyword::Do)
            && self.lookahead(|parser| {
                parser.bump();
                parser.bump();
                parser.peek_token_type()
            }) == TokenType::OpenBrace
    }

    /// Eat an expression in a non-position context.
    #[inline]
    fn eat_expression_not_in_position(&mut self) -> ParseResult<LocalNodeId<Expression>> {
        self.eat_expression_with_context_unchecked(self.flags.not_in_position())
    }

    /// Return true when a token ends the current block body.
    #[inline]
    fn is_block_body_terminator_token(&self, token_type: TokenType, form: BlockForm) -> bool {
        token_type == TokenType::End
            || (token_type == TokenType::CloseBrace && form != BlockForm::Implicit)
    }

    /// Try to consume one stray closing delimiter in an implicit statement body.
    fn try_consume_stray_close_delimiter_in_implicit_block_body(
        &mut self,
        form: BlockForm,
        statements: &mut Vec<LocalNodeId<Expression>>,
    ) -> ParseResult<bool> {
        if form != BlockForm::Implicit {
            return Ok(false);
        }

        let token = match self.peek() {
            Ok(token) if Self::is_close_delimiter_token(token.token.ty) => *token,
            _ => return Ok(false),
        };

        // stray closers should produce one error node and advance
        let error = ParseError::unexpected_for(token.span, NodeType::Expression);
        self.error(&error);
        self.bump();

        let error_id = self.tree.insert(Expression::Error, token.span);
        statements.push(error_id);

        Ok(true)
    }

    /// Return true when the current `identifier:` head can parse as a label.
    pub(super) fn can_parse_label_expression(&mut self) -> bool {
        // labels only start on `identifier:`
        if !self.lookahead(|parser| {
            parser.bump();
            parser.peek_is(TokenType::Colon)
        }) {
            return false;
        }

        // inspect the label target to determine whether label parsing is allowed here
        let (label_target_type, label_target_keyword) = self.lookahead(|parser| {
            parser.bump(); // label
            parser.bump(); // colon

            (parser.peek_token_type(), parser.current_keyword())
        });

        // label targets that are always expression statements
        let is_label_expression = matches!(
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

        // labeled blocks are only allowed in statement position
        let is_label_block = label_target_type == TokenType::OpenBrace;
        let is_in_statement_position = self.flags.is_in_statement_position();
        if is_in_statement_position && !self.language.is_destack() {
            return true;
        }

        is_label_expression
            || self.flags.is_in_module_directive()
            || (is_in_statement_position && is_label_block)
    }

    /// Eat one labeled expression shell after the caller accepted `identifier:`.
    pub(super) fn eat_label_expression_parts(
        &mut self,
    ) -> ParseResult<(StringId, Span, LocalNodeId<Expression>)> {
        // parse label prefix
        let (label, label_span) = self.eat_identifier_with_span()?;
        self.eat_colon()?;

        // allow empty labeled statements (`label:;`)
        let body = if self.peek_is(TokenType::Semicolon) {
            let body_start = self.span_start();
            self.bump(); // eat semicolon
            let block_id = self.insert_node(
                Block {
                    context: BlockContext::Statement,
                    form: BlockForm::Implicit,
                    leading_expressions: Vec::new(),
                    tail_expression: None,
                },
                self.get_span_from(&body_start),
            );
            self.tree
                .insert(Expression::Block(block_id), self.get_span_from(&body_start))
        } else {
            self.eat_expression_in_scope()?
        };

        // semicolon statement forms reject labeled declarations
        if !self.language.is_destack() && self.is_single_statement_declaration(body) {
            return Err(ParseError::unexpected(self.tree.get_span(body)));
        }

        Ok((label, label_span, body))
    }

    /// Try to parse a labeled statement before generic statement keyword dispatch.
    fn try_parse_label_statement_expression(
        &mut self,
        start: &ParserSpanStart,
    ) -> ParseResult<Option<LocalNodeId<Expression>>> {
        if !self.can_parse_label_expression() {
            return Ok(None);
        }

        let (label, label_span, body) = self.eat_label_expression_parts()?;

        // build labeled expression
        let label_id =
            self.insert_node(Expression::Label { label, body }, self.get_span_from(start));
        self.tree.set_main_span(label_id, label_span);

        Ok(Some(label_id))
    }

    /// Try to parse a statement expression that starts with an identifier.
    #[inline]
    fn try_dispatch_identifier_statement_expression(
        &mut self,
        start: &ParserSpanStart,
    ) -> ParseResult<Option<LocalNodeId<Expression>>> {
        // parse labeled statements before keyword and expression dispatch
        if let Some(expression_id) = self.try_parse_label_statement_expression(start)? {
            return Ok(Some(expression_id));
        }

        // direct keyword dispatch in statement position
        if let Some(keyword) = self.current_keyword() {
            if let Some(expression_id) =
                self.try_eat_direct_statement_keyword_expression(start, keyword)?
            {
                let expression = self.tree.get(expression_id);
                let is_terminal_statement = expression.is_statement_boundary();
                if is_terminal_statement {
                    return Ok(Some(expression_id));
                }

                let continuation_id =
                    self.eat_expression_continuation(start, expression_id, false)?;
                return Ok(Some(continuation_id));
            }

            // contextual declaration keywords become plain identifier expressions here
            if matches!(
                keyword,
                Keyword::Struct
                    | Keyword::Enum
                    | Keyword::Interface
                    | Keyword::Extension
                    | Keyword::Type
                    | Keyword::Using
            ) {
                // identifier-like keyword head
                let expression_id = self.eat_identifier_expression_path(start)?;

                // ordinary continuation
                let expression_id =
                    self.eat_expression_continuation(start, expression_id, false)?;
                return Ok(Some(expression_id));
            }

            return Ok(None);
        }

        // dispatch contextual declaration heads
        if self.should_parse_declaration_descriptor() {
            let expression_id = self.eat_expression_in_scope()?;
            return Ok(Some(expression_id));
        }

        // parse plain identifier paths after keyword dispatch already rejected
        if let Some(expression_id) = self.try_parse_plain_identifier_expression(start)? {
            return Ok(Some(expression_id));
        }

        Ok(None)
    }

    /// Try to dispatch a statement expression.
    #[inline]
    fn try_dispatch_statement_expression(
        &mut self,
        start: &ParserSpanStart,
        token_type: TokenType,
    ) -> ParseResult<Option<LocalNodeId<Expression>>> {
        // block statements stay in the statement dispatch
        if token_type == TokenType::OpenBrace {
            // object literals stay in value space when their property shape is explicit
            if self.can_parse_object_literal_in_statement_position() {
                return Ok(None);
            }

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

        Ok(None)
    }

    /// Eat one statement expression in the current parser flags.
    #[inline]
    pub(crate) fn eat_statement_expression(&mut self) -> ParseResult<LocalNodeId<Expression>> {
        // normalize to the next non-newline token once per dispatch
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
        // stack depth
        let depth = self.state.statement_stack_depth;
        self.state.statement_stack_depth = depth + 1;

        // guard interval
        #[cfg(debug_assertions)]
        let should_check_stack = depth != 0;

        // guard interval
        #[cfg(not(debug_assertions))]
        let should_check_stack =
            depth != 0 && depth.is_multiple_of(STATEMENT_STACK_GROW_CHECK_INTERVAL);

        // parse with stack guard
        let result = if should_check_stack {
            destack_core::ensure_sufficient_stack(|| {
                self.eat_statement_expression_from_token_kind_inner(token_type)
            })
        } else {
            self.eat_statement_expression_from_token_kind_inner(token_type)
        };

        // restore depth
        self.state.statement_stack_depth = depth;

        result
    }

    /// Eat one statement expression when the parser cursor is already normalized.
    #[inline]
    fn eat_statement_expression_from_token_kind_inner(
        &mut self,
        token_type: TokenType,
    ) -> ParseResult<LocalNodeId<Expression>> {
        // statement dispatch
        let start = self.span_start();
        if let Some(expression_id) = self.try_dispatch_statement_expression(&start, token_type)? {
            return Ok(expression_id);
        }

        // parenthesized lambda heads keep statement mode
        if token_type == TokenType::OpenParenthesis && self.flags.is_in_statement_position() {
            let has_lambda_follow = self.lookahead(|parser| {
                let mut depth = 0u32;
                while parser.has_more_tokens() {
                    let token_type = parser.peek_token_type();
                    if token_type == TokenType::OpenParenthesis {
                        depth += 1;
                    } else if token_type == TokenType::CloseParenthesis {
                        depth -= 1;
                        if depth == 0 {
                            parser.bump();
                            return matches!(
                                parser.peek_token_type(),
                                TokenType::Arrow | TokenType::ArrowWide | TokenType::Colon
                            );
                        }
                    }

                    parser.bump();
                }

                false
            });
            if has_lambda_follow {
                return self.eat_expression_in_scope();
            }
        }

        // non identifier starts parse outside statement mode
        if token_type != TokenType::Identifier {
            return self.eat_expression_outside_statement_position();
        }

        // plain identifiers parse outside statement mode
        if self.current_keyword().is_none() {
            return self.eat_expression_outside_statement_position();
        }

        // identifier keywords stay in the statement entry path
        self.eat_expression_in_scope()
    }

    /// Eat a block or a single statement wrapped in a block.
    pub fn eat_block_or_statement(&mut self) -> ParseResult<LocalNodeId<Block>> {
        // if it's a block, just eat it
        if self.is_block_start() {
            return self.eat_block(BlockContext::Statement);
        }

        let start = self.span_start();

        // empty statement (just semicolon, e.g., `for (x of y);`)
        if self.peek_is(TokenType::Semicolon) {
            self.bump();
            let block_id = self.insert_node(
                Block {
                    context: BlockContext::Statement,
                    form: BlockForm::Implicit,
                    leading_expressions: vec![],
                    tail_expression: None,
                },
                self.get_span_from(&start),
            );
            return Ok(block_id);
        }

        // otherwise, eat a single statement and wrap it in a block
        let (ambient_context, expression_context) = self.statement_position_contexts();
        let expression_id = self.with_flags(
            self.flags
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
        let block_id = self.insert_node(
            Block {
                context: BlockContext::Statement,
                form: BlockForm::Implicit,
                leading_expressions: vec![expression_id],
                tail_expression: None,
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
        let current = self.unwrap_label_expression(expression_id);

        // detect declaration expressions that are invalid in single statement contexts
        match self.tree.get(current) {
            Expression::Declaration(declaration_id) => !matches!(
                self.tree.get(*declaration_id),
                Declaration::Function(FunctionDeclaration { signature, .. })
                    if signature.form == FunctionForm::Lambda
            ),
            Expression::LetElse { .. }
            | Expression::Using { .. }
            | Expression::Import { .. }
            | Expression::Export { .. } => true,
            Expression::Let { .. } => true,
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
        let start = self.span_start();

        // `do` prefix
        let has_do_prefix = self.language.is_destack() && self.is_keyword(Keyword::Do);
        if has_do_prefix {
            self.bump(); // eat keyword
        }
        let form = if has_do_prefix {
            BlockForm::Do
        } else {
            BlockForm::Explicit
        };

        // body
        self.try_eat_token(TokenType::OpenBrace, TokenType::CloseBrace)
            .for_node_type(NodeType::Block)?;
        let (leading_expressions, tail_expression) = self
            .eat_block_body_parts_in_context(form, block_context)
            .for_node_type(NodeType::Block)?;
        self.eat_close_token_or_recover_missing(TokenType::CloseBrace, NodeType::Block)?;

        // block
        let block_id = self.insert_node(
            Block {
                context: block_context,
                form,
                leading_expressions,
                tail_expression,
            },
            self.get_span_from(&start),
        );

        Ok(block_id)
    }

    /// Eat a block of expressions (without the label, `{`, and `}`).
    /// ASI rules apply such that expressions are automatically coerced into statements in relevant positions.
    pub fn eat_block_body(&mut self, form: BlockForm) -> ParseResult<Vec<LocalNodeId<Expression>>> {
        let block_context = if form.is_explicit() {
            BlockContext::Expression
        } else {
            BlockContext::Statement
        };
        self.eat_block_body_in_context(form, block_context)
    }

    /// Eat a block body with an explicit block context.
    pub fn eat_block_body_in_context(
        &mut self,
        form: BlockForm,
        block_context: BlockContext,
    ) -> ParseResult<Vec<LocalNodeId<Expression>>> {
        let (mut leading_expressions, tail_expression) =
            self.eat_block_body_parts_in_context(form, block_context)?;
        if let Some(tail_expression) = tail_expression {
            leading_expressions.push(tail_expression);
        }

        Ok(leading_expressions)
    }

    /// Eat a block body with an explicit block context, preserving the tail split.
    fn eat_block_body_parts_in_context(
        &mut self,
        form: BlockForm,
        block_context: BlockContext,
    ) -> ParseResult<(
        Vec<LocalNodeId<Expression>>,
        Option<LocalNodeId<Expression>>,
    )> {
        // keep statement flags for the whole body to avoid per statement flag churn
        let (ambient_context, expression_context) = self.statement_position_contexts();
        if self.flags == ambient_context && self.flags == expression_context {
            return self.eat_block_body_parts_in_statement_position(form, block_context);
        }
        self.with_flags(
            self.flags
                .with_ambient_context(ambient_context)
                .with_expression_context(expression_context),
            |parser| parser.eat_block_body_parts_in_statement_position(form, block_context),
        )
    }

    /// Eat a block body while already in statement position.
    fn eat_block_body_parts_in_statement_position(
        &mut self,
        form: BlockForm,
        block_context: BlockContext,
    ) -> ParseResult<(
        Vec<LocalNodeId<Expression>>,
        Option<LocalNodeId<Expression>>,
    )> {
        // parse all statement items and keep at most one tail expression
        let mut statements: Vec<LocalNodeId<Expression>> = Vec::new();
        let mut pending_tail_expression: Option<LocalNodeId<Expression>> = None;

        loop {
            // read the current token once per iteration
            let token_type = self.peek_token_type();

            // stop at block terminators
            if self.is_block_body_terminator_token(token_type, form) {
                break;
            }

            // consume statement separators
            if token_type == TokenType::Semicolon {
                self.bump(); // eat semicolon
                continue;
            }

            // stray close delimiters in implicit bodies should recover once and advance
            if self
                .try_consume_stray_close_delimiter_in_implicit_block_body(form, &mut statements)?
            {
                continue;
            }

            // previous tail expressions are no longer block tails once a new item starts
            if let Some(pending_id) = pending_tail_expression.take() {
                statements.push(pending_id);
            }

            // parse and recover one statement item
            let start = self.span_start();
            let (expression_id, is_statement) = self
                .eat_statement_expression_from_token_kind_with_recovery(
                    &start,
                    token_type,
                    Some((form, block_context)),
                )?;

            // keep at most one tail candidate, emit statements directly
            if is_statement {
                statements.push(expression_id);
            } else {
                pending_tail_expression = Some(expression_id);
            }
        }

        // finalize the remaining tail expression
        let tail_expression = if let Some(expression_id) = pending_tail_expression {
            // explicit expression blocks can preserve one trailing value
            if form.is_explicit()
                && self.language.is_destack()
                && block_context == BlockContext::Expression
            {
                Some(expression_id)
            } else {
                statements.push(expression_id);
                None
            }
        } else {
            None
        };

        Ok((statements, tail_expression))
    }

    /// Try to eat a statement expression (return Expression::Error if error and recovery is possible).
    /// Returns whether the expression should be treated as a statement.
    pub fn try_eat_statement_expression_classified(
        &mut self,
    ) -> ParseResult<(LocalNodeId<Expression>, bool)> {
        let (ambient_context, expression_context) = self.statement_position_contexts();
        self.with_flags(
            self.flags
                .with_ambient_context(ambient_context)
                .with_expression_context(expression_context),
            |parser| parser.try_eat_statement_expression_in_statement_position(),
        )
    }

    /// Try to eat a statement expression while already in statement position.
    /// Returns whether the expression should be treated as a statement.
    #[inline]
    fn try_eat_statement_expression_in_statement_position(
        &mut self,
    ) -> ParseResult<(LocalNodeId<Expression>, bool)> {
        let start = self.span_start();
        let token_type = self.peek_token_type();

        self.eat_statement_expression_from_token_kind_with_recovery(&start, token_type, None)
    }

    /// Eat one statement expression from one normalized token kind and recover local statement errors.
    fn eat_statement_expression_from_token_kind_with_recovery(
        &mut self,
        start: &ParserSpanStart,
        token_type: TokenType,
        block_context: Option<(BlockForm, BlockContext)>,
    ) -> ParseResult<(LocalNodeId<Expression>, bool)> {
        let parsed_expression = self
            .eat_statement_expression_from_token_kind(token_type)
            .and_then(|expression_id| {
                self.classify_statement_expression(start, expression_id, block_context)
            });

        match parsed_expression {
            Ok(expression_id) => Ok(expression_id),
            Err(err) => {
                let err = err.for_node_type(NodeType::Expression);
                let span = err.leaf_span();
                let recovered_span = self.try_recover_in_statement_from_span(span, Some(err))?;
                let error_id = self.tree.insert(Expression::Error, recovered_span);
                Ok((error_id, true))
            }
        }
    }

    /// Finalize statement parsing with separator checks and statement coercion.
    #[inline]
    fn classify_statement_expression(
        &mut self,
        start: &ParserSpanStart,
        expression_id: LocalNodeId<Expression>,
        block_context: Option<(BlockForm, BlockContext)>,
    ) -> ParseResult<(LocalNodeId<Expression>, bool)> {
        // semicolon terminated expressions always become statement expressions
        if self.peek_token_type() == TokenType::Semicolon {
            self.bump(); // eat semicolon
            self.tree.set_side_span(
                expression_id,
                NodeSpanType::Region(NodeSpanRegion::Statement),
                self.get_span_from(start),
            );
            return Ok((expression_id, true));
        }

        // detect expression kinds that should stay statement-shaped
        let expression = self.tree.get(expression_id);
        let is_statement = expression.is_statement_boundary();
        let preserves_value_tail = expression.preserves_value_tail_in_expression_block();
        let next_token_type = self.peek_token_type();
        let has_separator = self.current_token_is_on_new_line()
            || matches!(next_token_type, TokenType::Semicolon | TokenType::End);

        // explicit expression blocks can keep value-capable control tails
        let keeps_value_tail = block_context.is_some_and(|(form, block_context)| {
            form.is_explicit()
                && block_context == BlockContext::Expression
                && self.language.is_destack()
                && preserves_value_tail
        });
        let stops_at_block_terminator = block_context
            .is_some_and(|(form, _)| self.is_block_body_terminator_token(next_token_type, form));

        // recover trailing statement junk after a committed expression
        //
        // this keeps the longest valid prefix as the statement shape instead of
        // collapsing the whole statement to `Expression::Error`
        if !is_statement && !has_separator && !stops_at_block_terminator {
            // keep a plausible next statement head for the outer block loop
            if Self::token_can_start_recovered_statement_item(next_token_type) {
                let error = ParseError::unexpected(self.peek()?.span);
                self.error(&error);

                self.tree.set_side_span(
                    expression_id,
                    NodeSpanType::Region(NodeSpanRegion::Statement),
                    self.get_span_from(start),
                );

                return Ok((expression_id, true));
            }

            let error = ParseError::unexpected(self.peek()?.span);
            let recovery_start = self.span_start();
            self.try_recover_in_statement(&recovery_start, Some(error))?;
            self.tree.set_side_span(
                expression_id,
                NodeSpanType::Region(NodeSpanRegion::Statement),
                self.get_span_from(start),
            );

            return Ok((expression_id, true));
        }

        if is_statement && keeps_value_tail && (stops_at_block_terminator || !has_separator) {
            return Ok((expression_id, false));
        }

        let is_statement_position = is_statement || has_separator || stops_at_block_terminator;
        if is_statement_position {
            self.tree.set_side_span(
                expression_id,
                NodeSpanType::Region(NodeSpanRegion::Statement),
                self.get_span_from(start),
            );
        }

        Ok((expression_id, is_statement))
    }

    /// Try to eat a statement expression (return Expression::Error if error and recovery is possible).
    /// Wraps semicolon expressions in a Statement expression, otherwise just returns the expression.
    pub fn try_eat_statement_expression(&mut self) -> ParseResult<LocalNodeId<Expression>> {
        let (expression_id, _is_statement) = self.try_eat_statement_expression_classified()?;
        Ok(expression_id)
    }

    /// Return true when the token after the current identifier ends a bare label form.
    #[inline]
    fn next_token_ends_label_statement(&mut self) -> bool {
        let next_token = self.next_token();
        next_token.token.is_on_new_line
            || matches!(
                next_token.token.ty,
                TokenType::Semicolon | TokenType::CloseBrace | TokenType::End
            )
    }

    /// Eat a break expression.
    ///
    /// Examples:
    /// ```
    /// break
    /// break :label
    /// break label
    /// break :label 15
    /// ```
    pub fn eat_break(&mut self) -> ParseResult<LocalNodeId<Expression>> {
        let start = self.span_start();
        self.eat_keyword(Keyword::Break)?;

        // label and value:
        // 1. `:identifier` → colon-prefixed label, optionally followed by value
        // 2. `identifier` at statement stop → bare label
        // 3. Otherwise → trailing value expression
        let can_insert_semicolon = self.can_insert_semicolon();
        let (label, label_span, value_id) =
            if !can_insert_semicolon && self.peek_is(TokenType::Colon) {
                // colon-prefixed label: break :label [value]
                self.bump(); // eat colon
                let (label, label_span) = self.eat_identifier_with_span()?;
                let value_id = if self.language.is_destack()
                    && !self.can_insert_semicolon()
                    && !self.is_any_stop()
                {
                    let value_id = self.eat_expression_not_in_position()?;
                    Some(value_id)
                } else {
                    None
                };
                (Some(label), Some(label_span), value_id)
            }
            // bare label: break label
            else if !can_insert_semicolon
                && self.peek_is(TokenType::Identifier)
                && self.next_token_ends_label_statement()
            {
                let (label, label_span) = self.eat_identifier_with_span()?;
                (Some(label), Some(label_span), None)
            }
            // trailing value: break value
            else if self.language.is_destack() && !can_insert_semicolon && !self.is_any_stop() {
                let value_id = self.eat_expression_not_in_position()?;
                (None, None, Some(value_id))
            } else {
                (None, None, None)
            };

        // break
        let break_id = self.insert_node(
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
    /// continue :label
    /// continue label
    /// ```
    pub fn eat_continue(&mut self) -> ParseResult<LocalNodeId<Expression>> {
        let start = self.span_start();
        self.eat_keyword(Keyword::Continue)?;

        // label parsing:
        // 1. `:identifier` → colon-prefixed label
        // 2. `identifier` at statement stop → bare label
        let (label, label_span) = if self.peek_is(TokenType::Colon) {
            // colon-prefixed label: continue :label
            self.bump(); // eat colon
            let (label, label_span) = self.eat_identifier_with_span()?;
            (Some(label), Some(label_span))
        } else if self.peek_is(TokenType::Identifier) && self.next_token_ends_label_statement() {
            // bare label: continue label
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
    /// await! someFallibleAsync()
    /// ```
    pub fn eat_await(&mut self) -> ParseResult<LocalNodeId<Expression>> {
        let start = self.span_start();

        // keyword
        self.eat_keyword(Keyword::Await)?;

        // check for adjacent await? or await! markers
        let previous_end = self.prev().map(|previous| previous.span.end);
        let current_start = self.peek().ok().map(|token| token.span.start);
        let marker_is_adjacent = previous_end.is_some() && previous_end == current_start;
        let is_maybe = marker_is_adjacent && self.peek_is(TokenType::Maybe);
        let is_must = marker_is_adjacent && self.peek_is(TokenType::Not);
        if is_maybe {
            self.bump(); // eat ?
        }
        if is_must {
            self.bump(); // eat !
        }

        // allow multiline await operands

        // expression
        let expression_id = self.eat_expression_not_in_position()?;

        // await, await?, or await!
        let expression = if is_maybe {
            Expression::AwaitMaybe {
                expression: expression_id,
            }
        } else if is_must {
            Expression::AwaitMust {
                expression: expression_id,
            }
        } else {
            Expression::Await {
                expression: expression_id,
            }
        };
        let await_id = self.insert_node(expression, self.get_span_from(&start));
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
        let start = self.span_start();

        // keyword
        self.eat_keyword(Keyword::Comptime)?;

        // body expression
        // parse body with comptime statement flags
        let ambient_context = self.flags.with_comptime(true);
        let expression_context = self.flags.not_in_position();
        let body_id = self.with_flags(
            self.flags
                .with_ambient_context(ambient_context)
                .with_expression_context(expression_context),
            |parser| parser.eat_statement_expression(),
        )?;

        // comptime
        let comptime_id = self.insert_node(
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
        let start = self.span_start();

        // keyword
        self.eat_keyword(Keyword::Yield)?;

        // stop when yield has no explicit operand in this position
        let mut operand_is_omitted = self.yield_operand_is_omitted();
        if operand_is_omitted {
            let yield_id = self.insert_node(
                Expression::Yield {
                    cardinality: YieldCardinality::Scalar,
                    value: None,
                },
                self.get_span_from(&start),
            );
            return Ok(yield_id);
        }

        // generator: `yield*` or `yield *` (space before *, but no newline)
        let cardinality = if self.peek_is(TokenType::Multiply) {
            self.bump(); // eat *
            operand_is_omitted = self.yield_operand_is_omitted();
            YieldCardinality::Generator
        } else {
            YieldCardinality::Scalar
        };

        // generator yield keeps one committed missing operand when the value is absent
        let value_id = if self.has_more_tokens()
            && !operand_is_omitted
            && !Self::is_expression_slot_boundary_token(self.peek_token_type())
        {
            Some(self.eat_expression_not_in_position()?)
        } else if cardinality == YieldCardinality::Generator {
            Some(self.recover_missing_expression_here(NodeType::Expression))
        } else {
            None
        };

        // yield
        let yield_id = self.insert_node(
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
        self.current_token_is_on_new_line()
            || matches!(
                self.peek_token_type(),
                TokenType::Semicolon | TokenType::CloseBrace | TokenType::End
            )
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
        let start = self.span_start();
        self.eat_keyword(Keyword::Throw)?;

        // value
        let starts_after_statement_boundary = self.current_token_is_on_new_line()
            || matches!(
                self.peek_token_type(),
                TokenType::Semicolon | TokenType::CloseBrace | TokenType::End
            );
        let value_id = if starts_after_statement_boundary
            || Self::is_expression_slot_boundary_token(self.peek_token_type())
        {
            self.recover_missing_expression_here(NodeType::Expression)
        } else {
            self.eat_expression_not_in_position()?
        };

        // throw
        let throw_id = self.insert_node(
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
        let start = self.span_start();
        self.eat_keyword(Keyword::Return)?;

        // value
        let starts_after_statement_boundary = self.current_token_is_on_new_line()
            || matches!(
                self.peek_token_type(),
                TokenType::Semicolon | TokenType::CloseBrace | TokenType::End
            );
        let value_id = if !starts_after_statement_boundary {
            let value_id = self
                .eat_expression_not_in_position()
                .for_node_type(NodeType::Expression)?;
            Some(value_id)
        } else {
            None
        };
        // return
        let return_id = self.insert_node(
            Expression::Return { value: value_id },
            self.get_span_from(&start),
        );
        Ok(return_id)
    }
}

#[cfg(test)]
mod tests {
    use destack_dir::{
        Block, BlockContext, CommentKind, Declaration, Expression, FunctionDeclaration,
        FunctionForm, IfForm, Key, LetKind, MatchCase, Name, NodeType, Property, ScalarLiteral,
        TokenType, YieldCardinality,
    };
    use destack_source::{LanguageType, NodeSpanRegion, NodeSpanType};

    use crate::{
        ParserOptions, TestParser, assert_comment, assert_expression_path, assert_node,
        assert_string, block_expression_ids,
    };

    #[test]
    fn test_parse_empty_block() {
        let mut test = TestParser::new("{}");
        let mut parser = test.prepare();
        let block_id = parser.eat_block(BlockContext::Expression).unwrap();
        let block = parser.tree.get(block_id);
        assert!(block.is_empty());
    }

    #[test]
    fn test_parse_block_with_missing_close_brace() {
        let mut test = TestParser::new("{ value");
        let mut parser = test.prepare();
        let block_id = parser.eat_block(BlockContext::Expression).unwrap();

        assert_eq!(parser.errors.len(), 1);

        assert_node!(parser.tree, block_id, Block { leading_expressions, tail_expression, .. } => {
            assert!(leading_expressions.is_empty());
            let tail_expression = tail_expression.expect("expected tail expression");
            assert_expression_path!(parser, parser.tree.get(tail_expression), "value");
        });
    }

    #[test]
    fn test_parse_root_unmatched_close_brace_recovery() {
        let mut test = TestParser::new("}\nnextValue");
        let mut parser = test.prepare();
        let expressions = parser.parse();

        assert_eq!(expressions.len(), 2);
        assert_node!(parser.tree, expressions[0], Expression::Error);
        assert_expression_path!(parser, parser.tree.get(expressions[1]), "nextValue");
    }

    #[test]
    fn test_parse_root_unmatched_close_parenthesis_recovery() {
        let mut test = TestParser::new(")\nnextValue");
        let mut parser = test.prepare();
        let expressions = parser.parse();

        assert_eq!(expressions.len(), 2);
        assert_node!(parser.tree, expressions[0], Expression::Error);
        assert_expression_path!(parser, parser.tree.get(expressions[1]), "nextValue");
    }

    #[test]
    fn test_statement_expression_separator_with_comment_newline() {
        let mut test = TestParser::new_with_language(
            "'use strict' /**/ \n nextValue",
            LanguageType::TypeScript,
        );
        let mut parser = test.prepare();
        let (directive_id, is_statement) =
            parser.try_eat_statement_expression_classified().unwrap();
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
            assert_node!(parser.tree, *expression, Expression::Call { position: _, left, generic_arguments: _, arguments } => {
                assert_expression_path!(parser, parser.tree.get(*left), "someFunction");
                assert!(arguments.is_empty());
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
            assert_node!(parser.tree, *expression, Expression::Call { position: _, left, generic_arguments: _, arguments } => {
                assert_expression_path!(parser, parser.tree.get(*left), "someFunction");
                assert!(arguments.is_empty());
            });
        });
    }

    #[test]
    fn test_await_must_expression() {
        let mut test = TestParser::new("await! someFunction()");
        let mut parser = test.prepare();
        let await_id = parser.eat_await().unwrap();

        // await! someFunction()
        assert_node!(parser.tree, await_id, Expression::AwaitMust { expression } => {
            // someFunction()
            assert_node!(parser.tree, *expression, Expression::Call { position: _, left, generic_arguments: _, arguments } => {
                assert_expression_path!(parser, parser.tree.get(*left), "someFunction");
                assert!(arguments.is_empty());
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
            assert_node!(parser.tree, *body, Expression::Call { position: _, left, generic_arguments: _, arguments } => {
                assert_expression_path!(parser, parser.tree.get(*left), "factorial");
                assert_eq!(arguments.len(), 1);
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
            assert_node!(parser.tree, value.unwrap(), Expression::Call { position: _, left, generic_arguments: _, arguments } => {
                assert_expression_path!(parser, parser.tree.get(*left), "someFunction");
                assert!(arguments.is_empty());
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
            assert_node!(parser.tree, value.unwrap(), Expression::Call { position: _, left, generic_arguments: _, arguments } => {
                assert_expression_path!(parser, parser.tree.get(*left), "someFunction");
                assert!(arguments.is_empty());
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
        let yield_id = parser.eat_yield().unwrap();

        // diagnostics
        test.assert_error_leaves(&parser, &[(Some(NodeType::Expression), None, "")]);

        // yield*
        assert_node!(parser.tree, yield_id, Expression::Yield { cardinality, value } => {
            assert_eq!(*cardinality, YieldCardinality::Generator);
            assert_node!(parser.tree, value.expect("expected missing generator operand"), Expression::Missing);
        });
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
        let language = LanguageType::JavaScript;
        let mut test = TestParser::new_with_language("yield\n*a", language);
        let mut parser = test.prepare();

        // yield parses fine with ASI
        let yield_id = parser.eat_yield().unwrap();
        assert_node!(parser.tree, yield_id, Expression::Yield { cardinality, value } => {
            assert_eq!(*cardinality, YieldCardinality::Scalar);
            assert!(value.is_none()); // ASI applied, no value
        });

        // try to parse *a as next statement: should fail in untyped value mode
        // because * is not valid as a unary prefix there
        let error = parser.eat_expression(parser.flags).unwrap_err();

        // *
        assert_eq!(parser.get_span_str(error.leaf_span()), "*");
    }

    /// `yield/*\n*/*a` should not be parsed as `yield* a`.
    #[test]
    fn test_yield_asi_with_block_comment_newline_js_mode() {
        // source: yield/*\n*/*a
        let mut test = TestParser::new_with_language("yield/*\n*/*a", LanguageType::JavaScript);
        let mut parser = test.prepare();

        let yield_id = parser.eat_yield().unwrap();
        assert_node!(parser.tree, yield_id, Expression::Yield { cardinality, value } => {
            assert_eq!(*cardinality, YieldCardinality::Scalar);
            assert!(value.is_none());
        });

        let error = parser.eat_expression(parser.flags).unwrap_err();

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
        let mut test = TestParser::new_with_language("throw /*\n*/ e", LanguageType::JavaScript);
        let mut parser = test.prepare();
        let throw_id = parser.eat_throw().unwrap();

        // diagnostics
        test.assert_error_leaves(&parser, &[(Some(NodeType::Expression), None, "e")]);

        // throw /*\n*/
        assert_node!(parser.tree, throw_id, Expression::Throw { value } => {
            assert_node!(parser.tree, *value, Expression::Missing);
        });

        // e
        assert!(parser.peek_is(TokenType::Identifier));
    }

    /// Reject throw expressions split by unicode line separator comments.
    #[test]
    fn test_reject_throw_expression_with_line_separator_comment() {
        // source: throw /* \u{2028} */ e
        let mut test =
            TestParser::new_with_language("throw /* \u{2028} */ e", LanguageType::JavaScript);
        let mut parser = test.prepare();
        let throw_id = parser.eat_throw().unwrap();

        // diagnostics
        test.assert_error_leaves(&parser, &[(Some(NodeType::Expression), None, "e")]);

        // throw /* \u{2028} */
        assert_node!(parser.tree, throw_id, Expression::Throw { value } => {
            assert_node!(parser.tree, *value, Expression::Missing);
        });

        // e
        assert!(parser.peek_is(TokenType::Identifier));
    }

    #[test]
    fn test_parse_throw_without_value_recovers_missing_expression() {
        let mut test = TestParser::new("throw");
        let mut parser = test.prepare();
        let throw_id = parser.eat_throw().unwrap();

        // diagnostics
        test.assert_error_leaves(&parser, &[(Some(NodeType::Expression), None, "")]);

        // throw
        assert_node!(parser.tree, throw_id, Expression::Throw { value } => {
            assert_node!(parser.tree, *value, Expression::Missing);
        });
    }

    #[test]
    fn test_parse_throw_without_value_before_newline_keeps_following_statement_shape() {
        let mut test = TestParser::new(
            r#"
throw
next()
"#,
        );
        let mut parser = test.prepare();
        let expressions = parser.parse();

        // diagnostics
        test.assert_error_leaves(&parser, &[(Some(NodeType::Expression), None, "next")]);

        // statements
        assert_eq!(expressions.len(), 2);

        // throw
        assert_node!(parser.tree, expressions[0], Expression::Throw { value } => {
                assert_node!(parser.tree, *value, Expression::Missing);
        });

        // next()
        assert_node!(parser.tree, expressions[1], Expression::Call { left, generic_arguments, arguments, .. } => {
                assert_expression_path!(parser, parser.tree.get(*left), "next");
                assert!(generic_arguments.is_empty());
                assert!(arguments.is_empty());
        });
    }

    #[test]
    fn test_parse_throw_without_value_before_newline_keeps_following_const_shape() {
        let mut test = TestParser::new(
            r#"
throw
const value = 1
"#,
        );
        let mut parser = test.prepare();
        let expressions = parser.parse();

        // diagnostics
        test.assert_error_leaves(&parser, &[(Some(NodeType::Expression), None, "const")]);

        // statements
        assert_eq!(expressions.len(), 2);

        // throw
        assert_node!(parser.tree, expressions[0], Expression::Throw { value } => {
                assert_node!(parser.tree, *value, Expression::Missing);
        });

        // const value = 1
        assert_node!(parser.tree, expressions[1], Expression::Let { declarators, .. } => {
            assert_eq!(declarators.len(), 1);
        });
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
    fn test_parse_block_const_then_return_cast() {
        let mut test = TestParser::new_with_language(
            "{\n  const result = CreateRecord(IntegerKey, value)\n  return result as never\n}",
            LanguageType::TypeScript,
        );
        let mut parser = test.prepare();
        let block_id = parser.eat_block(BlockContext::Expression).unwrap();
        let block = parser.tree.get(block_id);
        assert_eq!(block.leading_expressions.len(), 2);
        assert!(block.tail_expression.is_none());

        let let_expression_id = block.leading_expressions[0];
        assert_node!(parser.tree, let_expression_id, Expression::Let { kind, declarators, .. } => {
            assert_eq!(*kind, LetKind::Const);
            assert_eq!(declarators.len(), 1);
        });

        let return_expression_id = block.leading_expressions[1];
        assert_node!(parser.tree, return_expression_id, Expression::Return { value } => {
            let value = value.expect("expected return value");
            assert_node!(parser.tree, value, Expression::As { .. } => {
            });
        });
    }

    #[test]
    fn test_parse_return_ternary_with_newline_before_question() {
        let mut test = TestParser::new_with_language(
            "return Result.IsExtendsTrueLike(check)\n  ? TryInferResults(tail, right, [...result, head])\n  : undefined",
            LanguageType::TypeScript,
        );
        let mut parser = test.prepare();
        let return_id = parser.eat_return().unwrap();

        assert_node!(parser.tree, return_id, Expression::Return { value } => {
            let value = value.expect("expected return value");
            assert_node!(parser.tree, value, Expression::If { form, .. } => {
                assert_eq!(*form, IfForm::Ternary);
            });
        });
    }

    #[test]
    fn test_parse_block_statement_before_close_brace_without_semicolon() {
        let mut test =
            TestParser::new_with_language("{ process.exit(1)}", LanguageType::JavaScript);
        let mut parser = test.prepare();
        let block_id = parser.eat_block(BlockContext::Expression).unwrap();
        let block = parser.tree.get(block_id);

        // block should contain one leading statement expression
        assert_eq!(block.leading_expressions.len(), 1);
        assert!(block.tail_expression.is_none());
        assert_node!(
            parser.tree,
            block.leading_expressions[0],
            Expression::Call { .. }
        );
    }

    /// Parse semicolon led parenthesized calls without parenthesized wrappers.
    #[test]
    fn test_parse_statement_leading_semicolon_parenthesized_arrow_call_without_wrappers() {
        let mut test = TestParser::new_with_language("{\n;(()=>{})()\n}", LanguageType::Destack);
        let mut parser = test.prepare();
        parser.apply_options(ParserOptions {
            preserve_parenthesized_wrappers: false,
            ..ParserOptions::default()
        });
        let block_id = parser.eat_block(BlockContext::Expression).unwrap();
        let block = parser.tree.get(block_id);
        let expressions = block_expression_ids(block);

        assert_eq!(expressions.len(), 1);

        assert_node!(parser.tree, expressions[0], Expression::Call { left, arguments, .. } => {
            assert!(arguments.is_empty());
            assert_node!(parser.tree, *left, Expression::Declaration(declaration_id) => {
                assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, .. }) => {
                    assert_eq!(signature.form, FunctionForm::Lambda);
                });
            });
        });
    }

    /// Record statement source spans when parenthesized wrappers are skipped.
    #[test]
    fn test_parse_statement_span_preserves_skipped_parenthesized_wrapper() {
        let mut test = TestParser::new_with_language("(() => value);", LanguageType::TypeScript);
        let mut parser = test.prepare();
        parser.apply_options(ParserOptions {
            preserve_parenthesized_wrappers: false,
            ..ParserOptions::default()
        });

        let (expression_id, is_statement) =
            parser.try_eat_statement_expression_classified().unwrap();
        let statement_span = parser
            .tree
            .get_side_span(
                expression_id,
                NodeSpanType::Region(NodeSpanRegion::Statement),
            )
            .expect("missing statement span");

        assert!(is_statement);
        assert_eq!(parser.get_span_str(statement_span), "(() => value);");
    }

    /// Record statement source spans for root tail statements.
    #[test]
    fn test_parse_root_statement_span_preserves_skipped_parenthesized_wrapper() {
        let mut test = TestParser::new_with_language("(() => value);", LanguageType::TypeScript);
        let mut parser = test.prepare();
        parser.apply_options(ParserOptions {
            preserve_parenthesized_wrappers: false,
            ..ParserOptions::default()
        });

        let expressions = parser.parse();
        let statement_span = parser
            .tree
            .get_side_span(
                expressions[0],
                NodeSpanType::Region(NodeSpanRegion::Statement),
            )
            .expect("missing statement span");

        assert_eq!(expressions.len(), 1);
        assert_eq!(parser.get_span_str(statement_span), "(() => value);");
    }

    /// Record root expression statement spans before skipped wrappers.
    #[test]
    fn test_parse_root_statement_span_preserves_leading_parenthesized_wrapper() {
        let mut test = TestParser::new_with_language(
            "const a = 1\n\n;(function() {})()",
            LanguageType::JavaScript,
        );
        let mut parser = test.prepare();
        parser.apply_options(ParserOptions {
            preserve_parenthesized_wrappers: false,
            ..ParserOptions::default()
        });

        let expressions = parser.parse();
        let statement_span = parser
            .tree
            .get_side_span(
                expressions[1],
                NodeSpanType::Region(NodeSpanRegion::Statement),
            )
            .expect("missing statement span");

        assert_eq!(expressions.len(), 2);
        assert_eq!(parser.get_span_str(statement_span), "(function() {})()");
    }

    /// Parse if-body block tails as value expressions.
    #[test]
    fn test_parse_if_block_keeps_tail_expression_value() {
        let mut test = TestParser::new("if (x) { foo() }");
        let mut parser = test.prepare();
        let expressions = parser.parse();

        // if (x) { foo() }
        assert_eq!(expressions.len(), 1);
        let if_expression_id = parser.unwrap_label_expression(expressions[0]);
        assert_node!(parser.tree, if_expression_id, Expression::If { then_expression, .. } => {
            assert_node!(parser.tree, *then_expression, Expression::Block(block_id) => {
                let block = parser.tree.get(*block_id);
                assert!(block.leading_expressions.is_empty());
                let tail_expression = block.tail_expression.expect("expected tail expression");
                assert_node!(parser.tree, tail_expression, Expression::Call { left, .. } => {
                    assert_expression_path!(parser, parser.tree.get(*left), "foo");
                });
            });
        });
    }

    /// Parse function body tails as value expressions.
    #[test]
    fn test_parse_function_body_keeps_tail_expression_value() {
        let mut test = TestParser::new("function run() { foo() }");
        let mut parser = test.prepare();
        let expressions = parser.parse();

        // function run() { foo() }
        assert_eq!(expressions.len(), 1);
        let function_expression_id = parser.unwrap_label_expression(expressions[0]);
        assert_node!(parser.tree, function_expression_id, Expression::Declaration(function_id) => {
            assert_node!(parser.tree, *function_id, Declaration::Function(FunctionDeclaration { body: Some(body_id), .. }) => {
                assert_node!(parser.tree, *body_id, Expression::Block(block_id) => {
                    let block = parser.tree.get(*block_id);
                    assert!(block.leading_expressions.is_empty());
                    let tail_expression = block.tail_expression.expect("expected tail expression");
                    assert_node!(parser.tree, tail_expression, Expression::Call { left, .. } => {
                        assert_expression_path!(parser, parser.tree.get(*left), "foo");
                    });
                });
            });
        });
    }

    /// Parse object literal function body tails as value expressions.
    #[test]
    fn test_parse_function_body_keeps_object_literal_tail_expression_value() {
        let input = r#"
function next(value: number): IteratorResult<number> {
    drop(value);
    { done: true, value }
}
"#;
        let mut test = TestParser::new(input);
        let mut parser = test.prepare();
        let expressions = parser.parse();

        test.assert_no_errors(&parser);

        // function next(...) { drop(value); { done: true, value } }
        assert_eq!(expressions.len(), 1);
        let function_expression_id = parser.unwrap_label_expression(expressions[0]);
        assert_node!(parser.tree, function_expression_id, Expression::Declaration(function_id) => {
            assert_node!(parser.tree, *function_id, Declaration::Function(FunctionDeclaration { body: Some(body_id), .. }) => {
                assert_node!(parser.tree, *body_id, Expression::Block(block_id) => {
                    let block = parser.tree.get(*block_id);
                    assert_eq!(block.leading_expressions.len(), 1);
                    assert_node!(parser.tree, block.leading_expressions[0], Expression::Call { left, arguments, .. } => {
                        assert_expression_path!(parser, parser.tree.get(*left), "drop");
                        assert_eq!(arguments.len(), 1);
                    });

                    let tail_expression = block.tail_expression.expect("expected tail expression");
                    assert_node!(parser.tree, tail_expression, Expression::ObjectExpression { properties, .. } => {
                        assert_eq!(properties.len(), 2);
                        assert_node!(parser.tree, properties[0], Property::Field { key: Key::Name(Name::Identifier(name)), value, is_shorthand } => {
                            assert_string!(parser, *name, "done");
                            assert!(!*is_shorthand);
                            assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Boolean(true)));
                        });
                        assert_node!(parser.tree, properties[1], Property::Field { key: Key::Name(Name::Identifier(name)), value, is_shorthand } => {
                            assert_string!(parser, *name, "value");
                            assert!(*is_shorthand);
                            assert_expression_path!(parser, parser.tree.get(*value), "value");
                        });
                    });
                });
            });
        });
    }

    /// Parse object literal match branch tails as value expressions.
    #[test]
    fn test_parse_match_branch_keeps_object_literal_tail_expression_value() {
        let input = r#"
function apply(result: Result): IteratorResult<number> {
    match (result) {
        Yield { value } => {
            this.value = value;
            { done: false, value }
        }
        Return { value } => {
            { done: true, value }
        }
    }
}
"#;
        let mut test = TestParser::new(input);
        let mut parser = test.prepare();
        let expressions = parser.parse();

        test.assert_no_errors(&parser);

        // function apply(...) { match (...) { ... } }
        assert_eq!(expressions.len(), 1);
        let function_expression_id = parser.unwrap_label_expression(expressions[0]);
        assert_node!(parser.tree, function_expression_id, Expression::Declaration(function_id) => {
            assert_node!(parser.tree, *function_id, Declaration::Function(FunctionDeclaration { body: Some(body_id), .. }) => {
                assert_node!(parser.tree, *body_id, Expression::Block(function_block_id) => {
                    let function_block = parser.tree.get(*function_block_id);
                    let tail_expression = function_block.tail_expression.expect("expected match tail");

                    assert_node!(parser.tree, tail_expression, Expression::Match { cases, .. } => {
                        assert_eq!(cases.len(), 2);
                        for case_id in cases {
                            assert_node!(parser.tree, *case_id, MatchCase::Block { body: case_block_id, .. } => {
                                let case_block = parser.tree.get(*case_block_id);
                                let case_tail = case_block.tail_expression.expect("expected object tail");

                                assert_node!(parser.tree, case_tail, Expression::ObjectExpression { properties, .. } => {
                                    assert_eq!(properties.len(), 2);
                                    assert_node!(parser.tree, properties[0], Property::Field { key: Key::Name(Name::Identifier(name)), value, .. } => {
                                        assert_string!(parser, *name, "done");
                                        assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Boolean(_)));
                                    });
                                    assert_node!(parser.tree, properties[1], Property::Field { key: Key::Name(Name::Identifier(name)), value, is_shorthand } => {
                                        assert_string!(parser, *name, "value");
                                        assert!(*is_shorthand);
                                        assert_expression_path!(parser, parser.tree.get(*value), "value");
                                    });
                                });
                            });
                        }
                    });
                });
            });
        });
    }

    /// Parse multiline function body tails as value expressions.
    #[test]
    fn test_parse_multiline_function_body_keeps_tail_expression_value() {
        let mut test = TestParser::new(
            r#"
function run() {
    foo()
}
"#,
        );
        let mut parser = test.prepare();
        let expressions = parser.parse();

        // function run() { foo() }
        assert_eq!(expressions.len(), 1);
        let function_expression_id = parser.unwrap_label_expression(expressions[0]);
        assert_node!(parser.tree, function_expression_id, Expression::Declaration(function_id) => {
            assert_node!(parser.tree, *function_id, Declaration::Function(FunctionDeclaration { body: Some(body_id), .. }) => {
                assert_node!(parser.tree, *body_id, Expression::Block(block_id) => {
                    let block = parser.tree.get(*block_id);
                    assert!(block.leading_expressions.is_empty());
                    let tail_expression = block.tail_expression.expect("expected tail expression");
                    assert_node!(parser.tree, tail_expression, Expression::Call { left, .. } => {
                        assert_expression_path!(parser, parser.tree.get(*left), "foo");
                    });
                });
            });
        });
    }

    /// Parse function body if-else tails as value expressions.
    #[test]
    fn test_parse_function_body_keeps_if_else_tail_expression_value() {
        let mut test = TestParser::new(
            r#"
function choose(flag: boolean, a: int32, b: int32): int32 {
    if (flag) { a } else { b }
}
"#,
        );
        let mut parser = test.prepare();
        let expressions = parser.parse();

        // function choose(...) { if (flag) { a } else { b } }
        assert_eq!(expressions.len(), 1);
        let function_expression_id = parser.unwrap_label_expression(expressions[0]);
        assert_node!(parser.tree, function_expression_id, Expression::Declaration(function_id) => {
            assert_node!(parser.tree, *function_id, Declaration::Function(FunctionDeclaration { body: Some(body_id), .. }) => {
                assert_node!(parser.tree, *body_id, Expression::Block(block_id) => {
                    let block = parser.tree.get(*block_id);
                    assert_eq!(block.leading_expressions.len(), 0);

                    let tail_expression = block.tail_expression.expect("expected tail expression");
                    assert_node!(parser.tree, tail_expression, Expression::If { then_expression, else_expression, .. } => {
                        assert_node!(parser.tree, *then_expression, Expression::Block(then_block_id) => {
                            let then_block = parser.tree.get(*then_block_id);
                            assert_eq!(then_block.leading_expressions.len(), 0);
                            assert_expression_path!(
                                parser,
                                parser.tree.get(then_block.tail_expression.expect("expected then tail")),
                                "a"
                            );
                        });

                        let else_expression = else_expression.expect("expected else expression");
                        assert_node!(parser.tree, else_expression, Expression::Block(else_block_id) => {
                            let else_block = parser.tree.get(*else_block_id);
                            assert_eq!(else_block.leading_expressions.len(), 0);
                            assert_expression_path!(
                                parser,
                                parser.tree.get(else_block.tail_expression.expect("expected else tail")),
                                "b"
                            );
                        });
                    });
                });
            });
        });
    }

    /// Parse explicit branch semicolons as statements inside value-tail if expressions.
    #[test]
    fn test_parse_function_body_keeps_if_else_branch_semicolons_as_statements() {
        let mut test = TestParser::new(
            r#"
function choose(flag: boolean, a: int32, b: int32): int32 {
    if (flag) { a; } else { b; }
}
"#,
        );
        let mut parser = test.prepare();
        let expressions = parser.parse();

        assert_eq!(expressions.len(), 1);
        let function_expression_id = parser.unwrap_label_expression(expressions[0]);
        assert_node!(parser.tree, function_expression_id, Expression::Declaration(function_id) => {
            assert_node!(parser.tree, *function_id, Declaration::Function(FunctionDeclaration { body: Some(body_id), .. }) => {
                assert_node!(parser.tree, *body_id, Expression::Block(function_block_id) => {
                    let function_block = parser.tree.get(*function_block_id);
                    let tail_expression = function_block.tail_expression.expect("expected tail expression");

                    assert_node!(parser.tree, tail_expression, Expression::If { then_expression, else_expression, .. } => {
                        assert_node!(parser.tree, *then_expression, Expression::Block(then_block_id) => {
                            let then_block = parser.tree.get(*then_block_id);
                            assert_eq!(then_block.leading_expressions.len(), 1);
                            assert!(then_block.tail_expression.is_none());
                        });

                        let else_expression = else_expression.expect("expected else expression");
                        assert_node!(parser.tree, else_expression, Expression::Block(else_block_id) => {
                            let else_block = parser.tree.get(*else_block_id);
                            assert_eq!(else_block.leading_expressions.len(), 1);
                            assert!(else_block.tail_expression.is_none());
                        });
                    });
                });
            });
        });
    }

    /// Parse a function declaration followed by a call on the same line in JavaScript.
    #[test]
    fn test_parse_function_declaration_followed_by_call_without_newline() {
        let mut test = TestParser::new_with_language(
            "function main(){return 1}main().catch((function(error){console.error(error);process.exit(1)}));",
            LanguageType::JavaScript,
        );
        let mut parser = test.prepare();
        let expressions = parser.parse();

        // function main(){...} main().catch(...)
        assert_eq!(expressions.len(), 2);

        // first expression: function declaration
        let declaration_id = parser.unwrap_label_expression(expressions[0]);
        assert_node!(parser.tree, declaration_id, Expression::Declaration(function_id) => {
            assert_node!(parser.tree, *function_id, Declaration::Function(FunctionDeclaration { .. }));
        });

        // second expression: call expression on `main().catch`
        let call_id = parser.unwrap_label_expression(expressions[1]);
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
    fn test_parse_block_sequence_statement_with_newlines_after_commas() {
        let mut test = TestParser::new_with_language(
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

        let expressions = block_expression_ids(block);
        assert_eq!(expressions.len(), 1);
        assert!(block.tail_expression.is_none());

        // leading expression should be one sequence expression
        assert_node!(parser.tree, expressions[0], Expression::SequenceExpression { expressions } => {
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
        let mut test = TestParser::new_with_language("return/*\n*/value", LanguageType::JavaScript);
        let mut parser = test.prepare();
        let return_id = parser.eat_return().unwrap();

        assert_node!(parser.tree, return_id, Expression::Return { value } => {
            assert!(value.is_none());
        });
        let value_id = parser.eat_expression(parser.flags).unwrap();
        assert_expression_path!(parser, parser.tree.get(value_id), "value");
    }

    #[test]
    fn test_parse_throw_trailing_comment_on_statement_wrapper_owner() {
        let mut test =
            TestParser::new_with_language("throw error // throw-tail", LanguageType::TypeScript);
        let mut parser = test.prepare();
        let expressions = parser.parse();

        assert!(
            parser.errors.is_empty(),
            "unexpected parser errors: {:?}",
            parser.errors
        );
        assert_eq!(expressions.len(), 1);

        let throw_id = expressions[0];
        assert_node!(parser.tree, throw_id, Expression::Throw { .. } => {});
        let throw_annotations = parser.tree.get_decorators(throw_id.id);
        assert_eq!(throw_annotations.len(), 0);

        let annotations = parser.tree.get_decorators(throw_id.id);
        assert!(annotations.is_empty());
        assert_eq!(parser.tree.comments().len(), 1);
        assert_comment!(parser, 0, CommentKind::Line, "throw-tail");
    }

    #[test]
    fn test_parse_throw_semicolon_trailing_comment_on_statement_wrapper_owner() {
        let mut test =
            TestParser::new_with_language("throw error; // throw-tail", LanguageType::TypeScript);
        let mut parser = test.prepare();
        let expressions = parser.parse();

        assert!(
            parser.errors.is_empty(),
            "unexpected parser errors: {:?}",
            parser.errors
        );
        assert_eq!(expressions.len(), 1);

        let throw_id = expressions[0];
        assert_node!(parser.tree, throw_id, Expression::Throw { .. } => {});
        let throw_annotations = parser.tree.get_decorators(throw_id.id);
        assert_eq!(throw_annotations.len(), 0);

        let annotations = parser.tree.get_decorators(throw_id.id);
        assert!(annotations.is_empty());
        assert_eq!(parser.tree.comments().len(), 1);
        assert_comment!(parser, 0, CommentKind::Line, "throw-tail");
    }

    #[test]
    fn test_parse_return_semicolon_trailing_comment_on_statement_wrapper_owner() {
        let mut test =
            TestParser::new_with_language("return value; // return-tail", LanguageType::TypeScript);
        let mut parser = test.prepare();
        let expressions = parser.parse();

        assert!(
            parser.errors.is_empty(),
            "unexpected parser errors: {:?}",
            parser.errors
        );
        assert_eq!(expressions.len(), 1);

        let return_id = expressions[0];
        assert_node!(parser.tree, return_id, Expression::Return { value } => {
            assert!(value.is_some());
        });
        let return_annotations = parser.tree.get_decorators(return_id.id);
        assert_eq!(return_annotations.len(), 0);

        let annotations = parser.tree.get_decorators(return_id.id);
        assert!(annotations.is_empty());
        assert_eq!(parser.tree.comments().len(), 1);
        assert_comment!(parser, 0, CommentKind::Line, "return-tail");
    }

    #[test]
    fn test_parse_new_without_receiver_as_statement_recovers_missing_constructor() {
        let mut test = TestParser::new("new");
        let mut parser = test.prepare();
        let expressions = parser.parse();

        // diagnostics
        test.assert_error_leaves(&parser, &[(Some(NodeType::Expression), None, "")]);

        // top level statements
        assert_eq!(expressions.len(), 1);

        // new
        let new_id = expressions[0];
        assert_node!(parser.tree, new_id, Expression::New { left, generic_arguments, arguments } => {
            // missing constructor
            assert_node!(parser.tree, *left, Expression::Missing);

            // no generic arguments
            assert!(generic_arguments.is_empty());

            // no dynamic arguments
            assert!(arguments.is_empty());
        });
    }

    #[test]
    fn test_parse_new_without_receiver_before_newline_recovers_missing_constructor() {
        let mut test = TestParser::new(
            r#"
new
"#,
        );
        let mut parser = test.prepare();
        let expressions = parser.parse();

        // diagnostics
        test.assert_error_leaves(&parser, &[(Some(NodeType::Expression), None, "")]);

        // top level statements
        assert_eq!(expressions.len(), 1);

        // new
        let new_id = expressions[0];
        assert_node!(parser.tree, new_id, Expression::New { left, generic_arguments, arguments } => {
            // missing constructor
            assert_node!(parser.tree, *left, Expression::Missing);

            // no generic arguments
            assert!(generic_arguments.is_empty());

            // no dynamic arguments
            assert!(arguments.is_empty());
        });
    }

    #[test]
    fn test_parse_new_without_receiver_before_following_call_keeps_statement_shape() {
        let mut test = TestParser::new(
            r#"
new
next()
"#,
        );
        let mut parser = test.prepare();
        let expressions = parser.parse();

        // diagnostics
        test.assert_error_leaves(&parser, &[(Some(NodeType::Expression), None, "next")]);

        // statements
        assert_eq!(expressions.len(), 2);

        // new
        assert_node!(parser.tree, expressions[0], Expression::New { left, generic_arguments, arguments } => {
                assert_node!(parser.tree, *left, Expression::Missing);
                assert!(generic_arguments.is_empty());
                assert!(arguments.is_empty());
        });

        // next()
        assert_node!(parser.tree, expressions[1], Expression::Call { left, generic_arguments, arguments, .. } => {
                assert_expression_path!(parser, parser.tree.get(*left), "next");
                assert!(generic_arguments.is_empty());
                assert!(arguments.is_empty());
        });
    }

    #[test]
    fn test_parse_new_without_receiver_before_following_const_keeps_statement_shape() {
        let mut test = TestParser::new(
            r#"
new
const value = 1
"#,
        );
        let mut parser = test.prepare();
        let expressions = parser.parse();

        // diagnostics
        test.assert_error_leaves(&parser, &[(Some(NodeType::Expression), None, "const")]);

        // statements
        assert_eq!(expressions.len(), 2);

        // new
        assert_node!(parser.tree, expressions[0], Expression::New { left, generic_arguments, arguments } => {
                assert_node!(parser.tree, *left, Expression::Missing);
                assert!(generic_arguments.is_empty());
                assert!(arguments.is_empty());
        });

        // const value = 1
        assert_node!(parser.tree, expressions[1], Expression::Let { declarators, .. } => {
            assert_eq!(declarators.len(), 1);
        });
    }

    #[test]
    fn test_parse_function_throw_semicolon_trailing_comment_on_statement_wrapper_owner() {
        let mut test = TestParser::new_with_language(
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

        let function_expression_id = parser.unwrap_label_expression(expressions[0]);
        assert_node!(parser.tree, function_expression_id, Expression::Declaration(function_id) => {
            assert_node!(parser.tree, *function_id, Declaration::Function(FunctionDeclaration { body, .. }) => {
                let body_id = body.expect("expected function body");
                assert_node!(parser.tree, body_id, Expression::Block(block_id) => {
                    let block = parser.tree.get(*block_id);
                    assert_eq!(block.leading_expressions.len(), 1);
                    assert!(block.tail_expression.is_none());

                    let throw_id = block.leading_expressions[0];
                    assert_node!(parser.tree, throw_id, Expression::Throw { .. } => {});
                    let throw_annotations = parser.tree.get_decorators(throw_id.id);
                    assert_eq!(throw_annotations.len(), 0);

                    let annotations = parser.tree.get_decorators(throw_id.id);
                    assert!(annotations.is_empty());
                    assert_eq!(parser.tree.comments().len(), 1);
                    assert_comment!(parser, 0, CommentKind::Line, "throw-tail");
                });
            });
        });
    }
    #[test]
    fn test_parse_return_tree_literal_with_close_paren_text_in_ternary_before_tree() {
        let mut test = TestParser::new_with_language(
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

        let function_expression_id = parser.unwrap_label_expression(expressions[0]);
        assert_node!(parser.tree, function_expression_id, Expression::Declaration(function_id) => {
            assert_node!(parser.tree, *function_id, Declaration::Function(FunctionDeclaration { body: Some(body), .. }) => {
                assert_node!(parser.tree, *body, Expression::Block(block_id) => {
                    let block = parser.tree.get(*block_id);
                    assert_eq!(block.leading_expressions.len(), 1);
                    assert!(block.tail_expression.is_none());
                    let return_id =
                        parser.unwrap_label_expression(block.leading_expressions[0]);
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
        let mut test = TestParser::new_with_language(
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

        let first_annotations = parser.tree.get_decorators(expressions[0].id);
        assert!(first_annotations.is_empty());
        assert_eq!(parser.tree.comments().len(), 1);
        assert_comment!(parser, 0, CommentKind::Line, "<- keep-marker");
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

        let mut test = TestParser::new_with_language(&source, LanguageType::JavaScript);
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
