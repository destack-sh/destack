use super::common::{
    DECLARATION_START_TOKENS, DeclarationHeader, DescriptorHead, is_type_relation_keyword,
};
use super::lookahead::DelimiterAnalysis;
use crate::parse::parser::ParserOptions;
use crate::parse::prelude::*;
use crate::{ParseError, ParseResult, Parser, ParserMark};
use destack_source::Span;

use destack_ast::{
    BinaryOperator, BlockContext, Declaration, DependencyItem, Expression, Keyword, LocalNodeId,
    NodeType, ScalarLiteral, TokenType, TypeExpression, TypeUnaryOperator, UnaryOperator,
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

impl Parser {
    /// Return true when one identifier start is really a contextual declaration head.
    fn plain_identifier_starts_contextual_declaration(
        &mut self,
        pos_index: usize,
        next_raw_index: usize,
        next_raw_token_type: TokenType,
    ) -> bool {
        // contextual declaration identifiers use the declaration entry path
        let is_global_identifier = self.is_global_identifier_at(pos_index);
        let is_module_identifier =
            self.language.supports_module_declaration() && self.is_module_identifier_at(pos_index);
        if !is_global_identifier && !is_module_identifier {
            return false;
        }

        let next_cursor = self.scanner_cursor_from(next_raw_index);
        let next_token_type = next_cursor.token_type;
        let next_token_index = next_cursor.index;

        if is_global_identifier
            && matches!(
                next_token_type,
                TokenType::OpenBrace | TokenType::Identifier | TokenType::Literal
            )
        {
            return true;
        }

        is_module_identifier
            && !next_cursor.has_line_break_before
            && DECLARATION_START_TOKENS.contains(&next_token_type)
            && matches!(next_token_type, TokenType::Identifier | TokenType::Literal)
            && (next_token_type != TokenType::Identifier
                || !is_type_relation_keyword(self.keyword_for_index(next_token_index)))
            && next_raw_token_type == next_token_type
    }

    /// Eat one qualified identifier path in type or static space.
    fn eat_qualified_identifier_expression_path(
        &mut self,
        start: &ParserMark,
    ) -> ParseResult<LocalNodeId<Expression>> {
        // dotted reference path
        let (path, segment_spans) = self.eat_path_with_segment_spans()?;
        let generic_arguments =
            if self.peek_is(TokenType::LessThan) || self.peek_is(TokenType::ShiftLeft) {
                self.try_eat_generic_arguments(true, false)
                    .unwrap_or_default()
            } else {
                Vec::new()
            };

        // type space lowers to a wrapped reference node
        if self.options.is_in_type() {
            let type_expression_id = self.insert_node(
                TypeExpression::Reference {
                    path,
                    generic_arguments,
                },
                self.get_span_from(start),
            );
            let expression_id = self.insert_type_expression_value(type_expression_id);
            self.set_path_expression_spans(expression_id, &segment_spans);

            if let Some(main_span) = self.tree.get_main_span(expression_id) {
                self.tree.set_main_span(type_expression_id, main_span);
            }

            if let Some(head_span) = self.tree.get_head_span(expression_id) {
                self.tree.set_head_span(type_expression_id, head_span);
            }

            return Ok(expression_id);
        }

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

    /// Eat one identifier-led expression head.
    pub(crate) fn eat_identifier_expression_path(
        &mut self,
        start: &ParserMark,
    ) -> ParseResult<LocalNodeId<Expression>> {
        let _identifier_timing = self.timing_scope(tags::PARSE_EXPRESSION_PRIMARY_IDENTIFIER);

        // non value spaces lower the full path immediately
        if self.options.is_in_type() || self.options.is_in_static() {
            return self.eat_qualified_identifier_expression_path(start);
        }

        // destack value-space identifiers may still commit to a type head
        if self.language.is_destack() {
            let speculative_start = self.mark();
            let speculative_start_idx = self.tree.next_id();

            let speculative_expression_id = self.with_options(self.options.in_type(), |parser| {
                parser.eat_qualified_identifier_expression_path(start)
            });
            let speculative_expression_id = match speculative_expression_id {
                Ok(expression_id) => self.with_options(self.options.in_type(), |parser| {
                    parser.eat_type_postfix_continuation(start, expression_id)
                }),
                Err(_) => {
                    self.restore(speculative_start, speculative_start_idx);
                    return self.eat_value_identifier_expression(start);
                }
            };
            let speculative_expression_id = match speculative_expression_id {
                Ok(expression_id) => expression_id,
                Err(_) => {
                    self.restore(speculative_start, speculative_start_idx);
                    return self.eat_value_identifier_expression(start);
                }
            };
            let speculative_type_expression_id =
                self.expect_type_expression_value(speculative_expression_id)?;

            // before-block heads commit to type-space when `extends` follows
            let lowers_before_block_extends = self.options.is_in_before_block()
                && self.peek_is(TokenType::Identifier)
                && self.peek_any_keyword().ok() == Some(Keyword::Extends);

            // tagged object literal receivers also commit to type-space
            let lowers_tagged_object_literal = !self.options.is_in_before_block()
                && self.peek_is(TokenType::OpenBrace)
                && self.can_start_tagged_object_literal_type(speculative_type_expression_id);

            self.restore(speculative_start, speculative_start_idx);

            if lowers_before_block_extends || lowers_tagged_object_literal {
                return self.with_options(self.options.in_type(), |parser| {
                    let expression_id = parser.eat_qualified_identifier_expression_path(start)?;
                    parser.eat_type_postfix_continuation(start, expression_id)
                });
            }
        }

        // value contexts keep the bare identifier head
        self.eat_value_identifier_expression(start)
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

        self.stats.record_with_options_call();

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

        // type-space plain identifiers still yield to contextual literals, operators, and
        // declaration heads that need the full primary parser
        if self.options.is_in_type() {
            if self.should_try_contextual_type_literal() {
                return Ok(None);
            }

            if matches!(
                self.keyword_for_index(self.pos_index()),
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
        }

        // value-space plain identifiers yield to lambda heads, labels, and contextual declarations
        if !self.options.is_in_type() {
            if self.keyword_for_index(self.pos_index()).is_some()
                || self.should_try_contextual_type_literal()
            {
                return Ok(None);
            }

            let pos_index = self.pos_index();
            let next_raw_index = self.index_for_next();
            let next_raw_token_type = self.token_type_at(next_raw_index);
            if matches!(next_raw_token_type, TokenType::Arrow | TokenType::ArrowWide)
                || self.options.is_in_statement_position()
                    && next_raw_token_type == TokenType::Colon
            {
                return Ok(None);
            }

            if self.plain_identifier_starts_contextual_declaration(
                pos_index,
                next_raw_index,
                next_raw_token_type,
            ) {
                return Ok(None);
            }
        }

        let expression_id = self.eat_identifier_expression_path(start)?;
        let expression_id = self.eat_expression_continuation(start, expression_id)?;

        Ok(Some(expression_id))
    }

    /// Return one contextual type literal expression when the current token sequence allows it.
    fn try_eat_contextual_type_literal_expression(
        &mut self,
        start: &ParserMark,
    ) -> ParseResult<Option<LocalNodeId<Expression>>> {
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

        Ok(Some(self.insert_type_expression_value(type_expression_id)))
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

    /// Return one contextual keyword for an identifier-start primary expression.
    fn identifier_primary_keyword(&mut self) -> Option<Keyword> {
        let keyword = self.keyword_for_index(self.pos_index());

        // decorators only keep the small keyword subset that still behaves like operators or heads
        if self.options.is_in_decorator() && !self.options.is_in_type() {
            return match keyword {
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
            };
        }

        // typeof queries treat some identifiers as plain names
        if self.options.is_in_typeof_query() {
            return if matches!(keyword, Some(Keyword::Type | Keyword::Readonly)) {
                None
            } else {
                keyword
            };
        }

        keyword
    }

    /// Return true when the current identifier starts a contextual module declaration.
    fn identifier_starts_module_declaration(
        &mut self,
        next_token_type: TokenType,
        next_token_index: usize,
        next_has_line_break: bool,
        is_declaration_start: bool,
    ) -> bool {
        // module declarations only exist in value space
        if self.options.is_in_decorator()
            || self.options.is_in_type()
            || next_has_line_break
            || !is_declaration_start
            || !self.language.supports_module_declaration()
            || !self.is_module_identifier_at(self.pos_index())
        {
            return false;
        }

        // module names must not collide with relation keywords
        if !matches!(next_token_type, TokenType::Identifier | TokenType::Literal) {
            return false;
        }

        let next_keyword = if next_token_type == TokenType::Identifier {
            self.keyword_for_index(next_token_index)
        } else {
            None
        };

        !is_type_relation_keyword(next_keyword)
    }

    /// Try to parse one shorthand identifier lambda expression.
    fn try_eat_identifier_lambda_expression(
        &mut self,
        start: &ParserMark,
        header: DeclarationHeader,
        next_raw_token_type: TokenType,
    ) -> ParseResult<Option<LocalNodeId<Expression>>> {
        // shorthand lambdas only exist in value space
        if self.options.is_in_type() || self.options.is_in_match_case() {
            return Ok(None);
        }

        if !matches!(next_raw_token_type, TokenType::Arrow | TokenType::ArrowWide) {
            return Ok(None);
        }

        let lambda_id = self.eat_function(start, header, false, false)?;

        Ok(Some(self.insert_declaration_expression(start, lambda_id)))
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
    ) -> ParseResult<LocalNodeId<Expression>> {
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
                let value = self.eat_expression_or_recover_missing(
                    self.options
                        .with_ambient_context(right_ambient_context)
                        .with_expression_context(right_expression_context.with_type(false)),
                    NodeType::Expression,
                )?;

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
        let expression_id = self.insert_type_expression_value(type_expression_id);
        self.tree.set_main_span(type_expression_id, operator_span);
        self.tree.set_main_span(expression_id, operator_span);

        Ok(expression_id)
    }

    /// Parse one identifier-start primary expression after the caller selected the identifier arm.
    ///
    /// Examples:
    /// ```
    /// foo
    /// foo.bar
    /// foo<T>
    /// async () => value
    /// function named() {}
    /// type Result<T> = Ok<T> | Err
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
        let next_raw_index = self.index_for_next();
        let next_raw_token_type = self.token_type_at(next_raw_index);
        let next_cursor = self.scanner_cursor_from(next_raw_index);
        let next_token_type = next_cursor.token_type;
        let next_token_index = next_cursor.index;
        let next_has_line_break = next_cursor.has_line_break_before;
        let is_declaration_start = DECLARATION_START_TOKENS.contains(&next_token_type);

        // contextual keyword and declaration shape
        let keyword = self.identifier_primary_keyword();
        let is_unary_keyword = matches!(keyword, Some(Keyword::Typeof | Keyword::Void));
        let is_type_unary_keyword = matches!(keyword, Some(Keyword::Typeof | Keyword::Keyof));
        let is_module_declaration_start = self.identifier_starts_module_declaration(
            next_token_type,
            next_token_index,
            next_has_line_break,
            is_declaration_start,
        );

        // shorthand lambda form
        if let Some(expression_id) =
            self.try_eat_identifier_lambda_expression(start, header, next_raw_token_type)?
        {
            return Ok(expression_id);
        }

        // plain identifier path or contextual literal
        if keyword.is_none() && !self.options.is_in_decorator() && !is_module_declaration_start {
            if let Some(expression_id) = self.try_eat_contextual_type_literal_expression(start)? {
                return Ok(expression_id);
            }

            return self.eat_identifier_expression_path(start);
        }

        // unary keyword expressions
        if is_unary_keyword && !self.options.is_in_type() {
            return self.eat_value_unary_keyword_primary_expression(
                start,
                keyword.expect("checked unary keyword"),
            );
        }

        // type unary keyword expressions
        if is_type_unary_keyword {
            return self.eat_type_unary_keyword_primary_expression(
                start,
                keyword.expect("checked type unary keyword"),
            );
        }

        // do block expression or do while
        if keyword == Some(Keyword::Do) {
            if self.is_do_while_statement(next_token_type) {
                return self.eat_while();
            }

            if next_token_type == TokenType::OpenBrace {
                let block_id = self.eat_block(BlockContext::Expression)?;
                let expression_id =
                    self.insert_node(Expression::Block(block_id), self.get_span_from(start));

                return Ok(expression_id);
            }
        }

        // contextual module declaration
        if keyword.is_none() && is_module_declaration_start {
            let namespace_id = self.eat_namespace(start, header)?;
            return Ok(self.insert_declaration_expression(start, namespace_id));
        }

        // keyword expressions and declaration starters
        if let Some(keyword) = keyword {
            let _keyword_timing = self.timing_scope(tags::PARSE_EXPRESSION_PRIMARY_KEYWORD);
            if let Some(expression_id) = self.eat_keyword_expression(
                start,
                header,
                keyword,
                next_token_type,
                next_token_index,
                next_has_line_break,
                next_raw_token_type,
                is_declaration_start,
            )? {
                return Ok(expression_id);
            }
        }

        // contextual type literal
        if let Some(expression_id) = self.try_eat_contextual_type_literal_expression(start)? {
            return Ok(expression_id);
        }

        // plain identifier path
        self.eat_identifier_expression_path(start)
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
    ) -> ParseResult<LocalNodeId<Expression>> {
        let mut inner_options = self.options.nested().with_parenthesis(true);
        inner_options.set_allow_sequence_expression(true);
        if preserve_type_context && self.options.is_in_type() {
            inner_options = inner_options.in_type();
        }

        let expression_id = self.eat_expression(inner_options)?;
        self.eat_newlines_maybe()?;
        self.eat_close_token_or_recover_missing(TokenType::CloseParenthesis, NodeType::Expression)?;

        Ok(expression_id)
    }

    /// Finish a parenthesized expression after the inner value has been parsed.
    fn finish_parenthesized_expression(
        &mut self,
        start: &ParserMark,
        expression_id: LocalNodeId<Expression>,
        preserve_type_context: bool,
    ) -> LocalNodeId<Expression> {
        // parenthesized type expressions stay in type space
        if preserve_type_context
            && let Some(inner_expression_id) = self.expression_type_value_maybe(expression_id)
        {
            let parenthesized_id = self.insert_node(
                TypeExpression::Parenthesized {
                    expression: inner_expression_id,
                },
                self.get_span_from(start),
            );
            let expression_id = self.insert_type_expression_value(parenthesized_id);
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

    /// Try to parse a parenthesized lambda from one known group shape.
    fn try_eat_parenthesized_lambda_from_group(
        &mut self,
        start: &ParserMark,
        group: DelimiterAnalysis,
    ) -> ParseResult<Option<LocalNodeId<Expression>>> {
        let has_parenthesized_parameter_group =
            group.has_top_level_parameter_colon || group.has_top_level_comma || group.is_empty;
        let can_parse_lambda_in_arrow_return = !self.options.is_in_arrow_return_type()
            || self.options.is_in_type() && has_parenthesized_parameter_group;
        let Some(follow_token_type) = group.follow_token_type else {
            return Ok(None);
        };

        let is_colon_lambda = follow_token_type == TokenType::Colon;
        let is_parenthesized_arrow_value =
            is_colon_lambda && group.starts_with_nested_parenthesis && group.has_top_level_arrow;
        let is_colon_lambda_allowed = !is_colon_lambda
            || (self.language.is_destack() || self.language.is_typescript())
                && !self.options.is_in_before_type()
                && !self.options.is_in_match_case()
                && (!self.options.is_in_type() || !group.has_top_level_comma);
        if !can_parse_lambda_in_arrow_return
            || !is_colon_lambda_allowed
            || is_parenthesized_arrow_value
        {
            return Ok(None);
        }

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
                        (declaration.body.is_some() || self.options.is_in_type())
                            && has_ternary_delimiter
                    }
                    _ => has_ternary_delimiter,
                };
                if should_accept {
                    return Ok(Some(self.insert_declaration_expression(start, lambda_id)));
                }
            }

            self.restore(speculative_start, speculative_start_idx);
            return Ok(None);
        }

        let lambda_id = self.eat_function(start, DeclarationHeader::default(), false, false)?;

        Ok(Some(self.insert_declaration_expression(start, lambda_id)))
    }

    /// Parse a parenthesized primary expression from one known group shape.
    fn eat_parenthesized_primary_from_group(
        &mut self,
        start: &ParserMark,
        group: DelimiterAnalysis,
    ) -> ParseResult<LocalNodeId<Expression>> {
        let is_plain_js_or_ts_group = !self.language.is_destack()
            && !self.options.is_in_type()
            && !self.options.is_in_arrow_return_type()
            && group.follow_token_type.is_none();

        // plain js and ts groups
        self.stats.record_parenthesized_expression_plain_call();
        if is_plain_js_or_ts_group {
            self.stats.record_parenthesized_expression_plain_hit();
        } else {
            self.stats.record_parenthesized_expression_plain_miss();
        }

        if !is_plain_js_or_ts_group
            && let Some(lambda_expression_id) =
                self.try_eat_parenthesized_lambda_from_group(start, group)?
        {
            return Ok(lambda_expression_id);
        }

        self.bump(); // eat open parenthesis
        self.eat_newlines_maybe()?;

        // empty tuple or sequence when we immediately see a closing parenthesis
        if self.peek_is(TokenType::CloseParenthesis) {
            return Ok(self.eat_empty_parenthesized_primary(start));
        }

        // tuple when we see a named element or top level comma
        if (self.language.is_destack()
            && self.peek_is(TokenType::Identifier)
            && self.peek_next_is(TokenType::Colon))
            || group.has_top_level_comma
        {
            return self.eat_comma_parenthesized_primary(start);
        }

        // tuple or parenthesized expression for the remaining cases
        let preserve_type_context = !is_plain_js_or_ts_group;
        let expression_id = self.eat_parenthesized_inner_expression(preserve_type_context)?;
        let parenthesized_id =
            self.finish_parenthesized_expression(start, expression_id, preserve_type_context);

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

        // parenthesized comma groups become tuple types in type space
        if self.options.is_in_type() {
            let elements = self.build_tuple_elements_from_arguments(&tuple_elements)?;
            let tuple_id = self.insert_node(
                TypeExpression::Tuple { elements },
                self.get_span_from(start),
            );

            return Ok(self.insert_type_expression_value(tuple_id));
        }

        let tuple_id = self.insert_node(
            Expression::TupleExpression {
                elements: tuple_elements,
            },
            self.get_span_from(start),
        );

        Ok(tuple_id)
    }

    /// Parse an open parenthesis primary expression.
    fn eat_parenthesized_primary(
        &mut self,
        start: &ParserMark,
    ) -> ParseResult<LocalNodeId<Expression>> {
        let _group_timing = self.timing_scope(tags::PARSE_EXPRESSION_PRIMARY_GROUP);
        let group = self.parenthesized_group_shape()?;
        self.eat_parenthesized_primary_from_group(start, group)
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
        if self.options.is_in_type() {
            self.eat_newlines_maybe()?;
        }

        // operand
        let expression_id = self.eat_expression_in_scope()?;

        // value space only accepts explicit elementwise chains
        let expression = self.tree.get(expression_id);
        if !matches!(
            expression,
            Expression::Binary { operator, .. }
                if *operator == leading_binary_operator
        ) && !self.options.is_in_type()
        {
            return Err(ParseError::unexpected(self.get_span_from(start)));
        }

        // preserve the full root span and explicit leading separator
        let head_span = self.binary_chain_head_span(expression_id, leading_binary_operator);
        self.tree.set_span(expression_id, self.get_span_from(start));
        self.tree.set_head_span(expression_id, head_span);

        if let Expression::Type { value } = self.tree.get(expression_id) {
            let value = *value;
            self.tree.set_span(value, self.get_span_from(start));
            self.tree.set_head_span(value, head_span);
        }

        Ok(expression_id)
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
        if self.options.is_in_type()
            && let Some(TypeExpression::ScalarLiteral { value }) = self
                .expression_type_value_maybe(right)
                .map(|right| self.tree.get(right).clone())
        {
            let signed_literal = match (operator, value) {
                (UnaryOperator::Plus, value) => Some(value),
                (UnaryOperator::Negate, ScalarLiteral::Integer(value)) => {
                    value.checked_neg().map(ScalarLiteral::Integer)
                }
                (UnaryOperator::Negate, ScalarLiteral::Bigint(value)) => {
                    value.checked_neg().map(ScalarLiteral::Bigint)
                }
                (UnaryOperator::Negate, ScalarLiteral::Float(value)) => {
                    Some(ScalarLiteral::Float(-value))
                }
                _ => None,
            };

            if let Some(value) = signed_literal {
                let type_expression_id = self.insert_node(
                    TypeExpression::ScalarLiteral { value },
                    self.get_span_from(start),
                );
                let expression_id = self.insert_type_expression_value(type_expression_id);
                self.tree.set_main_span(type_expression_id, operator_span);
                self.tree.set_main_span(expression_id, operator_span);

                return Ok(expression_id);
            }

            return Err(ParseError::unexpected(self.tree.get_span(right)));
        }

        let expression_id = self.insert_node(expression, self.get_span_from(start));
        self.tree.set_main_span(expression_id, operator_span);

        Ok(expression_id)
    }

    /// Eat one type-space unary prefix expression.
    fn eat_type_unary_prefix_expression(
        &mut self,
        start: &ParserMark,
        operator: TypeUnaryOperator,
    ) -> ParseResult<LocalNodeId<Expression>> {
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
        let expression_id = self.insert_type_expression_value(type_expression_id);
        self.tree.set_main_span(type_expression_id, operator_span);
        self.tree.set_main_span(expression_id, operator_span);

        Ok(expression_id)
    }

    /// Eat one Destack `^` or `&` prefix expression family.
    fn eat_reference_or_value_of_expression(
        &mut self,
        start: &ParserMark,
        token_type: TokenType,
    ) -> ParseResult<LocalNodeId<Expression>> {
        let is_value_of = token_type == TokenType::ElementwiseXor;

        self.bump(); // eat ^ or &
        let mutability = self.eat_reference_mutability_maybe()?;
        let variance = self.eat_variance_bound_maybe()?;

        if self.options.is_in_type() {
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
            let type_expression_id = self.insert_node(type_expression, self.get_span_from(start));

            return Ok(self.insert_type_expression_value(type_expression_id));
        }

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

    /// Eat one bracket primary expression family.
    fn eat_bracket_primary_expression(
        &mut self,
        start: &ParserMark,
    ) -> ParseResult<LocalNodeId<Expression>> {
        let elements = self.with_options(self.options.not_in_position(), |parser| {
            parser.eat_array_literal()
        })?;

        if self.options.is_in_type() {
            let elements = self.build_tuple_elements_from_arguments(&elements)?;
            let type_expression_id = self.insert_node(
                TypeExpression::Tuple { elements },
                self.get_span_from(start),
            );

            return Ok(self.insert_type_expression_value(type_expression_id));
        }

        Ok(self.insert_node(
            Expression::ArrayExpression { elements },
            self.get_span_from(start),
        ))
    }

    /// Eat one brace primary expression family.
    fn eat_brace_primary_expression(
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

        let is_value_object_literal = self.options.is_in_statement_position()
            && self.can_parse_object_literal_in_statement_position();
        if self.options.is_in_type() && !is_value_object_literal {
            if self.can_start_type_mapped_expression() {
                return self.eat_type_mapped_expression();
            }

            let properties = self.with_options(self.options.not_in_position(), |parser| {
                parser.eat_type_object_literal()
            })?;
            let type_expression_id = self.insert_node(
                TypeExpression::Object { properties },
                self.get_span_from(start),
            );

            return Ok(self.insert_type_expression_value(type_expression_id));
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
            // grouped primaries and type pointers
            TokenType::OpenParenthesis => self.eat_parenthesized_primary(start),
            TokenType::Multiply if self.options.is_in_type() => {
                self.bump(); // eat *
                let mutability = self.eat_reference_mutability_maybe()?;
                let target_type = self.eat_type_expression_or_recover_missing(
                    self.options.not_in_position().with_type(true),
                    NodeType::Expression,
                )?;
                let type_expression_id = self.insert_node(
                    TypeExpression::PointerOf {
                        mutability,
                        target_type,
                    },
                    self.get_span_from(start),
                );

                Ok(self.insert_type_expression_value(type_expression_id))
            }

            // destack reference families
            TokenType::ElementwiseXor if self.language.is_destack() => {
                self.eat_reference_or_value_of_expression(start, token_type)
            }
            TokenType::ElementwiseAnd if self.language.is_destack() => {
                self.eat_reference_or_value_of_expression(start, token_type)
            }

            // collections and blocks
            TokenType::OpenBracket => self.eat_bracket_primary_expression(start),
            TokenType::OpenBrace => self.eat_brace_primary_expression(start),

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
                if self.options.is_in_type() {
                    let type_expression_id = self.eat_type_template_literal_expression()?;

                    Ok(self.insert_type_expression_value(type_expression_id))
                } else {
                    let template_literal = self.eat_template_literal()?;

                    Ok(self.insert_node(
                        Expression::TemplateExpression {
                            value: template_literal,
                        },
                        self.get_span_from(start),
                    ))
                }
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
                if self.options.is_in_type() {
                    let type_expression_id = self.insert_node(
                        TypeExpression::ScalarLiteral {
                            value: scalar_literal,
                        },
                        self.get_span_from(start),
                    );

                    Ok(self.insert_type_expression_value(type_expression_id))
                } else {
                    Ok(self.insert_node(
                        Expression::ScalarLiteral(scalar_literal),
                        self.get_span_from(start),
                    ))
                }
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

                Ok(self.insert_type_expression_value(type_expression_id))
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
                self.eat_type_unary_prefix_expression(start, operator)
            }

            // no primary matched
            _ => Err(ParseError::unexpected(self.peek()?.span)),
        }
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
