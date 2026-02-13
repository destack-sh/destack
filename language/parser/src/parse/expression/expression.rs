use super::common::{DECLARATION_START_TOKENS, DescriptorHead, is_type_relation_keyword};
use super::lookahead::ParenthesizedGroupShape;
use crate::parse::prelude::*;
use crate::{ParseError, ParseResult, Parser, ParserMark};

use destack_ast::{
    BinaryOperator, Block, BlockFormat, Declaration, DeclarationDescriptor, Decorator, Expression,
    Keyword, LocalNodeId, NodeType, TokenType, TypeUnaryOperator, UnaryOperator,
};

/// The recursion interval for stack growth checks in expression parsing.
const STACK_GROW_CHECK_INTERVAL: u32 = 256;

impl Parser {
    #[inline(always)]
    pub fn eat_expression(
        &mut self,
        options: ParserOptions,
    ) -> ParseResult<LocalNodeId<Expression>> {
        if self.options == options {
            return self.eat_expression_inner_with_stack_guard();
        }

        let old_options = self.options;
        self.options = options;
        let result = self.eat_expression_inner_with_stack_guard();
        self.options = old_options;
        result
    }

    /// Eat an expression in the current parser options.
    #[inline(always)]
    fn eat_expression_in_current_options(&mut self) -> ParseResult<LocalNodeId<Expression>> {
        self.eat_expression_inner_with_stack_guard()
    }

    /// Eat an expression after statement keyword dispatch already ran in the caller.
    #[inline(always)]
    pub(crate) fn eat_expression_without_statement_keyword_fast(
        &mut self,
    ) -> ParseResult<LocalNodeId<Expression>> {
        self.eat_expression_inner_with_stack_guard()
    }

    /// Eat an expression with statement position temporarily disabled.
    #[inline]
    pub(crate) fn eat_expression_without_statement_position_fast(
        &mut self,
    ) -> ParseResult<LocalNodeId<Expression>> {
        if !self.options.in_statement_position {
            return self.eat_expression_inner_with_stack_guard();
        }

        let old_in_statement_position = self.options.in_statement_position;
        self.options.in_statement_position = false;
        let result = self.eat_expression_inner_with_stack_guard();
        self.options.in_statement_position = old_in_statement_position;
        result
    }

    /// Eat an expression with stack growth checks.
    #[inline(always)]
    fn eat_expression_inner_with_stack_guard(&mut self) -> ParseResult<LocalNodeId<Expression>> {
        let _timing = self.timing_scope(tags::PARSE_EXPRESSION);
        let depth = self.expression_stack_depth;
        self.expression_stack_depth = depth + 1;
        let should_check_stack = (depth & (STACK_GROW_CHECK_INTERVAL - 1)) == 0;
        let result = if should_check_stack {
            destack_base::ensure_sufficient_stack(|| self.eat_expression_inner())
        } else {
            self.eat_expression_inner()
        };
        self.expression_stack_depth = depth;
        result
    }

    /// Try to eat an expression and recover to an error node.
    pub fn try_eat_expression(
        &mut self,
        recover: TokenType,
    ) -> ParseResult<LocalNodeId<Expression>> {
        match self.eat_expression_in_current_options() {
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

    /// Return the contextual keyword at the current identifier with split awareness.
    #[inline]
    fn current_identifier_keyword(&mut self, has_active_split: bool) -> Option<Keyword> {
        if has_active_split {
            self.peek_any_keyword().ok()
        } else {
            self.keyword_for_index_maybe_fast(self.pos_index())
        }
    }

    /// Return the contextual keyword allowed in decorator expression positions.
    #[inline]
    fn current_decorator_keyword(&mut self, has_active_split: bool) -> Option<Keyword> {
        let keyword = self.current_identifier_keyword(has_active_split);
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
    }

    /// Return the contextual keyword allowed inside `typeof` type queries.
    #[inline]
    fn current_typeof_query_keyword(&mut self, has_active_split: bool) -> Option<Keyword> {
        let keyword = self.current_identifier_keyword(has_active_split);
        if matches!(keyword, Some(Keyword::Type | Keyword::Readonly)) {
            None
        } else {
            keyword
        }
    }

    /// Try to parse a plain identifier expression and continuation in common value contexts.
    pub(crate) fn try_eat_plain_identifier_expression_fast(
        &mut self,
        start: &ParserMark,
    ) -> ParseResult<Option<LocalNodeId<Expression>>> {
        if self.options.in_type
            || self.options.in_match_case
            || self.options.in_decorator
            || self.options.in_typeof_query
        {
            return Ok(None);
        }

        if !self.peek_is(TokenType::Identifier) {
            return Ok(None);
        }

        let pos_index = self.pos_index();

        if self.has_active_split() {
            return Ok(None);
        }

        let next_raw_token_type = self.peek_next_token_type();
        if matches!(next_raw_token_type, TokenType::Arrow | TokenType::ArrowWide) {
            return Ok(None);
        }
        // labelled statements need the full expression entry path
        if self.options.in_statement_position && next_raw_token_type == TokenType::Colon {
            return Ok(None);
        }

        let next_cursor = self.non_newline_cursor_from(self.index_for_next());
        let next_token_type = next_cursor.token_type;
        let next_token_index = next_cursor.index;

        if self.keyword_for_index_maybe_fast(pos_index).is_some() {
            return Ok(None);
        }

        if self.should_try_contextual_type_literal() {
            return Ok(None);
        }
        // contextual global declarations need descriptor parsing even in non statement contexts
        let can_start_global_declaration = matches!(
            next_token_type,
            TokenType::OpenBrace | TokenType::Identifier | TokenType::Literal
        );
        if can_start_global_declaration && self.is_global_identifier_at(pos_index) {
            return Ok(None);
        }
        let is_module_declaration_start = if self.language.supports_module_declaration()
            && !next_cursor.has_line_break_before
            && DECLARATION_START_TOKENS.contains(&next_token_type)
            && self.is_module_identifier_at(pos_index)
        {
            let is_module_name_start =
                matches!(next_token_type, TokenType::Identifier | TokenType::Literal);
            let next_keyword = if next_token_type == TokenType::Identifier {
                self.keyword_for_index_maybe_fast(next_token_index)
            } else {
                None
            };
            is_module_name_start && !is_type_relation_keyword(next_keyword)
        } else {
            false
        };
        if is_module_declaration_start {
            return Ok(None);
        }

        let expression_cursor = self.scanner_cursor();
        let identifier_expression_id = self.eat_identifier_expression_path(start)?;
        if self.should_attach_expression_leading_annotations(
            expression_cursor.index,
            expression_cursor.has_line_break_before,
        ) {
            self.attach_inline_expression_leading_annotations_for_token(
                expression_cursor.index,
                expression_cursor.skipped_newline_count,
                identifier_expression_id.id,
            );
        }

        let expression_id = self.eat_expression_continuation(start, identifier_expression_id)?;

        Ok(Some(expression_id))
    }

    /// Try to parse a plain identifier path in type positions.
    fn try_eat_plain_type_identifier_expression_fast(
        &mut self,
        start: &ParserMark,
    ) -> ParseResult<Option<LocalNodeId<Expression>>> {
        // only run this fast path in plain type positions
        if !self.options.in_type
            || self.options.in_typeof_query
            || self.options.in_decorator
            || self.options.in_match_case
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
        let keyword = self.keyword_for_index_maybe_fast(self.pos_index());
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

        let expression_cursor = self.scanner_cursor();
        let identifier_expression_id = self.eat_identifier_expression_path(start)?;
        if self.should_attach_expression_leading_annotations(
            expression_cursor.index,
            expression_cursor.has_line_break_before,
        ) {
            self.attach_inline_expression_leading_annotations_for_token(
                expression_cursor.index,
                expression_cursor.skipped_newline_count,
                identifier_expression_id.id,
            );
        }

        let expression_id = self.eat_expression_continuation(start, identifier_expression_id)?;

        Ok(Some(expression_id))
    }

    /// Return true when the current identifier text matches a contextual type literal.
    #[inline]
    fn is_contextual_type_literal_identifier(&mut self) -> bool {
        if !self.peek_is(TokenType::Identifier) {
            return false;
        }

        let Some(identifier) = self.identifier_for_index(self.pos_index()) else {
            return false;
        };
        let type_identifiers = &self.type_literal_identifiers;

        matches!(
            identifier,
            id if id == type_identifiers.undefined
                || id == type_identifiers.unknown
                || id == type_identifiers.object
                || id == type_identifiers.null_
                || id == type_identifiers.any
                || id == type_identifiers.never
                || id == type_identifiers.boolean
                || id == type_identifiers.void
                || id == type_identifiers.character
                || id == type_identifiers.string
                || id == type_identifiers.bigint
                || id == type_identifiers.number
                || id == type_identifiers.int
                || id == type_identifiers.isize
                || id == type_identifiers.uint
                || id == type_identifiers.usize
                || id == type_identifiers.float
                || id == type_identifiers.symbol
                || id == type_identifiers.unique
        )
    }

    /// Return true when the current identifier should be parsed as a contextual type literal.
    #[inline]
    fn should_try_contextual_type_literal(&mut self) -> bool {
        if self.options.in_type || self.options.in_static {
            return true;
        }

        // contextual type literals in value positions are a destack only extension
        if !self.language.is_destack() {
            return false;
        }

        self.is_contextual_type_literal_identifier()
    }

    /// Try to parse a plain parenthesized expression without lambda lookahead.
    fn try_eat_parenthesized_expression_fast(
        &mut self,
        start: &ParserMark,
        group_shape: ParenthesizedGroupShape,
    ) -> ParseResult<Option<LocalNodeId<Expression>>> {
        if let Some(speculation_stats) = self.speculation_stats.as_mut() {
            speculation_stats.parenthesized_expression_fast_calls += 1;
        }

        // this fast path only applies to JS and TS value contexts
        if self.language.is_destack()
            || self.options.in_type
            || self.options.in_arrow_return_type
            || self.has_active_split()
        {
            if let Some(speculation_stats) = self.speculation_stats.as_mut() {
                speculation_stats.parenthesized_expression_fast_misses += 1;
            }
            return Ok(None);
        }

        // lambda and typed-lambda forms still need full lookahead handling
        if group_shape.has_arrow_follow || group_shape.has_colon_follow {
            if let Some(speculation_stats) = self.speculation_stats.as_mut() {
                speculation_stats.parenthesized_expression_fast_misses += 1;
            }
            return Ok(None);
        }

        // parse the grouped expression directly
        self.bump(); // eat open parenthesis
        self.eat_newlines_maybe()?;

        // js and ts: empty sequence expression
        if self.peek_is(TokenType::CloseParenthesis) {
            self.bump(); // eat closing parenthesis
            if let Some(speculation_stats) = self.speculation_stats.as_mut() {
                speculation_stats.parenthesized_expression_fast_hits += 1;
            }
            return Ok(Some(self.tree.insert(
                Expression::SequenceExpression {
                    expressions: vec![],
                },
                self.get_span_from(start),
            )));
        }

        // parse the grouped expression body
        let inner_start = self.pos();
        let mut inner_options = self.options.nested().in_parenthesis();
        inner_options.allow_sequence_expression = true;
        let expression_id = self.eat_expression(inner_options)?;
        self.eat_newlines_maybe()?;
        self.eat_token(TokenType::CloseParenthesis)?;

        // preserve tuple and sequence spans when nested expressions already produced them
        let inner_token_type = self.token_type_at(inner_start as usize);
        let expression_id = match self.tree.get(expression_id) {
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
            _ => self.tree.insert(
                Expression::Parenthesized {
                    expression: expression_id,
                },
                self.get_span_from(start),
            ),
        };

        if let Some(speculation_stats) = self.speculation_stats.as_mut() {
            speculation_stats.parenthesized_expression_fast_hits += 1;
        }

        Ok(Some(expression_id))
    }

    /// Parse a parenthesized primary expression using precomputed group metadata.
    fn eat_parenthesized_primary_from_shape(
        &mut self,
        start: &ParserMark,
        group_shape: ParenthesizedGroupShape,
    ) -> ParseResult<LocalNodeId<Expression>> {
        // fast path for non-lambda grouped expressions
        if let Some(group_expression_id) =
            self.try_eat_parenthesized_expression_fast(start, group_shape)?
        {
            return Ok(group_expression_id);
        }

        let has_top_level_comma = group_shape.has_top_level_comma;
        let mut lambda_expression_id = None;

        // parse direct arrow lambdas without deep group lookahead
        if !self.options.in_arrow_return_type && group_shape.has_arrow_follow {
            let lambda_id =
                self.eat_function(start, DeclarationDescriptor::default(), false, false)?;
            lambda_expression_id = Some(self.tree.insert(
                Expression::Declaration(lambda_id),
                self.get_span_from(start),
            ));
        }

        // look ahead for colon lambdas and tuple cues when needed
        if lambda_expression_id.is_none() {
            let ParenthesizedGroupShape {
                has_arrow_follow,
                has_colon_follow,
                has_top_level_parameter_colon,
                is_empty: is_empty_parenthesized_group,
                ..
            } = group_shape;
            let has_parenthesized_parameter_shape = has_top_level_parameter_colon
                || has_top_level_comma
                || is_empty_parenthesized_group;
            let is_colon_lambda_allowed = has_colon_follow
                && (self.language.is_destack() || self.language.is_typescript())
                && !self.options.in_before_type
                && !self.options.in_match_case
                && (!self.options.in_type || !has_top_level_comma);
            let can_parse_lambda_in_arrow_return = !self.options.in_arrow_return_type
                || self.options.in_type && has_parenthesized_parameter_shape;

            // parse lambda when we see a likely arrow or colon
            if (has_arrow_follow || is_colon_lambda_allowed) && can_parse_lambda_in_arrow_return {
                // avoid colon lambdas that steal ternary delimiters
                if self.options.in_ternary_condition && has_colon_follow {
                    let speculative_start = self.mark();
                    let speculative_start_idx = self.tree.next_id();
                    if let Ok(lambda_id) =
                        self.eat_function(start, DeclarationDescriptor::default(), false, false)
                    {
                        let has_ternary_delimiter = self.peek_is(TokenType::Colon)
                            || self.is_token_after_newlines(self.pos(), TokenType::Colon);
                        let should_accept = match self.tree.get(lambda_id) {
                            Declaration::Function { body, .. } => {
                                (body.is_some() || self.options.in_type) && has_ternary_delimiter
                            }
                            _ => has_ternary_delimiter,
                        };
                        if should_accept {
                            lambda_expression_id = Some(self.tree.insert(
                                Expression::Declaration(lambda_id),
                                self.get_span_from(start),
                            ));
                        } else {
                            self.restore(speculative_start, speculative_start_idx);
                        }
                    } else {
                        self.restore(speculative_start, speculative_start_idx);
                    }
                } else {
                    let lambda_id =
                        self.eat_function(start, DeclarationDescriptor::default(), false, false)?;
                    lambda_expression_id = Some(self.tree.insert(
                        Expression::Declaration(lambda_id),
                        self.get_span_from(start),
                    ));
                }
            }
        }

        // tuple or parenthesized expression
        if let Some(lambda_expression_id) = lambda_expression_id {
            return Ok(lambda_expression_id);
        }

        self.bump(); // eat open parenthesis
        self.eat_newlines_maybe()?;

        // empty tuple or sequence when we immediately see a closing parenthesis
        if self.peek_is(TokenType::CloseParenthesis) {
            self.bump(); // eat closing parenthesis

            // in Destack: empty tuple
            if self.language.is_destack() {
                return Ok(self.tree.insert(
                    Expression::TupleExpression { elements: vec![] },
                    self.get_span_from(start),
                ));
            }

            // in JS/TS: empty sequence expression
            return Ok(self.tree.insert(
                Expression::SequenceExpression {
                    expressions: vec![],
                },
                self.get_span_from(start),
            ));
        }

        // tuple when we see a named element or top-level comma
        if (self.language.is_destack()
            && self.peek_is(TokenType::Identifier)
            && self.peek_next_is(TokenType::Colon))
            || has_top_level_comma
        {
            let tuple_elements = self
                .eat_sequence_literal_body(None, TokenType::CloseParenthesis)
                .for_node_type(NodeType::Expression)?;
            self.eat_newlines_maybe()?;
            self.eat_token(TokenType::CloseParenthesis)?;
            return Ok(self.tree.insert(
                Expression::TupleExpression {
                    elements: tuple_elements,
                },
                self.get_span_from(start),
            ));
        }

        // tuple or parenthesized expression for the remaining cases
        let inner_start = self.pos();
        let mut inner_options = self.options.nested().in_parenthesis();
        inner_options.allow_sequence_expression = true;
        if self.options.in_type {
            inner_options = inner_options.in_type();
        }
        let expression_id = self.eat_expression(inner_options)?;
        self.eat_newlines_maybe()?;
        self.eat_token(TokenType::CloseParenthesis)?;
        let inner_token_type = self.token_type_at(inner_start as usize);
        let expression_id = match self.tree.get(expression_id) {
            // if it was a tuple starting here, expand it to cover the entire span
            // (except if that tuple has its own parenthesis already when nesting)
            Expression::TupleExpression { .. }
                if inner_token_type != TokenType::OpenParenthesis =>
            {
                self.tree.set_span(expression_id, self.get_span_from(start));
                expression_id
            }
            // if it was a sequence expression starting here, expand it to cover the entire span
            Expression::SequenceExpression { .. }
                if inner_token_type != TokenType::OpenParenthesis =>
            {
                self.tree.set_span(expression_id, self.get_span_from(start));
                expression_id
            }
            // otherwise it was a manually parenthesized expression, wrap it
            _ => self.tree.insert(
                Expression::Parenthesized {
                    expression: expression_id,
                },
                self.get_span_from(start),
            ),
        };

        Ok(expression_id)
    }

    /// Eat an expression body without stack growth checks.
    fn eat_expression_inner(&mut self) -> ParseResult<LocalNodeId<Expression>> {
        // collect decorator prefixes before parsing the next expression
        let mut expression_decorators = if !self.options.in_decorator && self.peek_is(TokenType::At)
        {
            self.eat_decorators_prefix_collect_maybe()?
        } else {
            Vec::new()
        };

        // capture expression span and scanner cursor metadata
        let start = self.mark_span();
        let expression_cursor = self.scanner_cursor();
        let expression_token_index = expression_cursor.index;
        let expression_skipped_newline_count = expression_cursor.skipped_newline_count;

        // labelled statement or expression (like `label: while(...)` or `label: loop {}`)
        // decorators treat keywords as identifiers, so skip label parsing there
        if !self.options.in_decorator
            && !self.options.in_match_case
            && self.peek_is(TokenType::Identifier)
            && self.peek_next_is(TokenType::Colon)
        {
            let colon_index = self.index_for_next();
            let label_target_index = self.next_non_newline_index_from(colon_index + 1);
            let label_target_token = self.token_at(label_target_index);
            let label_target_keyword = label_target_token
                .filter(|token| token.token.ty == TokenType::Identifier)
                .and_then(|_| self.keyword_for_index_maybe_fast(label_target_index));
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
            let can_parse_label = if self.options.in_statement_position
                && !self.language.is_destack()
            {
                true
            } else {
                is_labelled_expression || (self.options.in_statement_position && is_labelled_block)
            };
            if can_parse_label {
                let (label, label_span) = self.eat_identifier_with_span()?;
                self.eat_colon()?;
                self.eat_newlines_maybe()?;
                // allow empty statement bodies in labelled statements
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
                    self.eat_expression_in_current_options()?
                };
                // reject labelled declarations that are invalid labelled items in JS/TS
                if !self.language.is_destack() && self.is_single_statement_declaration(body) {
                    return Err(ParseError::unexpected(self.tree.get_span(body)));
                }
                let labelled_id = self.tree.insert(
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

        // fast path for plain identifier type expressions
        if self.options.in_type
            && self.peek_is(TokenType::Identifier)
            && let Some(identifier_expression_id) =
                self.try_eat_plain_type_identifier_expression_fast(&start)?
        {
            self.attach_pending_decorators_to_expression(
                &mut expression_decorators,
                identifier_expression_id,
            );
            return Ok(identifier_expression_id);
        }

        // fast path for plain identifier value expressions
        if !self.options.in_statement_position
            && self.peek_is(TokenType::Identifier)
            && let Some(identifier_expression_id) =
                self.try_eat_plain_identifier_expression_fast(&start)?
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
                        self.keyword_for_index_maybe_fast(pos_index)
                    };
                    let can_parse_declaration_descriptor = self.options.in_statement_position
                        || self.options.in_type
                        || self.options.in_variant
                        || self.options.in_declare_context
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
                    let next_raw_token_type = self.peek_next_token_type();
                    let next_cursor = self.non_newline_cursor_from(self.index_for_next());
                    let next_token_type = next_cursor.token_type;
                    let next_token_index = next_cursor.index;
                    let next_has_line_break = next_cursor.has_line_break_before;
                    let is_declaration_start = DECLARATION_START_TOKENS.contains(&next_token_type);
                    let has_active_split = self.has_active_split();
                    let module_identifier_matches = !has_active_split
                        && !self.options.in_decorator
                        && !self.options.in_type
                        && !next_has_line_break
                        && is_declaration_start
                        && self.language.supports_module_declaration()
                        && self.is_module_identifier_at(pos_index);
                    let is_module_declaration_start = if module_identifier_matches {
                        let is_module_name_start =
                            matches!(next_token_type, TokenType::Identifier | TokenType::Literal);
                        let next_keyword = if next_token_type == TokenType::Identifier {
                            self.keyword_for_index_maybe_fast(next_token_index)
                        } else {
                            None
                        };
                        is_module_name_start && !is_type_relation_keyword(next_keyword)
                    } else {
                        false
                    };

                    let mut primary_expression_id = None;

                    // shorthand lambda function value
                    if !self.options.in_type
                        && !self.options.in_match_case
                        && (next_raw_token_type == TokenType::Arrow
                            || next_raw_token_type == TokenType::ArrowWide)
                    {
                        let lambda_id =
                            self.eat_function(&start, descriptor.clone(), false, false)?;
                        primary_expression_id = Some(self.tree.insert(
                            Expression::Declaration(lambda_id),
                            self.get_span_from(&start),
                        ));
                    }

                    // keyword and split state
                    let keyword = if self.options.in_decorator && !self.options.in_type {
                        self.current_decorator_keyword(has_active_split)
                    } else if self.options.in_typeof_query {
                        self.current_typeof_query_keyword(has_active_split)
                    } else {
                        self.current_identifier_keyword(has_active_split)
                    };
                    let is_unary_keyword = matches!(keyword, Some(Keyword::Typeof | Keyword::Void));
                    let is_type_unary_keyword =
                        matches!(keyword, Some(Keyword::Typeof | Keyword::Keyof));

                    // fast path for plain identifiers
                    if primary_expression_id.is_none()
                        && keyword.is_none()
                        && !has_active_split
                        && !self.options.in_decorator
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
                            primary_expression_id = Some(self.tree.insert(
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
                        if is_unary_keyword && !self.options.in_type {
                            let operator = match keyword {
                                Some(Keyword::Typeof) => UnaryOperator::Typeof,
                                Some(Keyword::Void) => UnaryOperator::Void,
                                _ => unreachable!(),
                            };
                            let operator_start = self.mark_span();
                            self.bump(); // eat unary operator (always because right associative)
                            let operator_span = self.get_span_from(&operator_start);
                            let mut right_options = self
                                .options
                                .not_in_position()
                                .in_left_precedence(operator.precedence());
                            if self.options.in_type_conditional_right {
                                right_options = right_options.in_type_conditional_right();
                            }
                            let right = self.eat_expression(right_options)?;

                            // unparenthesized arrow functions are not unary operands
                            if self.is_unparenthesized_lambda_expression(right) {
                                return Err(ParseError::unexpected(self.tree.get_span(right)));
                            }

                            let expression = Expression::Unary { operator, right };
                            let expression_id =
                                self.tree.insert(expression, self.get_span_from(&start));
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
                            let operator_span = self.get_span_from(&operator_start);
                            let mut right_options = self
                                .options
                                .not_in_position()
                                .in_type()
                                .in_left_precedence(operator.precedence());

                            // parse typeof targets with contextual keyword tolerance
                            if operator == TypeUnaryOperator::Typeof {
                                right_options = right_options.in_typeof_query();
                            }

                            if self.options.in_type_conditional_right {
                                right_options = right_options.in_type_conditional_right();
                            }
                            let right = self.eat_expression(right_options)?;
                            let expression = Expression::TypeUnary { operator, right };
                            let expression_id =
                                self.tree.insert(expression, self.get_span_from(&start));
                            self.tree.set_main_span(expression_id, operator_span);
                            primary_expression_id = Some(expression_id);
                        }

                        // do block expression or do-while block
                        if primary_expression_id.is_none() && keyword == Some(Keyword::Do) {
                            if self.is_do_while_statement(next_token_type) {
                                primary_expression_id = Some(self.eat_while()?);
                            } else if next_token_type == TokenType::OpenBrace {
                                let block_id = self.eat_block()?;
                                primary_expression_id = Some(self.tree.insert(
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
                            let namespace_id = self.eat_namespace(&start, descriptor.clone())?;
                            primary_expression_id = Some(self.tree.insert(
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
                                descriptor.clone(),
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
                                primary_expression_id = Some(self.tree.insert(
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
                        if self.options.in_type {
                            self.eat_newlines_maybe()?;
                        }
                        let leading_binary_operator = match token_type {
                            TokenType::ElementwiseOr => BinaryOperator::ElementwiseOr,
                            TokenType::ElementwiseAnd => BinaryOperator::ElementwiseAnd,
                            _ => unreachable!(),
                        };

                        // eat expression
                        let expression_id = self.eat_expression_in_current_options()?;

                        // allow leading elementwise operators in type expressions
                        let expression = self.tree.get(expression_id);
                        if !matches!(
                            expression,
                            Expression::Binary { operator, .. }
                                if *operator == leading_binary_operator
                        ) && !self.options.in_type
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
                        let _group_timing = self.timing_scope(tags::PARSE_EXPRESSION_PRIMARY_GROUP);

                        // fast path: in JS/TS value contexts, branch on the token after ')'
                        let can_use_follow_fast_path = !self.language.is_destack()
                            && !self.options.in_type
                            && !self.options.in_arrow_return_type
                            && !self.has_active_split();
                        if can_use_follow_fast_path
                            && let Some(follow_token_type) = self.parenthesized_follow_token_type()
                        {
                            // direct arrow after ')' means this is a lambda head
                            if matches!(follow_token_type, TokenType::Arrow | TokenType::ArrowWide)
                            {
                                let lambda_id = self.eat_function(
                                    &start,
                                    DeclarationDescriptor::default(),
                                    false,
                                    false,
                                )?;
                                self.tree.insert(
                                    Expression::Declaration(lambda_id),
                                    self.get_span_from(&start),
                                )
                            }
                            // non-colon follow cannot be a typed lambda head
                            else if follow_token_type != TokenType::Colon {
                                let group_shape = ParenthesizedGroupShape::default();
                                self.eat_parenthesized_primary_from_shape(&start, group_shape)?
                            }
                            // colon follow needs the full shape pipeline for ternary/lambda disambiguation
                            else {
                                let group_shape = self.try_lookahead_parenthesized_group_shape()?;
                                self.eat_parenthesized_primary_from_shape(&start, group_shape)?
                            }
                        } else {
                            let group_shape = self.try_lookahead_parenthesized_group_shape()?;
                            self.eat_parenthesized_primary_from_shape(&start, group_shape)?
                        }
                    }
                    //
                    // ------------------------------------------------------------
                    // Unary operations (prefix, right associative)
                    // ------------------------------------------------------------
                    //

                    // pointer types
                    else if self.options.in_type && self.peek_is(TokenType::Multiply) {
                        self.bump(); // eat *
                        let mutability = self.eat_reference_mutability_maybe()?;
                        let right = self.eat_expression(self.options.not_in_position())?;
                        let expression = Expression::PointerOf { mutability, right };
                        self.tree.insert(expression, self.get_span_from(&start))
                    }
                    // unary prefix operations
                    else if let Some(operator) = self.peek_unary_prefix_operator_maybe() {
                        let operator_start = self.mark_span();
                        self.bump(); // eat unary operator (always because right associative)
                        let operator_span = self.get_span_from(&operator_start);
                        let mut right_options = self
                            .options
                            .not_in_position()
                            .in_left_precedence(operator.precedence());
                        if self.options.in_type_conditional_right {
                            right_options = right_options.in_type_conditional_right();
                        }
                        let right = self.eat_expression(right_options)?;
                        let expression = Expression::Unary { operator, right };
                        let expression_id =
                            self.tree.insert(expression, self.get_span_from(&start));
                        self.tree.set_main_span(expression_id, operator_span);
                        expression_id
                    }
                    // type unary operations
                    else if let Some(operator) = self.peek_type_unary_prefix_operator_maybe() {
                        let operator_start = self.mark_span();
                        self.bump(); // eat type unary operator (always because right associative)
                        let operator_span = self.get_span_from(&operator_start);
                        let mut right_options = self
                            .options
                            .not_in_position()
                            .in_type()
                            .in_left_precedence(operator.precedence());
                        if self.options.in_type_conditional_right {
                            right_options = right_options.in_type_conditional_right();
                        }
                        let right = self.eat_expression(right_options)?;
                        let expression = Expression::TypeUnary { operator, right };
                        let expression_id =
                            self.tree.insert(expression, self.get_span_from(&start));
                        self.tree.set_main_span(expression_id, operator_span);
                        expression_id
                    }
                    // value (`^` or `^readonly` or `^T`)
                    else if self.peek_is(TokenType::ElementwiseXor) && self.language.is_destack()
                    {
                        self.bump(); // eat ^
                        let mutability = self.eat_reference_mutability_maybe()?;
                        let variance = self.eat_variance_bound_maybe()?;
                        let right = self.eat_expression(self.options.not_in_position())?;
                        let expression = Expression::ValueOf {
                            mutability,
                            variance,
                            right,
                        };
                        self.tree.insert(expression, self.get_span_from(&start))
                    }
                    // reference (`&` or `&var` or `&T`)
                    else if self.peek_is(TokenType::ElementwiseAnd) && self.language.is_destack()
                    {
                        self.bump(); // eat &
                        let mutability = self.eat_reference_mutability_maybe()?;
                        let variance = self.eat_variance_bound_maybe()?;
                        let right = self.eat_expression(self.options.not_in_position())?;
                        let expression = Expression::ReferenceOf {
                            mutability,
                            variance,
                            right,
                        };
                        self.tree.insert(expression, self.get_span_from(&start))
                    }
                    //
                    // ------------------------------------------------------------
                    // Literals / Aliases / Values
                    // ------------------------------------------------------------
                    //
                    // array literal
                    else if token_type == TokenType::OpenBracket {
                        let old_options = self.swap_options(self.options.not_in_position());
                        let elements = self.eat_array_literal();
                        self.restore_options(old_options);
                        let elements = elements?;
                        self.tree.insert(
                            Expression::ArrayExpression { elements },
                            self.get_span_from(&start),
                        )
                    }
                    // object literal
                    else if token_type == TokenType::OpenBrace
                        && (!self.options.in_statement_position
                            || self.can_parse_object_literal_in_statement_position())
                    {
                        if self.options.in_type && self.can_start_type_mapped_expression() {
                            self.eat_type_mapped_expression()?
                        } else {
                            let object_options = if self.options.in_type {
                                self.options.not_in_position().in_type()
                            } else {
                                self.options.not_in_position()
                            };
                            let old_options = self.swap_options(object_options);
                            let properties = self.eat_object_literal();
                            self.restore_options(old_options);
                            let properties = properties?;
                            self.tree.insert(
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
                        let block_id = self.eat_block()?;
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
                        self.tree.insert(
                            Expression::Declaration(function_id),
                            self.get_span_from(&start),
                        )
                    }
                    // typescript angle bracket type assertion
                    else if token_type == TokenType::LessThan
                        && self.language.is_typescript()
                        && !self.language.supports_jsx()
                        && !self.options.in_type
                        && !self.options.in_new_receiver
                        && !self.options.disallow_ambiguous_tree_literal
                    {
                        self.eat_type_assertion_expression(&start)?
                    }
                    // tree literal
                    else if token_type == TokenType::LessThan && self.can_start_tree_literal() {
                        let old_options = self.options;
                        self.options = self.options.not_in_position();
                        let tree_expression = self.eat_tree_literal();
                        self.options = old_options;
                        tree_expression?
                    }
                    // template literal
                    else if self.is_template_literal_start() {
                        let _literal_timing =
                            self.timing_scope(tags::PARSE_EXPRESSION_PRIMARY_LITERAL);
                        if self.options.in_type {
                            self.eat_type_template_literal_expression()?
                        } else {
                            let template_literal = self.eat_template_literal()?;
                            self.tree.insert(
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
                        self.tree.insert(
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
                        self.tree.insert(
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
                        let expression_id = self.tree.insert(
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

        // attach leading annotations from scanner context before continuation parsing
        if self.should_attach_expression_leading_annotations(
            expression_token_index,
            expression_cursor.has_line_break_before,
        ) {
            self.attach_inline_expression_leading_annotations_for_token(
                expression_token_index,
                expression_skipped_newline_count,
                left_expression_id.id,
            );
        }

        // parse postfix and infix continuation for the primary expression
        let expression_id = self.eat_expression_continuation(&start, left_expression_id)?;
        Ok(expression_id)
    }

    /// Attach pending decorators to the best expression target.
    fn attach_pending_decorators_to_expression(
        &mut self,
        decorators: &mut Vec<LocalNodeId<Decorator>>,
        expression_id: LocalNodeId<Expression>,
    ) {
        let target_expression_id = self.decorator_target_expression(expression_id);
        self.attach_decorators_to_target(std::mem::take(decorators), target_expression_id.id);
    }

    /// Return the expression target that should own prefix decorators.
    fn decorator_target_expression(
        &self,
        expression_id: LocalNodeId<Expression>,
    ) -> LocalNodeId<Expression> {
        let expression = self.tree.get(expression_id);
        let Expression::Export { items, .. } = expression else {
            return expression_id;
        };

        for item_id in items {
            let item = self.tree.get(*item_id);
            let Some(value) = item.value else {
                continue;
            };
            if matches!(self.tree.get(value), Expression::Declaration(_)) {
                return value;
            }
        }

        expression_id
    }

    /// Return whether expression-leading annotations should attach in the current context.
    fn should_attach_expression_leading_annotations(
        &mut self,
        expression_token_index: usize,
        has_line_break_before: bool,
    ) -> bool {
        // wrapper-owned contexts control expression boundary attachments directly
        if !self.options.allow_expression_leading_annotations {
            return false;
        }

        // expression contexts always permit leading attachment
        if !self.options.in_statement_position {
            return true;
        }

        // statement wrappers own leading comment attachment
        let _ = expression_token_index;
        let _ = has_line_break_before;
        false
    }
}
