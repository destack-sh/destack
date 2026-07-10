use crate::parse::error::ParserResultExt;
use destack_dir::{
    Block, BlockContext, BlockForm, Declaration, Expression, FunctionDeclaration, FunctionForm,
    Keyword, LocalNodeId, NodeType, Token, TokenType, YieldCardinality,
};
use destack_source::{NodeSpanRegion, NodeSpanType};

use super::r#if::IfHead;
use crate::parse::flags::ParserFlags;
use crate::{Parser, ParserError, ParserResult, ParserSpanStart};

/// One pending statement-position if branch.
struct PendingStatementIf {
    /// The parsed if head.
    head: IfHead,
    /// The source start for the then block.
    block_start: ParserSpanStart,
}

impl Parser {
    /// Return true when a token can start a recovered statement item.
    #[inline]
    pub(crate) fn token_can_start_recovered_statement_item(token_type: TokenType) -> bool {
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

    /// Return parser flags for statement-position parsing.
    #[inline]
    pub(crate) fn statement_position_flags(&self) -> ParserFlags {
        let ambient_context = self.flags.nested().with_statement_context(true);
        let expression_context = self.flags.nested().with_statement_position(true);

        self.flags
            .with_ambient_context(ambient_context)
            .with_expression_context(expression_context)
    }

    /// Return true when the current token sequence starts a block.
    #[inline]
    pub(crate) fn is_block_start(&mut self) -> bool {
        self.peek_is(TokenType::OpenBrace)
            || self.is_keyword(Keyword::Do) && self.next_token_type() == TokenType::OpenBrace
    }

    /// Eat one control body as a block-like expression.
    pub(crate) fn eat_control_body_expression(&mut self) -> ParserResult<LocalNodeId<Expression>> {
        self.with_recursive_descent(NodeType::Expression, |parser| {
            parser.eat_control_body_expression_at_current_depth()
        })
    }

    /// Eat one control body after recursive descent state has been entered.
    fn eat_control_body_expression_at_current_depth(
        &mut self,
    ) -> ParserResult<LocalNodeId<Expression>> {
        let start = self.span_start();

        // parse one statement expression in statement mode
        let flags = self.statement_position_flags().in_before_block();
        let expression_id = self.with_flags(flags, |parser| parser.eat_statement_expression())?;

        // keep existing block-like expressions
        if matches!(
            self.tree.get(expression_id),
            Expression::Block { .. } | Expression::If { .. }
        ) {
            return Ok(expression_id);
        }

        // statement forms do not become value-producing branch tails
        let is_statement = self.tree.get(expression_id).is_statement_boundary()
            || self.peek_is(TokenType::Semicolon);
        if is_statement {
            return Ok(self.insert_block_expression(
                &start,
                BlockContext::Statement,
                BlockForm::Implicit,
                vec![expression_id],
                None,
            ));
        }

        Ok(self.insert_block_expression(
            &start,
            BlockContext::Expression,
            BlockForm::Implicit,
            vec![],
            Some(expression_id),
        ))
    }

    /// Eat an if expression after its head has been parsed.
    pub(crate) fn eat_if_after_head(
        &mut self,
        head: IfHead,
    ) -> ParserResult<LocalNodeId<Expression>> {
        if self.if_then_body_starts_statement_if_chain() {
            return self.eat_statement_if_chain(head);
        }

        let then_expression = self.eat_control_body_expression()?;

        self.finish_if_after_then_block(head, then_expression)
    }

    /// Return true when an if then body starts a statement if chain.
    fn if_then_body_starts_statement_if_chain(&mut self) -> bool {
        self.peek_is(TokenType::OpenBrace) && self.keyword_at_offset(1) == Some(Keyword::If)
    }

    /// Eat an if expression through an explicit statement-position chain.
    fn eat_statement_if_chain(&mut self, head: IfHead) -> ParserResult<LocalNodeId<Expression>> {
        self.with_flags(self.statement_position_flags(), |parser| {
            parser.eat_statement_if_chain_in_statement_position(head)
        })
    }

    /// Eat an if expression chain while already in statement position.
    fn eat_statement_if_chain_in_statement_position(
        &mut self,
        head: IfHead,
    ) -> ParserResult<LocalNodeId<Expression>> {
        let mut chain = Vec::new();
        let mut head = head;

        loop {
            // open this then block
            let block_start = self.span_start();
            self.try_eat_token(TokenType::OpenBrace, TokenType::CloseBrace)
                .for_node_type(NodeType::Block)?;
            chain.push(PendingStatementIf { head, block_start });

            // continue while the body starts with another if
            if self.is_keyword(Keyword::If) {
                head = self.eat_if_head()?;
                continue;
            }

            break;
        }

        // parse the innermost body
        let (leading_expressions, tail_expression) = self
            .eat_block_body_split_in_statement_position(
                BlockForm::Explicit,
                BlockContext::Expression,
                Vec::new(),
                None,
            )
            .for_node_type(NodeType::Block)?;
        self.eat_close_token_or_recover_missing(TokenType::CloseBrace, NodeType::Block)?;

        let pending_if = chain
            .pop()
            .ok_or_else(|| ParserError::unexpected(self.anchor_span_here()))?;
        let then_expression = self.insert_block_expression(
            &pending_if.block_start,
            BlockContext::Expression,
            BlockForm::Explicit,
            leading_expressions,
            tail_expression,
        );
        let mut if_expression =
            self.finish_if_after_then_block(pending_if.head, then_expression)?;

        // resume each parent block after the nested if statement
        while let Some(pending_if) = chain.pop() {
            let (statement_id, is_statement) = self.classify_statement_expression(
                &pending_if.head.start,
                if_expression,
                Some((BlockForm::Explicit, BlockContext::Expression)),
            );
            let mut statements = Vec::new();
            let mut pending_tail_expression = None;
            if is_statement {
                statements.push(statement_id);
            } else {
                pending_tail_expression = Some(statement_id);
            }

            let (leading_expressions, tail_expression) = self
                .eat_block_body_split_in_statement_position(
                    BlockForm::Explicit,
                    BlockContext::Expression,
                    statements,
                    pending_tail_expression,
                )
                .for_node_type(NodeType::Block)?;
            self.eat_close_token_or_recover_missing(TokenType::CloseBrace, NodeType::Block)?;

            let then_expression = self.insert_block_expression(
                &pending_if.block_start,
                BlockContext::Expression,
                BlockForm::Explicit,
                leading_expressions,
                tail_expression,
            );
            if_expression = self.finish_if_after_then_block(pending_if.head, then_expression)?;
        }

        Ok(if_expression)
    }

    /// Insert a block expression node.
    fn insert_block_expression(
        &mut self,
        start: &ParserSpanStart,
        context: BlockContext,
        form: BlockForm,
        leading_expressions: Vec<LocalNodeId<Expression>>,
        tail_expression: Option<LocalNodeId<Expression>>,
    ) -> LocalNodeId<Expression> {
        let block_id = self.insert_node(
            Block {
                context,
                form,
                leading_expressions,
                tail_expression,
            },
            self.get_span_from(start),
        );

        self.tree
            .insert(Expression::Block(block_id), self.tree.get_span(block_id))
    }

    /// Return true when a token ends the current block body.
    #[inline]
    fn is_block_body_terminator_token(&self, token_type: TokenType, form: BlockForm) -> bool {
        token_type == TokenType::End
            || (token_type == TokenType::CloseBrace && form != BlockForm::Implicit)
    }

    /// Try to consume one stray closing delimiter in an implicit statement body.
    fn eat_stray_close_delimiter_in_implicit_body(
        &mut self,
        form: BlockForm,
        statements: &mut Vec<LocalNodeId<Expression>>,
    ) -> ParserResult<bool> {
        if form != BlockForm::Implicit {
            return Ok(false);
        }

        let token = self.peek();
        if !Self::is_close_delimiter_token(token.token.ty()) {
            return Ok(false);
        }

        // stray closers should produce one error node and advance
        let error = ParserError::unexpected_for(token, NodeType::Expression);
        self.report_error(&error);
        self.bump();

        let error_id = self.tree.insert(Expression::Error, token.span);
        statements.push(error_id);

        Ok(true)
    }

    /// Return true when the current `identifier:` head can parse as a label.
    pub(super) fn can_parse_label_expression(&mut self) -> bool {
        // match selectors own their colon boundary
        if self.flags.is_in_match_case() {
            return false;
        }

        // labels only start on `identifier:`
        if self.next_token_type() != TokenType::Colon {
            return false;
        }

        let label_target = self.token_at_offset(2);

        self.label_target_is_valid(label_target)
    }

    /// Return whether the current `:` can continue an already consumed label.
    pub(super) fn can_parse_label_body(&mut self) -> bool {
        if self.flags.is_in_match_case() || !self.peek_is(TokenType::Colon) {
            return false;
        }

        let label_target = self.next_token();

        self.label_target_is_valid(label_target)
    }

    /// Return whether one token can begin the body of a label in this context.
    fn label_target_is_valid(&self, label_target: Token) -> bool {
        let label_target_type = label_target.ty();
        let label_target_keyword = label_target.keyword();

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

        is_label_expression || (is_in_statement_position && is_label_block)
    }

    /// Eat a label body after its identifier has been consumed.
    pub(super) fn eat_label_expression_body(&mut self) -> ParserResult<LocalNodeId<Expression>> {
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
            self.eat_expression(self.flags)?
        };

        Ok(body)
    }

    /// Try to parse a labeled statement before generic statement keyword dispatch.
    fn try_parse_label_statement_expression(
        &mut self,
        start: &ParserSpanStart,
    ) -> ParserResult<Option<LocalNodeId<Expression>>> {
        if !self.can_parse_label_expression() {
            return Ok(None);
        }

        let (label, label_span) = self.eat_identifier_with_span()?;
        let body = self.eat_label_expression_body()?;

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
    ) -> ParserResult<Option<LocalNodeId<Expression>>> {
        // parse labeled statements before keyword and expression dispatch
        if let Some(expression_id) = self.try_parse_label_statement_expression(start)? {
            return Ok(Some(expression_id));
        }

        // direct keyword dispatch in statement position
        if let Some(keyword) = self.current_keyword() {
            if let Some(expression_id) = self.eat_statement_keyword_expression(start, keyword)? {
                let expression = self.tree.get(expression_id);
                let is_terminal_statement = expression.is_statement_boundary();
                if is_terminal_statement
                    && !self.statement_keyword_expression_allows_value_tail(expression_id)
                {
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
            let expression_id = self.eat_expression(self.flags)?;
            return Ok(Some(expression_id));
        }

        // parse plain identifier paths after keyword dispatch already rejected
        if let Some(expression_id) = self.eat_plain_identifier_expression(start)? {
            return Ok(Some(expression_id));
        }

        Ok(None)
    }

    /// Return whether a statement keyword expression can continue as a value.
    fn statement_keyword_expression_allows_value_tail(
        &self,
        expression_id: LocalNodeId<Expression>,
    ) -> bool {
        matches!(
            self.tree.get(expression_id),
            Expression::Declaration(declaration_id)
                if matches!(
                    self.tree.get(*declaration_id),
                    Declaration::Function(FunctionDeclaration { signature, .. })
                        if signature.form == FunctionForm::Lambda
                )
        )
    }

    /// Try to dispatch a statement expression.
    #[inline]
    fn try_dispatch_statement_expression(
        &mut self,
        start: &ParserSpanStart,
        token_type: TokenType,
    ) -> ParserResult<Option<LocalNodeId<Expression>>> {
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
    pub(crate) fn eat_statement_expression(&mut self) -> ParserResult<LocalNodeId<Expression>> {
        // normalize to the next non-newline token once per dispatch
        let token_type = self.peek_token_type();
        self.eat_statement_expression_from_token_kind(token_type)
    }

    /// Eat one statement expression when the parser cursor is already normalized.
    #[inline]
    fn eat_statement_expression_from_token_kind(
        &mut self,
        token_type: TokenType,
    ) -> ParserResult<LocalNodeId<Expression>> {
        // statement dispatch
        let start = self.span_start();
        if let Some(expression_id) = self.try_dispatch_statement_expression(&start, token_type)? {
            return Ok(expression_id);
        }

        // parenthesized lambda heads keep statement mode
        if token_type == TokenType::OpenParenthesis
            && self.flags.is_in_statement_position()
            && self.parenthesized_statement_head_has_lambda_follow()
        {
            return self.eat_expression(self.flags);
        }

        // decorator prefixes keep statement mode for declaration dispatch
        if token_type == TokenType::At {
            return self.eat_expression(self.flags);
        }

        // non identifier starts parse outside statement mode
        if token_type != TokenType::Identifier {
            return self.eat_expression(self.flags.not_in_position());
        }

        // plain identifiers parse outside statement mode
        if self.current_keyword().is_none() {
            return self.eat_expression(self.flags.not_in_position());
        }

        // identifier keywords stay in the statement entry path
        self.eat_expression(self.flags)
    }

    /// Return whether a parenthesized statement head has a lambda follow token.
    fn parenthesized_statement_head_has_lambda_follow(&mut self) -> bool {
        self.lookahead(|parser| {
            matches!(
                parser.scan_parenthesized_follow_token_at_offset(0),
                Some(TokenType::ArrowWide | TokenType::Colon)
            )
        })
    }

    /// Eat a block or a single statement wrapped in a block.
    pub fn eat_block_or_statement(&mut self) -> ParserResult<LocalNodeId<Block>> {
        self.with_recursive_descent(NodeType::Block, |parser| {
            parser.eat_block_or_statement_at_current_depth()
        })
    }

    /// Eat a block or statement body after recursive descent state has been entered.
    fn eat_block_or_statement_at_current_depth(&mut self) -> ParserResult<LocalNodeId<Block>> {
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
        let expression_id = self.with_flags(self.statement_position_flags(), |parser| {
            parser.eat_statement_expression()
        })?;

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

    /// Eat a block (including the label, `{`, and `}`). Optional `do` prefix for disambiguation.
    ///
    /// Examples:
    /// ```
    /// { ... }
    /// block: { ... }
    /// ```
    pub fn eat_block(&mut self, block_context: BlockContext) -> ParserResult<LocalNodeId<Block>> {
        self.with_recursive_descent(NodeType::Block, |parser| {
            parser.eat_block_at_current_depth(block_context)
        })
    }

    /// Eat a block after recursive descent state has been entered.
    fn eat_block_at_current_depth(
        &mut self,
        block_context: BlockContext,
    ) -> ParserResult<LocalNodeId<Block>> {
        let start = self.span_start();

        // `do` prefix
        let has_do_prefix = self.is_keyword(Keyword::Do);
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
            .eat_block_body_split_in_context(form, block_context)
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
    pub fn eat_block_body(
        &mut self,
        form: BlockForm,
    ) -> ParserResult<Vec<LocalNodeId<Expression>>> {
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
    ) -> ParserResult<Vec<LocalNodeId<Expression>>> {
        let (mut leading_expressions, tail_expression) =
            self.eat_block_body_split_in_context(form, block_context)?;
        if let Some(tail_expression) = tail_expression {
            leading_expressions.push(tail_expression);
        }

        Ok(leading_expressions)
    }

    /// Eat a block body with an explicit block context, preserving the tail split.
    fn eat_block_body_split_in_context(
        &mut self,
        form: BlockForm,
        block_context: BlockContext,
    ) -> ParserResult<(
        Vec<LocalNodeId<Expression>>,
        Option<LocalNodeId<Expression>>,
    )> {
        // keep statement flags for the whole body to avoid per statement flag churn
        let flags = self.statement_position_flags();
        if self.flags == flags {
            return self.eat_block_body_split_in_statement_position(
                form,
                block_context,
                Vec::new(),
                None,
            );
        }
        self.with_flags(flags, |parser| {
            parser.eat_block_body_split_in_statement_position(form, block_context, Vec::new(), None)
        })
    }

    /// Eat a block body while already in statement position.
    pub(crate) fn eat_block_body_split_in_statement_position(
        &mut self,
        form: BlockForm,
        block_context: BlockContext,
        mut statements: Vec<LocalNodeId<Expression>>,
        mut pending_tail_expression: Option<LocalNodeId<Expression>>,
    ) -> ParserResult<(
        Vec<LocalNodeId<Expression>>,
        Option<LocalNodeId<Expression>>,
    )> {
        // parse all statement items and keep at most one tail expression
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
            if self.eat_stray_close_delimiter_in_implicit_body(form, &mut statements)? {
                continue;
            }

            // previous tail expressions are no longer block tails once a new item starts
            if let Some(pending_id) = pending_tail_expression.take() {
                statements.push(pending_id);
            }

            // parse and recover one statement item
            let start = self.span_start();
            let (expression_id, is_statement) = self
                .eat_statement_expression_from_token_kind_or_recover(
                    &start,
                    token_type,
                    Some((form, block_context)),
                );
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
            if form.is_explicit() && block_context == BlockContext::Expression {
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

    /// Eat and classify one statement expression, recovering malformed input locally.
    pub fn eat_classified_statement_expression_or_recover(
        &mut self,
    ) -> (LocalNodeId<Expression>, bool) {
        self.with_flags(self.statement_position_flags(), |parser| {
            let start = parser.span_start();
            let token_type = parser.peek_token_type();

            parser.eat_statement_expression_from_token_kind_or_recover(&start, token_type, None)
        })
    }

    /// Eat one statement expression from one normalized token kind and recover local statement errors.
    fn eat_statement_expression_from_token_kind_or_recover(
        &mut self,
        start: &ParserSpanStart,
        token_type: TokenType,
        block_context: Option<(BlockForm, BlockContext)>,
    ) -> (LocalNodeId<Expression>, bool) {
        let expression = self.eat_statement_expression_from_token_kind(token_type);

        match expression {
            Ok(expression_id) => {
                self.classify_statement_expression(start, expression_id, block_context)
            }
            Err(err) => {
                let err = err.for_node_type(NodeType::Expression);
                let span = err.span(self.file_id);
                let recovered_span = self.recover_statement(span, Some(err));
                let error_id = self.tree.insert(Expression::Error, recovered_span);

                (error_id, true)
            }
        }
    }

    /// Finalize statement parsing with separator checks and statement coercion.
    #[inline]
    pub(crate) fn classify_statement_expression(
        &mut self,
        start: &ParserSpanStart,
        expression_id: LocalNodeId<Expression>,
        block_context: Option<(BlockForm, BlockContext)>,
    ) -> (LocalNodeId<Expression>, bool) {
        // semicolon terminated expressions always become statement expressions
        if self.peek_token_type() == TokenType::Semicolon {
            self.bump(); // eat semicolon
            self.tree.set_side_span(
                expression_id,
                NodeSpanType::Region(NodeSpanRegion::Statement),
                self.get_span_from(start),
            );
            return (expression_id, true);
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
            form.is_explicit() && block_context == BlockContext::Expression && preserves_value_tail
        });
        let stops_at_block_terminator = block_context
            .is_some_and(|(form, _)| self.is_block_body_terminator_token(next_token_type, form));

        // recover trailing statement junk after a parsed expression
        //
        // this keeps the longest valid prefix as the statement shape instead of
        // collapsing the whole statement to `Expression::Error`
        if !is_statement && !has_separator && !stops_at_block_terminator {
            // keep a plausible next statement head for the outer block loop
            if Self::token_can_start_recovered_statement_item(next_token_type) {
                let error = ParserError::unexpected(self.peek());
                self.report_error(&error);

                self.tree.set_side_span(
                    expression_id,
                    NodeSpanType::Region(NodeSpanRegion::Statement),
                    self.get_span_from(start),
                );

                return (expression_id, true);
            }

            let error = ParserError::unexpected(self.peek());
            let recovery_start = self.span_start();
            self.recover_statement(self.get_span_from(&recovery_start), Some(error));
            self.tree.set_side_span(
                expression_id,
                NodeSpanType::Region(NodeSpanRegion::Statement),
                self.get_span_from(start),
            );

            return (expression_id, true);
        }

        if is_statement && keeps_value_tail && (stops_at_block_terminator || !has_separator) {
            return (expression_id, false);
        }

        let is_statement_position = is_statement || has_separator || stops_at_block_terminator;
        if is_statement_position {
            self.tree.set_side_span(
                expression_id,
                NodeSpanType::Region(NodeSpanRegion::Statement),
                self.get_span_from(start),
            );
        }

        (expression_id, is_statement)
    }

    /// Eat one statement expression, recovering malformed input locally.
    pub fn eat_statement_expression_or_recover(&mut self) -> LocalNodeId<Expression> {
        let (expression_id, _is_statement) = self.eat_classified_statement_expression_or_recover();

        expression_id
    }

    /// Return true when one token ends a bare label form.
    #[inline]
    fn token_ends_label_statement(&self, token: Token) -> bool {
        token.is_on_new_line()
            || matches!(
                token.ty(),
                TokenType::Semicolon | TokenType::CloseBrace | TokenType::End
            )
    }

    /// Return true when ASI prevents a keyword operand from starting here.
    #[inline]
    fn keyword_operand_is_omitted(&mut self) -> bool {
        self.current_token_is_on_new_line()
            || matches!(
                self.peek_token_type(),
                TokenType::Semicolon | TokenType::CloseBrace | TokenType::End
            )
    }

    /// Eat a break expression.
    ///
    /// A lone identifier operand is always a label as in TypeScript; every
    /// other same-line expression is a value, and `break (label)` forces an
    /// identifier-valued break.
    ///
    /// Examples:
    /// ```
    /// break
    /// break label
    /// break value * 2
    /// break (value)
    /// break label: value
    /// ```
    pub fn eat_break(&mut self) -> ParserResult<LocalNodeId<Expression>> {
        let start = self.span_start();
        self.eat_keyword(Keyword::Break)?;

        // no operand after a statement boundary
        let can_insert_semicolon = self.can_insert_semicolon();
        let (label, label_span, value_id) = if can_insert_semicolon {
            (None, None, None)
        }
        // identifier-headed forms are labels or expressions continuing past one
        else if self.peek_is(TokenType::Identifier) {
            let next_token = self.next_token();

            // labeled value: break label: value
            if !next_token.is_on_new_line() && next_token.ty() == TokenType::Colon {
                let (label, label_span) = self.eat_identifier_with_span()?;
                self.bump(); // eat colon
                let value_id = self.eat_expression(self.flags.not_in_position())?;
                (Some(label), Some(label_span), Some(value_id))
            }
            // bare label: break label
            else if self.token_ends_label_statement(next_token) {
                let (label, label_span) = self.eat_identifier_with_span()?;
                (Some(label), Some(label_span), None)
            }
            // identifier-headed value: break value * 2
            else {
                let value_id = self.eat_expression(self.flags.not_in_position())?;
                (None, None, Some(value_id))
            }
        }
        // trailing value: break "done"
        else if !self.is_any_stop() {
            let value_id = self.eat_expression(self.flags.not_in_position())?;
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
    /// continue label
    /// ```
    pub fn eat_continue(&mut self) -> ParserResult<LocalNodeId<Expression>> {
        let start = self.span_start();
        self.eat_keyword(Keyword::Continue)?;

        // bare label: continue label
        let (label, label_span) = if self.can_insert_semicolon() {
            (None, None)
        } else if self.peek_is(TokenType::Identifier) {
            let next_token = self.next_token();
            if !self.token_ends_label_statement(next_token) {
                return Err(ParserError::unexpected(self.peek()));
            }

            let (label, label_span) = self.eat_identifier_with_span()?;
            (Some(label), Some(label_span))
        } else if !self.is_any_stop() {
            return Err(ParserError::unexpected(self.peek()));
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
    pub fn eat_await(&mut self) -> ParserResult<LocalNodeId<Expression>> {
        let start = self.span_start();

        // keyword
        self.eat_keyword(Keyword::Await)?;

        // check for adjacent await? or await! markers
        let previous_end = self.prev().map(|previous| previous.span.end);
        let marker_is_adjacent = previous_end == Some(self.peek().span.start);
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
        let expression_id = self.eat_expression(self.flags.not_in_position())?;

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
    pub fn eat_comptime(&mut self) -> ParserResult<LocalNodeId<Expression>> {
        let start = self.span_start();

        // keyword
        self.eat_keyword(Keyword::Comptime)?;

        // body expression
        // parse body with comptime statement flags
        let expression_context = self.flags.not_in_position().in_before_block();
        let body_id = if self.is_block_start() {
            let block_id = self.with_flags(
                self.flags.with_expression_context(expression_context),
                |parser| parser.eat_block(BlockContext::Expression),
            )?;
            self.tree
                .insert(Expression::Block(block_id), self.tree.get_span(block_id))
        } else {
            self.with_flags(
                self.flags.with_expression_context(expression_context),
                |parser| parser.eat_statement_expression(),
            )?
        };

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
    pub fn eat_yield(&mut self) -> ParserResult<LocalNodeId<Expression>> {
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

        // generator yield keeps one missing operand when the value is absent
        let value_id = if self.has_more_tokens()
            && !operand_is_omitted
            && !Self::is_expression_slot_boundary_token(self.peek_token_type())
        {
            Some(self.eat_expression(self.flags.not_in_position())?)
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
    pub fn eat_throw(&mut self) -> ParserResult<LocalNodeId<Expression>> {
        let start = self.span_start();
        self.eat_keyword(Keyword::Throw)?;

        // value
        let value_id = if self.keyword_operand_is_omitted()
            || Self::is_expression_slot_boundary_token(self.peek_token_type())
        {
            self.recover_missing_expression_here(NodeType::Expression)
        } else {
            self.eat_expression(self.flags.not_in_position())?
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
    pub fn eat_return(&mut self) -> ParserResult<LocalNodeId<Expression>> {
        let start = self.span_start();
        self.eat_keyword(Keyword::Return)?;

        // value
        let value_id = if !self.keyword_operand_is_omitted() {
            let value_id = self
                .eat_expression(self.flags.not_in_position())
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
