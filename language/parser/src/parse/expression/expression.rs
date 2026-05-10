use super::common::{
    DECLARATION_START_TOKENS, DeclarationHeader, DescriptorHead, is_type_relation_keyword,
};
use super::lookahead::ParenthesizedGroupShape;
use crate::parse::parser::ParserFlags;
use crate::parse::prelude::*;
use crate::{ParseError, ParseResult, Parser, ParserSpanStart};
use destack_source::{NodeSpanBoundary, NodeSpanList, NodeSpanRegion, NodeSpanType, Span};
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

/// Lookahead after one identifier primary head.
struct IdentifierPrimaryLookahead {
    /// The next token type.
    next_token_type: TokenType,
    /// Whether the next token starts on a new source line.
    next_is_on_new_line: bool,
    /// The contextual keyword at the next token, if any.
    next_keyword: Option<Keyword>,
    /// The token type after the next token.
    following_token_type: TokenType,
    /// Whether the next token can start a declaration.
    next_is_declaration_start: bool,
    /// The contextual keyword at the current identifier, if any.
    keyword: Option<Keyword>,
    /// Whether the identifier starts a contextual module declaration.
    is_module_declaration_start: bool,
}

/// One expression before continuation parsing.
#[derive(Clone, Copy, Debug)]
struct ParsedExpression {
    /// The parsed expression id.
    expression_id: LocalNodeId<Expression>,
    /// Whether source syntax wrapped the expression in one transparent parenthesized group.
    is_parenthesized: bool,
}

impl ParsedExpression {
    /// Create one parsed expression without transparent parenthesized wrapping.
    const fn plain(expression_id: LocalNodeId<Expression>) -> Self {
        Self {
            expression_id,
            is_parenthesized: false,
        }
    }
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
                NodeSpanType::ListItem(NodeSpanList::Segment, segment_index),
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
        let generic_arguments = if !self.current_token_is_on_new_line()
            && (self.peek_is(TokenType::LessThan) || self.peek_is(TokenType::ShiftLeft))
        {
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
        start: &ParserSpanStart,
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
        start: &ParserSpanStart,
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
        start: &ParserSpanStart,
    ) -> ParseResult<LocalNodeId<Expression>> {
        // value identifiers stay bare until continuation parsing builds the chain
        let (name, name_span) = self.eat_identifier_with_span()?;
        let expression = Expression::Identifier { name };
        let expression_id = self.insert_node(expression, self.get_span_from(start));
        self.tree.set_main_span(expression_id, name_span);

        Ok(expression_id)
    }

    /// Return whether one type expression contains generic arguments.
    fn type_expression_contains_generic_arguments(
        &self,
        type_expression_id: LocalNodeId<TypeExpression>,
    ) -> bool {
        match self.tree.get(type_expression_id) {
            TypeExpression::Reference {
                generic_arguments, ..
            } => !generic_arguments.is_empty(),
            TypeExpression::Member {
                left,
                generic_arguments,
                ..
            } => {
                !generic_arguments.is_empty()
                    || self.type_expression_contains_generic_arguments(*left)
            }
            TypeExpression::Parenthesized { expression } => {
                self.type_expression_contains_generic_arguments(*expression)
            }
            _ => false,
        }
    }

    /// Return whether one type expression is a generic static projection.
    fn type_expression_is_generic_projection(
        &self,
        type_expression_id: LocalNodeId<TypeExpression>,
    ) -> bool {
        matches!(
            self.tree.get(type_expression_id),
            TypeExpression::Member { .. }
        ) && self.type_expression_contains_generic_arguments(type_expression_id)
    }

    /// Return whether the current identifier may start a forced type expression.
    fn current_identifier_can_start_forced_type_expression(&mut self) -> bool {
        let next_token_type = self.next_token_type();
        let next_keyword = self.next_keyword();

        // type relations start with ordinary identifier heads in value positions
        if matches!(
            next_keyword,
            Some(Keyword::In | Keyword::Extends | Keyword::Implements)
        ) {
            return true;
        }

        // class and extension heads may continue through `.` or generic arguments before `extends`
        if self.flags.is_in_before_block() {
            return matches!(
                next_token_type,
                TokenType::Dot | TokenType::LessThan | TokenType::ShiftLeft
            ) || next_keyword == Some(Keyword::Extends);
        }

        // tagged object literal receivers may continue before a later `{`
        matches!(
            next_token_type,
            TokenType::Dot | TokenType::LessThan | TokenType::ShiftLeft | TokenType::OpenBrace
        )
    }

    /// Return whether one forced identifier type expression should be committed.
    fn forced_identifier_type_expression_should_commit(
        &mut self,
        type_expression_id: LocalNodeId<TypeExpression>,
    ) -> bool {
        // generic projections are static type-member paths
        if self.type_expression_is_generic_projection(type_expression_id) {
            return true;
        }

        // type relation operators keep both operands in type space
        let is_type_relation_head = self.peek_is(TokenType::Identifier)
            && matches!(
                self.peek_any_keyword().ok(),
                Some(Keyword::In | Keyword::Extends | Keyword::Implements)
            );
        if is_type_relation_head {
            return true;
        }

        // before-block heads stay in type space when `extends` follows
        let is_before_block_extends_head = self.flags.is_in_before_block()
            && self.peek_is(TokenType::Identifier)
            && self.peek_any_keyword().ok() == Some(Keyword::Extends);

        // tagged object literal receivers also stay in type space
        let is_tagged_object_literal_head = !self.flags.is_in_before_block()
            && self.peek_is(TokenType::OpenBrace)
            && self.can_start_tagged_object_literal_type(type_expression_id);

        is_before_block_extends_head || is_tagged_object_literal_head
    }

    /// Try to eat one identifier head that must lower through type space.
    fn try_eat_forced_identifier_type_expression(
        &mut self,
        start: &ParserSpanStart,
    ) -> ParseResult<Option<LocalNodeId<Expression>>> {
        // only language modes with value-space type reentry admit this path
        if !self.language.is_destack() {
            return Ok(None);
        }

        // most identifier heads cannot possibly force type space
        if !self.current_identifier_can_start_forced_type_expression() {
            return Ok(None);
        }

        // speculate the identifier head in type space
        let speculative_start = self.checkpoint();
        let speculative_start_idx = self.tree.next_id();
        let type_expression_id = self.with_flags(self.flags.in_type(), |parser| {
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
        if self.forced_identifier_type_expression_should_commit(type_expression_id) {
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
        start: &ParserSpanStart,
    ) -> ParseResult<LocalNodeId<Expression>> {
        // static space always lowers to qualified references
        if self.flags.is_in_static() {
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
        context: ParserFlags,
    ) -> ParseResult<LocalNodeId<Expression>> {
        self.eat_expression(self.flags.with_expression_context(context))
    }

    /// Parse one expression with the current parser flags.
    #[inline(always)]
    pub fn parse_expression(&mut self) -> ParseResult<LocalNodeId<Expression>> {
        self.eat_expression(self.flags)
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
    pub(crate) fn eat_expression(
        &mut self,
        flags: ParserFlags,
    ) -> ParseResult<LocalNodeId<Expression>> {
        if self.flags == flags {
            return self.eat_expression_inner_with_stack_guard();
        }

        let old_flags = self.swap_flags(flags);
        let result = self.eat_expression_inner_with_stack_guard();
        self.restore_flags(old_flags);
        result
    }

    /// Eat one expression in the current parser flags.
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
        if !self.flags.is_in_statement_position() {
            return self.eat_expression_inner_with_stack_guard();
        }

        let context = self.flags.not_in_statement_position();
        self.eat_expression(self.flags.with_expression_context(context))
    }

    /// Eat an expression with stack growth checks.
    #[inline(always)]
    fn eat_expression_inner_with_stack_guard(&mut self) -> ParseResult<LocalNodeId<Expression>> {
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

    /// Try to eat an expression and recover to a statement boundary.
    pub fn try_eat_expression_until_statement_boundary(
        &mut self,
    ) -> ParseResult<LocalNodeId<Expression>> {
        match self.eat_expression_in_scope() {
            Ok(expression_id) => Ok(expression_id),
            Err(err) => {
                let err = err.for_node_type(NodeType::Expression);
                let span = err.leaf_span();
                let recovered_span = self.try_recover_in_statement_from_span(span, Some(err))?;
                let error_id = self.tree.insert(Expression::Error, recovered_span);
                Ok(error_id)
            }
        }
    }

    /// Try to parse a plain identifier expression and continuation in common value contexts.
    pub(crate) fn try_parse_plain_identifier_expression(
        &mut self,
        start: &ParserSpanStart,
    ) -> ParseResult<Option<LocalNodeId<Expression>>> {
        // plain identifier fast paths only exist on bare identifier heads
        if !self.peek_is(TokenType::Identifier)
            || self.flags.is_in_match_case()
            || self.flags.is_in_decorator()
            || self.flags.is_in_typeof_query()
        {
            return Ok(None);
        }

        // value-space plain identifiers yield to lambda heads, labels, and contextual declarations
        if self.current_keyword().is_some() || self.should_try_contextual_type_literal() {
            return Ok(None);
        }

        let next_token_type = self.next_token_type();
        if matches!(next_token_type, TokenType::Arrow | TokenType::ArrowWide)
            || self.flags.is_in_statement_position() && next_token_type == TokenType::Colon
        {
            return Ok(None);
        }

        // contextual declarations use the declaration entry path
        let is_global_identifier = self.is_global_identifier();
        let is_module_identifier =
            self.language.supports_module_declaration() && self.is_module_identifier();
        if is_global_identifier || is_module_identifier {
            let next_token = self.next_token();
            let next_token_type = next_token.token.ty;
            let next_keyword = self.next_keyword();

            if is_global_identifier
                && matches!(
                    next_token_type,
                    TokenType::OpenBrace | TokenType::Identifier | TokenType::Literal
                )
            {
                return Ok(None);
            }

            if is_module_identifier
                && !next_token.token.is_on_new_line
                && DECLARATION_START_TOKENS.contains(&next_token_type)
                && matches!(next_token_type, TokenType::Identifier | TokenType::Literal)
                && (next_token_type != TokenType::Identifier
                    || !is_type_relation_keyword(next_keyword))
            {
                return Ok(None);
            }
        }

        // type-space heads need the full identifier dispatch path
        if self.language.is_destack() && self.current_identifier_can_start_forced_type_expression()
        {
            return Ok(None);
        }

        let expression_id = self.eat_identifier_expression_path(start)?;
        let expression_id = self.eat_expression_continuation(start, expression_id, false)?;

        Ok(Some(expression_id))
    }

    /// Return one contextual type literal type expression when the current token sequence allows it.
    fn try_eat_contextual_type_literal_type_expression(
        &mut self,
        start: &ParserSpanStart,
    ) -> ParseResult<Option<LocalNodeId<TypeExpression>>> {
        // only contextual literal positions can use this path
        if !self.should_try_contextual_type_literal() {
            return Ok(None);
        }

        // type predicate subjects stay identifier shaped
        if self.next_keyword() == Some(Keyword::Is) {
            return Ok(None);
        }

        let Ok(type_literal) = self.peek_type_literal() else {
            return Ok(None);
        };
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
        start: &ParserSpanStart,
        expression_decorators: &mut PendingDecorators,
    ) -> ParseResult<IdentifierPrimaryLead> {
        let descriptor_head_keyword = self.current_keyword();
        let can_parse_declaration_descriptor = self.flags.is_in_statement_position()
            || self.flags.is_in_type()
            || self.flags.is_in_variant()
            || self.flags.is_in_declare_context()
            || self.language.is_declaration()
            || matches!(
                descriptor_head_keyword,
                Some(
                    Keyword::Export
                        | Keyword::Declare
                        | Keyword::Abstract
                        | Keyword::Shared
                        | Keyword::Static,
                )
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

    /// Return the lookahead after one identifier-start primary head.
    fn identifier_primary_lookahead(&mut self) -> IdentifierPrimaryLookahead {
        let next_token = self.next_token();
        let next_token_type = next_token.token.ty;
        let next_is_on_new_line = next_token.token.is_on_new_line;
        let next_keyword = self.next_keyword();
        let following_token_type = self.lookahead(|parser| {
            parser.bump();
            parser.bump();
            parser.peek_token_type()
        });
        let next_is_declaration_start = DECLARATION_START_TOKENS.contains(&next_token_type);
        let keyword = self.current_keyword();

        // decorators only keep the small keyword subset that still behaves like operators or heads
        let keyword = if self.flags.is_in_decorator() && !self.flags.is_in_type() {
            match keyword {
                Some(
                    Keyword::Async
                    | Keyword::Await
                    | Keyword::This
                    | Keyword::New
                    | Keyword::Function
                    | Keyword::Class
                    | Keyword::Typeof
                    | Keyword::Void,
                ) => keyword,
                _ => None,
            }
        }
        // typeof queries treat some identifiers as plain names
        else if self.flags.is_in_typeof_query() {
            if matches!(keyword, Some(Keyword::Type | Keyword::Readonly)) {
                None
            } else {
                keyword
            }
        } else {
            keyword
        };

        // module declarations only exist in value space
        let is_module_declaration_start = if self.flags.is_in_decorator()
            || self.flags.is_in_type()
            || next_is_on_new_line
            || !next_is_declaration_start
            || !self.language.supports_module_declaration()
            || !self.is_module_identifier()
            || !matches!(next_token_type, TokenType::Identifier | TokenType::Literal)
        {
            false
        } else {
            // module names must not collide with relation keywords
            !is_type_relation_keyword(next_keyword)
        };

        IdentifierPrimaryLookahead {
            next_token_type,
            next_is_on_new_line,
            next_keyword,
            following_token_type,
            next_is_declaration_start,
            keyword,
            is_module_declaration_start,
        }
    }

    /// Eat one value-space unary keyword primary expression.
    fn eat_value_unary_keyword_primary_expression(
        &mut self,
        start: &ParserSpanStart,
        keyword: Keyword,
    ) -> ParseResult<LocalNodeId<Expression>> {
        let operator = match keyword {
            Keyword::Typeof => UnaryOperator::Typeof,
            Keyword::Void => UnaryOperator::Void,
            _ => unreachable!(),
        };

        // operator head
        let operator_start = self.span_start();
        self.bump(); // eat unary operator
        let operator_span = self.get_span_from(&operator_start);

        // operand
        let mut right_flags = self
            .flags
            .not_in_position()
            .in_left_precedence(operator.precedence());
        if self.flags.is_in_type_conditional_right() {
            right_flags = right_flags.in_type_conditional_right();
        }
        let right = self.eat_expression_with_context_unchecked(right_flags)?;

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
        start: &ParserSpanStart,
        keyword: Keyword,
    ) -> ParseResult<LocalNodeId<TypeExpression>> {
        let operator = match keyword {
            Keyword::Typeof => TypeUnaryOperator::Typeof,
            Keyword::Keyof => TypeUnaryOperator::Keyof,
            Keyword::Readonly => TypeUnaryOperator::Readonly,
            Keyword::Shared if self.language.is_destack() => TypeUnaryOperator::Shared,
            _ => unreachable!(),
        };

        // operator head
        let operator_start = self.span_start();
        self.bump(); // eat type unary operator
        let operator_span = self.get_span_from(&operator_start);

        // operand context
        let mut right_expression_context = self
            .flags
            .not_in_position()
            .in_left_precedence(operator.precedence());
        let right_ambient_context = self
            .flags
            .with_type(true)
            .with_typeof_query(operator == TypeUnaryOperator::Typeof);
        if self.flags.is_in_type_conditional_right() {
            right_expression_context = right_expression_context.in_type_conditional_right();
        }
        if self.flags.is_disallow_type_conditional() {
            right_expression_context = right_expression_context.disallow_type_conditional();
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
                    self.flags
                        .with_ambient_context(right_ambient_context)
                        .with_expression_context(right_expression_context),
                    NodeType::Expression,
                )?;

                TypeExpression::KeyOf { target_type }
            }
            TypeUnaryOperator::Readonly => {
                let target_type = self.eat_type_expression_or_recover_missing(
                    self.flags
                        .with_ambient_context(right_ambient_context)
                        .with_expression_context(right_expression_context),
                    NodeType::Expression,
                )?;

                TypeExpression::Readonly { target_type }
            }
            TypeUnaryOperator::Shared => {
                let target_type = self.eat_type_expression_or_recover_missing(
                    self.flags
                        .with_ambient_context(right_ambient_context)
                        .with_expression_context(right_expression_context),
                    NodeType::Expression,
                )?;

                TypeExpression::Shared { target_type }
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
        right_ambient_context: ParserFlags,
        right_expression_context: ParserFlags,
    ) -> ParseResult<LocalNodeId<Expression>> {
        let operand_context = self
            .flags
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
        start: &ParserSpanStart,
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

        // lookahead after the identifier head
        let lookahead = self.identifier_primary_lookahead();
        let is_unary_keyword = matches!(lookahead.keyword, Some(Keyword::Typeof | Keyword::Void));
        let is_type_unary_keyword = matches!(
            lookahead.keyword,
            Some(Keyword::Typeof | Keyword::Keyof | Keyword::Readonly)
        ) || self.language.is_destack()
            && lookahead.keyword == Some(Keyword::Shared);

        // shorthand lambda form
        if !self.flags.is_in_match_case()
            && matches!(
                lookahead.next_token_type,
                TokenType::Arrow | TokenType::ArrowWide
            )
        {
            let lambda_id = self.eat_function(start, header, false, false)?;
            return Ok(self.insert_declaration_expression(start, lambda_id));
        }

        // plain identifier path or contextual literal
        if lookahead.keyword.is_none()
            && !self.flags.is_in_decorator()
            && !lookahead.is_module_declaration_start
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
                lookahead.keyword.expect("checked unary keyword"),
            );
        }

        // type unary keyword expressions
        if is_type_unary_keyword {
            let type_expression_id = self.eat_type_unary_keyword_primary_expression(
                start,
                lookahead.keyword.expect("checked type unary keyword"),
            )?;

            return Ok(self.wrap_type_expression(type_expression_id));
        }

        // do block expression or do while
        if lookahead.keyword == Some(Keyword::Do) {
            if self.is_do_while_statement(lookahead.next_token_type) {
                return self.eat_while();
            }

            if lookahead.next_token_type == TokenType::OpenBrace {
                let block_id = self.eat_block(BlockContext::Expression)?;
                let expression_id =
                    self.insert_node(Expression::Block(block_id), self.get_span_from(start));

                return Ok(expression_id);
            }
        }

        // contextual module declaration
        if lookahead.keyword.is_none() && lookahead.is_module_declaration_start {
            let namespace_id = self.eat_namespace(start, header)?;
            return Ok(self.insert_declaration_expression(start, namespace_id));
        }

        // keyword expressions and declaration starters
        if let Some(keyword) = lookahead.keyword
            && let Some(expression_id) = self.eat_keyword_expression(
                start,
                header,
                keyword,
                lookahead.next_token_type,
                lookahead.next_is_on_new_line,
                lookahead.next_keyword,
                lookahead.following_token_type,
                lookahead.next_is_declaration_start,
            )?
        {
            return Ok(expression_id);
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
        start: &ParserSpanStart,
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

        // lookahead after the identifier head
        let lookahead = self.identifier_primary_lookahead();
        let is_type_unary_keyword = matches!(
            lookahead.keyword,
            Some(Keyword::Typeof | Keyword::Keyof | Keyword::Readonly)
        ) || self.language.is_destack()
            && lookahead.keyword == Some(Keyword::Shared);

        // plain identifier path or contextual literal
        if lookahead.keyword.is_none()
            && !self.flags.is_in_decorator()
            && !lookahead.is_module_declaration_start
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
                lookahead.keyword.expect("checked type unary keyword"),
            );
        }

        // contextual module declaration
        if lookahead.keyword.is_none() && lookahead.is_module_declaration_start {
            let namespace_id = self.eat_namespace(start, header)?;
            return Ok(self.insert_declaration_type_expression(start, namespace_id));
        }

        // direct type keyword forms
        if let Some(keyword) = lookahead.keyword
            && let Some(type_expression_id) = self.eat_type_keyword_expression(
                start,
                header,
                keyword,
                lookahead.next_token_type,
                lookahead.next_is_on_new_line,
                lookahead.next_keyword,
                lookahead.following_token_type,
                lookahead.next_is_declaration_start,
            )?
        {
            return Ok(type_expression_id);
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

        let Ok(identifier_span) = self.peek().map(|token| token.span) else {
            return false;
        };

        let next_token = self.next_token();
        let next_identifier_span = if next_token.token.ty == TokenType::Identifier {
            Some(next_token.span)
        } else {
            None
        };
        let identifier = self.file.span_str(identifier_span);
        let next_identifier = next_identifier_span.map(|span| self.file.span_str(span));

        self.type_literal_identifier_str(identifier, next_identifier)
            .is_some()
    }

    /// Return whether the current identifier is followed by a value postfix token.
    #[inline]
    fn current_identifier_is_followed_by_value_postfix(&mut self) -> bool {
        matches!(
            self.next_token_type(),
            TokenType::Dot
                | TokenType::OpenBracket
                | TokenType::OpenParenthesis
                | TokenType::LessThan
                | TokenType::ShiftLeft
                | TokenType::Maybe
        )
    }

    /// Return true when the current identifier should be parsed as a contextual type literal.
    #[inline]
    fn should_try_contextual_type_literal(&mut self) -> bool {
        if self.flags.is_in_type() || self.flags.is_in_static() {
            return true;
        }

        // contextual type literals in value positions are only enabled in the extended grammar
        if !self.language.is_destack() {
            return false;
        }

        if self.current_identifier_is_followed_by_value_postfix() {
            return false;
        }

        self.is_contextual_type_literal_identifier()
    }

    /// Parse one grouped inner expression and consume the closing `)`.
    fn eat_parenthesized_inner_expression(&mut self) -> ParseResult<LocalNodeId<Expression>> {
        let mut inner_flags = self.flags.nested().with_parenthesis(true);
        inner_flags.set_allow_sequence_expression(true);

        let expression_id = self.eat_expression(inner_flags)?;
        if !self.preserves_parenthesized_wrappers() {
            let close_parenthesis_start = self.peek()?.span.start;
            self.set_node_trailing_span(expression_id, close_parenthesis_start);
        }
        self.eat_close_token_or_recover_missing(TokenType::CloseParenthesis, NodeType::Expression)?;

        Ok(expression_id)
    }

    /// Parse one grouped inner type expression and consume the closing `)`.
    fn eat_type_parenthesized_inner_expression(
        &mut self,
    ) -> ParseResult<LocalNodeId<TypeExpression>> {
        let mut inner_flags = self.flags.nested().with_parenthesis(true).in_type();
        inner_flags.set_allow_sequence_expression(true);

        let expression_id = self.with_flags(inner_flags, |parser| parser.eat_type_expression())?;
        if !self.preserves_parenthesized_wrappers() {
            let close_parenthesis_start = self.peek()?.span.start;
            self.set_node_trailing_span(expression_id, close_parenthesis_start);
        }
        self.eat_close_token_or_recover_missing(
            TokenType::CloseParenthesis,
            NodeType::TypeExpression,
        )?;

        Ok(expression_id)
    }

    /// Finish a parenthesized expression after the inner value has been parsed.
    fn finish_parenthesized_expression(
        &mut self,
        start: &ParserSpanStart,
        expression_id: LocalNodeId<Expression>,
    ) -> LocalNodeId<Expression> {
        // wrapped type expressions keep a type-space parenthesized node
        if let Some(inner_expression_id) = self.wrapped_type_expression_maybe(expression_id) {
            if !self.preserves_parenthesized_wrappers() {
                self.tree.set_side_span(
                    inner_expression_id,
                    NodeSpanType::Region(NodeSpanRegion::Wrapper),
                    self.get_span_from(start),
                );

                return self.wrap_type_expression(inner_expression_id);
            }

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

        if !self.preserves_parenthesized_wrappers() {
            self.tree.set_side_span(
                expression_id,
                NodeSpanType::Region(NodeSpanRegion::Wrapper),
                self.get_span_from(start),
            );

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
        start: &ParserSpanStart,
        expression_id: LocalNodeId<TypeExpression>,
    ) -> LocalNodeId<TypeExpression> {
        if !self.preserves_parenthesized_wrappers() {
            self.tree.set_side_span(
                expression_id,
                NodeSpanType::Region(NodeSpanRegion::Wrapper),
                self.get_span_from(start),
            );

            return expression_id;
        }

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
        if self.flags.is_in_arrow_return_type() && !has_parenthesized_parameter_group {
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
                && !self.flags.is_in_before_type()
                && !self.flags.is_in_match_case();
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
            && self.lookahead(|parser| {
                parser.bump();
                parser.peek_is(TokenType::Colon)
            });
        let starts_spread_tuple_element =
            self.language.is_destack() && self.peek_is(TokenType::Spread);

        starts_named_tuple_element || starts_spread_tuple_element || group.has_top_level_comma
    }

    /// Return whether the current parenthesized group starts a function type.
    fn can_start_parenthesized_function_type(&mut self) -> bool {
        if !self.peek_is(TokenType::OpenParenthesis) {
            return false;
        }

        let mark = self.checkpoint();
        let tree_mark = self.tree.next_id();
        let can_start = self
            .can_start_parenthesized_function_type_inner()
            .unwrap_or(false);
        self.restore(mark, tree_mark);

        can_start
    }

    /// Parse the current parenthesized head enough to identify function types.
    fn can_start_parenthesized_function_type_inner(&mut self) -> ParseResult<bool> {
        // open parameter list
        self.bump();

        // empty parameter list
        if self.peek_is(TokenType::CloseParenthesis) {
            self.bump();

            return Ok(matches!(
                self.peek_token_type(),
                TokenType::Arrow | TokenType::ArrowWide
            ));
        }

        // rest parameter
        if self.peek_is(TokenType::Spread) {
            return Ok(true);
        }

        // first parameter head
        if self.eat_function_type_parameter_head().is_err() {
            return Ok(false);
        }

        // annotated, optional, defaulted, or followed by more parameters
        if matches!(
            self.peek_token_type(),
            TokenType::Colon | TokenType::Comma | TokenType::Maybe | TokenType::Assign
        ) {
            return Ok(true);
        }

        // bare single parameter
        if !self.peek_is(TokenType::CloseParenthesis) {
            return Ok(false);
        }
        self.bump();

        Ok(!self.flags.is_in_arrow_return_type()
            && matches!(
                self.peek_token_type(),
                TokenType::Arrow | TokenType::ArrowWide
            ))
    }

    /// Eat one function type parameter head.
    fn eat_function_type_parameter_head(&mut self) -> ParseResult<()> {
        self.eat_binding_modifiers_prefix_maybe(
            true,
            false,
            self.flags.is_in_static(),
            false,
            true,
        )?;

        let starts_shared_pattern = matches!(
            self.peek_token_type(),
            TokenType::OpenBracket | TokenType::OpenBrace
        );
        let starts_destack_pattern = self.language.is_destack()
            && (self.peek_is(TokenType::OpenParenthesis) || self.peek_identifier_str_is("_"));
        if starts_shared_pattern || starts_destack_pattern {
            self.eat_pattern()?;
            return Ok(());
        }

        self.eat_binding_identifier_with_span()?;

        Ok(())
    }

    /// Try to parse one parenthesized lambda declaration from one known group shape.
    fn try_eat_parenthesized_lambda_declaration_from_group(
        &mut self,
        start: &ParserSpanStart,
        group: ParenthesizedGroupShape,
    ) -> ParseResult<Option<LocalNodeId<Declaration>>> {
        // require one lambda follow token first
        let Some(follow_token_type) = self.parenthesized_group_lambda_follow_token_maybe(group)
        else {
            return Ok(None);
        };
        let is_colon_lambda = follow_token_type == TokenType::Colon;

        // ternary conditions need one speculative parse to keep `?:` honest
        if is_colon_lambda && self.flags.is_in_ternary_condition() {
            let speculative_start = self.checkpoint();
            let speculative_start_idx = self.tree.next_id();
            if let Ok(lambda_id) =
                self.eat_function(start, DeclarationHeader::default(), false, false)
            {
                let has_ternary_delimiter =
                    self.peek_is(TokenType::Colon) || self.next_token_type() == TokenType::Colon;
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
        start: &ParserSpanStart,
        group: ParenthesizedGroupShape,
    ) -> ParseResult<Option<LocalNodeId<Expression>>> {
        // parse the shared lambda declaration form
        let lambda_id = self.try_eat_parenthesized_lambda_declaration_from_group(start, group)?;

        Ok(lambda_id.map(|lambda_id| self.insert_declaration_expression(start, lambda_id)))
    }

    /// Try to parse a parenthesized function type in strict type space.
    fn try_eat_parenthesized_function_type(
        &mut self,
        start: &ParserSpanStart,
        group: ParenthesizedGroupShape,
    ) -> ParseResult<Option<LocalNodeId<TypeExpression>>> {
        // conditional type branches own a following `:`
        if group.follow_token_type == Some(TokenType::Colon)
            && self.flags.is_in_type_conditional_right()
        {
            return Ok(None);
        }

        if !matches!(
            group.follow_token_type,
            Some(TokenType::Arrow | TokenType::ArrowWide | TokenType::Colon)
        ) {
            return Ok(None);
        }

        if !self.can_start_parenthesized_function_type() {
            return Ok(None);
        }

        let type_expression_id =
            self.eat_function_type_expression(start, DeclarationHeader::default(), false, false)?;

        Ok(Some(type_expression_id))
    }

    /// Parse a parenthesized primary expression from one known group shape.
    fn eat_parenthesized_primary_from_group(
        &mut self,
        start: &ParserSpanStart,
        group: ParenthesizedGroupShape,
    ) -> ParseResult<ParsedExpression> {
        // base grouped expressions can skip the full lambda path
        let can_parse_plain_group_directly = !self.language.is_destack()
            && !self.flags.is_in_arrow_return_type()
            && group.follow_token_type.is_none();

        if !can_parse_plain_group_directly
            && let Some(lambda_expression_id) =
                self.try_eat_parenthesized_lambda_from_group(start, group)?
        {
            return Ok(ParsedExpression::plain(lambda_expression_id));
        }

        // consume the grouped body
        self.bump(); // eat open parenthesis
        let open_parenthesis_end = self.prev_token_end();

        // empty tuple or sequence when we immediately see a closing parenthesis
        if self.peek_is(TokenType::CloseParenthesis) {
            return Ok(ParsedExpression::plain(
                self.eat_empty_parenthesized_primary(start),
            ));
        }

        // tuple when we see a named element or top level comma
        if self.parenthesized_group_has_tuple_shape(group) {
            return Ok(ParsedExpression::plain(
                self.eat_comma_parenthesized_primary(start)?,
            ));
        }

        // tuple or parenthesized expression for the remaining cases
        let expression_id = self.eat_parenthesized_inner_expression()?;
        self.set_node_leading_span(expression_id, open_parenthesis_end);
        let parenthesized_id = self.finish_parenthesized_expression(start, expression_id);
        let is_parenthesized = !self.preserves_parenthesized_wrappers();

        Ok(ParsedExpression {
            expression_id: parenthesized_id,
            is_parenthesized,
        })
    }

    /// Eat one empty parenthesized primary expression after `(` has been consumed.
    fn eat_empty_parenthesized_primary(
        &mut self,
        start: &ParserSpanStart,
    ) -> LocalNodeId<Expression> {
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
        start: &ParserSpanStart,
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
        start: &ParserSpanStart,
    ) -> ParseResult<LocalNodeId<Expression>> {
        let tuple_elements = self
            .eat_sequence_literal_body(None, TokenType::CloseParenthesis)
            .for_node_type(NodeType::Expression)?;
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
        start: &ParserSpanStart,
    ) -> ParseResult<LocalNodeId<TypeExpression>> {
        let elements = self.eat_type_tuple_elements_body(TokenType::CloseParenthesis)?;
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
        start: &ParserSpanStart,
    ) -> ParseResult<ParsedExpression> {
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
        start: &ParserSpanStart,
    ) -> ParseResult<LocalNodeId<TypeExpression>> {
        let group = self.parenthesized_group_shape()?;

        // parse function types before consuming the grouped body
        if let Some(type_expression_id) = self.try_eat_parenthesized_function_type(start, group)? {
            return Ok(type_expression_id);
        }

        // consume the grouped body
        self.bump(); // eat open parenthesis
        let open_parenthesis_end = self.prev_token_end();

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
        start: &ParserSpanStart,
        expression_decorators: &mut PendingDecorators,
    ) -> ParseResult<Option<LocalNodeId<Expression>>> {
        // decorators treat keywords as identifiers, so skip label parsing there
        if self.flags.is_in_decorator()
            || self.flags.is_in_match_case()
            || !self.peek_is(TokenType::Identifier)
            || !self.can_parse_labelled_expression()
        {
            return Ok(None);
        }

        let (label, label_span, body) = self.eat_labelled_expression_parts()?;

        // attach decorators to the labelled expression
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
        start: &ParserSpanStart,
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
        start: &ParserSpanStart,
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
        start: &ParserSpanStart,
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

        // operand
        let expression_id =
            self.eat_type_leading_elementwise_constituent(leading_binary_operator)?;
        let operand_span = self.tree.get_span(expression_id);
        let operand_head_span = self.type_expression_head_span(expression_id);

        // leading operators always create the visible chain head
        let expression_id = match leading_binary_operator {
            BinaryOperator::ElementwiseOr => self.insert_node(
                TypeExpression::Union {
                    elements: vec![expression_id],
                },
                operand_span,
            ),

            BinaryOperator::ElementwiseAnd => self.insert_node(
                TypeExpression::Intersection {
                    elements: vec![expression_id],
                },
                operand_span,
            ),

            _ => unreachable!(),
        };

        // the container owns the explicit leading prefix
        let full_span = self.get_span_from(start);
        let leading_span = Span::new(full_span.file, full_span.start, operand_span.start);

        self.tree.set_side_span(
            expression_id,
            NodeSpanType::Boundary(NodeSpanBoundary::Leading),
            leading_span,
        );
        self.tree.set_side_span(
            expression_id,
            NodeSpanType::Boundary(NodeSpanBoundary::LeadingOperator),
            Span::new(full_span.file, full_span.start, separator_end),
        );

        // preserve the explicit leading separator without widening the main span
        self.tree.set_head_span(expression_id, operand_head_span);

        Ok(expression_id)
    }

    /// Eat the first constituent after a leading union or intersection operator.
    fn eat_type_leading_elementwise_constituent(
        &mut self,
        operator: BinaryOperator,
    ) -> ParseResult<LocalNodeId<TypeExpression>> {
        let mut expression_context = self
            .flags
            .not_in_position()
            .in_left_precedence(operator.precedence());
        if self.flags.is_in_type_conditional_right() {
            expression_context = expression_context.in_type_conditional_right();
        }

        let ambient_context = self.flags.with_type(true);
        self.eat_type_expression_or_recover_missing(
            self.flags
                .with_ambient_context(ambient_context)
                .with_expression_context(expression_context),
            NodeType::Expression,
        )
    }

    /// Eat one signed scalar literal type.
    fn eat_type_signed_scalar_literal_expression(
        &mut self,
        start: &ParserSpanStart,
        operator: UnaryOperator,
    ) -> ParseResult<LocalNodeId<TypeExpression>> {
        let operator_start = self.span_start();
        self.bump(); // eat unary operator
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
        start: &ParserSpanStart,
        operator: UnaryOperator,
    ) -> ParseResult<LocalNodeId<Expression>> {
        let operator_start = self.span_start();
        self.bump(); // eat unary operator
        let operator_span = self.get_span_from(&operator_start);

        let mut right_flags = self
            .flags
            .not_in_position()
            .in_left_precedence(operator.precedence());
        if self.flags.is_in_type_conditional_right() {
            right_flags = right_flags.in_type_conditional_right();
        }

        let right = self.eat_expression_with_context_unchecked(right_flags)?;
        let expression = Expression::Unary { operator, right };

        let expression_id = self.insert_node(expression, self.get_span_from(start));
        self.tree.set_main_span(expression_id, operator_span);

        Ok(expression_id)
    }

    /// Eat one type-space unary prefix expression.
    fn eat_type_unary_prefix_expression(
        &mut self,
        start: &ParserSpanStart,
        operator: TypeUnaryOperator,
    ) -> ParseResult<LocalNodeId<TypeExpression>> {
        let operator_start = self.span_start();
        self.bump(); // eat type unary operator
        let operator_span = self.get_span_from(&operator_start);

        let mut right_flags = self
            .flags
            .not_in_position()
            .in_left_precedence(operator.precedence());
        if self.flags.is_in_type_conditional_right() {
            right_flags = right_flags.in_type_conditional_right();
        }
        if self.flags.is_disallow_type_conditional() {
            right_flags = right_flags.disallow_type_conditional();
        }

        let expression = match operator {
            TypeUnaryOperator::Typeof => {
                let value = self.eat_expression_or_recover_missing(
                    self.flags
                        .with_type(false)
                        .with_expression_context(right_flags),
                    NodeType::Expression,
                )?;

                TypeExpression::TypeOfValue { value }
            }
            TypeUnaryOperator::Keyof => {
                let target_type = self.eat_type_expression_or_recover_missing(
                    self.flags
                        .with_type(true)
                        .with_expression_context(right_flags),
                    NodeType::Expression,
                )?;

                TypeExpression::KeyOf { target_type }
            }
            TypeUnaryOperator::Readonly => {
                let target_type = self.eat_type_expression_or_recover_missing(
                    self.flags
                        .with_type(true)
                        .with_expression_context(right_flags),
                    NodeType::Expression,
                )?;

                TypeExpression::Readonly { target_type }
            }
            TypeUnaryOperator::Shared => {
                let target_type = self.eat_type_expression_or_recover_missing(
                    self.flags
                        .with_type(true)
                        .with_expression_context(right_flags),
                    NodeType::Expression,
                )?;

                TypeExpression::Shared { target_type }
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
        start: &ParserSpanStart,
        token_type: TokenType,
    ) -> ParseResult<LocalNodeId<TypeExpression>> {
        let is_value_of = token_type == TokenType::ElementwiseXor;

        self.bump(); // eat ^ or &
        let mutability = self.eat_reference_mutability_maybe()?;
        let variance = self.eat_variance_bound_maybe()?;
        let target_type = self.eat_type_expression_or_recover_missing(
            self.flags.not_in_position().with_type(true),
            NodeType::Expression,
        )?;

        let type_expression = if is_value_of {
            TypeExpression::OwnedOf {
                mutability,
                variance,
                target_type,
            }
        } else {
            TypeExpression::BorrowedOf {
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
        start: &ParserSpanStart,
        token_type: TokenType,
    ) -> ParseResult<LocalNodeId<Expression>> {
        let is_value_of = token_type == TokenType::ElementwiseXor;

        self.bump(); // eat ^ or &
        let mutability = self.eat_reference_mutability_maybe()?;
        let variance = self.eat_variance_bound_maybe()?;
        let right = self.eat_expression_with_context_unchecked(self.flags.not_in_position())?;
        let expression = if is_value_of {
            Expression::MoveOf {
                mutability,
                variance,
                right,
            }
        } else {
            Expression::BorrowOf {
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
        start: &ParserSpanStart,
    ) -> ParseResult<LocalNodeId<TypeExpression>> {
        self.eat_token(TokenType::OpenBracket)?;

        // empty bracket tuple
        if self.peek_is(TokenType::CloseBracket) {
            self.eat_close_token_or_recover_missing(
                TokenType::CloseBracket,
                NodeType::TypeExpression,
            )?;

            return Ok(self.insert_node(
                TypeExpression::ArrayTuple { elements: vec![] },
                self.get_span_from(start),
            ));
        }

        // tuple-only heads
        if self.starts_type_tuple_head() {
            let elements = self.eat_type_tuple_elements_body(TokenType::CloseBracket)?;
            self.eat_close_token_or_recover_missing(
                TokenType::CloseBracket,
                NodeType::TypeExpression,
            )?;

            return Ok(self.insert_node(
                TypeExpression::ArrayTuple { elements },
                self.get_span_from(start),
            ));
        }

        // first bracket element
        let element_start = self.span_start();
        let element = match self.eat_type_expression_or_recover_missing(
            self.flags.not_in_position().in_type(),
            NodeType::TypeExpression,
        ) {
            Ok(element) => element,
            Err(error) => {
                let elements =
                    self.recover_type_tuple_head(&element_start, TokenType::CloseBracket, error)?;
                self.eat_close_token_or_recover_missing(
                    TokenType::CloseBracket,
                    NodeType::TypeExpression,
                )?;

                return Ok(self.insert_node(
                    TypeExpression::ArrayTuple { elements },
                    self.get_span_from(start),
                ));
            }
        };

        // fixed array: `[T; N]`
        if self.language.is_destack() && self.peek_is(TokenType::Semicolon) {
            self.bump(); // eat ;
            let value_ambient_context = self.flags.with_type(false);
            let length_context = self
                .flags
                .not_in_position()
                .not_in_left_precedence()
                .not_in_sequence_expression();
            let length_flags = self
                .flags
                .with_ambient_context(value_ambient_context)
                .with_expression_context(length_context);
            let length =
                self.eat_expression_or_recover_missing(length_flags, NodeType::Expression)?;
            self.eat_close_token_or_recover_missing(
                TokenType::CloseBracket,
                NodeType::TypeExpression,
            )?;

            return Ok(self.insert_node(
                TypeExpression::FixedArray { element, length },
                self.get_span_from(start),
            ));
        }

        // slice: `[T]`
        let is_slice_close = self.peek_is(TokenType::CloseBracket);
        let is_slice_missing_close = self.language.is_destack()
            && !self.peek_is(TokenType::Comma)
            && !self.peek_is(TokenType::Maybe)
            && self.is_type_expression_boundary();
        if self.language.is_destack() && (is_slice_close || is_slice_missing_close) {
            self.eat_close_token_or_recover_missing(
                TokenType::CloseBracket,
                NodeType::TypeExpression,
            )?;

            return Ok(
                self.insert_node(TypeExpression::Slice { element }, self.get_span_from(start))
            );
        }

        // tuple continuation after a first unlabeled element
        let elements =
            self.eat_type_tuple_tail(&element_start, TokenType::CloseBracket, element)?;
        self.eat_close_token_or_recover_missing(TokenType::CloseBracket, NodeType::TypeExpression)?;

        Ok(self.insert_node(
            TypeExpression::ArrayTuple { elements },
            self.get_span_from(start),
        ))
    }

    /// Eat one value-space bracket primary expression.
    fn eat_value_bracket_primary_expression(
        &mut self,
        start: &ParserSpanStart,
    ) -> ParseResult<LocalNodeId<Expression>> {
        self.eat_bracket_literal_expression(start)
    }

    /// Eat one type-space brace primary expression.
    fn eat_type_brace_primary_expression(
        &mut self,
        start: &ParserSpanStart,
    ) -> ParseResult<LocalNodeId<TypeExpression>> {
        if self.can_start_type_mapped_expression() {
            return self.eat_type_mapped_expression();
        }

        let members = self.with_flags(self.flags.not_in_position(), |parser| {
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
        start: &ParserSpanStart,
    ) -> ParseResult<LocalNodeId<Expression>> {
        let can_start_object_literal = !self.flags.is_in_statement_position()
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

        let properties = self.with_flags(self.flags.not_in_position(), |parser| {
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
        start: &ParserSpanStart,
    ) -> ParseResult<ParsedExpression> {
        let token_type = self.peek_token_type();

        match token_type {
            // grouped primaries
            TokenType::OpenParenthesis => self.eat_parenthesized_primary(start),

            // value or reference families
            TokenType::ElementwiseXor if self.language.is_destack() => self
                .eat_value_reference_or_value_of_expression(start, token_type)
                .map(ParsedExpression::plain),
            TokenType::ElementwiseAnd if self.language.is_destack() => self
                .eat_value_reference_or_value_of_expression(start, token_type)
                .map(ParsedExpression::plain),
            // collections and blocks
            TokenType::OpenBracket => self
                .eat_value_bracket_primary_expression(start)
                .map(ParsedExpression::plain),
            TokenType::OpenBrace => self
                .eat_value_brace_primary_expression(start)
                .map(ParsedExpression::plain),

            // `<...>` ambiguities
            TokenType::LessThan if self.can_start_generic_arrow_expression() => {
                let function_id =
                    self.eat_function(start, DeclarationHeader::default(), false, false)?;

                Ok(ParsedExpression::plain(
                    self.insert_declaration_expression(start, function_id),
                ))
            }
            TokenType::LessThan if self.can_start_tree_literal() => self
                .with_flags(self.flags.not_in_position(), |parser| {
                    parser.eat_tree_literal()
                })
                .map(ParsedExpression::plain),

            // literal families
            TokenType::TemplateString | TokenType::TemplateStringStart
                if self.is_template_literal_start() =>
            {
                let template_literal = self.eat_template_literal()?;

                Ok(ParsedExpression::plain(self.insert_node(
                    Expression::TemplateExpression {
                        value: template_literal,
                    },
                    self.get_span_from(start),
                )))
            }
            TokenType::Divide | TokenType::DivideAssign => {
                let scalar_literal = self.eat_regex_literal()?;

                Ok(ParsedExpression::plain(self.insert_node(
                    Expression::ScalarLiteral(scalar_literal),
                    self.get_span_from(start),
                )))
            }
            TokenType::Literal if self.is_scalar_literal_start() => {
                let scalar_literal = self.eat_scalar_literal()?;

                Ok(ParsedExpression::plain(self.insert_node(
                    Expression::ScalarLiteral(scalar_literal),
                    self.get_span_from(start),
                )))
            }

            // contextual literals and private identifiers
            TokenType::Not if self.peek_type_literal().is_ok() => {
                let type_literal = self.eat_type_literal(None)?;
                let type_expression_id = self.insert_node(
                    TypeExpression::Literal {
                        value: type_literal,
                    },
                    self.get_span_from(start),
                );

                Ok(ParsedExpression::plain(
                    self.wrap_type_expression(type_expression_id),
                ))
            }
            TokenType::Hash
                if self.lookahead(|parser| {
                    parser.bump();
                    parser.peek_is(TokenType::Identifier)
                }) =>
            {
                let hash_span = self.peek()?.span;
                let identifier_span = self.next_token().span;
                if hash_span.end != identifier_span.start {
                    return Err(ParseError::unexpected(identifier_span));
                }

                self.bump(); // eat #
                let (name, name_span) = self.eat_identifier_with_span()?;
                let expression_id = self.insert_node(
                    Expression::PrivateIdentifier { name },
                    self.get_span_from(start),
                );
                self.tree.set_main_span(expression_id, name_span);

                Ok(ParsedExpression::plain(expression_id))
            }

            // generic unary prefixes
            _ if self.peek_unary_prefix_operator_maybe().is_some() => {
                let operator = self.peek_unary_prefix_operator_maybe().unwrap();
                self.eat_unary_prefix_expression(start, operator)
                    .map(ParsedExpression::plain)
            }
            _ if self.peek_type_unary_prefix_operator_maybe().is_some() => {
                let operator = self.peek_type_unary_prefix_operator_maybe().unwrap();
                let type_expression_id = self.eat_type_unary_prefix_expression(start, operator)?;

                Ok(ParsedExpression::plain(
                    self.wrap_type_expression(type_expression_id),
                ))
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
        start: &ParserSpanStart,
    ) -> ParseResult<LocalNodeId<TypeExpression>> {
        let token_type = self.peek_token_type();

        match token_type {
            // grouped primaries and generic function signatures
            TokenType::OpenParenthesis => self.eat_type_parenthesized_primary(start),
            TokenType::LessThan if self.can_start_generic_arrow_expression() => {
                self.eat_function_type_expression(start, DeclarationHeader::default(), false, false)
            }

            // type pointers
            TokenType::Multiply => {
                self.bump(); // eat *
                let mutability = self.eat_reference_mutability_maybe()?;
                let target_type = self.eat_type_expression_or_recover_missing(
                    self.flags.not_in_position().with_type(true),
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
                self.eat_type_template_literal_expression()
            }
            TokenType::Literal if self.is_scalar_literal_start() => {
                let scalar_literal = self.eat_scalar_literal()?;

                Ok(self.insert_node(
                    TypeExpression::ScalarLiteral {
                        value: scalar_literal,
                    },
                    self.get_span_from(start),
                ))
            }
            TokenType::Not if self.peek_type_literal().is_ok() => {
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
        if !self.peek_is(TokenType::Maybe) {
            return Ok(left_type_id);
        }

        // nested conditional right sides and constrained infer parses stop before the outer `?`
        if self.flags.is_in_type_conditional_right() || self.flags.is_disallow_type_conditional() {
            return Ok(left_type_id);
        }

        // tuple element optionals belong to the surrounding tuple parser
        let next_token_type = self.next_token_type();
        let is_tuple_optional =
            matches!(next_token_type, TokenType::Comma | TokenType::CloseBracket);
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
        start: &ParserSpanStart,
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
            if !self.flags.is_in_decorator() && self.peek_is(TokenType::At) {
                self.eat_decorators_maybe()?
            } else {
                PendingDecorators::new()
            };

        // capture expression span
        let start = self.span_start();

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
            if !self.flags.is_in_decorator() && self.peek_is(TokenType::At) {
                self.eat_decorators_maybe()?
            } else {
                PendingDecorators::new()
            };

        // capture expression span and scanner cursor metadata
        let start = self.span_start();

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
        let left_expression = if token_type == TokenType::Identifier {
            ParsedExpression::plain(
                self.eat_identifier_primary_expression(&start, &mut expression_decorators)?,
            )
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
            left_expression.expression_id,
        );

        // parse postfix and infix continuation for the primary expression
        let expression_id = self.eat_expression_continuation(
            &start,
            left_expression.expression_id,
            left_expression.is_parenthesized,
        )?;
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
