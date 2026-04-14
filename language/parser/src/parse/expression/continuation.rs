use crate::parse::parser::{NonNewlineTokenCursor, ParserOptions};
use crate::parse::prelude::*;
use crate::{ParseError, ParseResult, Parser, ParserMark};

use super::operator::ParseInfixOperator;
use destack_ast::{
    Argument, AssignOperator, BinaryOperator, Declaration, Expression, FunctionDeclaration,
    FunctionKind, GenericArgument, IfCondition, IfKind, Keyword, LiteralType, LocalNodeId,
    NodeType, PostfixPosition, TokenType, TypeBinaryOperator, TypeExpression, TypeUnaryOperator,
    UnaryOperator,
};
use destack_source::Span;

/// The normalized token facts for one continuation step.
#[derive(Clone, Copy, Debug)]
struct ContinuationToken {
    /// The token index after skipping leading newlines.
    index: usize,
    /// The token type after newline normalization.
    token_type: TokenType,
    /// Whether newline tokens were skipped before the token.
    has_pending_newlines: bool,
    /// Whether a line break exists before the token.
    has_line_break_before: bool,
    /// The number of skipped newline tokens.
    skipped_newline_count: usize,
}

/// The postfix grammar space for continuation scanning.
#[derive(Clone, Copy, Debug)]
enum PostfixSpace {
    /// Value-space postfix parsing.
    Value,
    /// Type-space postfix parsing.
    Type,
}

impl Parser {
    /// Eat one postfix `as comptime` operator and return its span when present.
    fn eat_as_comptime_postfix_operator_maybe(&mut self) -> ParseResult<Option<Span>> {
        let Some(operator) = self.peek_type_unary_postfix_operator_maybe() else {
            return Ok(None);
        };

        let operator_start = self.mark_span();
        self.bump(); // eat type unary operator
        if operator == TypeUnaryOperator::AsComptime {
            self.eat_newlines_maybe()?;
            self.bump(); // eat second token
        }
        let operator_span = self.get_span_from(&operator_start);

        if operator != TypeUnaryOperator::AsComptime {
            return Err(ParseError::unexpected(operator_span));
        }

        Ok(Some(operator_span))
    }

    /// Return one normalized continuation token at the current parser position.
    fn next_continuation_token_maybe(&mut self) -> Option<ContinuationToken> {
        let mut index = self.pos_index();
        self.ensure_token(index);
        let token = self.tokens().get(index)?;
        let mut token_type = token.token.ty;
        if token_type == TokenType::End {
            return None;
        }

        let mut skipped_newline_count = 0;
        let mut has_pending_newlines = false;
        let mut has_line_break_before = self.lexer.materialized_line_terminator_before(index);
        if token_type == TokenType::Newline {
            let cursor = self.scanner_cursor_from(index);
            token_type = cursor.token_type;
            if token_type == TokenType::End {
                return None;
            }

            index = cursor.index;
            skipped_newline_count = cursor.skipped_newline_count;
            has_pending_newlines = true;
            has_line_break_before = cursor.has_line_break_before;
        }

        Some(ContinuationToken {
            index,
            token_type,
            has_pending_newlines,
            has_line_break_before,
            skipped_newline_count,
        })
    }

    /// Align the parser cursor with one normalized continuation token.
    #[inline]
    fn advance_to_continuation_token(&mut self, token: ContinuationToken) {
        if token.has_pending_newlines {
            self.advance_to(token.index);
        }
    }

    /// Return one postfix continuation token after applying newline and boundary rules.
    fn next_postfix_continuation_token(
        &mut self,
        space: PostfixSpace,
        is_in_static: bool,
        is_in_ternary_or_match: bool,
    ) -> Option<ContinuationToken> {
        let token = self.next_continuation_token_maybe()?;
        let token_type = token.token_type;

        // some postfix forms may cross a newline, but only for specific tokens
        if token.has_pending_newlines {
            let can_continue_after_newline = match space {
                PostfixSpace::Value => matches!(
                    token_type,
                    TokenType::OpenParenthesis
                        | TokenType::Dot
                        | TokenType::Maybe
                        | TokenType::LessThan
                        | TokenType::ShiftLeft
                ),
                PostfixSpace::Type => token_type == TokenType::Dot,
            };
            if !can_continue_after_newline {
                return None;
            }
        }

        // postfix parsing must not consume ternary or match boundaries
        if is_in_ternary_or_match && token_type == TokenType::Colon {
            return None;
        }

        // static contexts stop before `>` closers
        if is_in_static && Self::starts_type_angle_close(token_type) {
            return None;
        }

        Some(token)
    }

    /// Return whether one statement expression must stop before continuation parsing.
    fn statement_expression_stops_continuation(
        &mut self,
        left_expression_id: LocalNodeId<Expression>,
    ) -> bool {
        if !self.options.is_in_statement_position() {
            return false;
        }

        let next_token_type = self.peek_token_type();
        let expression = self.tree.get(left_expression_id);
        let is_continuable_lambda_declaration = matches!(
            expression,
            Expression::Declaration(declaration_id)
                if matches!(
                    self.tree.get(*declaration_id),
                    Declaration::Function(FunctionDeclaration { signature, .. })
                        if signature.kind == FunctionKind::Lambda
                )
        ) && !matches!(
            next_token_type,
            TokenType::Newline | TokenType::Semicolon | TokenType::CloseBrace | TokenType::End
        );

        expression.is_statement_boundary() && !is_continuable_lambda_declaration
    }

    /// Return whether a type expression can start a tagged object literal postfix.
    #[inline]
    pub(super) fn can_start_tagged_object_literal_type(
        &self,
        type_expression_id: LocalNodeId<TypeExpression>,
    ) -> bool {
        match self.tree.get(type_expression_id) {
            TypeExpression::Parenthesized { expression } => {
                self.can_start_tagged_object_literal_type(*expression)
            }
            TypeExpression::Declaration { .. } | TypeExpression::Reference { .. } => true,
            TypeExpression::Member { left, .. } => self.can_start_tagged_object_literal_type(*left),
            _ => false,
        }
    }

    /// Return whether a newline direct call should terminate in statement position.
    #[inline]
    fn newline_direct_call_terminates_statement(
        &mut self,
        left_expression_id: LocalNodeId<Expression>,
        open_parenthesis_index: usize,
    ) -> bool {
        if !self.options.is_in_statement_context() {
            return false;
        }

        // break and continue cannot continue into newline-prefixed calls
        if matches!(
            self.tree.get(left_expression_id),
            Expression::Break { .. } | Expression::Continue { .. }
        ) {
            return true;
        }

        let close_parenthesis_index = self.find_matching_close_maybe(
            Some(open_parenthesis_index as u32),
            TokenType::OpenParenthesis,
            TokenType::CloseParenthesis,
        );
        let Some(close_parenthesis_index) = close_parenthesis_index else {
            return false;
        };

        let next_token_index = close_parenthesis_index as usize + 1;
        let next_token_type = self.token_type_at(next_token_index);
        let is_postfix_or_assign = UnaryOperator::from_postfix_token(next_token_type).is_some()
            || AssignOperator::from_token(next_token_type).is_some();
        let starts_lambda_head = matches!(
            next_token_type,
            TokenType::Arrow | TokenType::ArrowWide | TokenType::Colon
        );

        // postfix and assign continuations force statement termination:
        // `expr\n(arg).member` remains one continued expression
        // lambda-head follows cannot continue a direct call receiver safely
        is_postfix_or_assign || starts_lambda_head
    }

    /// Return true when assignment lhs form is invalid in ts/js grammar.
    #[inline]
    fn assignment_target_has_invalid_form(&self, expression_id: LocalNodeId<Expression>) -> bool {
        let inner_expression_id = self.without_parentheses_expression(expression_id);
        let is_parenthesized = inner_expression_id != expression_id;

        match self.tree.get(inner_expression_id) {
            // `satisfies` lhs is valid in parse output only when parenthesized
            Expression::Satisfies { .. } => !is_parenthesized,

            // `as` cast lhs is valid only when parenthesized
            Expression::As { .. } => !is_parenthesized,

            // all other lhs forms are handled by assignment-target validation later
            _ => false,
        }
    }

    /// Eat one value-space dot postfix continuation when the parser is already positioned at `.`.
    ///
    /// Examples:
    /// ```
    /// value.method
    /// value?.<T>()
    /// value.[index]
    /// value.!
    /// 0..toString()
    /// ```
    fn eat_value_dot_postfix_continuation(
        &mut self,
        start: &ParserMark,
        left_expression_id: LocalNodeId<Expression>,
    ) -> ParseResult<Option<LocalNodeId<Expression>>> {
        let dot_index = self.pos_index();
        let next_raw_index = dot_index.saturating_add(1);
        let next_token_type = self.token_type_at(next_raw_index);
        let next_cursor = self.scanner_cursor_from(next_raw_index);
        let next_token_type_after_newlines = next_cursor.token_type;

        // indirect calls stay in value space
        if next_token_type_after_newlines == TokenType::OpenParenthesis {
            let _call_timing = self.timing_scope(tags::PARSE_EXPRESSION_POSTFIX_CALL);
            self.bump(); // eat .
            self.eat_newlines_maybe()?;
            let expression_id =
                self.eat_call(left_expression_id, None, PostfixPosition::Indirect)?;

            return Ok(Some(expression_id));
        }

        // indirect instantiation or generic call
        if matches!(
            next_token_type_after_newlines,
            TokenType::LessThan | TokenType::ShiftLeft
        ) {
            // generic postfixes are only valid on compatible receivers
            if !self.can_start_postfix_generic_arguments(left_expression_id) {
                return Ok(None);
            }

            // indirect generic postfixes require optional chaining receivers
            if !matches!(self.tree.get(left_expression_id), Expression::Maybe { .. }) {
                return Ok(None);
            }

            // generic call or instantiation
            let expression_id =
                self.try_eat_postfix_generic_application(start, left_expression_id, true)?;

            return Ok(expression_id);
        }

        // indirect indexing stays in value space
        if next_token_type_after_newlines == TokenType::OpenBracket {
            self.bump(); // eat .
            self.eat_newlines_maybe()?;
            let expression_id = self.eat_index(left_expression_id, PostfixPosition::Indirect)?;

            return Ok(Some(expression_id));
        }

        // optional chaining stays in value space
        if next_token_type == TokenType::Maybe {
            self.bump(); // eat .
            self.bump(); // eat ?
            let expression_id = self.insert_node(
                Expression::Maybe {
                    left: left_expression_id,
                    position: PostfixPosition::Indirect,
                },
                self.get_span_from(start),
            );

            return Ok(Some(expression_id));
        }

        // `value.!`
        if next_token_type == TokenType::Not {
            self.bump(); // eat .
            self.bump(); // eat !
            let expression_id = self.insert_node(
                Expression::Must {
                    position: PostfixPosition::Indirect,
                    left: left_expression_id,
                },
                self.get_span_from(start),
            );

            return Ok(Some(expression_id));
        }

        // preserve a committed member access when the name slot is missing
        if Self::is_expression_slot_boundary_token(next_token_type_after_newlines) {
            self.bump(); // eat .
            self.eat_newlines_maybe()?;
            self.report_unexpected_for_here(NodeType::Expression);
            let expression_id = self.insert_node(
                Expression::Member {
                    left: left_expression_id,
                    name: None,
                    generic_arguments: vec![],
                },
                self.get_span_from(start),
            );

            return Ok(Some(expression_id));
        }

        // preserve a committed private member access when `#` has no identifier
        if next_token_type == TokenType::Hash {
            let private_name_index =
                self.first_non_newline_index_from(next_raw_index.saturating_add(1));
            let private_name_token_type = self.token_type_at(private_name_index);
            if Self::is_expression_slot_boundary_token(private_name_token_type) {
                self.bump(); // eat .
                self.bump(); // eat #
                self.eat_newlines_maybe()?;
                self.report_unexpected_for_here(NodeType::Expression);
                let expression_id = self.insert_node(
                    Expression::PrivateMember {
                        left: left_expression_id,
                        name: None,
                        generic_arguments: vec![],
                    },
                    self.get_span_from(start),
                );

                return Ok(Some(expression_id));
            }
        }

        let Some((member_index, is_private_member)) =
            self.peek_dot_member_target(dot_index, next_cursor, left_expression_id)
        else {
            return Ok(None);
        };

        let member_distance = member_index
            .saturating_sub(self.pos_index())
            .saturating_add(1);
        let member_distance = u8::try_from(member_distance).unwrap_or(u8::MAX);

        self.bump(); // eat .
        if self.pos_index() != member_index {
            self.advance_to(member_index);
        }

        // private members remain in value space
        if is_private_member {
            self.bump(); // eat #
            let (name, name_span) = self.eat_identifier_with_span()?;
            let generic_arguments = self
                .try_eat_generic_arguments(false, false)
                .unwrap_or_default();
            let expression_id = self.insert_value_private_member_expression(
                start,
                left_expression_id,
                name,
                name_span,
                generic_arguments,
            );

            return Ok(Some(expression_id));
        }

        if self.invalid_decimal_integer_member_access(left_expression_id, member_distance) {
            return Err(ParseError::unexpected(self.prev().expect("peeked").span));
        }

        let (name, name_span) = self.eat_member_name_with_span()?;
        let generic_arguments = self
            .try_eat_generic_arguments(false, false)
            .unwrap_or_default();
        let expression_id = self.insert_value_member_expression(
            start,
            left_expression_id,
            name,
            name_span,
            generic_arguments,
        );

        Ok(Some(expression_id))
    }

    /// Insert one canonical value member expression.
    fn insert_value_member_expression(
        &mut self,
        start: &ParserMark,
        left: LocalNodeId<Expression>,
        name: destack_source::StringId,
        name_span: Span,
        generic_arguments: Vec<LocalNodeId<GenericArgument>>,
    ) -> LocalNodeId<Expression> {
        let member_id = self.insert_node(
            Expression::Member {
                left,
                name: Some(name),
                generic_arguments: vec![],
            },
            self.get_span_from(start),
        );
        self.tree.set_main_span(member_id, name_span);

        if generic_arguments.is_empty() {
            return member_id;
        }

        self.insert_node(
            Expression::Instantiation {
                left: member_id,
                generic_arguments,
            },
            self.get_span_from(start),
        )
    }

    /// Insert one canonical private member expression.
    fn insert_value_private_member_expression(
        &mut self,
        start: &ParserMark,
        left: LocalNodeId<Expression>,
        name: destack_source::StringId,
        name_span: Span,
        generic_arguments: Vec<LocalNodeId<GenericArgument>>,
    ) -> LocalNodeId<Expression> {
        let member_id = self.insert_node(
            Expression::PrivateMember {
                left,
                name: Some(name),
                generic_arguments: vec![],
            },
            self.get_span_from(start),
        );
        self.tree.set_main_span(member_id, name_span);

        if generic_arguments.is_empty() {
            return member_id;
        }

        self.insert_node(
            Expression::Instantiation {
                left: member_id,
                generic_arguments,
            },
            self.get_span_from(start),
        )
    }

    /// Eat one type-space dot postfix continuation when the parser is already positioned at `.`.
    ///
    /// Examples:
    /// ```
    /// T.Item
    /// T.<U>
    /// T.!
    /// 0..Member
    /// ```
    fn eat_type_dot_postfix_continuation(
        &mut self,
        start: &ParserMark,
        left_expression_id: LocalNodeId<Expression>,
    ) -> ParseResult<Option<LocalNodeId<Expression>>> {
        let dot_index = self.pos_index();
        let next_raw_index = dot_index.saturating_add(1);
        let next_token_type = self.token_type_at(next_raw_index);
        let next_cursor = self.scanner_cursor_from(next_raw_index);
        let next_token_type_after_newlines = next_cursor.token_type;

        // indirect instantiation or generic call
        if matches!(
            next_token_type_after_newlines,
            TokenType::LessThan | TokenType::ShiftLeft
        ) {
            if !self.can_start_postfix_generic_arguments(left_expression_id) {
                return Ok(None);
            }

            return self.try_eat_postfix_generic_application(start, left_expression_id, true);
        }

        // `T.!`
        if next_token_type == TokenType::Not {
            self.bump(); // eat .
            self.bump(); // eat !
            let target_type = self.expect_type_expression_value(left_expression_id)?;
            let expression_id = self.insert_node(
                TypeExpression::Must { target_type },
                self.get_span_from(start),
            );

            return Ok(Some(self.insert_type_expression_value(expression_id)));
        }

        // preserve a committed type projection when the member slot is missing
        if Self::is_expression_slot_boundary_token(next_token_type_after_newlines) {
            self.bump(); // eat .
            self.eat_newlines_maybe()?;
            self.report_unexpected_for_here(NodeType::TypeExpression);
            let error_id = self.insert_node(TypeExpression::Error, self.get_span_from(start));

            return Ok(Some(self.insert_type_expression_value(error_id)));
        }

        let Some((member_index, is_private_member)) =
            self.peek_dot_member_target(dot_index, next_cursor, left_expression_id)
        else {
            return Ok(None);
        };

        let member_distance = member_index
            .saturating_sub(self.pos_index())
            .saturating_add(1);
        let member_distance = u8::try_from(member_distance).unwrap_or(u8::MAX);

        self.bump(); // eat .
        if self.pos_index() != member_index {
            self.advance_to(member_index);
        }

        // private members are not valid in type space
        if is_private_member {
            if self.peek_is(TokenType::Hash) {
                self.bump(); // eat #
            }
            let _ = self.eat_identifier_with_span()?;
            let _ = self
                .try_eat_generic_arguments(false, false)
                .unwrap_or_default();
            self.report_unexpected_for_here(NodeType::TypeExpression);
            let error_id = self.insert_node(TypeExpression::Error, self.get_span_from(start));

            return Ok(Some(self.insert_type_expression_value(error_id)));
        }

        if self.invalid_decimal_integer_member_access(left_expression_id, member_distance) {
            return Err(ParseError::unexpected(self.prev().expect("peeked").span));
        }

        let (name, name_span) = self.eat_member_name_with_span()?;
        let generic_arguments = self
            .try_eat_generic_arguments(false, false)
            .unwrap_or_default();
        let left_type = self.expect_type_expression_value(left_expression_id)?;
        let member_id = self.insert_node(
            TypeExpression::Member {
                left: left_type,
                name,
                generic_arguments,
            },
            self.get_span_from(start),
        );
        self.tree.set_main_span(member_id, name_span);

        Ok(Some(self.insert_type_expression_value(member_id)))
    }

    /// Eat one value-space postfix `?` or `as comptime` continuation.
    fn eat_value_postfix_operator_continuation(
        &mut self,
        start: &ParserMark,
        left_expression_id: LocalNodeId<Expression>,
        is_destack_language: bool,
    ) -> ParseResult<Option<LocalNodeId<Expression>>> {
        // `value as comptime`
        if let Some(operator_span) = self.eat_as_comptime_postfix_operator_maybe()? {
            let expression_id = self.insert_node(
                Expression::Comptime {
                    body: left_expression_id,
                },
                self.get_span_from(start),
            );
            self.tree.set_main_span(expression_id, operator_span);

            return Ok(Some(expression_id));
        }

        if !self.peek_is(TokenType::Maybe) {
            return Ok(None);
        }

        let is_optional_chain_after_maybe = self.is_optional_chain_after_maybe();
        let is_direct_postfix_maybe = is_destack_language
            && (self.is_next_any_stop() && self.prev_token_type() != TokenType::Newline
                || self.is_next_any_close_parenthesis()
                || self.peek_next_assign_operator_is());
        if !is_optional_chain_after_maybe && !is_direct_postfix_maybe {
            return Ok(None);
        }

        self.bump(); // eat ?
        let expression_id = self.insert_node(
            Expression::Maybe {
                left: left_expression_id,
                position: PostfixPosition::Direct,
            },
            self.get_span_from(start),
        );

        Ok(Some(expression_id))
    }

    /// Eat one type-space postfix `!` or `as comptime` continuation.
    fn eat_type_postfix_operator_continuation(
        &mut self,
        start: &ParserMark,
        left_expression_id: LocalNodeId<Expression>,
    ) -> ParseResult<Option<LocalNodeId<Expression>>> {
        // `T as comptime`
        if let Some(operator_span) = self.eat_as_comptime_postfix_operator_maybe()? {
            let target_type = self.expect_type_expression_value(left_expression_id)?;
            let expression_id = self.insert_node(
                TypeExpression::AsComptime { target_type },
                self.get_span_from(start),
            );
            self.tree.set_main_span(expression_id, operator_span);

            return Ok(Some(self.insert_type_expression_value(expression_id)));
        }

        if !self.peek_is(TokenType::Not) {
            return Ok(None);
        }

        self.bump(); // eat !
        let target_type = self.expect_type_expression_value(left_expression_id)?;
        let expression_id = self.insert_node(
            TypeExpression::Must { target_type },
            self.get_span_from(start),
        );

        Ok(Some(self.insert_type_expression_value(expression_id)))
    }

    /// Try to eat one tagged object literal postfix.
    fn try_eat_tagged_object_literal_postfix(
        &mut self,
        start: &ParserMark,
        left_expression_id: LocalNodeId<Expression>,
    ) -> ParseResult<Option<LocalNodeId<Expression>>> {
        if !self.language.is_destack()
            || !self.peek_is(TokenType::OpenBrace)
            || self.options.is_in_before_block()
        {
            return Ok(None);
        }

        let left_expression_id = self.without_parentheses_expression(left_expression_id);
        let Some(ty) = self.expression_type_value_maybe(left_expression_id) else {
            return Ok(None);
        };

        let properties = self.eat_object_literal()?;
        if !self.can_start_tagged_object_literal_type(ty) {
            return Err(ParseError::unexpected(
                self.tree.get_span(left_expression_id),
            ));
        }

        let expression_id = self.insert_node(
            Expression::ObjectExpression {
                ty: Some(ty),
                properties,
            },
            self.get_span_from(start),
        );

        Ok(Some(expression_id))
    }

    /// Try to eat one direct value call postfix.
    fn try_eat_value_call_postfix(
        &mut self,
        left_expression_id: LocalNodeId<Expression>,
        token: ContinuationToken,
        has_statement_boundary_newline: bool,
        is_in_new_receiver: bool,
    ) -> ParseResult<Option<LocalNodeId<Expression>>> {
        // maybe receivers and `new` receivers do not take direct calls here
        let left_is_maybe = matches!(self.tree.get(left_expression_id), Expression::Maybe { .. });
        if left_is_maybe || is_in_new_receiver {
            return Ok(None);
        }

        // newline direct calls may terminate the statement instead
        let terminates_statement = has_statement_boundary_newline
            && self.newline_direct_call_terminates_statement(left_expression_id, token.index);
        if terminates_statement {
            return Ok(None);
        }

        // unparenthesized lambdas need a separator before direct calls
        let left_is_unparenthesized_lambda =
            self.is_unparenthesized_lambda_expression(left_expression_id);
        if left_is_unparenthesized_lambda && has_statement_boundary_newline {
            return Ok(None);
        }

        if left_is_unparenthesized_lambda {
            return Err(ParseError::unexpected(self.peek()?.span));
        }

        let _call_timing = self.timing_scope(tags::PARSE_EXPRESSION_POSTFIX_CALL);
        self.advance_to_continuation_token(token);
        let expression_id = self.eat_call(left_expression_id, None, PostfixPosition::Direct)?;

        Ok(Some(expression_id))
    }

    /// Eat one tuple or sequence postfix inside parenthesis.
    fn eat_parenthesized_sequence_postfix(
        &mut self,
        start: &ParserMark,
        left_expression_id: LocalNodeId<Expression>,
    ) -> ParseResult<LocalNodeId<Expression>> {
        self.bump(); // eat comma
        self.eat_newlines_maybe()?;

        if self.language.is_destack() {
            let first_element_id = self.insert_node(
                Argument::Positional {
                    value: left_expression_id,
                },
                self.get_span_from(start),
            );
            self.stats.record_with_options_call();
            let tuple_elements = self.with_options(self.options.not_in_position(), |parser| {
                parser
                    .eat_sequence_literal_body(Some(first_element_id), TokenType::CloseParenthesis)
            })?;

            return Ok(self.insert_node(
                Expression::TupleExpression {
                    elements: tuple_elements,
                },
                self.get_span_from(start),
            ));
        }

        let mut expressions = vec![left_expression_id];
        while !self.peek_is(TokenType::CloseParenthesis) {
            if self.peek_is(TokenType::Comma) {
                self.bump(); // eat comma
                self.eat_newlines_maybe()?;
                continue;
            }

            let expression_options = self.options.not_in_position().not_in_sequence_expression();
            let expression_id = self.eat_expression_with_context_unchecked(expression_options)?;
            expressions.push(expression_id);
            self.eat_newlines_maybe()?;
        }

        Ok(self.insert_node(
            Expression::SequenceExpression { expressions },
            self.get_span_from(start),
        ))
    }

    /// Eat one value-space postfix step for one normalized continuation token.
    fn try_eat_value_postfix_step(
        &mut self,
        start: &ParserMark,
        left_expression_id: LocalNodeId<Expression>,
        token: ContinuationToken,
    ) -> ParseResult<Option<LocalNodeId<Expression>>> {
        // postfix unary operators are the simplest continuation form
        if let Some(operator) = UnaryOperator::from_postfix_token(token.token_type) {
            let operator_start = self.mark_span();
            self.bump(); // eat unary operator
            let operator_span = self.get_span_from(&operator_start);
            let expression_id = self.insert_node(
                Expression::Unary {
                    operator,
                    right: left_expression_id,
                },
                self.get_span_from(start),
            );
            self.tree.set_main_span(expression_id, operator_span);

            return Ok(Some(expression_id));
        }

        // direct calls depend on newline and `new` receiver context
        let has_statement_boundary_newline =
            token.has_pending_newlines || token.has_line_break_before;
        let is_in_new_receiver = self.options.is_in_new_receiver();

        match token.token_type {
            // tagged template literals
            TokenType::TemplateString | TokenType::TemplateStringStart => {
                if !self.is_template_literal_start() {
                    return Ok(None);
                }

                if !self.tagged_template_tag_is_valid(left_expression_id) {
                    return Err(ParseError::unexpected(self.peek()?.span));
                }

                let template_literal = self.eat_tagged_template_literal()?;
                let expression_id = self.insert_node(
                    Expression::TaggedTemplateExpression {
                        tag: left_expression_id,
                        value: template_literal,
                    },
                    self.get_span_from(start),
                );

                Ok(Some(expression_id))
            }

            // postfix calls
            TokenType::OpenParenthesis => self.try_eat_value_call_postfix(
                left_expression_id,
                token,
                has_statement_boundary_newline,
                is_in_new_receiver,
            ),

            // dot driven continuations
            TokenType::Dot => {
                self.advance_to_continuation_token(token);
                self.eat_value_dot_postfix_continuation(start, left_expression_id)
            }

            // direct indexing
            TokenType::OpenBracket => {
                let left_is_maybe =
                    matches!(self.tree.get(left_expression_id), Expression::Maybe { .. });
                if left_is_maybe {
                    return Ok(None);
                }

                self.advance_to_continuation_token(token);
                let expression_id = self.eat_index(left_expression_id, PostfixPosition::Direct)?;

                Ok(Some(expression_id))
            }

            // postfix generic arguments
            TokenType::LessThan | TokenType::ShiftLeft => {
                self.advance_to_continuation_token(token);

                // generic postfixes are only valid on compatible receivers
                if !self.can_start_postfix_generic_arguments(left_expression_id) {
                    return Ok(None);
                }

                // direct generic postfixes do not apply to optional chains
                if matches!(self.tree.get(left_expression_id), Expression::Maybe { .. }) {
                    return Ok(None);
                }

                // generic call or instantiation
                self.try_eat_postfix_generic_application(start, left_expression_id, false)
            }

            // postfix `?` and `as comptime`
            TokenType::Identifier | TokenType::Maybe => {
                self.advance_to_continuation_token(token);
                self.eat_value_postfix_operator_continuation(
                    start,
                    left_expression_id,
                    self.language.is_destack(),
                )
            }

            // direct must postfix
            TokenType::Not => {
                self.bump(); // eat !
                let expression_id = self.insert_node(
                    Expression::Must {
                        position: PostfixPosition::Direct,
                        left: left_expression_id,
                    },
                    self.get_span_from(start),
                );

                Ok(Some(expression_id))
            }

            // tuple or sequence continuations inside parenthesis
            TokenType::Comma if self.options.is_in_parenthesis() => {
                let expression_id =
                    self.eat_parenthesized_sequence_postfix(start, left_expression_id)?;

                Ok(Some(expression_id))
            }

            // done
            _ => Ok(None),
        }
    }

    /// Eat one type-space postfix step for one normalized continuation token.
    fn try_eat_type_postfix_step(
        &mut self,
        start: &ParserMark,
        left_expression_id: LocalNodeId<Expression>,
        token: ContinuationToken,
    ) -> ParseResult<Option<LocalNodeId<Expression>>> {
        match token.token_type {
            // dot driven continuations
            TokenType::Dot => {
                self.advance_to_continuation_token(token);
                self.eat_type_dot_postfix_continuation(start, left_expression_id)
            }

            // direct indexing
            TokenType::OpenBracket => {
                self.advance_to_continuation_token(token);
                let expression_id = self.eat_type_index(left_expression_id)?;

                Ok(Some(expression_id))
            }

            // postfix generic arguments
            TokenType::LessThan | TokenType::ShiftLeft => {
                self.advance_to_continuation_token(token);

                if !self.can_start_postfix_generic_arguments(left_expression_id) {
                    return Ok(None);
                }

                self.try_eat_postfix_generic_application(start, left_expression_id, false)
            }

            // postfix `as comptime` and direct must postfix
            TokenType::Identifier | TokenType::Not => {
                self.advance_to_continuation_token(token);
                self.eat_type_postfix_operator_continuation(start, left_expression_id)
            }

            // done
            _ => Ok(None),
        }
    }

    /// Parse value-space postfix continuation operators after a primary expression.
    ///
    /// Examples:
    /// ```
    /// value()
    /// value[index]
    /// value.method?.()
    /// value<T>()
    /// Vector2 { x: 0, y: 1 }
    /// ```
    fn eat_value_postfix_continuation(
        &mut self,
        start: &ParserMark,
        mut left_expression_id: LocalNodeId<Expression>,
    ) -> ParseResult<LocalNodeId<Expression>> {
        let _timing = self.timing_scope(tags::PARSE_EXPRESSION_POSTFIX);
        let is_in_static = self.options.is_in_static();
        let is_in_ternary_or_match =
            self.options.is_in_ternary_condition() || self.options.is_in_match_case();

        loop {
            // struct literal postfix with `{` (like `Vector2 { x: 0, y }`)
            if let Some(next_expression_id) =
                self.try_eat_tagged_object_literal_postfix(start, left_expression_id)?
            {
                left_expression_id = next_expression_id;
                continue;
            }

            // normalize the next postfix token once before branch dispatch
            let Some(token) = self.next_postfix_continuation_token(
                PostfixSpace::Value,
                is_in_static,
                is_in_ternary_or_match,
            ) else {
                break;
            };

            let Some(next_expression_id) =
                self.try_eat_value_postfix_step(start, left_expression_id, token)?
            else {
                break;
            };

            left_expression_id = next_expression_id;
        }

        Ok(left_expression_id)
    }

    /// Parse type-space postfix continuation operators after a primary expression.
    ///
    /// Examples:
    /// ```
    /// T[]
    /// T[K]
    /// Result<T>.Ok
    /// Promise<T>.!
    /// ```
    pub(super) fn eat_type_postfix_continuation(
        &mut self,
        start: &ParserMark,
        mut left_expression_id: LocalNodeId<Expression>,
    ) -> ParseResult<LocalNodeId<Expression>> {
        let _timing = self.timing_scope(tags::PARSE_EXPRESSION_POSTFIX);
        let is_in_static = self.options.is_in_static();
        let is_in_ternary_or_match =
            self.options.is_in_ternary_condition() || self.options.is_in_match_case();

        loop {
            // normalize the next postfix token once before branch dispatch
            let Some(token) = self.next_postfix_continuation_token(
                PostfixSpace::Type,
                is_in_static,
                is_in_ternary_or_match,
            ) else {
                break;
            };
            let Some(next_expression_id) =
                self.try_eat_type_postfix_step(start, left_expression_id, token)?
            else {
                break;
            };

            left_expression_id = next_expression_id;
        }

        Ok(left_expression_id)
    }

    /// Eat one type conditional expression after consuming `extends`.
    fn eat_type_conditional_expression(
        &mut self,
        start: &ParserMark,
        left_expression_id: LocalNodeId<Expression>,
        right_context: ParserOptions,
    ) -> ParseResult<Expression> {
        let left = self.expect_type_expression_value(left_expression_id)?;

        // right side of `extends`
        let right_ambient_context = self.options.with_type(true);
        let extends_context = right_context.not_in_left_precedence();
        let extends_type = self.eat_type_expression_node_or_recover_missing(
            self.options
                .with_ambient_context(right_ambient_context)
                .with_expression_context(extends_context),
            NodeType::Expression,
        )?;

        // `?` may follow on the same line or after a newline
        let has_conditional_marker = if self.peek_is(TokenType::Maybe) {
            true
        } else if self.peek_is(TokenType::Newline)
            && self.is_token_after_newlines(self.pos(), TokenType::Maybe)
        {
            self.eat_newlines_maybe()?;
            true
        } else {
            false
        };

        // branches
        let (then_type, else_type) = if has_conditional_marker {
            self.bump(); // eat ?
            self.eat_newlines_maybe()?;

            let then_context = self.options.not_in_position().in_ternary_condition();
            let then_type = self.eat_type_expression_node_or_recover_missing(
                self.options
                    .with_type(true)
                    .with_expression_context(then_context),
                NodeType::TypeExpression,
            )?;
            self.eat_newlines_maybe()?;
            self.eat_colon()?;
            self.eat_newlines_maybe()?;

            let mut else_context = self.options.not_in_position();
            if self.options.is_in_type_conditional_right() {
                else_context = else_context.in_type_conditional_right();
            }
            let else_type = self.eat_type_expression_node_or_recover_missing(
                self.options
                    .with_type(true)
                    .with_expression_context(else_context),
                NodeType::TypeExpression,
            )?;

            (then_type, else_type)
        } else {
            let then_type = self.insert_missing_type_expression_here();
            let else_type = self.insert_missing_type_expression_here();

            (then_type, else_type)
        };

        let type_expression_id = self.insert_node(
            TypeExpression::Conditional {
                left,
                extends_type,
                then_type,
                else_type,
            },
            self.get_span_from(start),
        );

        Ok(Expression::Type {
            value: type_expression_id,
        })
    }

    /// Eat one `as` or `satisfies` infix expression.
    fn eat_type_assertion_infix_expression(
        &mut self,
        left_expression_id: LocalNodeId<Expression>,
        right_operator: ParseInfixOperator,
        right_context: ParserOptions,
    ) -> ParseResult<(Expression, Option<u32>)> {
        // `as const`
        let parses_const_type_reference =
            right_operator == ParseInfixOperator::As && self.is_keyword(Keyword::Const);
        let mut as_const_operator_end = None;
        let target_type = if parses_const_type_reference {
            let const_span = self.eat_keyword(Keyword::Const)?.span;
            as_const_operator_end = Some(const_span.end);
            self.insert_node(TypeExpression::Const, const_span)
        } else {
            let right_ambient_context = self.options.with_type(true);
            self.eat_type_expression_node_or_recover_missing(
                self.options
                    .with_ambient_context(right_ambient_context)
                    .with_expression_context(right_context),
                NodeType::Expression,
            )?
        };

        // `as` and `satisfies` wrap the left expression
        let expression = match right_operator {
            ParseInfixOperator::As => Expression::As {
                expression: left_expression_id,
                target_type,
            },
            ParseInfixOperator::Satisfies => Expression::Satisfies {
                expression: left_expression_id,
                target_type,
            },
            _ => unreachable!(),
        };

        Ok((expression, as_const_operator_end))
    }

    /// Parse infix continuation operators after postfix parsing.
    ///
    /// Examples:
    /// ```
    /// a + b * c
    /// value as string
    /// value satisfies Foo
    /// T extends U ? X : Y
    /// x = y ?? z
    /// ```
    fn eat_infix_continuation(
        &mut self,
        start: &ParserMark,
        mut left_expression_id: LocalNodeId<Expression>,
    ) -> ParseResult<LocalNodeId<Expression>> {
        let left_is_statement = self.options.is_in_statement_position()
            && self
                .tree
                .get(left_expression_id)
                .ends_statement_on_newline();

        let _timing = self.timing_scope(tags::PARSE_EXPRESSION_INFIX);
        let left_precedence = self.options.left_precedence;
        loop {
            // normalize the next infix token once before operator analysis
            let Some(token) = self.next_continuation_token_maybe() else {
                break;
            };
            let cursor_index = token.index;
            let token_type = token.token_type;
            let newline_count = token.skipped_newline_count;
            let has_line_break_before = token.has_line_break_before;

            // new receivers stop before type argument delimiters at top-level receiver scope
            if self.options.is_in_new_receiver()
                && !self.options.is_in_parenthesis()
                && (token_type == TokenType::LessThan || token_type == TokenType::ShiftLeft)
            {
                break;
            }

            // stop before conditional boundaries so infix lookahead does not lex past `?`
            if token_type == TokenType::Maybe {
                break;
            }

            // stop before ternary or match case boundary so infix lookahead does not lex past `:`
            if (self.options.is_in_ternary_condition() || self.options.is_in_match_case())
                && token_type == TokenType::Colon
            {
                break;
            }

            // statement expressions do not continue across line breaks
            if left_is_statement && has_line_break_before {
                break;
            }

            // type expressions stop before tree literals after a line break
            if self.options.is_in_type()
                && has_line_break_before
                && self.can_start_tree_literal_after_line_break()
            {
                break;
            }

            // reject tokens that cannot start any infix or assignment operator
            let can_start_operator = if token_type == TokenType::Identifier {
                self.has_infix_or_assign_operator_at_index(cursor_index)
            } else {
                AssignOperator::from_token(token_type).is_some()
                    || BinaryOperator::from_token("", token_type).is_some()
            };

            if !can_start_operator {
                break;
            }

            let Some((right_operator, operator_offset)) =
                self.peek_infix_operator_at_index_maybe(cursor_index, has_line_break_before)
            else {
                break;
            };

            // mapped constraints stop before the remap `as`
            if self.options.is_in_type_mapped_constraint()
                && right_operator == ParseInfixOperator::As
            {
                break;
            }

            if has_line_break_before
                && newline_count > 1
                && self.options.is_in_type()
                && !matches!(
                    right_operator,
                    ParseInfixOperator::Binary(
                        BinaryOperator::ElementwiseOr | BinaryOperator::ElementwiseAnd
                    )
                )
            {
                break;
            }

            if let Some(left_precedence) = left_precedence
                && left_precedence >= right_operator.precedence()
            {
                break;
            }

            // reject assignment targets that are invalid in ts/js grammar
            if matches!(right_operator, ParseInfixOperator::Assign(_))
                && self.assignment_target_has_invalid_form(left_expression_id)
            {
                return Err(ParseError::unexpected(
                    self.tree.get_span(left_expression_id),
                ));
            }

            // align parser position with scanner cursor before consuming operator tokens
            self.advance_to_continuation_token(token);

            // capture operator span before eating
            let operator_start = self.mark_span();
            self.bump_by(operator_offset); // eat infix operator
            let operator_span = self.get_span_from(&operator_start);

            self.eat_newlines_maybe()?; // allow newlines after infix operator

            // eat right expression
            let subject_id = left_expression_id;
            let mut right_context = self
                .options
                .not_in_statement_position()
                .not_in_type_conditional_right()
                .in_left_precedence(right_operator.precedence());
            let parses_value_type_operator_right = !self.options.is_in_type()
                && matches!(
                    right_operator,
                    ParseInfixOperator::As | ParseInfixOperator::Satisfies
                );
            let parses_type_expression_right = parses_value_type_operator_right
                || matches!(right_operator, ParseInfixOperator::TypeBinary(_));
            let mut as_const_operator_end = None;
            let parses_type_conditional = right_operator
                == ParseInfixOperator::TypeBinary(TypeBinaryOperator::Extends)
                && (self.options.is_in_type() || self.options.is_in_before_block());

            // cast and satisfies in parenthesized value expressions need
            // the parenthesis flag so the type right side can stop at `)`
            if !parses_value_type_operator_right {
                right_context = right_context.not_in_parenthesis();
            }

            // type operators in value expressions parse a full type expression on the right
            if !self.options.is_in_type() && parses_type_expression_right {
                right_context = right_context.not_in_left_precedence();
            }

            // conditional type right sides must keep their boundary marker active
            if parses_value_type_operator_right
                || self.options.is_in_type_conditional_right()
                || matches!(
                    right_operator,
                    ParseInfixOperator::TypeBinary(TypeBinaryOperator::Extends)
                )
            {
                right_context = right_context.in_type_conditional_right();
            }

            // combine into the new left expression
            let left_expression = if matches!(
                right_operator,
                ParseInfixOperator::As | ParseInfixOperator::Satisfies
            ) {
                let (expression, operator_end) = self.eat_type_assertion_infix_expression(
                    left_expression_id,
                    right_operator,
                    right_context,
                )?;
                as_const_operator_end = operator_end;
                expression
            } else if parses_type_conditional {
                self.eat_type_conditional_expression(start, left_expression_id, right_context)?
            } else {
                let right_expression_id =
                    if parses_type_expression_right && self.is_type_expression_boundary() {
                        self.recover_missing_expression_here(NodeType::Expression)
                    } else {
                        let right_expression_result = if parses_type_expression_right {
                            let right_ambient_context = self.options.with_type(true);
                            self.with_options(
                                self.options
                                    .with_ambient_context(right_ambient_context)
                                    .with_expression_context(right_context),
                                |parser| parser.eat_expression_in_scope(),
                            )
                        } else {
                            self.with_options(
                                self.options.with_expression_context(right_context),
                                |parser| parser.eat_expression_in_scope(),
                            )
                        };
                        right_expression_result?
                    };

                self.make_infix_expression(left_expression_id, right_operator, right_expression_id)?
            };

            left_expression_id = self.insert_node(left_expression, self.get_span_from(start));

            // set main span to the operator
            let operator_main_span = if let Some(as_const_operator_end) = as_const_operator_end {
                destack_source::Span::new(
                    operator_span.file,
                    operator_span.start,
                    as_const_operator_end,
                )
            } else {
                operator_span
            };
            self.tree
                .set_main_span(left_expression_id, operator_main_span);

            if let Expression::Type { value } = self.tree.get(left_expression_id) {
                self.tree.set_main_span(*value, operator_main_span);
            }

            // wrapper operators inherit the wrapped head
            if matches!(
                self.tree.get(left_expression_id),
                Expression::As { .. } | Expression::Satisfies { .. }
            ) {
                let head_span = self.expression_head_span(subject_id);
                self.tree.set_head_span(left_expression_id, head_span);
            }

            // use the subject identifier for type predicate spans
            if let Expression::Type { value } = self.tree.get(left_expression_id)
                && matches!(self.tree.get(*value), TypeExpression::Predicate { .. })
            {
                let value = *value;
                let subject_span = self
                    .tree
                    .get_main_span(subject_id)
                    .unwrap_or_else(|| self.tree.get_span(subject_id));
                self.tree.set_main_span(left_expression_id, subject_span);
                self.tree.set_main_span(value, subject_span);
            }
        }

        Ok(left_expression_id)
    }

    /// Parse one value-space tail continuation after infix parsing.
    fn eat_value_tail_continuation(
        &mut self,
        start: &ParserMark,
        mut left_expression_id: LocalNodeId<Expression>,
    ) -> ParseResult<LocalNodeId<Expression>> {
        let left_precedence = self.options.left_precedence;
        let mut tail_cursor = self.scanner_cursor_from(self.pos_index());

        // ternary is the lowest precedence value continuation
        // NOTE #Cleanup: having multiple ternary parse locations feels icky
        if left_precedence.is_none() && tail_cursor.token_type == TokenType::Maybe {
            if tail_cursor.index != self.pos_index() {
                self.advance_to(tail_cursor.index);
            }
            self.bump(); // eat ?
            self.eat_newlines_maybe()?;

            let then_options = self
                .options
                .not_in_position()
                .in_ternary_condition()
                .not_in_sequence_expression();
            let then_expression_id = self.eat_expression_with_context_unchecked(then_options)?;
            self.eat_newlines_maybe()?;
            self.eat_colon()?;
            self.eat_newlines_maybe()?;

            let else_options = self.options.not_in_position().not_in_sequence_expression();
            let else_expression_id = self.eat_expression_with_context_unchecked(else_options)?;
            let expression = Expression::If {
                kind: IfKind::Ternary,
                condition: IfCondition::Expression {
                    condition: left_expression_id,
                },
                then_expression: then_expression_id,
                else_expression: Some(else_expression_id),
            };
            left_expression_id = self.insert_node(expression, self.get_span_from(start));
            tail_cursor = self.scanner_cursor_from(self.pos_index());
        }

        // sequence expressions only exist in JS and TS value space
        if left_precedence.is_none()
            && self.options.allows_sequence_expression()
            && (self.language.is_typescript() || self.language.is_javascript())
            && tail_cursor.token_type == TokenType::Comma
        {
            let mut expressions = vec![left_expression_id];
            while tail_cursor.token_type == TokenType::Comma {
                if tail_cursor.index != self.pos_index() {
                    self.advance_to(tail_cursor.index);
                }
                self.bump(); // eat comma
                self.eat_newlines_maybe()?;

                let expression_options =
                    self.options.not_in_position().not_in_sequence_expression();
                let expression_id =
                    self.eat_expression_with_context_unchecked(expression_options)?;
                expressions.push(expression_id);
                tail_cursor = self.scanner_cursor_from(self.pos_index());
            }

            let expression = Expression::SequenceExpression { expressions };
            left_expression_id = self.insert_node(expression, self.get_span_from(start));
        }

        Ok(left_expression_id)
    }

    /// Parse expression continuation operators after a primary expression.
    pub(crate) fn eat_expression_continuation(
        &mut self,
        start: &ParserMark,
        left_expression_id: LocalNodeId<Expression>,
    ) -> ParseResult<LocalNodeId<Expression>> {
        // type-space continuations stay in the type postfix and tail grammar
        if self.options.is_in_type() {
            let left_expression_id =
                self.eat_type_postfix_continuation(start, left_expression_id)?;
            let left_expression_id = self.eat_infix_continuation(start, left_expression_id)?;

            let tail_cursor = self.scanner_cursor_from(self.pos_index());
            if tail_cursor.token_type != TokenType::Maybe {
                return Ok(left_expression_id);
            }

            // tuple element optionals belong to the surrounding tuple parser
            if tail_cursor.index != self.pos_index() {
                self.advance_to(tail_cursor.index);
            }

            // nested conditional right sides stop before the outer `?`
            if self.options.is_in_type_conditional_right() {
                return Ok(left_expression_id);
            }

            // tuple element optionals belong to the surrounding tuple parser
            let question_pos = self.pos();
            let is_tuple_optional = self.is_token_after_newlines(question_pos, TokenType::Comma)
                || self.is_token_after_newlines(question_pos, TokenType::CloseBracket);
            if is_tuple_optional {
                return Ok(left_expression_id);
            }

            return Err(ParseError::unexpected(self.peek()?.span));
        }

        // statement expressions do not accept continuation operators
        if self.statement_expression_stops_continuation(left_expression_id) {
            return Ok(left_expression_id);
        }

        // value-space continuations use postfix, infix, then tail parsing
        let left_expression_id = self.eat_value_postfix_continuation(start, left_expression_id)?;
        let left_expression_id = self.eat_infix_continuation(start, left_expression_id)?;

        self.eat_value_tail_continuation(start, left_expression_id)
    }

    /// Return a valid dot-member target after `.` from a scanner cursor index.
    fn peek_dot_member_target(
        &mut self,
        dot_index: usize,
        next_cursor: NonNewlineTokenCursor,
        left_expression_id: LocalNodeId<Expression>,
    ) -> Option<(usize, bool)> {
        let member_index = next_cursor.index;
        let member_token_type = next_cursor.token_type;

        // private member: .#name
        if member_token_type == TokenType::Hash {
            let identifier_index = member_index.saturating_add(1);
            let has_identifier = self.token_type_at(identifier_index) == TokenType::Identifier;
            let has_adjacent_hash_identifier =
                has_identifier && self.tokens_are_adjacent(member_index, identifier_index);

            if has_adjacent_hash_identifier {
                return Some((member_index, true));
            }

            return None;
        }

        // member name: .name or .true / .false
        if self.token_is_dot_member_name(member_index) {
            return Some((member_index, false));
        }

        // decimal member separator: 0..a and 123..a
        let has_decimal_separator = member_token_type == TokenType::Dot
            && self.tokens_are_adjacent(dot_index, member_index)
            && self.expression_is_decimal_integer_before_dot(left_expression_id, dot_index);

        // parse second-dot member target when the receiver is a decimal integer literal
        if has_decimal_separator {
            let separated_member_index =
                self.next_non_newline_index_from(member_index.saturating_add(1));
            if self.token_is_dot_member_name(separated_member_index) {
                return Some((separated_member_index, false));
            }
        }

        None
    }

    /// Return true when a token index holds a valid dot-member name.
    #[inline]
    fn token_is_dot_member_name(&mut self, token_index: usize) -> bool {
        let token_type = self.token_type_at(token_index);

        if token_type == TokenType::Identifier {
            return true;
        }

        token_type == TokenType::Literal
            && self.token_ref_at(token_index).is_some_and(|token| {
                matches!(token.token.literal, Some(LiteralType::Boolean { .. }))
            })
    }

    /// Check whether the current token sequence can start postfix generic arguments.
    fn can_start_postfix_generic_arguments(
        &mut self,
        left_expression_id: LocalNodeId<Expression>,
    ) -> bool {
        // shape: `<...>` or `.<...>`
        let has_generic_argument_start = self.peek_is(TokenType::LessThan)
            || self.peek_is(TokenType::ShiftLeft)
            || self.starts_indirect_postfix_generic_arguments();
        if !has_generic_argument_start {
            return false;
        }

        // postfix generic arguments are disabled in `new` receiver and tree contexts
        if matches!(self.tree.get(left_expression_id), Expression::New { .. }) {
            return false;
        }
        let ambient = self.options;
        if ambient.is_in_new_receiver() || ambient.is_in_tree_literal() {
            return false;
        }
        if self.language.is_javascript() {
            return false;
        }

        true
    }

    /// Check whether postfix generic arguments are attached through optional chaining.
    fn starts_indirect_postfix_generic_arguments(&mut self) -> bool {
        self.peek_is(TokenType::Dot)
            && (self.peek_next_is(TokenType::LessThan) || self.peek_next_is(TokenType::ShiftLeft))
    }

    /// Speculatively parse postfix generic arguments into either call or instantiation.
    fn try_eat_postfix_generic_application(
        &mut self,
        start: &ParserMark,
        left_expression_id: LocalNodeId<Expression>,
        has_indirect_generic: bool,
    ) -> ParseResult<Option<LocalNodeId<Expression>>> {
        // speculative boundary for optional chaining style generic arguments
        let speculative_start = self.mark();
        let speculative_start_idx = self.tree.next_id();

        let position = if has_indirect_generic {
            self.bump(); // eat .
            PostfixPosition::Indirect
        } else {
            PostfixPosition::Direct
        };

        // tagged object literals can follow postfix instantiations on the same receiver shapes
        let allow_object_literal = self
            .expression_type_value_maybe(self.without_parentheses_expression(left_expression_id))
            .is_some_and(|type_expression_id| {
                self.can_start_tagged_object_literal_type(type_expression_id)
            });

        // parse `<...>` with regular speculative follow validation
        let generic_arguments = match self.try_eat_generic_arguments(allow_object_literal, false) {
            Some(generic_arguments) => generic_arguments,
            None => {
                self.restore(speculative_start, speculative_start_idx);
                return Ok(None);
            }
        };

        // call with generic arguments
        if self.peek_is(TokenType::OpenParenthesis) {
            let expression_id =
                self.eat_call(left_expression_id, Some(generic_arguments), position)?;
            return Ok(Some(expression_id));
        }

        // keep type instantiations in type space
        if self.options.is_in_type() {
            let left_type = self.expect_type_expression_value(left_expression_id)?;

            let type_expression = match self.tree.get(left_type).clone() {
                TypeExpression::Reference { path, .. } => TypeExpression::Reference {
                    path,
                    generic_arguments,
                },
                TypeExpression::Member { left, name, .. } => TypeExpression::Member {
                    left,
                    name,
                    generic_arguments,
                },
                TypeExpression::Import {
                    target,
                    arguments,
                    qualifier,
                    ..
                } => TypeExpression::Import {
                    target,
                    arguments,
                    qualifier,
                    generic_arguments,
                },
                _ => {
                    self.restore(speculative_start, speculative_start_idx);
                    return Ok(None);
                }
            };

            let expression_id = self.insert_node(type_expression, self.get_span_from(start));
            return Ok(Some(self.insert_type_expression_value(expression_id)));
        }

        let expression_id = self.insert_node(
            Expression::Instantiation {
                left: left_expression_id,
                generic_arguments,
            },
            self.get_span_from(start),
        );
        Ok(Some(expression_id))
    }
}
