use crate::parse::prelude::*;
use crate::{ParseError, ParseResult, Parser, ParserMark};

use destack_ast::{
    Argument, BinaryOperator, Declaration, Expression, FunctionKind, IfCondition, IfKind,
    InfixOperator, LocalNodeId, PostfixPosition, TokenType, TypeBinaryOperator, TypeUnaryOperator,
};

impl Parser {
    /// Parse expression continuation operators after a primary expression.
    pub(super) fn eat_expression_continuation(
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
            while self.has_more_tokens() {
                // stop before ternary or switch case boundary so postfix parsing does not consume ':'
                if (self.options.in_ternary_condition || self.options.in_match_case)
                    && (self.peek_is(TokenType::Colon)
                        || self.peek_is(TokenType::Newline) && self.peek_next_is(TokenType::Colon))
                {
                    break;
                }
                // stop before static boundary so postfix parsing does not consume '>'
                if self.options.in_static
                    && (self.peek_is(TokenType::GreaterThan)
                        || self.peek_is(TokenType::Newline)
                            && self.peek_next_is(TokenType::GreaterThan))
                {
                    break;
                }

                if self.is_template_literal_start() {
                    // tagged template receivers must be left hand side expressions
                    if !self.tagged_template_tag_is_valid(left_expression_id) {
                        return Err(ParseError::unexpected(self.peek()?.span));
                    }

                    let template_literal = self.eat_template_literal()?;
                    left_expression_id = self.tree.insert(
                        Expression::TaggedTemplateExpression {
                            tag: left_expression_id,
                            value: template_literal,
                        },
                        self.get_span_from(start),
                    );
                    continue;
                }

                // call parsing flags
                let has_direct_call = self.peek_is(TokenType::OpenParenthesis);
                let has_direct_call_after_newlines = self.peek_is(TokenType::Newline)
                    && self
                        .peek_token_after_newlines(self.pos(), TokenType::OpenParenthesis)
                        .is_ok();
                let has_indirect_call =
                    self.peek_is(TokenType::Dot) && self.peek_next_is(TokenType::OpenParenthesis);
                let can_direct_call = (has_direct_call || has_direct_call_after_newlines)
                    && !self.options.in_type
                    && !matches!(self.tree.get(left_expression_id), Expression::Maybe { .. })
                    && !self.options.in_new_receiver;
                let should_parse_call =
                    (can_direct_call || has_indirect_call) && !self.options.in_type;

                // unary postfix operations
                if let Ok(operator) = self.peek_unary_postfix_operator() {
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
                else if let Ok(operator) = self.peek_type_unary_postfix_operator() {
                    // avoid consuming conditional type ? as a type maybe
                    let operator_start = self.mark_span();
                    self.bump(); // eat type unary operator
                    if matches!(
                        operator,
                        TypeUnaryOperator::AsConst | TypeUnaryOperator::AsComptime
                    ) {
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
                // range (`..`, `..=`)
                else if self.peek_is(TokenType::Range) {
                    self.bump(); // eat ..
                    let is_inclusive = if self.peek_is(TokenType::Assign) {
                        self.bump(); // eat =
                        true
                    } else {
                        false
                    };
                    let right_expression_id = self
                        .with_options(self.options.not_in_position(), |parser| {
                            parser.eat_expression()
                        })?;
                    left_expression_id = self.tree.insert(
                        Expression::RangeExpression {
                            start: left_expression_id,
                            end: right_expression_id,
                            is_inclusive,
                        },
                        self.get_span_from(start),
                    );
                }
                // private member (also works across newline)
                else if let Ok(distance) = self.peek_private_member() {
                    self.bump_by(distance - 1); // keep the identifier
                    let (name, name_span) = self.eat_identifier_with_span()?;
                    // speculatively unwrap postfix static parameterisation with `<`
                    //  (might also be just a comparison operator)
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
                }
                // member (also works across newline)
                else if let Ok(distance) = self.peek_member_name() {
                    self.bump_by(distance - 1); // keep the identifier

                    // decimal integer literals need a separator before member access
                    if self.invalid_decimal_integer_member_access(left_expression_id, distance) {
                        return Err(ParseError::unexpected(self.prev().expect("peeked").span));
                    }

                    let (name, name_span) = self.eat_member_name_with_span()?;
                    // speculatively unwrap postfix static parameterisation with `<`
                    //  (might also be just a comparison operator)
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
                }
                // index (like `[]`)
                else if self.peek_is(TokenType::OpenBracket)
                    && !matches!(self.tree.get(left_expression_id), Expression::Maybe { .. })
                    || self.peek_is(TokenType::Dot) && self.peek_next_is(TokenType::OpenBracket)
                {
                    let position = if self.peek_is(TokenType::Dot) {
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
                    if self.peek_is(TokenType::Newline)
                        && self
                            .peek_token_after_newlines(self.pos(), TokenType::OpenParenthesis)
                            .is_ok()
                    {
                        self.eat_newlines_maybe()?; // eat newlines
                    }
                    let position = if self.peek_is(TokenType::Dot) {
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
                else if self.peek_is(TokenType::Maybe)
                    || self.peek_is(TokenType::Dot) && self.peek_next_is(TokenType::Maybe)
                    || self.optional_chain_starts_after_newlines()
                {
                    // capture type conditional operands when in type contexts
                    let type_conditional_operands = if self.options.in_type {
                        self.split_type_conditional_operands(left_expression_id)
                    } else {
                        None
                    };
                    let is_type_conditional = type_conditional_operands.is_some();

                    // normalize newlines before checking postfix markers
                    self.eat_newlines_maybe()?; // eat newlines

                    // classify optional chain and postfix maybe
                    let is_optional_chain_after_maybe = self.is_optional_chain_after_maybe();
                    let is_direct_postfix_maybe = self.language.is_destack()
                        && (self.peek_next_any_stop().is_ok()
                            && self.prev_token_type() != TokenType::Newline
                            || self.peek_next_any_close_parenthesis().is_ok()
                            || self.peek_next_assign_operator().is_ok());
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
                        let then_expression_id = self.with_options(
                            self.options
                                .not_in_position()
                                .in_type()
                                .in_ternary_condition(),
                            |parser| parser.eat_expression(),
                        )?;
                        self.eat_newlines_maybe()?;
                        self.eat_colon()?;
                        self.eat_newlines_maybe()?;
                        let mut else_options = self.options.not_in_position().in_type();
                        if self.options.in_type_conditional_right {
                            else_options = else_options.in_type_conditional_right();
                        }
                        let else_expression_id =
                            self.with_options(else_options, |parser| parser.eat_expression())?;
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
                else if self.peek_is(TokenType::Not)
                    || self.peek_is(TokenType::Dot) && self.peek_next_is(TokenType::Not)
                {
                    let position = if self.peek_is(TokenType::Dot) {
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
                else if self.options.in_parenthesis && self.peek_is(TokenType::Comma) {
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
                            let expr_id = self.with_options(
                                self.options.not_in_position().not_in_sequence_expression(),
                                |parser| parser.eat_expression(),
                            )?;
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
            while self.has_more_tokens() {
                // new receivers stop before type argument delimiters at top-level receiver scope
                if self.options.in_new_receiver
                    && !self.options.in_parenthesis
                    && (self.peek_is(TokenType::LessThan)
                        || self.peek_is(TokenType::ShiftLeft)
                        || self.peek_is(TokenType::Newline)
                            && (self.peek_next_is(TokenType::LessThan)
                                || self.peek_next_is(TokenType::ShiftLeft)))
                {
                    break;
                }
                // stop before conditional boundaries so infix lookahead does not lex past `?`
                if self.peek_is(TokenType::Maybe)
                    || self.peek_is(TokenType::Newline) && self.peek_next_is(TokenType::Maybe)
                {
                    break;
                }
                // stop before ternary or match case boundary so infix lookahead does not lex past `:`
                if (self.options.in_ternary_condition || self.options.in_match_case)
                    && (self.peek_is(TokenType::Colon)
                        || self.peek_is(TokenType::Newline) && self.peek_next_is(TokenType::Colon))
                {
                    break;
                }
                // statement expressions do not continue across newlines
                if left_is_statement && self.peek_is(TokenType::Newline) {
                    break;
                }
                // type expressions stop before tree literals on a new line
                if self.options.in_type
                    && self.peek_is(TokenType::Newline)
                    && self.can_start_tree_literal_after_newline()
                {
                    break;
                }
                let (right_operator, operator_offset) = {
                    // infix operator on same line with higher precedence
                    if let Ok((operator, operator_offset)) = self.peek_infix_operator()
                        && (self.options.left_precedence.is_none()
                            || self.options.left_precedence.unwrap() < operator.precedence())
                    {
                        (operator, operator_offset)
                    }
                    // infix operator on next line with higher precedence
                    else if self.peek_is(TokenType::Newline)
                        && let Ok((operator, operator_offset)) = self.peek_next_infix_operator()
                        && (self.options.left_precedence.is_none()
                            || self.options.left_precedence.unwrap() < operator.precedence())
                    {
                        (operator, operator_offset)
                    }
                    // infix operator after multiple newlines
                    else if self.peek_is(TokenType::Newline)
                        && let Ok((operator, operator_offset)) =
                            self.peek_infix_operator_after_newlines()
                        && (!self.options.in_type
                            || matches!(
                                operator,
                                InfixOperator::Binary(
                                    BinaryOperator::ElementwiseOr | BinaryOperator::ElementwiseAnd
                                )
                            ))
                        && (self.options.left_precedence.is_none()
                            || self.options.left_precedence.unwrap() < operator.precedence())
                    {
                        (operator, operator_offset)
                    }
                    // no infix operator with higher precedence
                    else {
                        break;
                    }
                };
                if self.peek_is(TokenType::Newline) {
                    self.eat_newlines_maybe()?; // eat newlines
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
                let right_expression_id =
                    self.with_options(right_options, |parser| parser.eat_expression())?;

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
        if !self.options.in_type
            && self.options.left_precedence.is_none()
            && (self.peek_is(TokenType::Maybe)
                || self.peek_is(TokenType::Newline) && self.peek_next_is(TokenType::Maybe))
        {
            self.eat_newlines_maybe()?;
            self.bump(); // eat ?
            self.eat_newlines_maybe()?;
            let then_expression_id = self.with_options(
                self.options
                    .not_in_position()
                    .in_ternary_condition()
                    .not_in_sequence_expression(),
                |parser| parser.eat_expression(),
            )?;
            self.eat_newlines_maybe()?;
            self.eat_colon()?;
            self.eat_newlines_maybe()?;
            let else_expression_id = self.with_options(
                self.options.not_in_position().not_in_sequence_expression(),
                |parser| parser.eat_expression(),
            )?;
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
        if !self.options.in_type
            && self.options.left_precedence.is_none()
            && self.options.allow_sequence_expression
            && (self.language.is_typescript() || self.language.is_javascript())
            && (self.peek_is(TokenType::Comma)
                || self.peek_is(TokenType::Newline) && self.peek_next_is(TokenType::Comma))
        {
            let mut expressions = vec![left_expression_id];
            loop {
                if self.peek_is(TokenType::Newline) {
                    let has_comma_after_newlines = self
                        .peek_token_after_newlines(self.pos(), TokenType::Comma)
                        .is_ok();
                    if !has_comma_after_newlines {
                        break;
                    }
                    self.eat_newlines_maybe()?;
                }
                if !self.peek_is(TokenType::Comma) {
                    break;
                }
                self.bump(); // eat comma
                self.eat_newlines_maybe()?;
                let expression_id = self.with_options(
                    self.options.not_in_position().not_in_sequence_expression(),
                    |parser| parser.eat_expression(),
                )?;
                expressions.push(expression_id);
            }

            let expression = Expression::SequenceExpression { expressions };
            left_expression_id = self.tree.insert(expression, self.get_span_from(start));
        }

        // type conditional expression
        if self.options.in_type
            && (self.peek_is(TokenType::Maybe)
                || self.peek_is(TokenType::Newline) && self.peek_next_is(TokenType::Maybe))
        {
            // avoid consuming nested conditional tokens in the right side
            let conditional_operands = self.split_type_conditional_operands(left_expression_id);
            if self.options.in_type_conditional_right && conditional_operands.is_none() {
                return Ok(left_expression_id);
            }
            let optional_tuple_pos = if self.peek_is(TokenType::Maybe) {
                Some(self.pos())
            } else if self.peek_is(TokenType::Newline) && self.peek_next_is(TokenType::Maybe) {
                Some(self.pos().saturating_add(1))
            } else {
                None
            };
            let is_optional_tuple = optional_tuple_pos.is_some_and(|pos| {
                self.peek_token_after_newlines(pos, TokenType::Comma)
                    .is_ok()
                    || self
                        .peek_token_after_newlines(pos, TokenType::CloseBracket)
                        .is_ok()
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
            let then_expression_id = self.with_options(
                self.options
                    .not_in_position()
                    .in_type()
                    .in_ternary_condition(),
                |parser| parser.eat_expression(),
            )?;
            self.eat_newlines_maybe()?;
            self.eat_colon()?;
            self.eat_newlines_maybe()?;
            let mut else_options = self.options.not_in_position().in_type();
            if self.options.in_type_conditional_right {
                else_options = else_options.in_type_conditional_right();
            }
            let else_expression_id =
                self.with_options(else_options, |parser| parser.eat_expression())?;
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
