use crate::parse::prelude::*;
use crate::{ParseError, ParseResult, Parser, ParserMark};

use destack_ast::{
    Argument, AssignOperator, BinaryOperator, Declaration, Expression, FunctionKind, IfCondition,
    IfKind, InfixOperator, LiteralType, LocalNodeId, PostfixPosition, TokenType,
    TypeBinaryOperator, TypeUnaryOperator, UnaryOperator,
};

impl Parser {
    /// Return whether a newline direct call should terminate in statement position.
    #[inline]
    fn newline_direct_call_terminates_statement(&mut self, open_parenthesis_index: usize) -> bool {
        if !self.options.in_statement_context {
            return false;
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
        let is_followup_chain = matches!(next_token_type, TokenType::Dot | TokenType::Maybe);

        is_postfix_or_assign || is_followup_chain
    }

    /// Parse expression continuation operators after a primary expression.
    pub(crate) fn eat_expression_continuation(
        &mut self,
        start: &ParserMark,
        mut left_expression_id: LocalNodeId<Expression>,
    ) -> ParseResult<LocalNodeId<Expression>> {
        // statement expressions do not accept postfix or infix operators
        if self.options.in_statement_position {
            let expression = self.tree.get(left_expression_id);
            if expression.is_top_level_statement() {
                let is_lambda_declaration = match expression {
                    Expression::Declaration(declaration_id) => {
                        matches!(
                            self.tree.get(*declaration_id),
                            Declaration::Function { signature, .. }
                                if signature.kind == FunctionKind::Lambda
                        )
                    }
                    _ => false,
                };
                if !is_lambda_declaration {
                    return Ok(left_expression_id);
                }
            }
        }

        //
        // ------------------------------------------------------------
        // Postfix operations
        // ------------------------------------------------------------
        //

        {
            let _timing = self.timing_scope(tags::PARSE_EXPRESSION_POSTFIX);
            // struct literal postfix with `{` (like `Vector2 { x: 0, y }`)
            if let Expression::Path { .. } = self.tree.get(left_expression_id)
                && self.peek_is(TokenType::OpenBrace)
                && !self.options.in_before_block
                && self.language.is_destack()
            {
                let properties = self.eat_object_literal()?;
                left_expression_id = self.tree.insert(
                    Expression::ObjectExpression {
                        ty: Some(left_expression_id),
                        properties,
                    },
                    self.get_span_from(start),
                );
            }
            // eat all regular postfix operators
            loop {
                // load scanner state for this postfix step
                let cursor = self.peek_scanner_cursor();
                let token_type = cursor.token_type;

                if token_type == TokenType::End {
                    break;
                }

                let cursor_index = cursor.index;
                let has_pending_newline_tokens = cursor.index != self.pos_index();
                let has_line_break_before = cursor.has_line_break_before;
                let next_token_type = if matches!(token_type, TokenType::Dot | TokenType::Maybe) {
                    self.token_type_at(cursor_index.saturating_add(1))
                } else {
                    TokenType::End
                };
                let dot_member_target = if token_type == TokenType::Dot {
                    self.peek_dot_member_target(cursor_index, left_expression_id)
                } else {
                    None
                };

                // most postfix operators are not allowed across newline tokens
                if has_pending_newline_tokens
                    && !matches!(
                        token_type,
                        TokenType::OpenParenthesis | TokenType::Dot | TokenType::Maybe
                    )
                {
                    break;
                }

                // stop before ternary or switch case boundary so postfix parsing does not consume ':'
                if (self.options.in_ternary_condition || self.options.in_match_case)
                    && token_type == TokenType::Colon
                {
                    break;
                }
                // stop before static boundary so postfix parsing does not consume '>'
                if self.options.in_static && token_type == TokenType::GreaterThan {
                    break;
                }

                if self.is_template_literal_start() {
                    // tagged template receivers must be left hand side expressions
                    if !self.tagged_template_tag_is_valid(left_expression_id) {
                        return Err(ParseError::unexpected(self.peek()?.span));
                    }

                    let template_literal = self.eat_tagged_template_literal()?;
                    left_expression_id = self.tree.insert(
                        Expression::TaggedTemplateExpression {
                            tag: left_expression_id,
                            value: template_literal,
                        },
                        self.get_span_from(start),
                    );
                    continue;
                }

                // call
                let has_direct_call =
                    token_type == TokenType::OpenParenthesis && !has_line_break_before;
                let has_direct_call_after_newlines =
                    token_type == TokenType::OpenParenthesis && has_line_break_before;
                let should_terminate_newline_direct_call = has_direct_call_after_newlines
                    && self.newline_direct_call_terminates_statement(cursor_index);
                let has_indirect_call =
                    token_type == TokenType::Dot && next_token_type == TokenType::OpenParenthesis;
                let can_direct_call = (has_direct_call
                    || has_direct_call_after_newlines && !should_terminate_newline_direct_call)
                    && !self.options.in_type
                    && !matches!(self.tree.get(left_expression_id), Expression::Maybe { .. })
                    && !self.options.in_new_receiver;

                // direct and indirect call dispatch share the same continuation lane
                let should_parse_call =
                    (can_direct_call || has_indirect_call) && !self.options.in_type;

                // unary postfix operations
                if let Some(operator) = UnaryOperator::from_postfix_token(token_type) {
                    let operator_start = self.mark_span();
                    self.bump(); // eat unary operator
                    let operator_span = self.get_span_from(&operator_start);
                    left_expression_id = self.tree.insert(
                        Expression::Unary {
                            operator,
                            right: left_expression_id,
                        },
                        self.get_span_from(start),
                    );
                    self.tree.set_main_span(left_expression_id, operator_span);
                }
                // type unary postfix operations
                else if matches!(token_type, TokenType::Maybe | TokenType::Identifier)
                    && let Some(operator) = self.peek_type_unary_postfix_operator_maybe()
                {
                    // avoid consuming conditional type ? as a type maybe
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
                    left_expression_id = self.tree.insert(
                        Expression::TypeUnary {
                            operator,
                            right: left_expression_id,
                        },
                        self.get_span_from(start),
                    );
                    self.tree.set_main_span(left_expression_id, operator_span);
                }
                // dot member and private member dispatch via scanner cursor
                else if let Some((member_index, is_private_member)) = dot_member_target {
                    let member_distance = member_index
                        .saturating_sub(self.pos_index())
                        .saturating_add(1);
                    let member_distance = u8::try_from(member_distance).unwrap_or(u8::MAX);

                    if has_pending_newline_tokens {
                        self.advance_to(cursor_index);
                    }

                    // consume the dot and jump to member token index when needed
                    self.bump(); // eat .
                    if self.pos_index() != member_index {
                        self.advance_to(member_index);
                    }

                    if is_private_member {
                        self.bump(); // eat #
                        let (name, name_span) = self.eat_identifier_with_span()?;
                        let static_arguments = self.eat_static_arguments_in_expression(false);
                        left_expression_id = self.tree.insert(
                            Expression::PrivateMember {
                                left: left_expression_id,
                                name,
                                static_arguments,
                            },
                            self.get_span_from(start),
                        );
                        self.tree.set_main_span(left_expression_id, name_span);
                        continue;
                    }

                    // decimal integer literals need a separator before member access
                    if self
                        .invalid_decimal_integer_member_access(left_expression_id, member_distance)
                    {
                        return Err(ParseError::unexpected(self.prev().expect("peeked").span));
                    }

                    let (name, name_span) = self.eat_member_name_with_span()?;
                    let static_arguments = self.eat_static_arguments_in_expression(false);
                    left_expression_id = self.tree.insert(
                        Expression::Member {
                            left: left_expression_id,
                            name,
                            static_arguments,
                        },
                        self.get_span_from(start),
                    );
                    self.tree.set_main_span(left_expression_id, name_span);
                    continue;
                }
                // index (like `[]`)
                else if token_type == TokenType::OpenBracket
                    && !matches!(self.tree.get(left_expression_id), Expression::Maybe { .. })
                    || token_type == TokenType::Dot && next_token_type == TokenType::OpenBracket
                {
                    if has_pending_newline_tokens {
                        self.advance_to(cursor_index);
                    }
                    let position = if token_type == TokenType::Dot {
                        self.bump(); // eat .
                        PostfixPosition::Indirect
                    } else {
                        PostfixPosition::Direct
                    };
                    left_expression_id = self.eat_index(left_expression_id, position)?;
                }
                // call (like `()`)
                else if should_parse_call {
                    // unparenthesized arrow functions cannot be direct call receivers
                    if has_direct_call
                        && self.is_unparenthesized_lambda_expression(left_expression_id)
                    {
                        return Err(ParseError::unexpected(self.peek()?.span));
                    }

                    let _call_timing = self.timing_scope(tags::PARSE_EXPRESSION_POSTFIX_CALL);
                    if has_pending_newline_tokens {
                        self.advance_to(cursor_index);
                    }
                    let position = if token_type == TokenType::Dot {
                        self.bump(); // eat .
                        PostfixPosition::Indirect
                    } else {
                        PostfixPosition::Direct
                    };
                    left_expression_id = self.eat_call(left_expression_id, None, position)?;
                }
                // statically parameterized call or instantiation expression (like `(expr)<T>()` or `(expr)<T>`)
                else if self.can_start_postfix_static_arguments(left_expression_id) {
                    // direct static arguments and optional chain static arguments are exclusive
                    let has_indirect_static = self.has_indirect_postfix_static_arguments();
                    let is_optional_chain =
                        matches!(self.tree.get(left_expression_id), Expression::Maybe { .. });
                    if has_indirect_static == is_optional_chain {
                        let next_expression_id = self.eat_postfix_static_call_or_instantiation(
                            start,
                            left_expression_id,
                            has_indirect_static,
                        )?;
                        let Some(next_expression_id) = next_expression_id else {
                            break;
                        };

                        left_expression_id = next_expression_id;
                    } else {
                        break;
                    }
                }
                // optional chaining or type conditional boundary
                // (like `x?`, `x.?`, `x?.`)
                else if token_type == TokenType::Maybe
                    || token_type == TokenType::Dot && next_token_type == TokenType::Maybe
                {
                    if has_pending_newline_tokens {
                        self.advance_to(cursor_index);
                    }
                    // capture type conditional operands when in type contexts
                    let type_conditional_operands = if self.options.in_type {
                        self.split_type_conditional_operands(left_expression_id)
                    } else {
                        None
                    };
                    let is_type_conditional = type_conditional_operands.is_some();

                    // classify optional chain and postfix maybe
                    let is_optional_chain_after_maybe = self.is_optional_chain_after_maybe();
                    let is_direct_postfix_maybe = self.language.is_destack()
                        && (self.is_next_any_stop()
                            && self.prev_token_type() != TokenType::Newline
                            || self.is_next_any_close_parenthesis()
                            || self.peek_next_assign_operator_is());
                    let is_postfix_maybe = !self.options.in_type
                        && self.peek_is(TokenType::Maybe)
                        && (is_optional_chain_after_maybe || is_direct_postfix_maybe);

                    // postfix maybe
                    if is_postfix_maybe {
                        self.bump(); // eat ?
                        // (don't consume delimiter/stop)
                        left_expression_id = self.tree.insert(
                            Expression::Maybe {
                                left: left_expression_id,
                                position: PostfixPosition::Direct,
                            },
                            self.get_span_from(start),
                        );
                    }
                    // dot maybe
                    else if !self.options.in_type
                        && self.peek_is(TokenType::Dot)
                        && self.peek_next_is(TokenType::Maybe)
                    {
                        self.bump(); // eat .
                        self.bump(); // eat ?
                        left_expression_id = self.tree.insert(
                            Expression::Maybe {
                                left: left_expression_id,
                                position: PostfixPosition::Indirect,
                            },
                            self.get_span_from(start),
                        );
                    }
                    // type conditional expressions in type contexts
                    else {
                        if !self.options.in_type || !is_type_conditional {
                            break;
                        }

                        self.bump(); // eat ?
                        self.eat_newlines_maybe()?;
                        let Some((left, right)) = type_conditional_operands else {
                            break;
                        };
                        // type conditional expression
                        let then_options = self
                            .options
                            .not_in_position()
                            .in_type()
                            .in_ternary_condition();
                        let then_expression_id = self.eat_expression(then_options)?;
                        self.eat_newlines_maybe()?;
                        self.eat_colon()?;
                        self.eat_newlines_maybe()?;
                        let mut else_options = self.options.not_in_position().in_type();
                        if self.options.in_type_conditional_right {
                            else_options = else_options.in_type_conditional_right();
                        }
                        let else_expression_id = self.eat_expression(else_options)?;
                        let expression = Expression::TypeConditional {
                            left,
                            right,
                            then_type: then_expression_id,
                            else_type: else_expression_id,
                        };
                        left_expression_id =
                            self.tree.insert(expression, self.get_span_from(start));
                    }
                }
                // must
                else if token_type == TokenType::Not
                    || token_type == TokenType::Dot && next_token_type == TokenType::Not
                {
                    let position = if token_type == TokenType::Dot {
                        self.bump(); // eat .
                        PostfixPosition::Indirect
                    } else {
                        PostfixPosition::Direct
                    };
                    self.bump(); // eat !
                    left_expression_id = self.tree.insert(
                        Expression::Must {
                            position,
                            left: left_expression_id,
                        },
                        self.get_span_from(start),
                    );
                }
                // tuple (Destack) or sequence expression (JS/TS)
                // (if we have a delimiter following an expression inside parentheses)
                else if self.options.in_parenthesis && token_type == TokenType::Comma {
                    self.bump(); // eat comma
                    self.eat_newlines_maybe()?;
                    if self.language.is_destack() {
                        // build a tuple
                        let first_element_id = self.tree.insert(
                            Argument::Positional {
                                modifiers: None,
                                value: left_expression_id,
                            },
                            self.get_span_from(start),
                        );
                        // parse remaining elements
                        let tuple_elements =
                            self.with_options(self.options.not_in_position(), |parser| {
                                parser.eat_sequence_literal_body(
                                    Some(first_element_id),
                                    TokenType::CloseParenthesis,
                                )
                            })?;
                        // build tuple literal
                        left_expression_id = self.tree.insert(
                            Expression::TupleExpression {
                                elements: tuple_elements,
                            },
                            self.get_span_from(start),
                        );
                    } else {
                        // build a sequence expression (comma operator)
                        let mut expressions = vec![left_expression_id];
                        // parse remaining expressions until we see the close parenthesis
                        // (mirrors eat_sequence_literal_body behavior for consistency)
                        while !self.peek_is(TokenType::CloseParenthesis) {
                            // consume any comma delimiter
                            if self.peek_is(TokenType::Comma) {
                                self.bump(); // eat comma
                                self.eat_newlines_maybe()?;
                                continue;
                            }
                            // parse next expression
                            let expression_options =
                                self.options.not_in_position().not_in_sequence_expression();
                            let expr_id = self.eat_expression(expression_options)?;
                            expressions.push(expr_id);
                            self.eat_newlines_maybe()?;
                        }
                        // build sequence expression
                        left_expression_id = self.tree.insert(
                            Expression::SequenceExpression { expressions },
                            self.get_span_from(start),
                        );
                    }
                }
                // done
                else {
                    break;
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
        let left_is_statement = self.options.in_statement_position
            && self
                .tree
                .get(left_expression_id)
                .ends_statement_on_newline();

        {
            let _timing = self.timing_scope(tags::PARSE_EXPRESSION_INFIX);
            let left_precedence = self.options.left_precedence;
            loop {
                let cursor = self.peek_scanner_cursor();
                let token_type = cursor.token_type;
                if token_type == TokenType::End {
                    break;
                }

                // load cursor details once per infix iteration
                let cursor_index = cursor.index;
                let newline_count = cursor.skipped_newline_count;
                let has_line_break_before = cursor.has_line_break_before;
                let has_pending_newline_tokens = cursor_index != self.pos_index();

                // new receivers stop before type argument delimiters at top-level receiver scope
                if self.options.in_new_receiver
                    && !self.options.in_parenthesis
                    && (token_type == TokenType::LessThan || token_type == TokenType::ShiftLeft)
                {
                    break;
                }
                // stop before conditional boundaries so infix lookahead does not lex past `?`
                if token_type == TokenType::Maybe {
                    break;
                }
                // stop before ternary or match case boundary so infix lookahead does not lex past `:`
                if (self.options.in_ternary_condition || self.options.in_match_case)
                    && token_type == TokenType::Colon
                {
                    break;
                }
                // statement expressions do not continue across line breaks
                if left_is_statement && has_line_break_before {
                    break;
                }
                // type expressions stop before tree literals after a line break
                if self.options.in_type
                    && has_line_break_before
                    && self.can_start_tree_literal_after_line_break()
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
                    && self.options.in_type
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
                let mut right_options = self
                    .options
                    .not_in_position()
                    .in_left_precedence(right_operator.precedence());
                let parses_value_type_operator_right = !self.options.in_type
                    && matches!(
                        right_operator,
                        InfixOperator::TypeBinary(
                            TypeBinaryOperator::Cast | TypeBinaryOperator::Satisfies
                        )
                    );

                // type operators in value expressions parse a full type expression on the right
                if !self.options.in_type && matches!(right_operator, InfixOperator::TypeBinary(_)) {
                    right_options = right_options.not_in_left_precedence();
                }

                // type binary operators parse the right side as a type expression
                if matches!(right_operator, InfixOperator::TypeBinary(_)) {
                    right_options = right_options.in_type();
                }
                if parses_value_type_operator_right {
                    right_options = right_options.in_type_conditional_right();
                }
                if self.options.in_type_conditional_right
                    || matches!(
                        right_operator,
                        InfixOperator::TypeBinary(TypeBinaryOperator::Extends)
                    )
                {
                    right_options = right_options.in_type_conditional_right();
                }
                let right_expression_id = self.eat_expression(right_options)?;

                // combine into new left expression
                let left_expression = self.make_infix_expression(
                    left_expression_id,
                    right_operator,
                    right_expression_id,
                );
                left_expression_id = self.tree.insert(left_expression, self.get_span_from(start));

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

        // value ternary after infix to keep lowest precedence
        // NOTE #Cleanup: having multiple ternary parse locations feels icky (but non-trivial to "fix")
        let ternary_cursor = self.peek_scanner_cursor();
        if !self.options.in_type
            && self.options.left_precedence.is_none()
            && ternary_cursor.token_type == TokenType::Maybe
        {
            // move to the ternary marker and parse then and else branches
            if ternary_cursor.index != self.pos_index() {
                self.advance_to(ternary_cursor.index);
            }
            self.bump(); // eat ?

            // normalize scanner state before parsing the then branch
            let then_cursor = self.peek_scanner_cursor();
            if then_cursor.index != self.pos_index() {
                self.advance_to(then_cursor.index);
            }
            let then_options = self
                .options
                .not_in_position()
                .in_ternary_condition()
                .not_in_sequence_expression();
            let then_expression_id = self.eat_expression(then_options)?;

            // consume the ternary separator after scanner normalization
            let colon_cursor = self.peek_scanner_cursor();
            if colon_cursor.index != self.pos_index() {
                self.advance_to(colon_cursor.index);
            }
            self.eat_colon()?;

            // normalize scanner state before parsing the else branch
            let else_cursor = self.peek_scanner_cursor();
            if else_cursor.index != self.pos_index() {
                self.advance_to(else_cursor.index);
            }
            let else_options = self.options.not_in_position().not_in_sequence_expression();
            let else_expression_id = self.eat_expression(else_options)?;

            let expression = Expression::If {
                kind: IfKind::Ternary,
                condition: IfCondition::Expression {
                    condition: left_expression_id,
                },
                then_expression: then_expression_id,
                else_expression: Some(else_expression_id),
            };
            left_expression_id = self.tree.insert(expression, self.get_span_from(start));
        }

        // sequence expression (comma operator) in JS/TS
        let sequence_cursor = self.peek_scanner_cursor();
        if !self.options.in_type
            && self.options.left_precedence.is_none()
            && self.options.allow_sequence_expression
            && (self.language.is_typescript() || self.language.is_javascript())
            && sequence_cursor.token_type == TokenType::Comma
        {
            // gather comma-separated expressions into a sequence expression
            let mut expressions = vec![left_expression_id];
            loop {
                let comma_cursor = self.peek_scanner_cursor();
                if comma_cursor.token_type != TokenType::Comma {
                    break;
                }
                if comma_cursor.index != self.pos_index() {
                    self.advance_to(comma_cursor.index);
                }
                self.bump(); // eat comma
                self.eat_newlines_maybe()?;
                let expression_options =
                    self.options.not_in_position().not_in_sequence_expression();
                let expression_id = self.eat_expression(expression_options)?;
                expressions.push(expression_id);
            }

            let expression = Expression::SequenceExpression { expressions };
            left_expression_id = self.tree.insert(expression, self.get_span_from(start));
        }

        // type conditional expression
        let type_conditional_cursor = self.peek_scanner_cursor();
        if self.options.in_type && type_conditional_cursor.token_type == TokenType::Maybe {
            // align cursor at conditional marker and split operands
            if type_conditional_cursor.index != self.pos_index() {
                self.advance_to(type_conditional_cursor.index);
            }
            // avoid consuming nested conditional tokens in the right side
            let conditional_operands = self.split_type_conditional_operands(left_expression_id);
            if self.options.in_type_conditional_right && conditional_operands.is_none() {
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
            let then_cursor = self.peek_scanner_cursor();
            if then_cursor.index != self.pos_index() {
                self.advance_to(then_cursor.index);
            }
            let then_options = self
                .options
                .not_in_position()
                .in_type()
                .in_ternary_condition();
            let then_expression_id = self.eat_expression(then_options)?;

            let colon_cursor = self.peek_scanner_cursor();
            if colon_cursor.index != self.pos_index() {
                self.advance_to(colon_cursor.index);
            }
            self.eat_colon()?;
            self.eat_newlines_maybe()?;
            let else_cursor = self.peek_scanner_cursor();
            if else_cursor.index != self.pos_index() {
                self.advance_to(else_cursor.index);
            }
            let mut else_options = self.options.not_in_position().in_type();
            if self.options.in_type_conditional_right {
                else_options = else_options.in_type_conditional_right();
            }
            let else_expression_id = self.eat_expression(else_options)?;

            let expression = Expression::TypeConditional {
                left,
                right,
                then_type: then_expression_id,
                else_type: else_expression_id,
            };
            left_expression_id = self.tree.insert(expression, self.get_span_from(start));
        }

        Ok(left_expression_id)
    }

    /// Return a valid dot-member target after `.` from a scanner cursor index.
    fn peek_dot_member_target(
        &mut self,
        dot_index: usize,
        left_expression_id: LocalNodeId<Expression>,
    ) -> Option<(usize, bool)> {
        let member_index = self.next_non_newline_index_from(dot_index.saturating_add(1));
        let member_token_type = self.token_type_at(member_index);

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
        if self.options.in_new_receiver || self.options.in_tree_literal {
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

        // instantiation expression
        if self.can_follow_type_arguments_in_expression() {
            let expression_id = self.tree.insert(
                Expression::Instantiation {
                    left: left_expression_id,
                    static_arguments,
                },
                self.get_span_from(start),
            );
            return Ok(Some(expression_id));
        }

        // rollback when follow token cannot continue an expression
        self.restore(speculative_start, speculative_start_idx);
        Ok(None)
    }
}
