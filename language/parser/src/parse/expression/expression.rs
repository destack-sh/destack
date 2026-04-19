use super::common::{
    DECLARATION_START_TOKENS, DeclarationHeader, DescriptorHead, is_type_relation_keyword,
};
use super::lookahead::ParenthesizedGroupShape;
use crate::parse::parser::ParserOptions;
use crate::parse::prelude::*;
use crate::{ParseError, ParseResult, Parser, ParserMark};
use destack_source::{NodeSpanType, Span};
use smallvec::SmallVec;

use super::operator::TypeUnaryOperator;
use destack_ast::{
    BinaryOperator, BlockContext, Declaration, DependencyItem, Expression, GenericArgument,
    Keyword, LocalNodeId, NodeType, Path, ScalarLiteral, TokenType, TypeExpression, UnaryOperator,
};

use super::super::PendingDecorators;

/// The recursion interval for stack growth checks in expression parsing.
#[cfg(not(debug_assertions))]
const STACK_GROW_CHECK_INTERVAL: u32 = 256;

/// Parsed lead for one identifier-start primary expression.
enum IdentifierPrimaryLead {
    /// Parsed declaration header prefixes.
    Header(DeclarationHeader),
    /// Parsed expression that consumed the identifier start.
    Expression(LocalNodeId<Expression>),
}

/// The normalized facts after an identifier primary head.
struct IdentifierPrimaryFacts {
    /// The raw token type immediately after the identifier.
    next_raw_token_type: TokenType,
    /// The normalized next token type after skipping newlines.
    next_token_type: TokenType,
    /// The normalized next token index after skipping newlines.
    next_token_index: usize,
    /// Whether the normalized next token starts after a line break.
    next_has_line_break: bool,
    /// Whether the normalized next token can start a declaration.
    is_declaration_start: bool,
    /// The contextual keyword at the current identifier, if any.
    keyword: Option<Keyword>,
    /// Whether the identifier starts a contextual module declaration.
    is_module_declaration_start: bool,
}

impl Parser {
    /// Insert one type expression in expression position.
    pub(crate) fn wrap_type_expression(
        &mut self,
        type_expression_id: LocalNodeId<TypeExpression>,
    ) -> LocalNodeId<Expression> {
        let expression_id = self.insert_node(
            Expression::Type {
                value: type_expression_id,
            },
            self.tree.get_span(type_expression_id),
        );

        if let Some(main_span) = self.tree.get_main_span(type_expression_id) {
            self.tree.set_main_span(expression_id, main_span);
        }

        if let Some(head_span) = self.tree.get_head_span(type_expression_id) {
            self.tree.set_head_span(expression_id, head_span);
        }

        expression_id
    }

    /// Return the wrapped type expression for one expression node.
    pub(crate) fn wrapped_type_expression_maybe(
        &self,
        expression_id: LocalNodeId<Expression>,
    ) -> Option<LocalNodeId<TypeExpression>> {
        match self.tree.get(expression_id) {
            Expression::Type { value } => Some(*value),
            _ => None,
        }
    }

    /// Return the wrapped type expression for one expression node or fail.
    pub(crate) fn expect_wrapped_type_expression(
        &self,
        expression_id: LocalNodeId<Expression>,
    ) -> ParseResult<LocalNodeId<TypeExpression>> {
        self.wrapped_type_expression_maybe(expression_id)
            .ok_or_else(|| ParseError::unexpected(self.tree.get_span(expression_id)))
    }

    /// Set path segment spans on one type expression.
    fn set_path_type_expression_spans(
        &mut self,
        type_expression_id: LocalNodeId<TypeExpression>,
        segment_spans: &[Span],
    ) {
        let first_span = *segment_spans.first().expect("path has no segments");
        let last_span = *segment_spans.last().expect("path has no segments");

        self.tree.set_main_span(type_expression_id, last_span);
        self.tree.set_head_span(type_expression_id, first_span);

        // record each path segment so semantic consumers can target the exact token
        for (index, segment_span) in segment_spans.iter().copied().enumerate() {
            let segment_index =
                u16::try_from(index).expect("path expression segment index overflow");

            self.tree.set_side_span(
                type_expression_id,
                NodeSpanType::Segment(segment_index),
                segment_span,
            );
        }
    }

    /// Eat one qualified identifier path with optional generic arguments.
    fn eat_identifier_path_with_generic_arguments(
        &mut self,
    ) -> ParseResult<(Path, SmallVec<[Span; 3]>, Vec<LocalNodeId<GenericArgument>>)> {
        // dotted reference path
        let (path, segment_spans) = self.eat_path_with_segment_spans()?;

        // optional generic arguments
        let generic_arguments =
            if self.peek_is(TokenType::LessThan) || self.peek_is(TokenType::ShiftLeft) {
                self.try_eat_generic_arguments(true, false)
                    .unwrap_or_default()
            } else {
                Vec::new()
            };

        Ok((path, segment_spans, generic_arguments))
    }

    /// Eat one qualified identifier path in type space.
    fn eat_type_identifier_expression_path(
        &mut self,
        start: &ParserMark,
    ) -> ParseResult<LocalNodeId<TypeExpression>> {
        let (path, segment_spans, generic_arguments) =
            self.eat_identifier_path_with_generic_arguments()?;

        let type_expression_id = self.insert_node(
            TypeExpression::Reference {
                path,
                generic_arguments,
            },
            self.get_span_from(start),
        );
        self.set_path_type_expression_spans(type_expression_id, &segment_spans);

        Ok(type_expression_id)
    }

    /// Eat one qualified identifier path in static space.
    fn eat_static_identifier_expression_path(
        &mut self,
        start: &ParserMark,
    ) -> ParseResult<LocalNodeId<Expression>> {
        let (path, segment_spans, generic_arguments) =
            self.eat_identifier_path_with_generic_arguments()?;

        // static space lowers to a qualified reference node
        let expression_id = self.insert_node(
            Expression::QualifiedReference {
                path,
                generic_arguments,
            },
            self.get_span_from(start),
        );
        self.set_path_expression_spans(expression_id, &segment_spans);

        Ok(expression_id)
    }

    /// Eat one value-space bare identifier expression.
    fn eat_value_identifier_expression(
        &mut self,
        start: &ParserMark,
    ) -> ParseResult<LocalNodeId<Expression>> {
        // value identifiers stay bare until continuation parsing builds the chain
        let (name, name_span) = self.eat_identifier_with_span()?;
        let expression = Expression::Identifier { name };
        let expression_id = self.insert_node(expression, self.get_span_from(start));
        self.tree.set_main_span(expression_id, name_span);

        Ok(expression_id)
    }

    /// Return whether the current identifier head could be forced into type space.
    fn identifier_head_may_need_type_space(&mut self) -> bool {
        let next_index = self.next_non_newline_index_from(self.pos_index().saturating_add(1));
        let next_token_type = self.token_type_at(next_index);

        // class and extension heads may continue through `.` or generic arguments before `extends`
        if self.options.is_in_before_block() {
            return matches!(
                next_token_type,
                TokenType::Dot | TokenType::LessThan | TokenType::ShiftLeft
            ) || next_token_type == TokenType::Identifier
                && self.keyword_for_index(next_index) == Some(Keyword::Extends);
        }

        // tagged object literal receivers may continue through `.` or generic arguments before `{`
        matches!(
            next_token_type,
            TokenType::Dot | TokenType::LessThan | TokenType::ShiftLeft | TokenType::OpenBrace
        )
    }

    /// Return whether one parsed identifier type head must stay in type space.
    fn identifier_type_head_stays_in_type_space(
        &mut self,
        type_expression_id: LocalNodeId<TypeExpression>,
    ) -> bool {
        // before-block heads stay in type space when `extends` follows
        let is_before_block_extends_head = self.options.is_in_before_block()
            && self.peek_is(TokenType::Identifier)
            && self.peek_any_keyword().ok() == Some(Keyword::Extends);

        // tagged object literal receivers also stay in type space
        let is_tagged_object_literal_head = !self.options.is_in_before_block()
            && self.peek_is(TokenType::OpenBrace)
            && self.can_start_tagged_object_literal_type(type_expression_id);

        is_before_block_extends_head || is_tagged_object_literal_head
    }

    /// Try to eat one identifier head that must lower through type space.
    fn try_eat_forced_identifier_type_expression(
        &mut self,
        start: &ParserMark,
    ) -> ParseResult<Option<LocalNodeId<Expression>>> {
        // only language modes with value-space type reentry admit this path
        if self.options.is_in_static() || !self.language.is_destack() {
            return Ok(None);
        }

        // most identifier heads cannot possibly force type space
        if !self.identifier_head_may_need_type_space() {
            return Ok(None);
        }

        // speculate the identifier head in type space
        let speculative_start = self.mark();
        let speculative_start_idx = self.tree.next_id();
        let type_expression_id = self.with_options(self.options.in_type(), |parser| {
            let type_expression_id = parser.eat_type_identifier_expression_path(start)?;

            parser.eat_type_postfix_continuation(start, type_expression_id)
        });
        let type_expression_id = match type_expression_id {
            Ok(type_expression_id) => type_expression_id,
            Err(_) => {
                self.restore(speculative_start, speculative_start_idx);
                return Ok(None);
            }
        };

        // keep only the shapes that the surrounding syntax forces into type space
        if self.identifier_type_head_stays_in_type_space(type_expression_id) {
            return Ok(Some(self.wrap_type_expression(type_expression_id)));
        }

        self.restore(speculative_start, speculative_start_idx);

        // otherwise the identifier head belongs to value space
        Ok(None)
    }

    /// Eat one identifier-led expression head.
    ///
    /// Examples:
    /// ```
    /// foo
    /// foo.bar
    /// Foo extends Bar
    /// Tag { value: 1 }
    /// ```
    pub(crate) fn eat_identifier_expression_path(
        &mut self,
        start: &ParserMark,
    ) -> ParseResult<LocalNodeId<Expression>> {
        let _identifier_timing = self.timing_scope(tags::PARSE_EXPRESSION_PRIMARY_IDENTIFIER);

        // static space always lowers to qualified references
        if self.options.is_in_static() {
            return self.eat_static_identifier_expression_path(start);
        }

        // explicit type-space heads win when the surrounding syntax requires them
        if let Some(expression_id) = self.try_eat_forced_identifier_type_expression(start)? {
            return Ok(expression_id);
        }

        // value contexts keep the bare identifier head
        self.eat_value_identifier_expression(start)
    }

    /// Eat one expression with a replacement expression-local context.
    #[inline(always)]
    pub(crate) fn eat_expression_with_context_unchecked(
        &mut self,
        context: ParserOptions,
    ) -> ParseResult<LocalNodeId<Expression>> {
        self.eat_expression(self.options.with_expression_context(context))
    }

    /// Eat one expression in one explicit parser context.
    ///
    /// Examples:
    /// ```
    /// value.member(arg)
    /// value as string
    /// value is string
    /// value ? then_value : else_value
    /// ```
    #[inline(always)]
    pub fn eat_expression(
        &mut self,
        options: ParserOptions,
    ) -> ParseResult<LocalNodeId<Expression>> {
        if self.options == options {
            return self.eat_expression_inner_with_stack_guard();
        }

        self.stats.record_with_options_call();

        let old_options = self.swap_options(options);
        let result = self.eat_expression_inner_with_stack_guard();
        self.restore_options(old_options);
        result
    }

    /// Eat one expression in the current parser options.
    ///
    /// Examples:
    /// ```
    /// value
    /// value.member(arg)
    /// value ? then_value : else_value
    /// ```
    #[inline(always)]
    pub(crate) fn eat_expression_in_scope(&mut self) -> ParseResult<LocalNodeId<Expression>> {
        self.eat_expression_inner_with_stack_guard()
    }

    /// Eat one expression with statement position temporarily disabled.
    ///
    /// Examples:
    /// ```
    /// value:
    /// value ? then_value : else_value
    /// value as string
    /// ```
    #[inline]
    pub(crate) fn eat_expression_outside_statement_position(
        &mut self,
    ) -> ParseResult<LocalNodeId<Expression>> {
        if !self.options.is_in_statement_position() {
            return self.eat_expression_inner_with_stack_guard();
        }

        let context = self.options.not_in_statement_position();
        self.stats.record_with_options_call();
        self.eat_expression(self.options.with_expression_context(context))
    }

    /// Eat an expression with stack growth checks.
    #[inline(always)]
    fn eat_expression_inner_with_stack_guard(&mut self) -> ParseResult<LocalNodeId<Expression>> {
        let _timing = self.timing_scope(tags::PARSE_EXPRESSION);

        // stack depth
        let depth = self.state.expression_stack_depth;
        self.state.expression_stack_depth = depth + 1;

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
        self.state.expression_stack_depth = depth;

        result
    }

    /// Eat one type expression with stack growth checks.
    #[inline(always)]
    pub(crate) fn eat_type_expression_inner_with_stack_guard(
        &mut self,
    ) -> ParseResult<LocalNodeId<TypeExpression>> {
        let _timing = self.timing_scope(tags::PARSE_EXPRESSION);

        // stack depth
        let depth = self.state.expression_stack_depth;
        self.state.expression_stack_depth = depth + 1;

        // guard interval
        #[cfg(debug_assertions)]
        let should_check_stack = depth != 0;

        // guard interval
        #[cfg(not(debug_assertions))]
        let should_check_stack = depth != 0 && depth.is_multiple_of(STACK_GROW_CHECK_INTERVAL);

        // parse with stack guard
        let result = if should_check_stack {
            destack_core::ensure_sufficient_stack(|| self.eat_type_expression_inner())
        } else {
            self.eat_type_expression_inner()
        };

        // restore depth
        self.state.expression_stack_depth = depth;

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
        // plain identifier fast paths only exist on bare identifier heads
        if !self.peek_is(TokenType::Identifier)
            || self.options.is_in_match_case()
            || self.options.is_in_decorator()
            || self.options.is_in_typeof_query()
        {
            return Ok(None);
        }

        // value-space plain identifiers yield to lambda heads, labels, and contextual declarations
        if self.keyword_for_index(self.pos_index()).is_some()
            || self.should_try_contextual_type_literal()
        {
            return Ok(None);
        }

        let next_raw_index = self.index_for_next();
        let next_raw_token_type = self.token_type_at(next_raw_index);
        if matches!(next_raw_token_type, TokenType::Arrow | TokenType::ArrowWide)
            || self.options.is_in_statement_position() && next_raw_token_type == TokenType::Colon
        {
            return Ok(None);
        }

        // contextual declarations use the declaration entry path
        let is_global_identifier = self.is_global_identifier_at(self.pos_index());
        let is_module_identifier = self.language.supports_module_declaration()
            && self.is_module_identifier_at(self.pos_index());
        if is_global_identifier || is_module_identifier {
            let next_cursor = self.scanner_cursor_from(next_raw_index);
            let next_token_type = next_cursor.token_type;
            let next_token_index = next_cursor.index;

            if is_global_identifier
                && matches!(
                    next_token_type,
                    TokenType::OpenBrace | TokenType::Identifier | TokenType::Literal
                )
            {
                return Ok(None);
            }

            if is_module_identifier
                && !next_cursor.has_line_break_before
                && DECLARATION_START_TOKENS.contains(&next_token_type)
                && matches!(next_token_type, TokenType::Identifier | TokenType::Literal)
                && (next_token_type != TokenType::Identifier
                    || !is_type_relation_keyword(self.keyword_for_index(next_token_index)))
                && next_raw_token_type == next_token_type
            {
                return Ok(None);
            }
        }

        let expression_id = self.eat_identifier_expression_path(start)?;
        let expression_id = self.eat_expression_continuation(start, expression_id)?;

        Ok(Some(expression_id))
    }

    /// Return one contextual type literal type expression when the current token sequence allows it.
    fn try_eat_contextual_type_literal_type_expression(
        &mut self,
        start: &ParserMark,
    ) -> ParseResult<Option<LocalNodeId<TypeExpression>>> {
        // only contextual literal positions can use this path
        if !self.should_try_contextual_type_literal() {
            return Ok(None);
        }

        let Ok(type_literal) = self.peek_type_literal() else {
            return Ok(None);
        };

        let _literal_timing = self.timing_scope(tags::PARSE_EXPRESSION_PRIMARY_LITERAL);
        let type_literal = self.eat_type_literal(Some(type_literal))?;
        let type_expression_id = self.insert_node(
            TypeExpression::Literal {
                value: type_literal,
            },
            self.get_span_from(start),
        );

        Ok(Some(type_expression_id))
    }

    /// Return one declaration header or one early expression for an identifier start.
    fn eat_identifier_primary_lead(
        &mut self,
        start: &ParserMark,
        expression_decorators: &mut PendingDecorators,
    ) -> ParseResult<IdentifierPrimaryLead> {
        let descriptor_head_keyword = self.keyword_for_index(self.pos_index());
        let can_parse_declaration_descriptor = self.options.is_in_statement_position()
            || self.options.is_in_type()
            || self.options.is_in_variant()
            || self.options.is_in_declare_context()
            || self.language.is_declaration()
            || matches!(
                descriptor_head_keyword,
                Some(Keyword::Export | Keyword::Declare | Keyword::Abstract | Keyword::Static)
            );

        // non declaration positions always keep the empty header
        if !can_parse_declaration_descriptor || !self.should_parse_declaration_descriptor() {
            return Ok(IdentifierPrimaryLead::Header(DeclarationHeader::default()));
        }

        match self.eat_declaration_descriptor(start)? {
            DescriptorHead::Header {
                header,
                mut decorators,
            } => {
                expression_decorators.append(&mut decorators);
                Ok(IdentifierPrimaryLead::Header(header))
            }
            DescriptorHead::Expression(expression_id) => {
                Ok(IdentifierPrimaryLead::Expression(expression_id))
            }
        }
    }

    /// Return the normalized parse facts after one identifier-start primary head.
    fn identifier_primary_facts(&mut self) -> IdentifierPrimaryFacts {
        let next_raw_index = self.index_for_next();
        let next_raw_token_type = self.token_type_at(next_raw_index);
        let next_cursor = self.scanner_cursor_from(next_raw_index);
        let next_token_type = next_cursor.token_type;
        let next_token_index = next_cursor.index;
        let next_has_line_break = next_cursor.has_line_break_before;
        let is_declaration_start = DECLARATION_START_TOKENS.contains(&next_token_type);
        let keyword = self.keyword_for_index(self.pos_index());

        // decorators only keep the small keyword subset that still behaves like operators or heads
        let keyword = if self.options.is_in_decorator() && !self.options.is_in_type() {
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
        // typeof queries treat some identifiers as plain names
        else if self.options.is_in_typeof_query() {
            if matches!(keyword, Some(Keyword::Type | Keyword::Readonly)) {
                None
            } else {
                keyword
            }
        } else {
            keyword
        };

        // module declarations only exist in value space
        let is_module_declaration_start = if self.options.is_in_decorator()
            || self.options.is_in_type()
            || next_has_line_break
            || !is_declaration_start
            || !self.language.supports_module_declaration()
            || !self.is_module_identifier_at(self.pos_index())
        {
            false
        }
        // module names must not collide with relation keywords
        else if !matches!(next_token_type, TokenType::Identifier | TokenType::Literal) {
            false
        } else {
            let next_keyword = if next_token_type == TokenType::Identifier {
                self.keyword_for_index(next_token_index)
            } else {
                None
            };

            !is_type_relation_keyword(next_keyword)
        };

        IdentifierPrimaryFacts {
            next_raw_token_type,
            next_token_type,
            next_token_index,
            next_has_line_break,
            is_declaration_start,
            keyword,
            is_module_declaration_start,
        }
    }

    /// Eat one value-space unary keyword primary expression.
    fn eat_value_unary_keyword_primary_expression(
        &mut self,
        start: &ParserMark,
        keyword: Keyword,
    ) -> ParseResult<LocalNodeId<Expression>> {
        let operator = match keyword {
            Keyword::Typeof => UnaryOperator::Typeof,
            Keyword::Void => UnaryOperator::Void,
            _ => unreachable!(),
        };

        // operator head
        let operator_start = self.mark_span();
        self.bump(); // eat unary operator
        self.eat_newlines_maybe()?;
        let operator_span = self.get_span_from(&operator_start);

        // operand
        let mut right_options = self
            .options
            .not_in_position()
            .in_left_precedence(operator.precedence());
        if self.options.is_in_type_conditional_right() {
            right_options = right_options.in_type_conditional_right();
        }
        let right = self.eat_expression_with_context_unchecked(right_options)?;

        // unparenthesized arrow functions are not unary operands
        if self.is_unparenthesized_lambda_expression(right) {
            return Err(ParseError::unexpected(self.tree.get_span(right)));
        }

        // unary expression
        let expression = Expression::Unary { operator, right };
        let expression_id = self.insert_node(expression, self.get_span_from(start));
        self.tree.set_main_span(expression_id, operator_span);

        Ok(expression_id)
    }

    /// Eat one type-space unary keyword primary expression.
    fn eat_type_unary_keyword_primary_expression(
        &mut self,
        start: &ParserMark,
        keyword: Keyword,
    ) -> ParseResult<LocalNodeId<TypeExpression>> {
        let operator = match keyword {
            Keyword::Typeof => TypeUnaryOperator::Typeof,
            Keyword::Keyof => TypeUnaryOperator::Keyof,
            _ => unreachable!(),
        };

        // operator head
        let operator_start = self.mark_span();
        self.bump(); // eat type unary operator
        self.eat_newlines_maybe()?;
        let operator_span = self.get_span_from(&operator_start);

        // operand context
        let mut right_expression_context = self
            .options
            .not_in_position()
            .in_left_precedence(operator.precedence());
        let right_ambient_context = self
            .options
            .with_type(true)
            .with_typeof_query(operator == TypeUnaryOperator::Typeof);
        if self.options.is_in_type_conditional_right() {
            right_expression_context = right_expression_context.in_type_conditional_right();
        }

        // unary type expression
        let expression = match operator {
            TypeUnaryOperator::Typeof => {
                let value =
                    self.eat_typeof_query_operand(right_ambient_context, right_expression_context)?;

                TypeExpression::TypeOfValue { value }
            }
            TypeUnaryOperator::Keyof => {
                let target_type = self.eat_type_expression_or_recover_missing(
                    self.options
                        .with_ambient_context(right_ambient_context)
                        .with_expression_context(right_expression_context),
                    NodeType::Expression,
                )?;

                TypeExpression::KeyOf { target_type }
            }
            _ => unreachable!(),
        };

        let type_expression_id = self.insert_node(expression, self.get_span_from(start));
        self.tree.set_main_span(type_expression_id, operator_span);

        Ok(type_expression_id)
    }

    /// Return whether the current `typeof` query operand must start in type space.
    fn typeof_query_operand_requires_type_space(&mut self) -> bool {
        let token_type = self.peek_token_type();
        let keyword = if token_type == TokenType::Identifier {
            self.peek_any_keyword().ok()
        } else {
            None
        };

        // strict type-only keywords cannot enter through value parsing
        if matches!(
            keyword,
            Some(Keyword::Infer | Keyword::Keyof | Keyword::Readonly | Keyword::Typeof)
        ) {
            return true;
        }

        // strict type-only literals and composites must lower through type space
        let starts_type_only_composite = matches!(
            token_type,
            TokenType::OpenBrace | TokenType::OpenBracket | TokenType::Multiply
        ) || token_type == TokenType::LessThan
            && self.can_start_generic_arrow_expression()
            || token_type == TokenType::Not && self.peek_type_literal().is_ok()
            || self.peek_type_unary_prefix_operator_maybe().is_some();
        if starts_type_only_composite {
            return true;
        }

        // elementwise reference families are also strict type starts here
        self.language.is_destack()
            && matches!(
                token_type,
                TokenType::ElementwiseXor | TokenType::ElementwiseAnd
            )
    }

    /// Eat one `typeof` query operand as either a value expression or a wrapped type expression.
    ///
    /// Examples:
    /// ```
    /// typeof value
    /// typeof Foo.Bar
    /// typeof keyof T
    /// typeof { x: string }
    /// ```
    fn eat_typeof_query_operand(
        &mut self,
        right_ambient_context: ParserOptions,
        right_expression_context: ParserOptions,
    ) -> ParseResult<LocalNodeId<Expression>> {
        let operand_context = self
            .options
            .with_ambient_context(right_ambient_context)
            .with_expression_context(right_expression_context);

        // strict type-only starts lower directly to wrapped type expressions
        if self.typeof_query_operand_requires_type_space() {
            let type_expression_id = self.eat_type_expression_node_or_recover_missing(
                operand_context.with_type(true),
                NodeType::Expression,
            )?;

            return Ok(self.wrap_type_expression(type_expression_id));
        }

        // otherwise parse a normal value expression operand
        self.eat_expression_or_recover_missing(
            operand_context.with_type(false),
            NodeType::Expression,
        )
    }

    /// Parse one identifier-start primary expression after the caller selected the identifier arm.
    ///
    /// Examples:
    /// ```
    /// foo
    /// foo.bar
    /// foo<T>
    /// typeof value
    /// async () => value
    /// export { Foo } from "mod"
    /// module Foo
    /// ```
    fn eat_identifier_primary_expression(
        &mut self,
        start: &ParserMark,
        expression_decorators: &mut PendingDecorators,
    ) -> ParseResult<LocalNodeId<Expression>> {
        // declaration header prefixes or early declaration expressions
        let header = match self.eat_identifier_primary_lead(start, expression_decorators)? {
            IdentifierPrimaryLead::Header(header) => header,
            IdentifierPrimaryLead::Expression(expression_id) => {
                self.attach_pending_decorators_to_expression(expression_decorators, expression_id);

                return Ok(expression_id);
            }
        };

        // next token facts after the identifier head
        let facts = self.identifier_primary_facts();
        let is_unary_keyword = matches!(facts.keyword, Some(Keyword::Typeof | Keyword::Void));
        let is_type_unary_keyword = matches!(facts.keyword, Some(Keyword::Typeof | Keyword::Keyof));

        // shorthand lambda form
        if !self.options.is_in_match_case()
            && matches!(
                facts.next_raw_token_type,
                TokenType::Arrow | TokenType::ArrowWide
            )
        {
            let lambda_id = self.eat_function(start, header, false, false)?;
            return Ok(self.insert_declaration_expression(start, lambda_id));
        }

        // plain identifier path or contextual literal
        if facts.keyword.is_none()
            && !self.options.is_in_decorator()
            && !facts.is_module_declaration_start
        {
            if let Some(type_expression_id) =
                self.try_eat_contextual_type_literal_type_expression(start)?
            {
                return Ok(self.wrap_type_expression(type_expression_id));
            }

            return self.eat_identifier_expression_path(start);
        }

        // unary keyword expressions
        if is_unary_keyword {
            return self.eat_value_unary_keyword_primary_expression(
                start,
                facts.keyword.expect("checked unary keyword"),
            );
        }

        // type unary keyword expressions
        if is_type_unary_keyword {
            let type_expression_id = self.eat_type_unary_keyword_primary_expression(
                start,
                facts.keyword.expect("checked type unary keyword"),
            )?;

            return Ok(self.wrap_type_expression(type_expression_id));
        }

        // do block expression or do while
        if facts.keyword == Some(Keyword::Do) {
            if self.is_do_while_statement(facts.next_token_type) {
                return self.eat_while();
            }

            if facts.next_token_type == TokenType::OpenBrace {
                let block_id = self.eat_block(BlockContext::Expression)?;
                let expression_id =
                    self.insert_node(Expression::Block(block_id), self.get_span_from(start));

                return Ok(expression_id);
            }
        }

        // contextual module declaration
        if facts.keyword.is_none() && facts.is_module_declaration_start {
            let namespace_id = self.eat_namespace(start, header)?;
            return Ok(self.insert_declaration_expression(start, namespace_id));
        }

        // keyword expressions and declaration starters
        if let Some(keyword) = facts.keyword {
            let _keyword_timing = self.timing_scope(tags::PARSE_EXPRESSION_PRIMARY_KEYWORD);
            if let Some(expression_id) = self.eat_keyword_expression(
                start,
                header,
                keyword,
                facts.next_token_type,
                facts.next_token_index,
                facts.next_has_line_break,
                facts.next_raw_token_type,
                facts.is_declaration_start,
            )? {
                return Ok(expression_id);
            }
        }

        // contextual type literal
        if let Some(type_expression_id) =
            self.try_eat_contextual_type_literal_type_expression(start)?
        {
            return Ok(self.wrap_type_expression(type_expression_id));
        }

        // plain identifier path
        self.eat_identifier_expression_path(start)
    }

    /// Parse one identifier-start primary expression in strict type space.
    ///
    /// Examples:
    /// ```
    /// Foo
    /// keyof T
    /// typeof value
    /// module Foo
    /// ```
    fn eat_type_identifier_primary_expression(
        &mut self,
        start: &ParserMark,
        expression_decorators: &mut PendingDecorators,
    ) -> ParseResult<LocalNodeId<TypeExpression>> {
        // declaration header prefixes or early declaration expressions
        let header = match self.eat_identifier_primary_lead(start, expression_decorators)? {
            IdentifierPrimaryLead::Header(header) => header,
            IdentifierPrimaryLead::Expression(expression_id) => {
                self.attach_pending_decorators_to_expression(expression_decorators, expression_id);
                let declaration_id = match self.tree.get(expression_id) {
                    Expression::Declaration(declaration_id) => *declaration_id,
                    _ => return Err(ParseError::unexpected(self.tree.get_span(expression_id))),
                };

                return Ok(self.insert_declaration_type_expression(start, declaration_id));
            }
        };

        // next token facts after the identifier head
        let facts = self.identifier_primary_facts();
        let is_type_unary_keyword = matches!(facts.keyword, Some(Keyword::Typeof | Keyword::Keyof));

        // plain identifier path or contextual literal
        if facts.keyword.is_none()
            && !self.options.is_in_decorator()
            && !facts.is_module_declaration_start
        {
            if let Some(type_expression_id) =
                self.try_eat_contextual_type_literal_type_expression(start)?
            {
                return Ok(type_expression_id);
            }

            return self.eat_type_identifier_expression_path(start);
        }

        // type unary keyword expressions
        if is_type_unary_keyword {
            return self.eat_type_unary_keyword_primary_expression(
                start,
                facts.keyword.expect("checked type unary keyword"),
            );
        }

        // contextual module declaration
        if facts.keyword.is_none() && facts.is_module_declaration_start {
            let namespace_id = self.eat_namespace(start, header)?;
            return Ok(self.insert_declaration_type_expression(start, namespace_id));
        }

        // direct type keyword forms
        if let Some(keyword) = facts.keyword {
            let _keyword_timing = self.timing_scope(tags::PARSE_EXPRESSION_PRIMARY_KEYWORD);
            if let Some(type_expression_id) = self.eat_type_keyword_expression(
                start,
                header,
                keyword,
                facts.next_token_type,
                facts.next_token_index,
                facts.next_has_line_break,
                facts.next_raw_token_type,
                facts.is_declaration_start,
            )? {
                return Ok(type_expression_id);
            }
        }

        // contextual type literal
        if let Some(type_expression_id) =
            self.try_eat_contextual_type_literal_type_expression(start)?
        {
            return Ok(type_expression_id);
        }

        // plain identifier path
        self.eat_type_identifier_expression_path(start)
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

        // contextual type literals in value positions are only enabled in the extended grammar
        if !self.language.is_destack() {
            return false;
        }

        self.is_contextual_type_literal_identifier()
    }

    /// Parse one grouped inner expression and consume the closing `)`.
    fn eat_parenthesized_inner_expression(&mut self) -> ParseResult<LocalNodeId<Expression>> {
        let mut inner_options = self.options.nested().with_parenthesis(true);
        inner_options.set_allow_sequence_expression(true);

        let expression_id = self.eat_expression(inner_options)?;
        self.eat_newlines_maybe()?;
        self.eat_close_token_or_recover_missing(TokenType::CloseParenthesis, NodeType::Expression)?;

        Ok(expression_id)
    }

    /// Parse one grouped inner type expression and consume the closing `)`.
    fn eat_type_parenthesized_inner_expression(
        &mut self,
    ) -> ParseResult<LocalNodeId<TypeExpression>> {
        let mut inner_options = self.options.nested().with_parenthesis(true).in_type();
        inner_options.set_allow_sequence_expression(true);

        let expression_id =
            self.with_options(inner_options, |parser| parser.eat_type_expression())?;
        self.eat_newlines_maybe()?;
        self.eat_close_token_or_recover_missing(
            TokenType::CloseParenthesis,
            NodeType::TypeExpression,
        )?;

        Ok(expression_id)
    }

    /// Finish a parenthesized expression after the inner value has been parsed.
    fn finish_parenthesized_expression(
        &mut self,
        start: &ParserMark,
        expression_id: LocalNodeId<Expression>,
    ) -> LocalNodeId<Expression> {
        // wrapped type expressions keep a type-space parenthesized node
        if let Some(inner_expression_id) = self.wrapped_type_expression_maybe(expression_id) {
            let parenthesized_id = self.insert_node(
                TypeExpression::Parenthesized {
                    expression: inner_expression_id,
                },
                self.get_span_from(start),
            );
            let expression_id = self.wrap_type_expression(parenthesized_id);
            let inner_head_span = self.expression_head_span(expression_id);
            self.tree.set_head_span(parenthesized_id, inner_head_span);
            self.tree.set_head_span(expression_id, inner_head_span);

            return expression_id;
        }

        let parenthesized_id = self.insert_node(
            Expression::Parenthesized {
                expression: expression_id,
            },
            self.get_span_from(start),
        );

        let inner_head_span = self.expression_head_span(expression_id);

        // preserved groups inherit the wrapped head
        self.tree.set_head_span(parenthesized_id, inner_head_span);

        parenthesized_id
    }

    /// Finish one parenthesized type expression after the inner value has been parsed.
    fn finish_type_parenthesized_expression(
        &mut self,
        start: &ParserMark,
        expression_id: LocalNodeId<TypeExpression>,
    ) -> LocalNodeId<TypeExpression> {
        let parenthesized_id = self.insert_node(
            TypeExpression::Parenthesized {
                expression: expression_id,
            },
            self.get_span_from(start),
        );

        let inner_head_span = self.type_expression_head_span(expression_id);
        self.tree.set_head_span(parenthesized_id, inner_head_span);

        parenthesized_id
    }

    /// Return the lambda follow token when one parenthesized group can parse as a lambda.
    fn parenthesized_group_lambda_follow_token_maybe(
        &self,
        group: ParenthesizedGroupShape,
    ) -> Option<TokenType> {
        // require one lambda-shaped parameter group in arrow return positions
        let has_parenthesized_parameter_group =
            group.has_top_level_parameter_colon || group.has_top_level_comma || group.is_empty;
        if self.options.is_in_arrow_return_type() && !has_parenthesized_parameter_group {
            return None;
        }

        // inspect the follow token and reject nested `(() => x): ...`
        let follow_token_type = group.follow_token_type?;
        let is_colon_lambda = follow_token_type == TokenType::Colon;
        let is_parenthesized_arrow_value =
            is_colon_lambda && group.starts_with_nested_parenthesis && group.has_top_level_arrow;
        if is_parenthesized_arrow_value {
            return None;
        }

        // allow colon lambdas only in the extended lambda grammar
        let allows_colon_lambda = !is_colon_lambda
            || (self.language.is_destack() || self.language.is_typescript())
                && !self.options.is_in_before_type()
                && !self.options.is_in_match_case()
                && !group.has_top_level_comma;
        if !allows_colon_lambda {
            return None;
        }

        Some(follow_token_type)
    }

    /// Return true when one parenthesized group starts a tuple shape.
    fn parenthesized_group_has_tuple_shape(&mut self, group: ParenthesizedGroupShape) -> bool {
        // named tuple elements start with `name:`
        let starts_named_tuple_element = self.language.is_destack()
            && self.peek_is(TokenType::Identifier)
            && self.peek_next_is(TokenType::Colon);

        starts_named_tuple_element || group.has_top_level_comma
    }

    /// Try to parse one parenthesized lambda declaration from one known group shape.
    fn try_eat_parenthesized_lambda_declaration_from_group(
        &mut self,
        start: &ParserMark,
        group: ParenthesizedGroupShape,
    ) -> ParseResult<Option<LocalNodeId<Declaration>>> {
        // require one lambda follow token first
        let Some(follow_token_type) = self.parenthesized_group_lambda_follow_token_maybe(group)
        else {
            return Ok(None);
        };
        let is_colon_lambda = follow_token_type == TokenType::Colon;

        // ternary conditions need one speculative parse to keep `?:` honest
        if is_colon_lambda && self.options.is_in_ternary_condition() {
            let speculative_start = self.mark();
            let speculative_start_idx = self.tree.next_id();
            if let Ok(lambda_id) =
                self.eat_function(start, DeclarationHeader::default(), false, false)
            {
                let has_ternary_delimiter = self.peek_is(TokenType::Colon)
                    || self.is_token_after_newlines(self.pos(), TokenType::Colon);
                let should_accept = match self.tree.get(lambda_id) {
                    Declaration::Function(declaration) => {
                        declaration.body.is_some() && has_ternary_delimiter
                    }
                    _ => has_ternary_delimiter,
                };
                if should_accept {
                    return Ok(Some(lambda_id));
                }
            }

            self.restore(speculative_start, speculative_start_idx);
            return Ok(None);
        }

        // otherwise parse the lambda directly
        let lambda_id = self.eat_function(start, DeclarationHeader::default(), false, false)?;

        Ok(Some(lambda_id))
    }

    /// Try to parse a parenthesized lambda from one known group shape.
    fn try_eat_parenthesized_lambda_from_group(
        &mut self,
        start: &ParserMark,
        group: ParenthesizedGroupShape,
    ) -> ParseResult<Option<LocalNodeId<Expression>>> {
        // parse the shared lambda declaration form
        let lambda_id = self.try_eat_parenthesized_lambda_declaration_from_group(start, group)?;

        Ok(lambda_id.map(|lambda_id| self.insert_declaration_expression(start, lambda_id)))
    }

    /// Try to parse a parenthesized lambda from one known group shape in strict type space.
    fn try_eat_type_parenthesized_lambda_from_group(
        &mut self,
        start: &ParserMark,
        group: ParenthesizedGroupShape,
    ) -> ParseResult<Option<LocalNodeId<TypeExpression>>> {
        // parse the shared lambda declaration form
        let lambda_id = self.try_eat_parenthesized_lambda_declaration_from_group(start, group)?;

        Ok(lambda_id.map(|lambda_id| self.insert_declaration_type_expression(start, lambda_id)))
    }

    /// Parse a parenthesized primary expression from one known group shape.
    fn eat_parenthesized_primary_from_group(
        &mut self,
        start: &ParserMark,
        group: ParenthesizedGroupShape,
    ) -> ParseResult<LocalNodeId<Expression>> {
        // base grouped expressions can skip the full lambda path
        let can_parse_plain_group_directly = !self.language.is_destack()
            && !self.options.is_in_arrow_return_type()
            && group.follow_token_type.is_none();

        // plain groups
        self.stats.record_parenthesized_expression_plain_call();
        if can_parse_plain_group_directly {
            self.stats.record_parenthesized_expression_plain_hit();
        } else {
            self.stats.record_parenthesized_expression_plain_miss();
        }

        if !can_parse_plain_group_directly
            && let Some(lambda_expression_id) =
                self.try_eat_parenthesized_lambda_from_group(start, group)?
        {
            return Ok(lambda_expression_id);
        }

        // consume the grouped body
        self.bump(); // eat open parenthesis
        self.eat_newlines_maybe()?;

        // empty tuple or sequence when we immediately see a closing parenthesis
        if self.peek_is(TokenType::CloseParenthesis) {
            return Ok(self.eat_empty_parenthesized_primary(start));
        }

        // tuple when we see a named element or top level comma
        if self.parenthesized_group_has_tuple_shape(group) {
            return self.eat_comma_parenthesized_primary(start);
        }

        // tuple or parenthesized expression for the remaining cases
        let expression_id = self.eat_parenthesized_inner_expression()?;
        let parenthesized_id = self.finish_parenthesized_expression(start, expression_id);

        Ok(parenthesized_id)
    }

    /// Eat one empty parenthesized primary expression after `(` has been consumed.
    fn eat_empty_parenthesized_primary(&mut self, start: &ParserMark) -> LocalNodeId<Expression> {
        self.bump(); // eat closing parenthesis

        if self.language.is_destack() {
            return self.insert_node(
                Expression::TupleExpression { elements: vec![] },
                self.get_span_from(start),
            );
        }

        self.insert_node(
            Expression::SequenceExpression {
                expressions: vec![],
            },
            self.get_span_from(start),
        )
    }

    /// Eat one empty type parenthesized primary expression after `(` has been consumed.
    fn eat_empty_type_parenthesized_primary(
        &mut self,
        start: &ParserMark,
    ) -> ParseResult<LocalNodeId<TypeExpression>> {
        self.bump(); // eat closing parenthesis

        if self.language.is_destack() {
            return Ok(self.insert_node(
                TypeExpression::Tuple { elements: vec![] },
                self.get_span_from(start),
            ));
        }

        let close_span = self
            .prev()
            .map(|token| token.span)
            .unwrap_or(self.eof_span());

        Err(ParseError::unexpected(close_span))
    }

    /// Eat one comma parenthesized primary expression after `(` has been consumed.
    fn eat_comma_parenthesized_primary(
        &mut self,
        start: &ParserMark,
    ) -> ParseResult<LocalNodeId<Expression>> {
        let tuple_elements = self
            .eat_sequence_literal_body(None, TokenType::CloseParenthesis)
            .for_node_type(NodeType::Expression)?;
        self.eat_newlines_maybe()?;
        self.eat_close_token_or_recover_missing(TokenType::CloseParenthesis, NodeType::Expression)?;

        let tuple_id = self.insert_node(
            Expression::TupleExpression {
                elements: tuple_elements,
            },
            self.get_span_from(start),
        );

        Ok(tuple_id)
    }

    /// Eat one comma grouped type primary expression after `(` has been consumed.
    fn eat_type_comma_parenthesized_primary(
        &mut self,
        start: &ParserMark,
    ) -> ParseResult<LocalNodeId<TypeExpression>> {
        let elements = self.eat_type_tuple_elements_body(TokenType::CloseParenthesis)?;
        self.eat_newlines_maybe()?;
        self.eat_close_token_or_recover_missing(
            TokenType::CloseParenthesis,
            NodeType::TypeExpression,
        )?;

        Ok(self.insert_node(
            TypeExpression::Tuple { elements },
            self.get_span_from(start),
        ))
    }

    /// Parse one open parenthesis primary expression.
    ///
    /// Examples:
    /// ```
    /// (value)
    /// ()
    /// (a, b)
    /// (value): string => other
    /// ```
    fn eat_parenthesized_primary(
        &mut self,
        start: &ParserMark,
    ) -> ParseResult<LocalNodeId<Expression>> {
        let _group_timing = self.timing_scope(tags::PARSE_EXPRESSION_PRIMARY_GROUP);
        let group = self.parenthesized_group_shape()?;
        self.eat_parenthesized_primary_from_group(start, group)
    }

    /// Parse one open parenthesis primary expression in strict type space.
    ///
    /// Examples:
    /// ```
    /// (T)
    /// ()
    /// (left, right)
    /// (value: T) => U
    /// ```
    fn eat_type_parenthesized_primary(
        &mut self,
        start: &ParserMark,
    ) -> ParseResult<LocalNodeId<TypeExpression>> {
        let _group_timing = self.timing_scope(tags::PARSE_EXPRESSION_PRIMARY_GROUP);
        let group = self.parenthesized_group_shape()?;

        // parse lambda heads before consuming the grouped body
        if let Some(type_expression_id) =
            self.try_eat_type_parenthesized_lambda_from_group(start, group)?
        {
            return Ok(type_expression_id);
        }

        // consume the grouped body
        self.bump(); // eat open parenthesis
        let open_parenthesis_end = self.prev_token_end();
        self.eat_newlines_maybe()?;

        // empty tuple when we immediately see a closing parenthesis
        if self.peek_is(TokenType::CloseParenthesis) {
            return self.eat_empty_type_parenthesized_primary(start);
        }

        // tuple when we see a named element or top level comma
        if self.parenthesized_group_has_tuple_shape(group) {
            return self.eat_type_comma_parenthesized_primary(start);
        }

        let expression_id = self.eat_type_parenthesized_inner_expression()?;
        self.set_node_leading_span(expression_id, open_parenthesis_end);
        let parenthesized_id = self.finish_type_parenthesized_expression(start, expression_id);

        Ok(parenthesized_id)
    }

    /// Try to parse a labelled statement or labelled expression.
    fn try_eat_labelled_expression(
        &mut self,
        start: &ParserMark,
        expression_decorators: &mut PendingDecorators,
    ) -> ParseResult<Option<LocalNodeId<Expression>>> {
        // decorators treat keywords as identifiers, so skip label parsing there
        if self.options.is_in_decorator()
            || self.options.is_in_match_case()
            || !self.peek_is(TokenType::Identifier)
            || !self.can_parse_labelled_expression()
        {
            return Ok(None);
        }

        let (label, label_span, body) = self.eat_labelled_expression_parts()?;

        // attach decorators to the labelled shell
        let labelled_id = self.insert_node(
            Expression::Labelled { label, body },
            self.get_span_from(start),
        );
        self.tree.set_main_span(labelled_id, label_span);
        self.attach_pending_decorators_to_expression(expression_decorators, labelled_id);

        Ok(Some(labelled_id))
    }

    /// Try to parse the plain identifier fast path before full primary dispatch.
    fn try_eat_plain_identifier_primary(
        &mut self,
        start: &ParserMark,
        expression_decorators: &mut PendingDecorators,
    ) -> ParseResult<Option<LocalNodeId<Expression>>> {
        // identifier fast paths
        let identifier_expression_id = if !self.peek_is(TokenType::Identifier) {
            None
        } else {
            // this also applies in statement position when it is not a labelled or declaration start
            self.try_parse_plain_identifier_expression(start)?
        };
        let Some(identifier_expression_id) = identifier_expression_id else {
            return Ok(None);
        };

        self.attach_pending_decorators_to_expression(
            expression_decorators,
            identifier_expression_id,
        );

        Ok(Some(identifier_expression_id))
    }

    /// Eat one leading elementwise chain head.
    fn eat_leading_elementwise_expression(
        &mut self,
        start: &ParserMark,
        token_type: TokenType,
    ) -> ParseResult<LocalNodeId<Expression>> {
        let leading_binary_operator = match token_type {
            TokenType::ElementwiseOr => BinaryOperator::ElementwiseOr,
            TokenType::ElementwiseAnd => BinaryOperator::ElementwiseAnd,
            _ => unreachable!(),
        };

        // separator and following trivia
        self.bump(); // eat elementwise operator

        // operand
        let expression_id = self.eat_expression_in_scope()?;

        // value space only accepts explicit elementwise chains
        let expression = self.tree.get(expression_id);
        if !matches!(
            expression,
            Expression::Binary { operator, .. }
                if *operator == leading_binary_operator
        ) {
            return Err(ParseError::unexpected(self.get_span_from(start)));
        }

        // preserve the full root span and explicit leading separator
        let head_span = self.binary_chain_head_span(expression_id, leading_binary_operator);
        self.tree.set_span(expression_id, self.get_span_from(start));
        self.tree.set_head_span(expression_id, head_span);

        Ok(expression_id)
    }

    /// Eat one leading elementwise type chain head.
    fn eat_type_leading_elementwise_expression(
        &mut self,
        start: &ParserMark,
        token_type: TokenType,
    ) -> ParseResult<LocalNodeId<TypeExpression>> {
        let leading_binary_operator = match token_type {
            TokenType::ElementwiseOr => BinaryOperator::ElementwiseOr,
            TokenType::ElementwiseAnd => BinaryOperator::ElementwiseAnd,
            _ => unreachable!(),
        };

        // separator and following trivia
        self.bump(); // eat elementwise operator
        let separator_end = self.prev_token_end();
        self.eat_newlines_maybe()?;

        // operand
        let expression_id = self.eat_type_expression_inner_with_stack_guard()?;
        let operand_span = self.tree.get_span(expression_id);

        // explicit leading separators always keep the corresponding chain shell
        let full_span = self.get_span_from(start);
        let expression_id = match (
            leading_binary_operator,
            self.tree.get(expression_id).clone(),
        ) {
            (BinaryOperator::ElementwiseOr, TypeExpression::Union { .. })
            | (BinaryOperator::ElementwiseAnd, TypeExpression::Intersection { .. }) => {
                expression_id
            }

            (BinaryOperator::ElementwiseOr, _) => self.insert_node(
                TypeExpression::Union {
                    elements: vec![expression_id],
                },
                operand_span,
            ),

            (BinaryOperator::ElementwiseAnd, _) => self.insert_node(
                TypeExpression::Intersection {
                    elements: vec![expression_id],
                },
                operand_span,
            ),

            _ => unreachable!(),
        };

        // the chain shell owns the explicit leading prefix
        let shell_leading_span = Span::new(full_span.file, full_span.start, operand_span.start);

        // preserve the full root span
        self.tree.set_span(expression_id, full_span);

        self.tree
            .set_side_span(expression_id, NodeSpanType::Leading, shell_leading_span);

        // the first arm owns trivia after the explicit separator
        let first_element_id = match (leading_binary_operator, self.tree.get(expression_id)) {
            (BinaryOperator::ElementwiseOr, TypeExpression::Union { elements }) => {
                *elements.first().expect("union has no elements")
            }

            (BinaryOperator::ElementwiseAnd, TypeExpression::Intersection { elements }) => {
                *elements.first().expect("intersection has no elements")
            }

            _ => expression_id,
        };
        self.set_node_leading_span(first_element_id, separator_end);

        // preserve the explicit leading separator
        let head_span = self.type_expression_head_span(expression_id);
        self.tree.set_head_span(expression_id, head_span);

        Ok(expression_id)
    }

    /// Eat one signed scalar literal type.
    fn eat_type_signed_scalar_literal_expression(
        &mut self,
        start: &ParserMark,
        operator: UnaryOperator,
    ) -> ParseResult<LocalNodeId<TypeExpression>> {
        let operator_start = self.mark_span();
        self.bump(); // eat unary operator
        self.eat_newlines_maybe()?;
        let operator_span = self.get_span_from(&operator_start);

        let scalar_literal = self.eat_scalar_literal()?;
        let scalar_literal = match (operator, scalar_literal) {
            (UnaryOperator::Plus, value) => value,
            (UnaryOperator::Negate, ScalarLiteral::Integer(value)) => ScalarLiteral::Integer(
                value
                    .checked_neg()
                    .ok_or_else(|| ParseError::unexpected(operator_span))?,
            ),
            (UnaryOperator::Negate, ScalarLiteral::Bigint(value)) => ScalarLiteral::Bigint(
                value
                    .checked_neg()
                    .ok_or_else(|| ParseError::unexpected(operator_span))?,
            ),
            (UnaryOperator::Negate, ScalarLiteral::Float(value)) => ScalarLiteral::Float(-value),
            _ => return Err(ParseError::unexpected(operator_span)),
        };

        let type_expression_id = self.insert_node(
            TypeExpression::ScalarLiteral {
                value: scalar_literal,
            },
            self.get_span_from(start),
        );
        self.tree.set_main_span(type_expression_id, operator_span);

        Ok(type_expression_id)
    }

    /// Eat one value-space unary prefix expression.
    fn eat_unary_prefix_expression(
        &mut self,
        start: &ParserMark,
        operator: UnaryOperator,
    ) -> ParseResult<LocalNodeId<Expression>> {
        let operator_start = self.mark_span();
        self.bump(); // eat unary operator
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

        let expression_id = self.insert_node(expression, self.get_span_from(start));
        self.tree.set_main_span(expression_id, operator_span);

        Ok(expression_id)
    }

    /// Eat one type-space unary prefix expression.
    fn eat_type_unary_prefix_expression(
        &mut self,
        start: &ParserMark,
        operator: TypeUnaryOperator,
    ) -> ParseResult<LocalNodeId<TypeExpression>> {
        let operator_start = self.mark_span();
        self.bump(); // eat type unary operator
        self.eat_newlines_maybe()?;
        let operator_span = self.get_span_from(&operator_start);

        let mut right_options = self
            .options
            .not_in_position()
            .in_left_precedence(operator.precedence());
        if self.options.is_in_type_conditional_right() {
            right_options = right_options.in_type_conditional_right();
        }

        let expression = match operator {
            TypeUnaryOperator::Typeof => {
                let value = self.eat_expression_or_recover_missing(
                    self.options
                        .with_type(false)
                        .with_expression_context(right_options),
                    NodeType::Expression,
                )?;

                TypeExpression::TypeOfValue { value }
            }
            TypeUnaryOperator::Keyof => {
                let target_type = self.eat_type_expression_or_recover_missing(
                    self.options
                        .with_type(true)
                        .with_expression_context(right_options),
                    NodeType::Expression,
                )?;

                TypeExpression::KeyOf { target_type }
            }
            _ => unreachable!(),
        };

        let type_expression_id = self.insert_node(expression, self.get_span_from(start));
        self.tree.set_main_span(type_expression_id, operator_span);

        Ok(type_expression_id)
    }

    /// Eat one type-space `^` or `&` prefix expression family.
    fn eat_type_reference_or_value_of_expression(
        &mut self,
        start: &ParserMark,
        token_type: TokenType,
    ) -> ParseResult<LocalNodeId<TypeExpression>> {
        let is_value_of = token_type == TokenType::ElementwiseXor;

        self.bump(); // eat ^ or &
        let mutability = self.eat_reference_mutability_maybe()?;
        let variance = self.eat_variance_bound_maybe()?;
        let target_type = self.eat_type_expression_or_recover_missing(
            self.options.not_in_position().with_type(true),
            NodeType::Expression,
        )?;

        let type_expression = if is_value_of {
            TypeExpression::ValueOf {
                mutability,
                variance,
                target_type,
            }
        } else {
            TypeExpression::ReferenceOf {
                mutability,
                variance,
                target_type,
            }
        };

        Ok(self.insert_node(type_expression, self.get_span_from(start)))
    }

    /// Eat one value-space `^` or `&` prefix expression family.
    fn eat_value_reference_or_value_of_expression(
        &mut self,
        start: &ParserMark,
        token_type: TokenType,
    ) -> ParseResult<LocalNodeId<Expression>> {
        let is_value_of = token_type == TokenType::ElementwiseXor;

        self.bump(); // eat ^ or &
        let mutability = self.eat_reference_mutability_maybe()?;
        let variance = self.eat_variance_bound_maybe()?;
        let right = self.eat_expression_with_context_unchecked(self.options.not_in_position())?;
        let expression = if is_value_of {
            Expression::ValueOf {
                mutability,
                variance,
                right,
            }
        } else {
            Expression::ReferenceOf {
                mutability,
                variance,
                right,
            }
        };

        Ok(self.insert_node(expression, self.get_span_from(start)))
    }

    /// Eat one type-space bracket primary expression.
    fn eat_type_bracket_primary_expression(
        &mut self,
        start: &ParserMark,
    ) -> ParseResult<LocalNodeId<TypeExpression>> {
        self.eat_token(TokenType::OpenBracket)?;
        let elements = self.eat_type_tuple_elements_body(TokenType::CloseBracket)?;
        self.eat_newlines_maybe()?;
        self.eat_close_token_or_recover_missing(TokenType::CloseBracket, NodeType::TypeExpression)?;

        Ok(self.insert_node(
            TypeExpression::Tuple { elements },
            self.get_span_from(start),
        ))
    }

    /// Eat one value-space bracket primary expression.
    fn eat_value_bracket_primary_expression(
        &mut self,
        start: &ParserMark,
    ) -> ParseResult<LocalNodeId<Expression>> {
        let elements = self.with_options(self.options.not_in_position(), |parser| {
            parser.eat_array_literal()
        })?;

        Ok(self.insert_node(
            Expression::ArrayExpression { elements },
            self.get_span_from(start),
        ))
    }

    /// Eat one type-space brace primary expression.
    fn eat_type_brace_primary_expression(
        &mut self,
        start: &ParserMark,
    ) -> ParseResult<LocalNodeId<TypeExpression>> {
        if self.can_start_type_mapped_expression() {
            return self.eat_type_mapped_expression();
        }

        let members = self.with_options(self.options.not_in_position(), |parser| {
            parser.eat_type_object_literal()
        })?;

        Ok(self.insert_node(
            TypeExpression::Object { members },
            self.get_span_from(start),
        ))
    }

    /// Eat one value-space brace primary expression.
    fn eat_value_brace_primary_expression(
        &mut self,
        start: &ParserMark,
    ) -> ParseResult<LocalNodeId<Expression>> {
        let can_start_object_literal = !self.options.is_in_statement_position()
            || self.can_parse_object_literal_in_statement_position();
        if !can_start_object_literal {
            if !self.is_block_start() {
                return Err(ParseError::unexpected(self.peek()?.span));
            }

            let block_id = self.eat_block(BlockContext::Expression)?;

            return Ok(self
                .tree
                .insert(Expression::Block(block_id), self.get_span_from(start)));
        }

        let properties = self.with_options(self.options.not_in_position(), |parser| {
            parser.eat_object_literal()
        })?;

        Ok(self.insert_node(
            Expression::ObjectExpression {
                ty: None,
                properties,
            },
            self.get_span_from(start),
        ))
    }

    /// Eat one non identifier primary expression.
    ///
    /// Examples:
    /// ```
    /// (value)
    /// [a, b, c]
    /// { x: 1, y: 2 }
    /// <T>(value)
    /// <div />
    /// "text"
    /// ```
    fn eat_non_identifier_primary_expression(
        &mut self,
        start: &ParserMark,
    ) -> ParseResult<LocalNodeId<Expression>> {
        let _timing = self.timing_scope(tags::PARSE_EXPRESSION_PRIMARY);
        let token_type = self.peek_token_type();

        match token_type {
            // grouped primaries
            TokenType::OpenParenthesis => self.eat_parenthesized_primary(start),

            // value or reference families
            TokenType::ElementwiseXor if self.language.is_destack() => {
                self.eat_value_reference_or_value_of_expression(start, token_type)
            }
            TokenType::ElementwiseAnd if self.language.is_destack() => {
                self.eat_value_reference_or_value_of_expression(start, token_type)
            }

            // collections and blocks
            TokenType::OpenBracket => self.eat_value_bracket_primary_expression(start),
            TokenType::OpenBrace => self.eat_value_brace_primary_expression(start),

            // `<...>` ambiguities
            TokenType::LessThan if self.can_start_generic_arrow_expression() => {
                let function_id =
                    self.eat_function(start, DeclarationHeader::default(), false, false)?;

                Ok(self.insert_declaration_expression(start, function_id))
            }
            TokenType::LessThan if self.can_start_tree_literal() => self
                .with_options(self.options.not_in_position(), |parser| {
                    parser.eat_tree_literal()
                }),

            // literal families
            TokenType::TemplateString | TokenType::TemplateStringStart
                if self.is_template_literal_start() =>
            {
                let _literal_timing = self.timing_scope(tags::PARSE_EXPRESSION_PRIMARY_LITERAL);
                let template_literal = self.eat_template_literal()?;

                Ok(self.insert_node(
                    Expression::TemplateExpression {
                        value: template_literal,
                    },
                    self.get_span_from(start),
                ))
            }
            TokenType::Divide | TokenType::DivideAssign => {
                let _literal_timing = self.timing_scope(tags::PARSE_EXPRESSION_PRIMARY_LITERAL);
                let scalar_literal = self.eat_regex_literal()?;

                Ok(self.insert_node(
                    Expression::ScalarLiteral(scalar_literal),
                    self.get_span_from(start),
                ))
            }
            TokenType::Literal if self.is_scalar_literal_start() => {
                let _literal_timing = self.timing_scope(tags::PARSE_EXPRESSION_PRIMARY_LITERAL);
                let scalar_literal = self.eat_scalar_literal()?;

                Ok(self.insert_node(
                    Expression::ScalarLiteral(scalar_literal),
                    self.get_span_from(start),
                ))
            }

            // contextual literals and private identifiers
            TokenType::Not if self.peek_type_literal().is_ok() => {
                let _literal_timing = self.timing_scope(tags::PARSE_EXPRESSION_PRIMARY_LITERAL);
                let type_literal = self.eat_type_literal(None)?;
                let type_expression_id = self.insert_node(
                    TypeExpression::Literal {
                        value: type_literal,
                    },
                    self.get_span_from(start),
                );

                Ok(self.wrap_type_expression(type_expression_id))
            }
            TokenType::Hash if self.peek_next_is(TokenType::Identifier) => {
                let hash_index = self.pos_index();
                let ident_index = hash_index + 1;
                self.check_tokens_are_adjacent(hash_index, ident_index)?;

                self.bump(); // eat #
                let (name, name_span) = self.eat_identifier_with_span()?;
                let expression_id = self.insert_node(
                    Expression::PrivateIdentifier { name },
                    self.get_span_from(start),
                );
                self.tree.set_main_span(expression_id, name_span);

                Ok(expression_id)
            }

            // generic unary prefixes
            _ if self.peek_unary_prefix_operator_maybe().is_some() => {
                let operator = self.peek_unary_prefix_operator_maybe().unwrap();
                self.eat_unary_prefix_expression(start, operator)
            }
            _ if self.peek_type_unary_prefix_operator_maybe().is_some() => {
                let operator = self.peek_type_unary_prefix_operator_maybe().unwrap();
                let type_expression_id = self.eat_type_unary_prefix_expression(start, operator)?;

                Ok(self.wrap_type_expression(type_expression_id))
            }

            // no primary matched
            _ => Err(ParseError::unexpected(self.peek()?.span)),
        }
    }

    /// Eat one strict type non identifier primary expression.
    ///
    /// Examples:
    /// ```
    /// (T)
    /// [A, B]
    /// { foo: string }
    /// infer T
    /// ```
    fn eat_type_non_identifier_primary_expression(
        &mut self,
        start: &ParserMark,
    ) -> ParseResult<LocalNodeId<TypeExpression>> {
        let _timing = self.timing_scope(tags::PARSE_EXPRESSION_PRIMARY);
        let token_type = self.peek_token_type();

        match token_type {
            // grouped primaries and generic function signatures
            TokenType::OpenParenthesis => self.eat_type_parenthesized_primary(start),
            TokenType::LessThan if self.can_start_generic_arrow_expression() => {
                let function_id =
                    self.eat_function(start, DeclarationHeader::default(), false, false)?;
                Ok(self.insert_declaration_type_expression(start, function_id))
            }

            // type pointers
            TokenType::Multiply => {
                self.bump(); // eat *
                let mutability = self.eat_reference_mutability_maybe()?;
                let target_type = self.eat_type_expression_or_recover_missing(
                    self.options.not_in_position().with_type(true),
                    NodeType::Expression,
                )?;

                Ok(self.insert_node(
                    TypeExpression::PointerOf {
                        mutability,
                        target_type,
                    },
                    self.get_span_from(start),
                ))
            }

            // value or reference type families
            TokenType::ElementwiseXor if self.language.is_destack() => {
                self.eat_type_reference_or_value_of_expression(start, token_type)
            }
            TokenType::ElementwiseAnd if self.language.is_destack() => {
                self.eat_type_reference_or_value_of_expression(start, token_type)
            }

            // collections
            TokenType::OpenBracket => self.eat_type_bracket_primary_expression(start),
            TokenType::OpenBrace => self.eat_type_brace_primary_expression(start),

            // literal families
            TokenType::TemplateString | TokenType::TemplateStringStart
                if self.is_template_literal_start() =>
            {
                let _literal_timing = self.timing_scope(tags::PARSE_EXPRESSION_PRIMARY_LITERAL);
                self.eat_type_template_literal_expression()
            }
            TokenType::Literal if self.is_scalar_literal_start() => {
                let _literal_timing = self.timing_scope(tags::PARSE_EXPRESSION_PRIMARY_LITERAL);
                let scalar_literal = self.eat_scalar_literal()?;

                Ok(self.insert_node(
                    TypeExpression::ScalarLiteral {
                        value: scalar_literal,
                    },
                    self.get_span_from(start),
                ))
            }
            TokenType::Not if self.peek_type_literal().is_ok() => {
                let _literal_timing = self.timing_scope(tags::PARSE_EXPRESSION_PRIMARY_LITERAL);
                let type_literal = self.eat_type_literal(None)?;

                Ok(self.insert_node(
                    TypeExpression::Literal {
                        value: type_literal,
                    },
                    self.get_span_from(start),
                ))
            }

            // signed literal types
            _ if self.peek_unary_prefix_operator_maybe().is_some() => {
                let operator = self.peek_unary_prefix_operator_maybe().unwrap();
                self.eat_type_signed_scalar_literal_expression(start, operator)
            }
            _ if self.peek_type_unary_prefix_operator_maybe().is_some() => {
                let operator = self.peek_type_unary_prefix_operator_maybe().unwrap();
                self.eat_type_unary_prefix_expression(start, operator)
            }

            // no strict type primary matched
            _ => Err(ParseError::unexpected(self.peek()?.span)),
        }
    }

    /// Finish one parsed type expression after postfix and infix continuation.
    fn finish_type_expression_tail(
        &mut self,
        left_type_id: LocalNodeId<TypeExpression>,
    ) -> ParseResult<LocalNodeId<TypeExpression>> {
        // trailing `?` is either an outer conditional boundary or a tuple optional marker
        let tail_cursor = self.scanner_cursor_from(self.pos_index());
        if tail_cursor.token_type != TokenType::Maybe {
            return Ok(left_type_id);
        }

        if tail_cursor.index != self.pos_index() {
            self.advance_to(tail_cursor.index);
        }

        // nested conditional right sides and constrained infer parses stop before the outer `?`
        if self.options.is_in_type_conditional_right()
            || self.options.is_disallow_type_conditional()
        {
            return Ok(left_type_id);
        }

        // tuple element optionals belong to the surrounding tuple parser
        let question_pos = self.pos();
        let is_tuple_optional = self.is_token_after_newlines(question_pos, TokenType::Comma)
            || self.is_token_after_newlines(question_pos, TokenType::CloseBracket);
        if is_tuple_optional {
            return Ok(left_type_id);
        }

        Err(ParseError::unexpected(self.peek()?.span))
    }

    /// Eat one strict type primary expression.
    ///
    /// Examples:
    /// ```
    /// Foo
    /// (T)
    /// [K in keyof T]: U
    /// typeof value
    /// ```
    fn eat_type_primary_expression(
        &mut self,
        start: &ParserMark,
        expression_decorators: &mut PendingDecorators,
    ) -> ParseResult<LocalNodeId<TypeExpression>> {
        // primary expression
        let token_type = self.peek_token_type();
        if token_type == TokenType::Identifier {
            return self.eat_type_identifier_primary_expression(start, expression_decorators);
        }

        if token_type == TokenType::ElementwiseOr
            || token_type == TokenType::ElementwiseAnd && !self.language.is_destack()
        {
            return self.eat_type_leading_elementwise_expression(start, token_type);
        };

        self.eat_type_non_identifier_primary_expression(start)
    }

    /// Eat a type expression body without stack growth checks.
    ///
    /// Examples:
    /// ```
    /// Foo.Bar<T>
    /// A | B & C
    /// value is string
    /// T extends U ? X : Y
    /// ```
    fn eat_type_expression_inner(&mut self) -> ParseResult<LocalNodeId<TypeExpression>> {
        // collect decorator prefixes before parsing the next type expression
        let mut expression_decorators =
            if !self.options.is_in_decorator() && self.peek_is(TokenType::At) {
                self.eat_decorators_maybe()?
            } else {
                PendingDecorators::new()
            };

        // capture expression span
        let start = self.mark_span();

        // primary expression
        let left_type_id = self.eat_type_primary_expression(&start, &mut expression_decorators)?;

        // type postfix and infix continuation
        let left_type_id = self.eat_type_postfix_continuation(&start, left_type_id)?;
        let left_type_id = self.eat_type_infix_continuation(&start, left_type_id)?;

        // attach parsed decorators before finishing the outer tail
        self.attach_pending_decorators_to_type_expression(&mut expression_decorators, left_type_id);

        self.finish_type_expression_tail(left_type_id)
    }

    /// Eat an expression body without stack growth checks.
    ///
    /// Examples:
    /// ```
    /// value.member(arg)
    /// value as string
    /// value satisfies Shape
    /// value ? then_value : else_value
    /// ```
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

        // labelled statements and labelled expressions
        if let Some(labelled_id) =
            self.try_eat_labelled_expression(&start, &mut expression_decorators)?
        {
            return Ok(labelled_id);
        }

        // plain identifier fast paths
        if let Some(identifier_expression_id) =
            self.try_eat_plain_identifier_primary(&start, &mut expression_decorators)?
        {
            return Ok(identifier_expression_id);
        }

        // primary expression
        let token_type = self.peek_token_type();
        let left_expression_id: LocalNodeId<Expression> = if token_type == TokenType::Identifier {
            self.eat_identifier_primary_expression(&start, &mut expression_decorators)?
        } else if token_type == TokenType::ElementwiseOr
            || token_type == TokenType::ElementwiseAnd && !self.language.is_destack()
        {
            let expression_id = self.eat_leading_elementwise_expression(&start, token_type)?;
            self.attach_pending_decorators_to_expression(&mut expression_decorators, expression_id);

            return Ok(expression_id);
        } else {
            self.eat_non_identifier_primary_expression(&start)?
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

    /// Return one stored expression head span.
    #[inline]
    fn stored_expression_head_span(&self, expression_id: LocalNodeId<Expression>) -> Span {
        self.tree
            .get_head_span(expression_id)
            .or_else(|| self.tree.get_main_span(expression_id))
            .unwrap_or_else(|| self.tree.get_span(expression_id))
    }

    /// Return one stored type expression head span.
    #[inline]
    fn stored_type_expression_head_span(
        &self,
        type_expression_id: LocalNodeId<TypeExpression>,
    ) -> Span {
        self.tree
            .get_head_span(type_expression_id)
            .or_else(|| self.tree.get_main_span(type_expression_id))
            .unwrap_or_else(|| self.tree.get_span(type_expression_id))
    }

    /// Return the head span for one type expression.
    pub(crate) fn type_expression_head_span(
        &self,
        type_expression_id: LocalNodeId<TypeExpression>,
    ) -> Span {
        match self.tree.get(type_expression_id) {
            // transparent wrappers
            TypeExpression::Parenthesized { expression } => {
                self.type_expression_head_span(*expression)
            }

            // type templates inherit the first interpolation head when present
            TypeExpression::TemplateLiteral { spans, .. } => spans
                .first()
                .map(|span_expression_id| self.type_expression_head_span(*span_expression_id))
                .unwrap_or_else(|| self.stored_type_expression_head_span(type_expression_id)),

            // leaf head
            _ => self.stored_type_expression_head_span(type_expression_id),
        }
    }

    /// Return the head span for one expression.
    pub(crate) fn expression_head_span(&self, expression_id: LocalNodeId<Expression>) -> Span {
        match self.tree.get(expression_id) {
            // transparent wrappers
            Expression::Parenthesized { expression }
            | Expression::As {
                expression,
                target_type: _,
            }
            | Expression::Satisfies {
                expression,
                target_type: _,
            } => self.expression_head_span(*expression),

            // wrapped type expressions use type head rules
            Expression::Type { value } => self.type_expression_head_span(*value),

            // leaf head
            _ => self.stored_expression_head_span(expression_id),
        }
    }

    /// Return the head span for the leftmost operand in one binary chain.
    pub(crate) fn binary_chain_head_span(
        &self,
        expression_id: LocalNodeId<Expression>,
        operator: BinaryOperator,
    ) -> Span {
        match self.tree.get(expression_id) {
            Expression::Parenthesized { expression } => {
                self.binary_chain_head_span(*expression, operator)
            }
            Expression::Binary {
                operator: inner_operator,
                left,
                ..
            } if *inner_operator == operator => self.binary_chain_head_span(*left, operator),
            _ => self.expression_head_span(expression_id),
        }
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

    /// Attach pending decorators to one parsed type expression.
    fn attach_pending_decorators_to_type_expression(
        &mut self,
        decorators: &mut PendingDecorators,
        type_expression_id: LocalNodeId<TypeExpression>,
    ) {
        if decorators.is_empty() {
            return;
        }

        self.attach_decorators(type_expression_id.id, std::mem::take(decorators));
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

                // export wrappers forward decorator ownership to the exported declaration expression
                Expression::Export { items, .. } => {
                    let mut exported_declaration_expression = None;
                    for item_id in items {
                        let item = self.tree.get(*item_id);
                        let DependencyItem::Item {
                            value: Some(value_expression_id),
                            ..
                        } = item
                        else {
                            continue;
                        };
                        if matches!(
                            self.tree.get(*value_expression_id),
                            Expression::Declaration(_)
                        ) {
                            exported_declaration_expression = Some(*value_expression_id);
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
