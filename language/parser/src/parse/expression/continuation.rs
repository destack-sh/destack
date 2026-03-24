use crate::parse::parser::NonNewlineTokenCursor;
use crate::parse::prelude::*;
use crate::{ParseError, ParseResult, Parser, ParserMark};

use destack_ast::{
    Argument, AssignOperator, BinaryOperator, Declaration, Expression, FunctionKind, IfCondition,
    IfKind, InfixOperator, LiteralType, LocalNodeId, NodeType, PostfixPosition, TokenType,
    TypeBinaryOperator, TypeUnaryOperator, UnaryOperator,
};

impl Parser {
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

        let close_parenthesis_index = self.find_matching_close(
            Some(open_parenthesis_index as u32),
            TokenType::OpenParenthesis,
            TokenType::CloseParenthesis,
        );
        let Ok(close_parenthesis_index) = close_parenthesis_index else {
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

    /// Return true when assignment lhs syntax is invalid in ts/js grammar.
    #[inline]
    fn assignment_target_has_invalid_syntax(&self, expression_id: LocalNodeId<Expression>) -> bool {
        let mut expression_id = expression_id;
        let mut is_parenthesized = false;

        loop {
            match self.tree.get(expression_id) {
                // parenthesized wrappers can legalize cast lhs forms
                Expression::Parenthesized { expression } => {
                    is_parenthesized = true;
                    expression_id = *expression;
                }

                // `satisfies` lhs is valid in parse output only when parenthesized
                Expression::TypeBinary {
                    operator: TypeBinaryOperator::Satisfies,
                    ..
                } => {
                    return !is_parenthesized;
                }

                // `as` cast lhs is valid only when parenthesized
                Expression::TypeBinary {
                    operator: TypeBinaryOperator::Cast,
                    ..
                } => {
                    return !is_parenthesized;
                }

                // all other lhs forms are handled by assignment-target validation later
                _ => {
                    return false;
                }
            }
        }
    }

    /// Parse expression continuation operators after a primary expression.
    pub(crate) fn eat_expression_continuation(
        &mut self,
        start: &ParserMark,
        mut left_expression_id: LocalNodeId<Expression>,
    ) -> ParseResult<LocalNodeId<Expression>> {
        // statement expressions do not accept postfix or infix operators
        if self.options.is_in_statement_position() {
            let next_token_type = self.peek_token_type();
            let expression = self.tree.get(left_expression_id);
            let is_continuable_lambda_declaration = matches!(
                expression,
                Expression::Declaration(declaration_id)
                    if matches!(
                        self.tree.get(*declaration_id),
                        Declaration::Function { signature, .. }
                            if signature.kind == FunctionKind::Lambda
                    )
            ) && !matches!(
                next_token_type,
                TokenType::Newline | TokenType::Semicolon | TokenType::CloseBrace | TokenType::End
            );
            if expression.is_statement_boundary() && !is_continuable_lambda_declaration {
                return Ok(left_expression_id);
            }
        }

        //
        // ------------------------------------------------------------
        // Postfix operations
        // ------------------------------------------------------------
        //

        {
            let _timing = self.timing_scope(tags::PARSE_EXPRESSION_POSTFIX);
            let is_in_type = self.options.is_in_type();
            let is_in_new_receiver = self.options.is_in_new_receiver();
            let is_in_static = self.options.is_in_static();
            let is_in_ternary_or_match =
                self.options.is_in_ternary_condition() || self.options.is_in_match_case();
            let is_destack_language = self.language.is_destack();

            // struct literal postfix with `{` (like `Vector2 { x: 0, y }`)
            if let Expression::Path { .. } = self.tree.get(left_expression_id)
                && self.peek_is(TokenType::OpenBrace)
                && !self.options.is_in_before_block()
                && is_destack_language
            {
                let properties = self.eat_object_literal()?;
                left_expression_id = self.insert_node(
                    Expression::ObjectExpression {
                        ty: Some(left_expression_id),
                        properties,
                    },
                    self.get_span_from(start),
                );
            }
            // eat all regular postfix operators
            loop {
                // load the raw token facts once and only normalize across newlines when needed
                let mut cursor_index = self.pos_index();
                let mut token_type = if let Some(token) = self.token_stream.active_split_token() {
                    token.token.ty
                } else {
                    self.ensure_token(cursor_index);
                    let Some(token) = self.tokens().get(cursor_index) else {
                        break;
                    };
                    token.token.ty
                };
                if token_type == TokenType::End {
                    break;
                }

                let mut has_pending_newline_tokens = false;
                let mut has_line_break_before = self
                    .token_stream
                    .materialized_line_terminator_before(cursor_index);
                if token_type == TokenType::Newline {
                    let cursor = self.scanner_cursor_from(cursor_index);
                    token_type = cursor.token_type;
                    if token_type == TokenType::End {
                        break;
                    }

                    cursor_index = cursor.index;
                    has_pending_newline_tokens = true;
                    has_line_break_before = cursor.has_line_break_before;

                    let can_continue_after_newline = matches!(
                        token_type,
                        TokenType::OpenParenthesis | TokenType::Dot | TokenType::Maybe
                    ) || (!is_in_type
                        && matches!(token_type, TokenType::LessThan | TokenType::ShiftLeft));

                    if !can_continue_after_newline {
                        break;
                    }
                }
                let has_statement_boundary_newline =
                    has_pending_newline_tokens || has_line_break_before;

                // stop before ternary or switch case boundary so postfix parsing does not consume ':'
                if is_in_ternary_or_match && token_type == TokenType::Colon {
                    break;
                }

                // stop before static boundary so postfix parsing does not consume '>'
                if is_in_static && token_type == TokenType::GreaterThan {
                    break;
                }

                // unary postfix operations
                if let Some(operator) = UnaryOperator::from_postfix_token(token_type) {
                    let operator_start = self.mark_span();
                    self.bump(); // eat unary operator
                    let operator_span = self.get_span_from(&operator_start);
                    left_expression_id = self.insert_node(
                        Expression::Unary {
                            operator,
                            right: left_expression_id,
                        },
                        self.get_span_from(start),
                    );
                    self.tree.set_main_span(left_expression_id, operator_span);
                    continue;
                }
                match token_type {
                    // tagged template literals
                    TokenType::TemplateString | TokenType::TemplateStringStart => {
                        if !self.is_template_literal_start() {
                            break;
                        }

                        if !self.tagged_template_tag_is_valid(left_expression_id) {
                            return Err(ParseError::unexpected(self.peek()?.span));
                        }

                        let template_literal = self.eat_tagged_template_literal()?;
                        left_expression_id = self.insert_node(
                            Expression::TaggedTemplateExpression {
                                tag: left_expression_id,
                                value: template_literal,
                            },
                            self.get_span_from(start),
                        );
                    }

                    // postfix calls
                    TokenType::OpenParenthesis => {
                        let left_is_maybe =
                            matches!(self.tree.get(left_expression_id), Expression::Maybe { .. });
                        if is_in_type || left_is_maybe || is_in_new_receiver {
                            break;
                        }

                        let should_terminate_newline_direct_call = has_statement_boundary_newline
                            && self.newline_direct_call_terminates_statement(
                                left_expression_id,
                                cursor_index,
                            );
                        if should_terminate_newline_direct_call {
                            break;
                        }

                        if self.is_unparenthesized_lambda_expression(left_expression_id)
                            && has_statement_boundary_newline
                        {
                            break;
                        }
                        if self.is_unparenthesized_lambda_expression(left_expression_id) {
                            return Err(ParseError::unexpected(self.peek()?.span));
                        }

                        let _call_timing = self.timing_scope(tags::PARSE_EXPRESSION_POSTFIX_CALL);
                        if has_pending_newline_tokens {
                            self.advance_to(cursor_index);
                        }
                        left_expression_id =
                            self.eat_call(left_expression_id, None, PostfixPosition::Direct)?;
                    }

                    // dot driven continuations
                    TokenType::Dot => {
                        if has_pending_newline_tokens {
                            self.advance_to(cursor_index);
                        }

                        let dot_index = self.pos_index();
                        let next_raw_index = dot_index.saturating_add(1);
                        let next_token_type = self.token_type_at(next_raw_index);
                        let next_cursor = self.scanner_cursor_from(next_raw_index);
                        let next_token_type_after_newlines = next_cursor.token_type;

                        if next_token_type_after_newlines == TokenType::OpenParenthesis {
                            if is_in_type {
                                break;
                            }

                            let _call_timing =
                                self.timing_scope(tags::PARSE_EXPRESSION_POSTFIX_CALL);
                            self.bump(); // eat .
                            self.eat_newlines_maybe()?;
                            left_expression_id =
                                self.eat_call(left_expression_id, None, PostfixPosition::Indirect)?;
                            continue;
                        }

                        if matches!(
                            next_token_type_after_newlines,
                            TokenType::LessThan | TokenType::ShiftLeft
                        ) {
                            if !self.can_start_postfix_static_arguments(left_expression_id) {
                                break;
                            }

                            let has_indirect_static = self.has_indirect_postfix_static_arguments();
                            let is_optional_chain = matches!(
                                self.tree.get(left_expression_id),
                                Expression::Maybe { .. }
                            );
                            if has_indirect_static != is_optional_chain {
                                break;
                            }

                            let next_expression_id = self
                                .eat_postfix_static_call_or_instantiation(
                                    start,
                                    left_expression_id,
                                    has_indirect_static,
                                )?;
                            let Some(next_expression_id) = next_expression_id else {
                                break;
                            };

                            left_expression_id = next_expression_id;
                            continue;
                        }

                        if next_token_type_after_newlines == TokenType::OpenBracket {
                            self.bump(); // eat .
                            self.eat_newlines_maybe()?;
                            left_expression_id =
                                self.eat_index(left_expression_id, PostfixPosition::Indirect)?;
                            continue;
                        }

                        if next_token_type == TokenType::Maybe {
                            if is_in_type {
                                break;
                            }

                            self.bump(); // eat .
                            self.bump(); // eat ?
                            left_expression_id = self.insert_node(
                                Expression::Maybe {
                                    left: left_expression_id,
                                    position: PostfixPosition::Indirect,
                                },
                                self.get_span_from(start),
                            );
                            continue;
                        }

                        if next_token_type == TokenType::Not {
                            self.bump(); // eat .
                            self.bump(); // eat !
                            left_expression_id = self.insert_node(
                                Expression::Must {
                                    position: PostfixPosition::Indirect,
                                    left: left_expression_id,
                                },
                                self.get_span_from(start),
                            );
                            continue;
                        }

                        // preserve a committed member access when the name slot is missing
                        if Self::is_expression_slot_boundary_token(next_token_type_after_newlines) {
                            self.bump(); // eat .
                            self.eat_newlines_maybe()?;
                            self.report_unexpected_for_here(NodeType::Expression);
                            left_expression_id = self.insert_node(
                                Expression::Member {
                                    left: left_expression_id,
                                    name: None,
                                    static_arguments: None,
                                },
                                self.get_span_from(start),
                            );
                            continue;
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
                                left_expression_id = self.insert_node(
                                    Expression::PrivateMember {
                                        left: left_expression_id,
                                        name: None,
                                        static_arguments: None,
                                    },
                                    self.get_span_from(start),
                                );
                                continue;
                            }
                        }

                        let Some((member_index, is_private_member)) = self.peek_dot_member_target(
                            cursor_index,
                            next_cursor,
                            left_expression_id,
                        ) else {
                            break;
                        };

                        let member_distance = member_index
                            .saturating_sub(self.pos_index())
                            .saturating_add(1);
                        let member_distance = u8::try_from(member_distance).unwrap_or(u8::MAX);

                        self.bump(); // eat .
                        if self.pos_index() != member_index {
                            self.advance_to(member_index);
                        }

                        if is_private_member {
                            self.bump(); // eat #
                            let (name, name_span) = self.eat_identifier_with_span()?;
                            let static_arguments = self.eat_static_arguments_in_expression(false);
                            left_expression_id = self.insert_node(
                                Expression::PrivateMember {
                                    left: left_expression_id,
                                    name: Some(name),
                                    static_arguments,
                                },
                                self.get_span_from(start),
                            );
                            self.tree.set_main_span(left_expression_id, name_span);
                            continue;
                        }

                        if self.invalid_decimal_integer_member_access(
                            left_expression_id,
                            member_distance,
                        ) {
                            return Err(ParseError::unexpected(self.prev().expect("peeked").span));
                        }

                        let (name, name_span) = self.eat_member_name_with_span()?;
                        let static_arguments = self.eat_static_arguments_in_expression(false);
                        left_expression_id = self.insert_node(
                            Expression::Member {
                                left: left_expression_id,
                                name: Some(name),
                                static_arguments,
                            },
                            self.get_span_from(start),
                        );
                        self.tree.set_main_span(left_expression_id, name_span);
                    }

                    // direct indexing
                    TokenType::OpenBracket => {
                        let left_is_maybe =
                            matches!(self.tree.get(left_expression_id), Expression::Maybe { .. });
                        if left_is_maybe {
                            break;
                        }

                        if has_pending_newline_tokens {
                            self.advance_to(cursor_index);
                        }

                        left_expression_id =
                            self.eat_index(left_expression_id, PostfixPosition::Direct)?;
                    }

                    // postfix static arguments
                    TokenType::LessThan | TokenType::ShiftLeft => {
                        if has_pending_newline_tokens {
                            self.advance_to(cursor_index);
                        }

                        if !self.can_start_postfix_static_arguments(left_expression_id) {
                            break;
                        }

                        let has_indirect_static = self.has_indirect_postfix_static_arguments();
                        let is_optional_chain =
                            matches!(self.tree.get(left_expression_id), Expression::Maybe { .. });
                        if has_indirect_static != is_optional_chain {
                            break;
                        }

                        let next_expression_id = self.eat_postfix_static_call_or_instantiation(
                            start,
                            left_expression_id,
                            has_indirect_static,
                        )?;
                        let Some(next_expression_id) = next_expression_id else {
                            break;
                        };

                        left_expression_id = next_expression_id;
                    }

                    // type unary postfix operators
                    TokenType::Identifier | TokenType::Maybe => {
                        if has_pending_newline_tokens {
                            self.advance_to(cursor_index);
                            token_type = self.peek_token_type();
                        }

                        if let Some(operator) = self.peek_type_unary_postfix_operator_maybe() {
                            let operator_start = self.mark_span();
                            self.bump(); // eat type unary operator
                            if matches!(
                                operator,
                                TypeUnaryOperator::AsConst | TypeUnaryOperator::AsComptime
                            ) {
                                self.eat_newlines_maybe()?;
                                self.bump(); // eat second token
                            }
                            let operator_span = self.get_span_from(&operator_start);
                            left_expression_id = self.insert_node(
                                Expression::TypeUnary {
                                    operator,
                                    right: left_expression_id,
                                },
                                self.get_span_from(start),
                            );
                            self.tree.set_main_span(left_expression_id, operator_span);
                            continue;
                        }

                        if token_type != TokenType::Maybe {
                            break;
                        }

                        let type_conditional_operands = if is_in_type {
                            self.split_type_conditional_operands(left_expression_id)
                        } else {
                            None
                        };
                        let is_type_conditional = type_conditional_operands.is_some();
                        let is_direct_current_maybe = self.peek_is(TokenType::Maybe);
                        let is_indirect_current_maybe = !is_direct_current_maybe
                            && self.peek_is(TokenType::Dot)
                            && self.peek_next_is(TokenType::Maybe);
                        let is_optional_chain_after_maybe = self.is_optional_chain_after_maybe();
                        let is_direct_postfix_maybe = is_destack_language
                            && (self.is_next_any_stop()
                                && self.prev_token_type() != TokenType::Newline
                                || self.is_next_any_close_parenthesis()
                                || self.peek_next_assign_operator_is());
                        let is_postfix_maybe = !is_in_type
                            && is_direct_current_maybe
                            && (is_optional_chain_after_maybe || is_direct_postfix_maybe);

                        if is_postfix_maybe {
                            self.bump(); // eat ?
                            left_expression_id = self.insert_node(
                                Expression::Maybe {
                                    left: left_expression_id,
                                    position: PostfixPosition::Direct,
                                },
                                self.get_span_from(start),
                            );
                            continue;
                        }

                        if !is_in_type && is_indirect_current_maybe {
                            self.bump(); // eat .
                            self.bump(); // eat ?
                            left_expression_id = self.insert_node(
                                Expression::Maybe {
                                    left: left_expression_id,
                                    position: PostfixPosition::Indirect,
                                },
                                self.get_span_from(start),
                            );
                            continue;
                        }

                        if !self.options.is_in_type() || !is_type_conditional {
                            break;
                        }

                        self.bump(); // eat ?
                        self.eat_newlines_maybe()?;
                        let Some((left, right)) = type_conditional_operands else {
                            break;
                        };
                        let then_ambient_context = self.options.with_type(true);
                        let then_expression_context =
                            self.options.not_in_position().in_ternary_condition();
                        let then_expression_id = self.eat_expression(
                            self.options
                                .with_ambient_context(then_ambient_context)
                                .with_expression_context(then_expression_context),
                        )?;
                        self.eat_newlines_maybe()?;
                        self.eat_colon()?;
                        self.eat_newlines_maybe()?;
                        let mut else_expression_context = self.options.not_in_position();
                        if self.options.is_in_type_conditional_right() {
                            else_expression_context =
                                else_expression_context.in_type_conditional_right();
                        }
                        let else_expression_id = self.eat_expression(
                            self.options
                                .with_type(true)
                                .with_expression_context(else_expression_context),
                        )?;
                        let expression = Expression::TypeConditional {
                            left,
                            right,
                            then_type: then_expression_id,
                            else_type: else_expression_id,
                        };
                        left_expression_id =
                            self.insert_node(expression, self.get_span_from(start));
                    }

                    // direct must postfix
                    TokenType::Not => {
                        self.bump(); // eat !
                        left_expression_id = self.insert_node(
                            Expression::Must {
                                position: PostfixPosition::Direct,
                                left: left_expression_id,
                            },
                            self.get_span_from(start),
                        );
                    }

                    // tuple or sequence continuations inside parenthesis
                    TokenType::Comma if self.options.is_in_parenthesis() => {
                        self.bump(); // eat comma
                        self.eat_newlines_maybe()?;
                        if self.language.is_destack() {
                            let first_element_id = self.insert_node(
                                Argument::Positional {
                                    modifiers: None,
                                    value: left_expression_id,
                                },
                                self.get_span_from(start),
                            );
                            if let Some(speculation_stats) = self.speculation_stats.as_mut() {
                                speculation_stats.with_options_calls += 1;
                            }
                            let tuple_elements =
                                self.with_options(self.options.not_in_position(), |parser| {
                                    parser.eat_sequence_literal_body(
                                        Some(first_element_id),
                                        TokenType::CloseParenthesis,
                                    )
                                })?;
                            left_expression_id = self.insert_node(
                                Expression::TupleExpression {
                                    elements: tuple_elements,
                                },
                                self.get_span_from(start),
                            );
                        } else {
                            let mut expressions = vec![left_expression_id];
                            while !self.peek_is(TokenType::CloseParenthesis) {
                                if self.peek_is(TokenType::Comma) {
                                    self.bump(); // eat comma
                                    self.eat_newlines_maybe()?;
                                    continue;
                                }

                                let expression_options =
                                    self.options.not_in_position().not_in_sequence_expression();
                                let expr_id =
                                    self.eat_expression_with_context_unchecked(expression_options)?;
                                expressions.push(expr_id);
                                self.eat_newlines_maybe()?;
                            }
                            left_expression_id = self.insert_node(
                                Expression::SequenceExpression { expressions },
                                self.get_span_from(start),
                            );
                        }
                    }

                    // done
                    _ => {
                        break;
                    }
                }
            }
        }

        //
        // ------------------------------------------------------------
        // Infix operations (binary and assign, left associative)
        // ------------------------------------------------------------
        //

        // eat infix expressions while left precedence is weaker than right precedence
        // track statement newline boundaries before entering the loop
        let left_is_statement = self.options.is_in_statement_position()
            && self
                .tree
                .get(left_expression_id)
                .ends_statement_on_newline();

        {
            let _timing = self.timing_scope(tags::PARSE_EXPRESSION_INFIX);
            let left_precedence = self.options.left_precedence;
            loop {
                // infix parsing only needs newline normalization when the raw token is newline
                let mut cursor_index = self.pos_index();
                let mut token_type = if let Some(token) = self.token_stream.active_split_token() {
                    token.token.ty
                } else {
                    self.ensure_token(cursor_index);
                    let Some(token) = self.tokens().get(cursor_index) else {
                        break;
                    };
                    token.token.ty
                };
                if token_type == TokenType::End {
                    break;
                }

                let mut newline_count = 0;
                let mut has_line_break_before = self
                    .token_stream
                    .materialized_line_terminator_before(cursor_index);
                let mut has_pending_newline_tokens = false;
                if token_type == TokenType::Newline {
                    let cursor = self.scanner_cursor_from(cursor_index);
                    token_type = cursor.token_type;
                    if token_type == TokenType::End {
                        break;
                    }

                    cursor_index = cursor.index;
                    newline_count = cursor.skipped_newline_count;
                    has_line_break_before = cursor.has_line_break_before;
                    has_pending_newline_tokens = true;
                }

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

                // quick reject: avoid the slower operator shape analysis for obvious non operators
                if token_type == TokenType::Identifier {
                    if !self.has_infix_or_assign_operator_at_index(cursor_index) {
                        break;
                    }
                } else if AssignOperator::from_token(token_type).is_none()
                    && BinaryOperator::from_token("", token_type).is_none()
                {
                    break;
                }

                let Some((right_operator, operator_offset)) =
                    self.peek_infix_operator_at_index_maybe(cursor_index, has_line_break_before)
                else {
                    break;
                };

                if has_line_break_before
                    && newline_count > 1
                    && self.options.is_in_type()
                    && !matches!(
                        right_operator,
                        InfixOperator::Binary(
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
                if matches!(right_operator, InfixOperator::Assign(_))
                    && self.assignment_target_has_invalid_syntax(left_expression_id)
                {
                    return Err(ParseError::unexpected(
                        self.tree.get_span(left_expression_id),
                    ));
                }

                // align parser position with scanner cursor before consuming operator tokens
                if has_pending_newline_tokens {
                    self.advance_to(cursor_index);
                }

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
                        InfixOperator::TypeBinary(
                            TypeBinaryOperator::Cast | TypeBinaryOperator::Satisfies
                        )
                    );

                // cast and satisfies in parenthesized value expressions need
                // the parenthesis flag so the type right side can stop at `)`
                if !parses_value_type_operator_right {
                    right_context = right_context.not_in_parenthesis();
                }

                // type operators in value expressions parse a full type expression on the right
                if !self.options.is_in_type()
                    && matches!(right_operator, InfixOperator::TypeBinary(_))
                {
                    right_context = right_context.not_in_left_precedence();
                }

                // conditional-type right sides must keep their boundary marker active
                if parses_value_type_operator_right
                    || self.options.is_in_type_conditional_right()
                    || matches!(
                        right_operator,
                        InfixOperator::TypeBinary(TypeBinaryOperator::Extends)
                    )
                {
                    right_context = right_context.in_type_conditional_right();
                }

                // type binary operators parse the right side as a type expression
                let right_expression_id = if matches!(right_operator, InfixOperator::TypeBinary(_))
                    && self.is_type_expression_boundary()
                {
                    self.recover_missing_expression_here(NodeType::Expression)
                } else {
                    let right_expression_result =
                        if matches!(right_operator, InfixOperator::TypeBinary(_)) {
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

                // combine into new left expression
                let left_expression = self.make_infix_expression(
                    left_expression_id,
                    right_operator,
                    right_expression_id,
                );
                left_expression_id = self.insert_node(left_expression, self.get_span_from(start));

                // set main span to the operator
                self.tree.set_main_span(left_expression_id, operator_span);

                // use the subject identifier for type predicate spans
                if matches!(
                    self.tree.get(left_expression_id),
                    Expression::TypePredicate { .. }
                ) {
                    let subject_span = self
                        .tree
                        .get_main_span(subject_id)
                        .unwrap_or_else(|| self.tree.get_span(subject_id));
                    self.tree.set_main_span(left_expression_id, subject_span);
                }
            }
        }

        // tail expressions
        let is_in_type = self.options.is_in_type();
        let left_precedence = self.options.left_precedence;
        let mut tail_cursor = self.scanner_cursor_from(self.pos_index());

        // type conditional expression
        if is_in_type {
            if tail_cursor.token_type != TokenType::Maybe {
                return Ok(left_expression_id);
            }

            // align cursor at conditional marker and split operands
            if tail_cursor.index != self.pos_index() {
                self.advance_to(tail_cursor.index);
            }
            // avoid consuming nested conditional tokens in the right side
            let conditional_operands = self.split_type_conditional_operands(left_expression_id);
            if self.options.is_in_type_conditional_right() && conditional_operands.is_none() {
                return Ok(left_expression_id);
            }
            let optional_tuple_pos = if self.peek_is(TokenType::Maybe) {
                Some(self.pos())
            } else {
                None
            };
            let is_optional_tuple = optional_tuple_pos.is_some_and(|pos| {
                self.is_token_after_newlines(pos, TokenType::Comma)
                    || self.is_token_after_newlines(pos, TokenType::CloseBracket)
            });
            let Some((left, right)) = conditional_operands else {
                if is_optional_tuple {
                    return Ok(left_expression_id);
                }
                return Err(ParseError::unexpected(self.peek()?.span));
            };

            self.eat_newlines_maybe()?;
            self.bump(); // eat ?
            self.eat_newlines_maybe()?;
            let then_ambient_context = self.options.with_type(true);
            let then_expression_context = self.options.not_in_position().in_ternary_condition();
            let then_expression_id = self.eat_expression(
                self.options
                    .with_ambient_context(then_ambient_context)
                    .with_expression_context(then_expression_context),
            )?;

            self.eat_newlines_maybe()?;
            self.eat_colon()?;
            self.eat_newlines_maybe()?;
            let mut else_expression_context = self.options.not_in_position();
            if self.options.is_in_type_conditional_right() {
                else_expression_context = else_expression_context.in_type_conditional_right();
            }
            let else_expression_id = self.eat_expression(
                self.options
                    .with_type(true)
                    .with_expression_context(else_expression_context),
            )?;

            let expression = Expression::TypeConditional {
                left,
                right,
                then_type: then_expression_id,
                else_type: else_expression_id,
            };
            left_expression_id = self.insert_node(expression, self.get_span_from(start));
        }
        // value ternary after infix to keep lowest precedence
        // NOTE #Cleanup: having multiple ternary parse locations feels icky (but non-trivial to "fix")
        else if left_precedence.is_none() {
            if tail_cursor.token_type == TokenType::Maybe {
                // move to the ternary marker and parse then and else branches
                if tail_cursor.index != self.pos_index() {
                    self.advance_to(tail_cursor.index);
                }
                self.bump(); // eat ?

                // normalize scanner state before parsing the then branch
                self.eat_newlines_maybe()?;
                let then_options = self
                    .options
                    .not_in_position()
                    .in_ternary_condition()
                    .not_in_sequence_expression();
                let then_expression_id =
                    self.eat_expression_with_context_unchecked(then_options)?;

                // consume the ternary separator after scanner normalization
                self.eat_newlines_maybe()?;
                self.eat_colon()?;

                // normalize scanner state before parsing the else branch
                self.eat_newlines_maybe()?;
                let else_options = self.options.not_in_position().not_in_sequence_expression();
                let else_expression_id =
                    self.eat_expression_with_context_unchecked(else_options)?;

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

            // sequence expression (comma operator) in JS/TS
            if self.options.allows_sequence_expression()
                && (self.language.is_typescript() || self.language.is_javascript())
                && tail_cursor.token_type == TokenType::Comma
            {
                // gather comma-separated expressions into a sequence expression
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
        }

        Ok(left_expression_id)
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

        // static member: .name or .true / .false
        if self.token_is_static_member_name(member_index) {
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
            if self.token_is_static_member_name(separated_member_index) {
                return Some((separated_member_index, false));
            }
        }

        None
    }

    /// Return true when a token index holds a valid static member name.
    #[inline]
    fn token_is_static_member_name(&mut self, token_index: usize) -> bool {
        let token_type = self.token_type_at(token_index);

        if token_type == TokenType::Identifier {
            return true;
        }

        token_type == TokenType::Literal
            && self.token_ref_at(token_index).is_some_and(|token| {
                matches!(token.token.literal, Some(LiteralType::Boolean { .. }))
            })
    }

    /// Check whether the current token sequence can start postfix static arguments.
    fn can_start_postfix_static_arguments(
        &mut self,
        left_expression_id: LocalNodeId<Expression>,
    ) -> bool {
        // shape: `<...>` or `.<...>`
        let has_static_argument_start = self.peek_is(TokenType::LessThan)
            || self.peek_is(TokenType::ShiftLeft)
            || self.has_indirect_postfix_static_arguments();
        if !has_static_argument_start {
            return false;
        }

        // context gates
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

    /// Check whether postfix static arguments are attached through optional chaining.
    fn has_indirect_postfix_static_arguments(&mut self) -> bool {
        self.peek_is(TokenType::Dot)
            && (self.peek_next_is(TokenType::LessThan) || self.peek_next_is(TokenType::ShiftLeft))
    }

    /// Speculatively parse postfix static arguments into either call or instantiation.
    fn eat_postfix_static_call_or_instantiation(
        &mut self,
        start: &ParserMark,
        left_expression_id: LocalNodeId<Expression>,
        has_indirect_static: bool,
    ) -> ParseResult<Option<LocalNodeId<Expression>>> {
        // speculative boundary for optional chaining style static arguments
        let speculative_start = self.mark();
        let speculative_start_idx = self.tree.next_id();

        let position = if has_indirect_static {
            self.bump(); // eat .
            PostfixPosition::Indirect
        } else {
            PostfixPosition::Direct
        };

        // parse `<...>` with regular speculative follow validation
        let static_arguments =
            match self.eat_static_arguments_with_follow_maybe(false, false, false) {
                Some(static_arguments) => static_arguments,
                None => {
                    self.restore(speculative_start, speculative_start_idx);
                    return Ok(None);
                }
            };

        // call with static arguments
        if self.peek_is(TokenType::OpenParenthesis) {
            let expression_id =
                self.eat_call(left_expression_id, Some(static_arguments), position)?;
            return Ok(Some(expression_id));
        }

        let expression_id = self.insert_node(
            Expression::Instantiation {
                left: left_expression_id,
                static_arguments,
            },
            self.get_span_from(start),
        );
        Ok(Some(expression_id))
    }
}
