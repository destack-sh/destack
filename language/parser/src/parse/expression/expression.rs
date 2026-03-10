use super::common::{DECLARATION_START_TOKENS, DescriptorHead, is_type_relation_keyword};
use super::lookahead::DelimiterAnalysis;
use crate::parse::parser::ParserOptions;
use crate::parse::prelude::*;
use crate::{ParseError, ParseResult, Parser, ParserMark};

use destack_ast::{
    BinaryOperator, Block, BlockContext, BlockFormat, Declaration, DeclarationDescriptor,
    Expression, Keyword, LocalNodeId, NodeType, TokenType, TypeUnaryOperator, UnaryOperator,
};

use super::super::annotation::PendingDecorators;

/// The recursion interval for stack growth checks in expression parsing.
#[cfg(not(debug_assertions))]
const STACK_GROW_CHECK_INTERVAL: u32 = 256;

impl Parser {
    /// Try to parse a plain identifier expression once the caller proved the current token shape.
    #[inline]
    pub(crate) fn try_parse_plain_identifier_expression_from_identifier(
        &mut self,
        start: &ParserMark,
        pos_index: usize,
        next_raw_index: usize,
        next_raw_token_type: TokenType,
    ) -> ParseResult<Option<LocalNodeId<Expression>>> {
        if self.should_try_contextual_type_literal() {
            return Ok(None);
        }

        if matches!(next_raw_token_type, TokenType::Arrow | TokenType::ArrowWide) {
            return Ok(None);
        }

        // labelled statements need the full expression entry path
        if self.options.is_in_statement_position() && next_raw_token_type == TokenType::Colon {
            return Ok(None);
        }

        // declaration disambiguation only applies to `global` and `module`
        let declaration_identifier_start = self
            .token_ref_at(pos_index)
            .map(|token| token.span.start as usize);
        let has_declaration_identifier_prefix = declaration_identifier_start
            .and_then(|start| self.file.text().as_bytes().get(start))
            .copied()
            .is_some_and(|first_byte| first_byte == b'g' || first_byte == b'm');
        if has_declaration_identifier_prefix {
            let is_global_identifier = self.is_global_identifier_at(pos_index);
            let is_module_identifier = self.language.supports_module_declaration()
                && self.is_module_identifier_at(pos_index);
            if is_global_identifier || is_module_identifier {
                let next_cursor = self.scanner_cursor_from(next_raw_index);
                let next_token_type = next_cursor.token_type;
                let next_token_index = next_cursor.index;
                let next_has_line_break = next_cursor.has_line_break_before;

                // contextual global declarations need descriptor parsing even in non statement contexts
                let can_start_global_declaration = matches!(
                    next_token_type,
                    TokenType::OpenBrace | TokenType::Identifier | TokenType::Literal
                );
                if is_global_identifier && can_start_global_declaration {
                    return Ok(None);
                }

                let is_module_declaration_start = is_module_identifier
                    && !next_has_line_break
                    && DECLARATION_START_TOKENS.contains(&next_token_type)
                    && matches!(next_token_type, TokenType::Identifier | TokenType::Literal)
                    && (next_token_type != TokenType::Identifier
                        || !is_type_relation_keyword(self.keyword_for_index(next_token_index)));
                if is_module_declaration_start {
                    return Ok(None);
                }
            }
        }

        let identifier_expression_id = self.eat_identifier_expression_path(start)?;
        let expression_id = self.eat_expression_continuation(start, identifier_expression_id)?;

        Ok(Some(expression_id))
    }

    /// Eat an expression with a replacement expression-local context.
    #[inline(always)]
    pub(crate) fn eat_expression_with_context_unchecked(
        &mut self,
        context: ParserOptions,
    ) -> ParseResult<LocalNodeId<Expression>> {
        self.eat_expression(self.options.with_expression_context(context))
    }

    #[inline(always)]
    pub fn eat_expression(
        &mut self,
        options: ParserOptions,
    ) -> ParseResult<LocalNodeId<Expression>> {
        if self.options == options {
            return self.eat_expression_inner_with_stack_guard();
        }

        if let Some(speculation_stats) = self.speculation_stats.as_mut() {
            speculation_stats.with_options_calls += 1;
        }

        let old_options = self.swap_options(options);
        let result = self.eat_expression_inner_with_stack_guard();
        self.restore_options(old_options);
        result
    }

    /// Eat an expression in the current parser options.
    #[inline(always)]
    pub(crate) fn eat_expression_in_scope(&mut self) -> ParseResult<LocalNodeId<Expression>> {
        self.eat_expression_inner_with_stack_guard()
    }

    /// Eat an expression after statement keyword dispatch already ran in the caller.
    #[inline(always)]
    pub(crate) fn eat_expression_after_statement_keyword_dispatch(
        &mut self,
    ) -> ParseResult<LocalNodeId<Expression>> {
        self.eat_expression_inner_with_stack_guard()
    }

    /// Eat an expression with statement position temporarily disabled.
    #[inline]
    pub(crate) fn eat_expression_outside_statement_position(
        &mut self,
    ) -> ParseResult<LocalNodeId<Expression>> {
        if !self.options.is_in_statement_position() {
            return self.eat_expression_inner_with_stack_guard();
        }

        let context = self.options.not_in_statement_position();
        if let Some(speculation_stats) = self.speculation_stats.as_mut() {
            speculation_stats.with_options_calls += 1;
        }
        self.eat_expression(self.options.with_expression_context(context))
    }

    /// Eat an expression with stack growth checks.
    #[inline(always)]
    fn eat_expression_inner_with_stack_guard(&mut self) -> ParseResult<LocalNodeId<Expression>> {
        let _timing = self.timing_scope(tags::PARSE_EXPRESSION);

        // stack depth
        let depth = self.expression_stack_depth;
        self.expression_stack_depth = depth + 1;

        // guard interval
        #[cfg(debug_assertions)]
        let should_check_stack = depth != 0;

        // guard interval
        #[cfg(not(debug_assertions))]
        let should_check_stack = depth != 0 && depth.is_multiple_of(STACK_GROW_CHECK_INTERVAL);

        // parse with stack guard
        let result = if should_check_stack {
            destack_core::ensure_sufficient_stack(|| self.eat_expression_inner())
        } else {
            self.eat_expression_inner()
        };

        // restore depth
        self.expression_stack_depth = depth;

        result
    }

    /// Try to eat an expression and recover to an error node.
    pub fn try_eat_expression(
        &mut self,
        recover: TokenType,
    ) -> ParseResult<LocalNodeId<Expression>> {
        match self.eat_expression_in_scope() {
            Ok(expression_id) => Ok(expression_id),
            Err(err) => {
                let err = err.for_node_type(NodeType::Expression);
                let span = err.leaf_span();
                let start = ParserMark::from_span(span);
                self.try_recover(&start, recover, Some(err))?;
                let error_id = self
                    .tree
                    .insert(Expression::Error, self.get_span_from(&start));
                Ok(error_id)
            }
        }
    }

    /// Try to parse a plain identifier expression and continuation in common value contexts.
    pub(crate) fn try_parse_plain_identifier_expression(
        &mut self,
        start: &ParserMark,
    ) -> ParseResult<Option<LocalNodeId<Expression>>> {
        if self.options.is_in_type()
            || self.options.is_in_match_case()
            || self.options.is_in_decorator()
            || self.options.is_in_typeof_query()
        {
            return Ok(None);
        }

        if self.has_active_split() || self.peek_token_type() != TokenType::Identifier {
            return Ok(None);
        }

        let pos_index = self.pos_index();
        if self.keyword_for_index(pos_index).is_some() {
            return Ok(None);
        }

        let next_raw_index = self.index_for_next();
        let next_raw_token_type = self.token_type_at(next_raw_index);
        self.try_parse_plain_identifier_expression_from_identifier(
            start,
            pos_index,
            next_raw_index,
            next_raw_token_type,
        )
    }

    /// Try to parse a plain identifier path in type positions.
    fn try_parse_plain_type_identifier_expression(
        &mut self,
        start: &ParserMark,
    ) -> ParseResult<Option<LocalNodeId<Expression>>> {
        // only run this plain path in plain type positions
        if !self.options.is_in_type()
            || self.options.is_in_typeof_query()
            || self.options.is_in_decorator()
            || self.options.is_in_match_case()
        {
            return Ok(None);
        }

        // require a plain identifier token
        if !self.peek_is(TokenType::Identifier) || self.has_active_split() {
            return Ok(None);
        }

        // contextual type literals still use the existing type literal parser
        if self.should_try_contextual_type_literal() {
            return Ok(None);
        }

        // type keywords and contextual declarations still use the existing keyword parser
        let keyword = self.keyword_for_index(self.pos_index());
        if matches!(
            keyword,
            Some(
                Keyword::Typeof
                    | Keyword::Keyof
                    | Keyword::Infer
                    | Keyword::Asserts
                    | Keyword::Readonly
                    | Keyword::This
                    | Keyword::New
                    | Keyword::Type
                    | Keyword::Newtype
                    | Keyword::Namespace
                    | Keyword::Interface
                    | Keyword::Class
                    | Keyword::Struct
                    | Keyword::Enum
                    | Keyword::Function
                    | Keyword::Async
                    | Keyword::Await
                    | Keyword::Import
                    | Keyword::Extension
            )
        ) {
            return Ok(None);
        }

        let identifier_expression_id = self.eat_identifier_expression_path(start)?;

        let expression_id = self.eat_expression_continuation(start, identifier_expression_id)?;

        Ok(Some(expression_id))
    }

    /// Return true when the current identifier text matches a contextual type literal.
    #[inline]
    fn is_contextual_type_literal_identifier(&mut self) -> bool {
        if !self.peek_is(TokenType::Identifier) {
            return false;
        }

        let Ok(token) = self.peek().copied() else {
            return false;
        };
        let identifier = self.file.span_str(token.span);

        matches!(
            identifier,
            "undefined"
                | "unknown"
                | "object"
                | "null"
                | "any"
                | "never"
                | "boolean"
                | "void"
                | "character"
                | "string"
                | "bigint"
                | "number"
                | "int"
                | "isize"
                | "uint"
                | "usize"
                | "float"
                | "symbol"
                | "unique"
        )
    }

    /// Return true when the current identifier should be parsed as a contextual type literal.
    #[inline]
    fn should_try_contextual_type_literal(&mut self) -> bool {
        if self.options.is_in_type() || self.options.is_in_static() {
            return true;
        }

        // contextual type literals in value positions are a destack only extension
        if !self.language.is_destack() {
            return false;
        }

        self.is_contextual_type_literal_identifier()
    }

    /// Parse one grouped inner expression and consume the closing `)`.
    fn eat_parenthesized_inner_expression(
        &mut self,
        preserve_type_context: bool,
    ) -> ParseResult<(LocalNodeId<Expression>, TokenType)> {
        let inner_start = self.pos();
        let mut inner_options = self.options.nested().with_parenthesis(true);
        inner_options.set_allow_sequence_expression(true);
        if preserve_type_context && self.options.is_in_type() {
            inner_options = inner_options.in_type();
        }

        let expression_id = self.eat_expression(inner_options)?;
        self.eat_newlines_maybe()?;
        self.eat_token(TokenType::CloseParenthesis)?;

        Ok((expression_id, self.token_type_at(inner_start as usize)))
    }

    /// Finish a parenthesized expression after the inner value has been parsed.
    fn finish_parenthesized_expression(
        &mut self,
        start: &ParserMark,
        expression_id: LocalNodeId<Expression>,
        inner_token_type: TokenType,
    ) -> LocalNodeId<Expression> {
        match self.tree.get(expression_id) {
            Expression::TupleExpression { .. }
                if inner_token_type != TokenType::OpenParenthesis =>
            {
                self.tree.set_span(expression_id, self.get_span_from(start));
                expression_id
            }
            Expression::SequenceExpression { .. }
                if inner_token_type != TokenType::OpenParenthesis =>
            {
                self.tree.set_span(expression_id, self.get_span_from(start));
                expression_id
            }
            _ => self.insert_node(
                Expression::Parenthesized {
                    expression: expression_id,
                },
                self.get_span_from(start),
            ),
        }
    }

    /// Try to parse a parenthesized lambda from one known group shape.
    fn try_eat_parenthesized_lambda_from_shape(
        &mut self,
        start: &ParserMark,
        group_shape: DelimiterAnalysis,
    ) -> ParseResult<Option<LocalNodeId<Expression>>> {
        let has_parenthesized_parameter_shape = group_shape.has_top_level_parameter_colon
            || group_shape.has_top_level_comma
            || group_shape.is_empty;
        let can_parse_lambda_in_arrow_return = !self.options.is_in_arrow_return_type()
            || self.options.is_in_type() && has_parenthesized_parameter_shape;
        let Some(follow_token_type) = group_shape.follow_token_type else {
            return Ok(None);
        };

        let is_colon_lambda = follow_token_type == TokenType::Colon;
        let is_colon_lambda_allowed = !is_colon_lambda
            || (self.language.is_destack() || self.language.is_typescript())
                && !self.options.is_in_before_type()
                && !self.options.is_in_match_case()
                && (!self.options.is_in_type() || !group_shape.has_top_level_comma);
        if !can_parse_lambda_in_arrow_return || !is_colon_lambda_allowed {
            return Ok(None);
        }

        if is_colon_lambda && self.options.is_in_ternary_condition() {
            let speculative_start = self.mark();
            let speculative_start_idx = self.tree.next_id();
            if let Ok(lambda_id) =
                self.eat_function(start, DeclarationDescriptor::default(), false, false)
            {
                let has_ternary_delimiter = self.peek_is(TokenType::Colon)
                    || self.is_token_after_newlines(self.pos(), TokenType::Colon);
                let should_accept = match self.tree.get(lambda_id) {
                    Declaration::Function { body, .. } => {
                        (body.is_some() || self.options.is_in_type()) && has_ternary_delimiter
                    }
                    _ => has_ternary_delimiter,
                };
                if should_accept {
                    return Ok(Some(self.insert_node(
                        Expression::Declaration(lambda_id),
                        self.get_span_from(start),
                    )));
                }
            }

            self.restore(speculative_start, speculative_start_idx);
            return Ok(None);
        }

        let lambda_id = self.eat_function(start, DeclarationDescriptor::default(), false, false)?;

        Ok(Some(self.insert_node(
            Expression::Declaration(lambda_id),
            self.get_span_from(start),
        )))
    }

    /// Parse a parenthesized primary expression from one known group shape.
    fn eat_parenthesized_primary_from_shape(
        &mut self,
        start: &ParserMark,
        group_shape: DelimiterAnalysis,
    ) -> ParseResult<LocalNodeId<Expression>> {
        let is_plain_js_or_ts_group = !self.language.is_destack()
            && !self.options.is_in_type()
            && !self.options.is_in_arrow_return_type()
            && !self.has_active_split()
            && group_shape.follow_token_type.is_none();

        // plain js and ts groups
        if let Some(speculation_stats) = self.speculation_stats.as_mut() {
            speculation_stats.parenthesized_expression_plain_calls += 1;
        }
        if is_plain_js_or_ts_group {
            if let Some(speculation_stats) = self.speculation_stats.as_mut() {
                speculation_stats.parenthesized_expression_plain_hits += 1;
            }
        } else if let Some(speculation_stats) = self.speculation_stats.as_mut() {
            speculation_stats.parenthesized_expression_plain_misses += 1;
        }

        if !is_plain_js_or_ts_group
            && let Some(lambda_expression_id) =
                self.try_eat_parenthesized_lambda_from_shape(start, group_shape)?
        {
            return Ok(lambda_expression_id);
        }

        self.bump(); // eat open parenthesis
        self.eat_newlines_maybe()?;

        // empty tuple or sequence when we immediately see a closing parenthesis
        if self.peek_is(TokenType::CloseParenthesis) {
            self.bump(); // eat closing parenthesis

            // in Destack: empty tuple
            if self.language.is_destack() {
                return Ok(self.insert_node(
                    Expression::TupleExpression { elements: vec![] },
                    self.get_span_from(start),
                ));
            }

            // in JS/TS: empty sequence expression
            return Ok(self.insert_node(
                Expression::SequenceExpression {
                    expressions: vec![],
                },
                self.get_span_from(start),
            ));
        }

        // tuple when we see a named element or top level comma
        if (self.language.is_destack()
            && self.peek_is(TokenType::Identifier)
            && self.peek_next_is(TokenType::Colon))
            || group_shape.has_top_level_comma
        {
            let tuple_elements = self
                .eat_sequence_literal_body(None, TokenType::CloseParenthesis)
                .for_node_type(NodeType::Expression)?;
            self.eat_newlines_maybe()?;
            self.eat_token(TokenType::CloseParenthesis)?;
            return Ok(self.insert_node(
                Expression::TupleExpression {
                    elements: tuple_elements,
                },
                self.get_span_from(start),
            ));
        }

        // tuple or parenthesized expression for the remaining cases
        let preserve_type_context = !is_plain_js_or_ts_group;
        let (expression_id, inner_token_type) =
            self.eat_parenthesized_inner_expression(preserve_type_context)?;

        Ok(self.finish_parenthesized_expression(start, expression_id, inner_token_type))
    }

    /// Parse an open parenthesis primary expression.
    fn eat_parenthesized_primary(
        &mut self,
        start: &ParserMark,
    ) -> ParseResult<LocalNodeId<Expression>> {
        let _group_timing = self.timing_scope(tags::PARSE_EXPRESSION_PRIMARY_GROUP);
        let group_shape = self.parenthesized_group_shape()?;
        self.eat_parenthesized_primary_from_shape(start, group_shape)
    }

    /// Eat an expression body without stack growth checks.
    fn eat_expression_inner(&mut self) -> ParseResult<LocalNodeId<Expression>> {
        // collect decorator prefixes before parsing the next expression
        let mut expression_decorators =
            if !self.options.is_in_decorator() && self.peek_is(TokenType::At) {
                self.eat_decorators_maybe()?
            } else {
                PendingDecorators::new()
            };

        // capture expression span and scanner cursor metadata
        let start = self.mark_span();

        // labelled statement or expression (like `label: while(...)` or `label: loop {}`)
        // decorators treat keywords as identifiers, so skip label parsing there
        if !self.options.is_in_decorator()
            && !self.options.is_in_match_case()
            && self.peek_is(TokenType::Identifier)
            && self.peek_next_is(TokenType::Colon)
        {
            let colon_index = self.index_for_next();
            let label_target_index = self.next_non_newline_index_from(colon_index + 1);
            let label_target_token = self.token_at(label_target_index);
            let label_target_keyword = label_target_token
                .filter(|token| token.token.ty == TokenType::Identifier)
                .and_then(|_| self.keyword_for_index(label_target_index));
            // label targets that are always expressions
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
            let can_parse_label =
                if self.options.is_in_statement_position() && !self.language.is_destack() {
                    true
                } else {
                    is_labelled_expression
                        || (self.options.is_in_statement_position() && is_labelled_block)
                };
            if can_parse_label {
                let (label, label_span) = self.eat_identifier_with_span()?;
                self.eat_colon()?;
                self.eat_newlines_maybe()?;
                // allow empty statement bodies in labelled statements
                let body = if self.peek_is(TokenType::Semicolon) {
                    let body_start = self.mark_span();
                    self.bump(); // eat semicolon
                    let block_id = self.insert_node(
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
                    self.eat_expression_in_scope()?
                };
                // reject labelled declarations that are invalid labelled items in JS/TS
                if !self.language.is_destack() && self.is_single_statement_declaration(body) {
                    return Err(ParseError::unexpected(self.tree.get_span(body)));
                }
                let labelled_id = self.insert_node(
                    Expression::Labelled { label, body },
                    self.get_span_from(&start),
                );
                self.tree.set_main_span(labelled_id, label_span);
                self.attach_pending_decorators_to_expression(
                    &mut expression_decorators,
                    labelled_id,
                );
                return Ok(labelled_id);
            }
        }

        // plain path for plain identifier type expressions
        if self.options.is_in_type()
            && self.peek_is(TokenType::Identifier)
            && let Some(identifier_expression_id) =
                self.try_parse_plain_type_identifier_expression(&start)?
        {
            self.attach_pending_decorators_to_expression(
                &mut expression_decorators,
                identifier_expression_id,
            );
            return Ok(identifier_expression_id);
        }

        // plain path for plain identifier value expressions
        // this also applies in statement position when it is not a labelled/declaration start
        if self.peek_is(TokenType::Identifier)
            && let Some(identifier_expression_id) =
                self.try_parse_plain_identifier_expression(&start)?
        {
            self.attach_pending_decorators_to_expression(
                &mut expression_decorators,
                identifier_expression_id,
            );
            return Ok(identifier_expression_id);
        }
        // ------------------------------------------------------------
        // Main expression
        // ------------------------------------------------------------
        //

        let left_expression_id: LocalNodeId<Expression> = {
            let _timing = self.timing_scope(tags::PARSE_EXPRESSION_PRIMARY);

            // resolve the current token type for primary dispatch
            let token_type = self.peek_token_type();

            //
            // ------------------------------------------------------------
            // Grouping
            // ------------------------------------------------------------
            //

            // identifier paths and keyword expressions
            match token_type {
                TokenType::Identifier => {
                    // declaration descriptor parsing only matters for identifier starts
                    let pos_index = self.pos_index();
                    let has_active_split = self.has_active_split();
                    let descriptor_head_keyword = if has_active_split {
                        self.peek_any_keyword().ok()
                    } else {
                        self.keyword_for_index(pos_index)
                    };

                    let can_parse_declaration_descriptor = self.options.is_in_statement_position()
                        || self.options.is_in_type()
                        || self.options.is_in_variant()
                        || self.options.is_in_declare_context()
                        || self.language.is_declaration()
                        || matches!(
                            descriptor_head_keyword,
                            Some(
                                Keyword::Export
                                    | Keyword::Declare
                                    | Keyword::Abstract
                                    | Keyword::Static
                            )
                        );
                    let descriptor = if can_parse_declaration_descriptor
                        && self.should_parse_declaration_descriptor()
                    {
                        match self.eat_declaration_descriptor(&start)? {
                            DescriptorHead::Descriptor {
                                descriptor,
                                mut decorators,
                            } => {
                                expression_decorators.append(&mut decorators);
                                descriptor
                            }
                            DescriptorHead::Expression(expression_id) => {
                                self.attach_pending_decorators_to_expression(
                                    &mut expression_decorators,
                                    expression_id,
                                );
                                return Ok(expression_id);
                            }
                        }
                    } else {
                        DeclarationDescriptor::default()
                    };

                    // identifier context setup
                    let pos_index = self.pos_index();
                    let next_raw_index = self.index_for_next();
                    let next_raw_token_type = self.token_type_at(next_raw_index);
                    let next_cursor = self.scanner_cursor_from(next_raw_index);
                    let next_token_type = next_cursor.token_type;
                    let next_token_index = next_cursor.index;
                    let next_has_line_break = next_cursor.has_line_break_before;
                    let is_declaration_start = DECLARATION_START_TOKENS.contains(&next_token_type);
                    let has_active_split = self.has_active_split();
                    let module_identifier_matches = !has_active_split
                        && !self.options.is_in_decorator()
                        && !self.options.is_in_type()
                        && !next_has_line_break
                        && is_declaration_start
                        && self.language.supports_module_declaration()
                        && self.is_module_identifier_at(pos_index);
                    let is_module_declaration_start = if module_identifier_matches {
                        let is_module_name_start =
                            matches!(next_token_type, TokenType::Identifier | TokenType::Literal);
                        let next_keyword = if next_token_type == TokenType::Identifier {
                            self.keyword_for_index(next_token_index)
                        } else {
                            None
                        };
                        is_module_name_start && !is_type_relation_keyword(next_keyword)
                    } else {
                        false
                    };

                    let mut primary_expression_id = None;

                    // shorthand lambda function value
                    if !self.options.is_in_type()
                        && !self.options.is_in_match_case()
                        && (next_raw_token_type == TokenType::Arrow
                            || next_raw_token_type == TokenType::ArrowWide)
                    {
                        let lambda_id = self.eat_function(&start, descriptor, false, false)?;
                        primary_expression_id = Some(self.insert_node(
                            Expression::Declaration(lambda_id),
                            self.get_span_from(&start),
                        ));
                    }

                    // keyword and split state
                    let keyword = {
                        let keyword = if has_active_split {
                            self.peek_any_keyword().ok()
                        } else {
                            self.keyword_for_index(self.pos_index())
                        };
                        if self.options.is_in_decorator() && !self.options.is_in_type() {
                            match keyword {
                                Some(
                                    Keyword::Async
                                    | Keyword::Await
                                    | Keyword::This
                                    | Keyword::New
                                    | Keyword::Delete
                                    | Keyword::Function
                                    | Keyword::Class
                                    | Keyword::Typeof
                                    | Keyword::Void,
                                ) => keyword,
                                _ => None,
                            }
                        } else if self.options.is_in_typeof_query() {
                            if matches!(keyword, Some(Keyword::Type | Keyword::Readonly)) {
                                None
                            } else {
                                keyword
                            }
                        } else {
                            keyword
                        }
                    };
                    let is_unary_keyword = matches!(keyword, Some(Keyword::Typeof | Keyword::Void));
                    let is_type_unary_keyword =
                        matches!(keyword, Some(Keyword::Typeof | Keyword::Keyof));

                    // plain path for plain identifiers
                    if primary_expression_id.is_none()
                        && keyword.is_none()
                        && !has_active_split
                        && !self.options.is_in_decorator()
                        && !is_module_declaration_start
                    {
                        // prefer contextual type literals when in type or static positions
                        let should_try_type_literal = self.should_try_contextual_type_literal();
                        if should_try_type_literal
                            && let Ok(type_literal) = self.peek_type_literal()
                        {
                            let _literal_timing =
                                self.timing_scope(tags::PARSE_EXPRESSION_PRIMARY_LITERAL);
                            let type_literal = self.eat_type_literal(Some(type_literal))?;
                            primary_expression_id = Some(self.insert_node(
                                Expression::TypeLiteral(type_literal),
                                self.get_span_from(&start),
                            ));
                        } else {
                            // fall back to an identifier path
                            primary_expression_id =
                                Some(self.eat_identifier_expression_path(&start)?);
                        }
                    }

                    // keyword-specific expression parsing
                    if primary_expression_id.is_none() {
                        // unary prefix operations
                        if is_unary_keyword && !self.options.is_in_type() {
                            let operator = match keyword {
                                Some(Keyword::Typeof) => UnaryOperator::Typeof,
                                Some(Keyword::Void) => UnaryOperator::Void,
                                _ => unreachable!(),
                            };
                            let operator_start = self.mark_span();
                            self.bump(); // eat unary operator (always because right associative)
                            self.eat_newlines_maybe()?;
                            let operator_span = self.get_span_from(&operator_start);
                            let mut right_options = self
                                .options
                                .not_in_position()
                                .in_left_precedence(operator.precedence());
                            if self.options.is_in_type_conditional_right() {
                                right_options = right_options.in_type_conditional_right();
                            }
                            let right =
                                self.eat_expression_with_context_unchecked(right_options)?;

                            // unparenthesized arrow functions are not unary operands
                            if self.is_unparenthesized_lambda_expression(right) {
                                return Err(ParseError::unexpected(self.tree.get_span(right)));
                            }

                            let expression = Expression::Unary { operator, right };
                            let expression_id =
                                self.insert_node(expression, self.get_span_from(&start));
                            self.tree.set_main_span(expression_id, operator_span);
                            primary_expression_id = Some(expression_id);
                        }

                        // type unary operations
                        if primary_expression_id.is_none() && is_type_unary_keyword {
                            let operator = match keyword {
                                Some(Keyword::Typeof) => TypeUnaryOperator::Typeof,
                                Some(Keyword::Keyof) => TypeUnaryOperator::Keyof,
                                _ => unreachable!(),
                            };
                            let operator_start = self.mark_span();
                            self.bump(); // eat type unary operator (always because right associative)
                            self.eat_newlines_maybe()?;
                            let operator_span = self.get_span_from(&operator_start);
                            let mut right_expression_context = self
                                .options
                                .not_in_position()
                                .in_left_precedence(operator.precedence());
                            let right_ambient_context = self
                                .options
                                .with_type(true)
                                .with_typeof_query(operator == TypeUnaryOperator::Typeof);

                            // parse typeof targets with contextual keyword tolerance
                            if self.options.is_in_type_conditional_right() {
                                right_expression_context =
                                    right_expression_context.in_type_conditional_right();
                            }
                            let right = self.eat_expression(
                                self.options
                                    .with_ambient_context(right_ambient_context)
                                    .with_expression_context(right_expression_context),
                            )?;
                            let expression = Expression::TypeUnary { operator, right };
                            let expression_id =
                                self.insert_node(expression, self.get_span_from(&start));
                            self.tree.set_main_span(expression_id, operator_span);
                            primary_expression_id = Some(expression_id);
                        }

                        // do block expression or do-while block
                        if primary_expression_id.is_none() && keyword == Some(Keyword::Do) {
                            if self.is_do_while_statement(next_token_type) {
                                primary_expression_id = Some(self.eat_while()?);
                            } else if next_token_type == TokenType::OpenBrace {
                                let block_id = self.eat_block(BlockContext::Expression)?;
                                primary_expression_id = Some(self.insert_node(
                                    Expression::Block(block_id),
                                    self.get_span_from(&start),
                                ));
                            }
                        }

                        // keyword or contextual module declaration
                        if primary_expression_id.is_none()
                            && keyword.is_none()
                            && is_module_declaration_start
                        {
                            // parse contextual module declarations after other identifier paths
                            let namespace_id = self.eat_namespace(&start, descriptor)?;
                            primary_expression_id = Some(self.insert_node(
                                Expression::Declaration(namespace_id),
                                self.get_span_from(&start),
                            ));
                        }

                        if primary_expression_id.is_none()
                            && let Some(keyword) = keyword
                        {
                            // parse keyword expressions and declaration starters
                            let _keyword_timing =
                                self.timing_scope(tags::PARSE_EXPRESSION_PRIMARY_KEYWORD);
                            if let Some(keyword_expression_id) = self.eat_keyword_expression(
                                &start,
                                descriptor,
                                keyword,
                                next_token_type,
                                next_token_index,
                                next_has_line_break,
                                next_raw_token_type,
                                is_declaration_start,
                            )? {
                                primary_expression_id = Some(keyword_expression_id);
                            }
                        }

                        // fallback to contextual type literals when keyword parsing did not match
                        if primary_expression_id.is_none() {
                            let should_try_type_literal = self.should_try_contextual_type_literal();
                            if should_try_type_literal
                                && let Ok(type_literal) = self.peek_type_literal()
                            {
                                let _literal_timing =
                                    self.timing_scope(tags::PARSE_EXPRESSION_PRIMARY_LITERAL);
                                let type_literal = self.eat_type_literal(Some(type_literal))?;
                                primary_expression_id = Some(self.insert_node(
                                    Expression::TypeLiteral(type_literal),
                                    self.get_span_from(&start),
                                ));
                            }
                        }
                    }

                    if let Some(primary_expression_id) = primary_expression_id {
                        primary_expression_id
                    } else {
                        // final fallback for identifier paths
                        self.eat_identifier_expression_path(&start)?
                    }
                }
                _ => {
                    // eat leading elementwise operator
                    if token_type == TokenType::ElementwiseOr
                        || token_type == TokenType::ElementwiseAnd && !self.language.is_destack()
                    {
                        self.bump(); // eat elementwise operator
                        if self.options.is_in_type() {
                            self.eat_newlines_maybe()?;
                        }
                        let leading_binary_operator = match token_type {
                            TokenType::ElementwiseOr => BinaryOperator::ElementwiseOr,
                            TokenType::ElementwiseAnd => BinaryOperator::ElementwiseAnd,
                            _ => unreachable!(),
                        };

                        // eat expression
                        let expression_id = self.eat_expression_in_scope()?;

                        // allow leading elementwise operators in type expressions
                        let expression = self.tree.get(expression_id);
                        if !matches!(
                            expression,
                            Expression::Binary { operator, .. }
                                if *operator == leading_binary_operator
                        ) && !self.options.is_in_type()
                        {
                            return Err(ParseError::unexpected(self.get_span_from(&start)));
                        }

                        // expand span
                        self.tree
                            .set_span(expression_id, self.get_span_from(&start));

                        // forward the expression (no need to parse further here)
                        self.attach_pending_decorators_to_expression(
                            &mut expression_decorators,
                            expression_id,
                        );
                        return Ok(expression_id);
                    }
                    // parenthesis
                    // may be tuple, lambda, or parenthesized expression
                    else if token_type == TokenType::OpenParenthesis {
                        self.eat_parenthesized_primary(&start)?
                    }
                    //
                    // ------------------------------------------------------------
                    // Unary operations (prefix, right associative)
                    // ------------------------------------------------------------
                    //

                    // pointer types
                    else if self.options.is_in_type() && self.peek_is(TokenType::Multiply) {
                        self.bump(); // eat *
                        let mutability = self.eat_reference_mutability_maybe()?;
                        let right = self.eat_expression_with_context_unchecked(
                            self.options.not_in_position(),
                        )?;
                        let expression = Expression::PointerOf { mutability, right };
                        self.insert_node(expression, self.get_span_from(&start))
                    }
                    // unary prefix operations
                    else if let Some(operator) = self.peek_unary_prefix_operator_maybe() {
                        let operator_start = self.mark_span();
                        self.bump(); // eat unary operator (always because right associative)
                        self.eat_newlines_maybe()?;
                        let operator_span = self.get_span_from(&operator_start);
                        let mut right_options = self
                            .options
                            .not_in_position()
                            .in_left_precedence(operator.precedence());
                        if self.options.is_in_type_conditional_right() {
                            right_options = right_options.in_type_conditional_right();
                        }
                        let right = self.eat_expression_with_context_unchecked(right_options)?;
                        let expression = Expression::Unary { operator, right };
                        let expression_id =
                            self.insert_node(expression, self.get_span_from(&start));
                        self.tree.set_main_span(expression_id, operator_span);
                        expression_id
                    }
                    // type unary operations
                    else if let Some(operator) = self.peek_type_unary_prefix_operator_maybe() {
                        let operator_start = self.mark_span();
                        self.bump(); // eat type unary operator (always because right associative)
                        self.eat_newlines_maybe()?;
                        let operator_span = self.get_span_from(&operator_start);
                        let mut right_options = self
                            .options
                            .not_in_position()
                            .in_left_precedence(operator.precedence());
                        if self.options.is_in_type_conditional_right() {
                            right_options = right_options.in_type_conditional_right();
                        }
                        let right = self.eat_expression(
                            self.options
                                .with_type(true)
                                .with_expression_context(right_options),
                        )?;
                        let expression = Expression::TypeUnary { operator, right };
                        let expression_id =
                            self.insert_node(expression, self.get_span_from(&start));
                        self.tree.set_main_span(expression_id, operator_span);
                        expression_id
                    }
                    // value (`^` or `^readonly` or `^T`)
                    else if self.peek_is(TokenType::ElementwiseXor) && self.language.is_destack()
                    {
                        self.bump(); // eat ^
                        let mutability = self.eat_reference_mutability_maybe()?;
                        let variance = self.eat_variance_bound_maybe()?;
                        let right = self.eat_expression_with_context_unchecked(
                            self.options.not_in_position(),
                        )?;
                        let expression = Expression::ValueOf {
                            mutability,
                            variance,
                            right,
                        };
                        self.insert_node(expression, self.get_span_from(&start))
                    }
                    // reference (`&` or `&var` or `&T`)
                    else if self.peek_is(TokenType::ElementwiseAnd) && self.language.is_destack()
                    {
                        self.bump(); // eat &
                        let mutability = self.eat_reference_mutability_maybe()?;
                        let variance = self.eat_variance_bound_maybe()?;
                        let right = self.eat_expression_with_context_unchecked(
                            self.options.not_in_position(),
                        )?;
                        let expression = Expression::ReferenceOf {
                            mutability,
                            variance,
                            right,
                        };
                        self.insert_node(expression, self.get_span_from(&start))
                    }
                    //
                    // ------------------------------------------------------------
                    // Literals / Aliases / Values
                    // ------------------------------------------------------------
                    //
                    // array literal
                    else if token_type == TokenType::OpenBracket {
                        let elements = self
                            .with_options(self.options.not_in_position(), |parser| {
                                parser.eat_array_literal()
                            })?;
                        self.insert_node(
                            Expression::ArrayExpression { elements },
                            self.get_span_from(&start),
                        )
                    }
                    // object literal
                    else if token_type == TokenType::OpenBrace
                        && (!self.options.is_in_statement_position()
                            || self.can_parse_object_literal_in_statement_position())
                    {
                        if self.options.is_in_type() && self.can_start_type_mapped_expression() {
                            self.eat_type_mapped_expression()?
                        } else {
                            let properties = self
                                .with_options(self.options.not_in_position(), |parser| {
                                    parser.eat_object_literal()
                                })?;
                            self.insert_node(
                                Expression::ObjectExpression {
                                    ty: None,
                                    properties,
                                },
                                self.get_span_from(&start),
                            )
                        }
                    }
                    // block
                    else if self.is_block_start() {
                        let block_id = self.eat_block(BlockContext::Expression)?;
                        self.tree
                            .insert(Expression::Block(block_id), self.get_span_from(&start))
                    }
                    // statically parameterized lambda: <T>(...) or <T,>(...)
                    // (also handles multiline in type context: `<\nT\n>(...) => ...`)
                    else if token_type == TokenType::LessThan
                        && self.can_start_generic_arrow_expression()
                    {
                        let function_id = self.eat_function(
                            &start,
                            DeclarationDescriptor::default(),
                            false,
                            false,
                        )?;
                        self.insert_node(
                            Expression::Declaration(function_id),
                            self.get_span_from(&start),
                        )
                    }
                    // typescript angle bracket type assertion
                    else if token_type == TokenType::LessThan
                        && self.language.is_typescript()
                        && !self.language.supports_jsx()
                        && !self.options.is_in_type()
                        && !self.options.is_in_new_receiver()
                        && !self.options.is_disallow_ambiguous_tree_literal()
                    {
                        self.eat_type_assertion_expression(&start)?
                    }
                    // tree literal
                    else if token_type == TokenType::LessThan && self.can_start_tree_literal() {
                        self.with_options(self.options.not_in_position(), |parser| {
                            parser.eat_tree_literal()
                        })?
                    }
                    // template literal
                    else if self.is_template_literal_start() {
                        let _literal_timing =
                            self.timing_scope(tags::PARSE_EXPRESSION_PRIMARY_LITERAL);
                        if self.options.is_in_type() {
                            self.eat_type_template_literal_expression()?
                        } else {
                            let template_literal = self.eat_template_literal()?;
                            self.insert_node(
                                Expression::TemplateExpression {
                                    value: template_literal,
                                },
                                self.get_span_from(&start),
                            )
                        }
                    }
                    // scalar literal
                    else if self.is_scalar_literal_start() {
                        let _literal_timing =
                            self.timing_scope(tags::PARSE_EXPRESSION_PRIMARY_LITERAL);
                        let scalar_literal = self.eat_scalar_literal()?;
                        self.insert_node(
                            Expression::ScalarLiteral(scalar_literal),
                            self.get_span_from(&start),
                        )
                    }
                    // type literal
                    // (type literals are contextual, most are only parsed inside type context to avoid shadowing)
                    else if token_type == TokenType::Not
                        && let Ok(type_literal) = self.peek_type_literal()
                    {
                        let _literal_timing =
                            self.timing_scope(tags::PARSE_EXPRESSION_PRIMARY_LITERAL);
                        let type_literal = self.eat_type_literal(Some(type_literal))?;
                        self.insert_node(
                            Expression::TypeLiteral(type_literal),
                            self.get_span_from(&start),
                        )
                    }
                    // private identifier
                    else if token_type == TokenType::Hash
                        && self.peek_next_is(TokenType::Identifier)
                    {
                        // require the hash and identifier to be adjacent
                        let hash_index = self.pos_index();
                        let ident_index = hash_index + 1;
                        self.check_tokens_are_adjacent(hash_index, ident_index)?;

                        self.bump(); // eat #
                        let (name, name_span) = self.eat_identifier_with_span()?;
                        let expression_id = self.insert_node(
                            Expression::PrivateIdentifier { name },
                            self.get_span_from(&start),
                        );
                        self.tree.set_main_span(expression_id, name_span);
                        expression_id
                    }
                    //
                    // ------------------------------------------------------------
                    // Error
                    // ------------------------------------------------------------
                    //
                    else {
                        return Err(ParseError::unexpected(self.peek()?.span));
                    }
                }
            }
        };

        // attach parsed decorators before continuation parsing
        self.attach_pending_decorators_to_expression(
            &mut expression_decorators,
            left_expression_id,
        );

        // parse postfix and infix continuation for the primary expression
        let expression_id = self.eat_expression_continuation(&start, left_expression_id)?;
        Ok(expression_id)
    }

    /// Attach pending decorators to the best structural target node.
    fn attach_pending_decorators_to_expression(
        &mut self,
        decorators: &mut PendingDecorators,
        expression_id: LocalNodeId<Expression>,
    ) {
        if decorators.is_empty() {
            return;
        }

        let target_node_id = self.decorator_target_node_id(expression_id);
        self.attach_decorators(target_node_id, std::mem::take(decorators));
    }

    /// Return the structural node id that should own prefix decorators.
    fn decorator_target_node_id(&self, expression_id: LocalNodeId<Expression>) -> u32 {
        let mut current_expression_id = expression_id;

        loop {
            let expression = self.tree.get(current_expression_id);
            match expression {
                // declaration expressions bind decorators to the declaration node itself
                Expression::Declaration(declaration_id) => {
                    return declaration_id.id;
                }

                // statement wrappers around declarations are transparent for decorator ownership
                Expression::Statement(inner_expression_id)
                    if matches!(
                        self.tree.get(*inner_expression_id),
                        Expression::Declaration(_)
                    ) =>
                {
                    current_expression_id = *inner_expression_id;
                }

                // export wrappers forward decorator ownership to the exported declaration expression
                Expression::Export { items, .. } => {
                    let mut exported_declaration_expression = None;
                    for item_id in items {
                        let item = self.tree.get(*item_id);
                        let Some(value_expression_id) = item.value else {
                            continue;
                        };
                        if matches!(
                            self.tree.get(value_expression_id),
                            Expression::Declaration(_)
                        ) {
                            exported_declaration_expression = Some(value_expression_id);
                            break;
                        }
                    }

                    if let Some(exported_declaration_expression) = exported_declaration_expression {
                        current_expression_id = exported_declaration_expression;
                    } else {
                        return current_expression_id.id;
                    }
                }

                // all other expressions own their decorators directly
                _ => {
                    return current_expression_id.id;
                }
            }
        }
    }
}
