use super::common::{DECLARATION_START_TOKENS, DescriptorHead, is_type_relation_keyword};
use super::lookahead::ParenthesizedGroupShape;
use crate::parse::prelude::*;
use crate::{ParseError, ParseResult, Parser, ParserMark};

use destack_ast::{
    BinaryOperator, Block, BlockFormat, Declaration, Expression, Keyword, LocalNodeId, NodeType,
    TokenType, TypeUnaryOperator, UnaryOperator,
};

impl Parser {
    pub fn eat_expression(&mut self) -> ParseResult<LocalNodeId<Expression>> {
        let _timing = self.timing_scope(tags::PARSE_EXPRESSION);

        destack_base::ensure_sufficient_stack(|| self.eat_expression_inner())
    }

    /// Try to eat an expression and recover to an error node.
    pub fn try_eat_expression(
        &mut self,
        recover: TokenType,
    ) -> ParseResult<LocalNodeId<Expression>> {
        match self.eat_expression() {
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
            self.keyword_for_index(self.pos_index())
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

    /// Return true when the current identifier should be parsed as a contextual type literal.
    #[inline]
    fn should_try_contextual_type_literal(&mut self) -> bool {
        if self.options.in_type || self.options.in_static {
            return true;
        }

        let pos = self.pos_index();
        self.token_stream.ensure_token(pos);
        let Some(token) = self.tokens().get(pos) else {
            return false;
        };
        if token.token.ty != TokenType::Identifier {
            return false;
        }

        matches!(
            self.get_span_str(token.span),
            "undefined" | "unknown" | "object" | "null" | "any" | "never"
        )
    }

    /// Eat an expression body without stack growth.
    fn eat_expression_inner(&mut self) -> ParseResult<LocalNodeId<Expression>> {
        // consume decorator prefixes before parsing the next expression
        if !self.options.in_decorator && self.peek_is(TokenType::At) {
            self.eat_decorators_prefix_maybe()?;
        }

        let start = self.mark_span();

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
                    self.eat_expression()?
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
                return Ok(labelled_id);
            }
        }

        //
        // ------------------------------------------------------------
        // Modifiers
        // ------------------------------------------------------------
        //

        let descriptor = match self.eat_declaration_descriptor(&start)? {
            DescriptorHead::Descriptor(descriptor) => descriptor,
            DescriptorHead::Expression(expression_id) => return Ok(expression_id),
        };

        //
        // ------------------------------------------------------------
        // Main expression
        // ------------------------------------------------------------
        //

        let left_expression_id: LocalNodeId<Expression> = {
            let _timing = self.timing_scope(tags::PARSE_EXPRESSION_PRIMARY);
            let token_type = self.peek_token_type();

            //
            // ------------------------------------------------------------
            // Grouping
            // ------------------------------------------------------------
            //

            // identifier paths and keyword expressions
            match token_type {
                TokenType::Identifier => {
                    // identifier context setup
                    let next_token_type = self.peek_next_token_type();
                    let is_declaration_start = DECLARATION_START_TOKENS.contains(&next_token_type);
                    let module_identifier_matches = !self.options.in_decorator
                        && !self.options.in_type
                        && self.language.supports_module_declaration()
                        && self.identifier_equals_at(self.pos_index(), "module");
                    let next_keyword = if next_token_type == TokenType::Identifier {
                        self.keyword_for_index(self.index_for_next())
                    } else {
                        None
                    };
                    let is_module_name_start =
                        matches!(next_token_type, TokenType::Identifier | TokenType::Literal);
                    let is_module_type_operator = is_type_relation_keyword(next_keyword);
                    let is_module_declaration_start = module_identifier_matches
                        && is_declaration_start
                        && is_module_name_start
                        && !is_module_type_operator;
                    let mut primary_expression_id = None;

                    // shorthand lambda function value
                    if !self.options.in_type
                        && !self.options.in_match_case
                        && (next_token_type == TokenType::Arrow
                            || next_token_type == TokenType::ArrowWide)
                    {
                        let lambda_id =
                            self.eat_function(&start, descriptor.clone(), false, false)?;
                        primary_expression_id = Some(self.tree.insert(
                            Expression::Declaration(lambda_id),
                            self.get_span_from(&start),
                        ));
                    }

                    // keyword and split state
                    let has_active_split = self.has_active_split();
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

                    // fast path for non-keyword identifiers
                    if primary_expression_id.is_none()
                        && keyword.is_none()
                        && !has_active_split
                        && !self.options.in_decorator
                    {
                        // prefer module declarations when the identifier matches the module root
                        if is_module_declaration_start {
                            let namespace_id = self.eat_namespace(&start, descriptor.clone())?;
                            primary_expression_id = Some(self.tree.insert(
                                Expression::Declaration(namespace_id),
                                self.get_span_from(&start),
                            ));
                        } else {
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
                    }

                    // unary prefix operations
                    if primary_expression_id.is_none() && is_unary_keyword && !self.options.in_type
                    {
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
                        let right =
                            self.with_options(right_options, |parser| parser.eat_expression())?;

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
                        let right =
                            self.with_options(right_options, |parser| parser.eat_expression())?;
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
                            primary_expression_id =
                                Some(self.tree.insert(
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
                            is_declaration_start,
                        )? {
                            primary_expression_id = Some(keyword_expression_id);
                        }
                    }

                    // type literal
                    if primary_expression_id.is_none() {
                        // late fallback for contextual type literals
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
                        let expression_id = self.eat_expression()?;

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
                        return Ok(expression_id);
                    }
                    // parenthesis
                    // may be tuple, lambda, or parenthesized expression
                    else if token_type == TokenType::OpenParenthesis {
                        let _group_timing = self.timing_scope(tags::PARSE_EXPRESSION_PRIMARY_GROUP);
                        // look ahead for lambda and tuple cues without committing tokens
                        let group_shape = self.try_lookahead_parenthesized_group_shape()?;
                        let ParenthesizedGroupShape {
                            has_top_level_comma,
                            has_arrow_follow,
                            has_colon_follow,
                            has_top_level_type_union_or_intersection:
                                _has_top_level_type_union_or_intersection,
                            has_top_level_parameter_colon,
                            is_empty: is_empty_parenthesized_group,
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
                        let mut lambda_expression_id = None;

                        // parse lambda when we see a likely arrow or colon
                        if (has_arrow_follow || is_colon_lambda_allowed)
                            && can_parse_lambda_in_arrow_return
                        {
                            // avoid colon lambdas that steal ternary delimiters
                            if self.options.in_ternary_condition && has_colon_follow {
                                let speculative_start = self.mark();
                                let speculative_start_idx = self.tree.next_id();
                                if let Ok(lambda_id) =
                                    self.eat_function(&start, descriptor, false, false)
                                {
                                    let has_ternary_delimiter = self.peek_is(TokenType::Colon)
                                        || self
                                            .peek_token_after_newlines(self.pos(), TokenType::Colon)
                                            .is_ok();
                                    let should_accept = match self.tree.get(lambda_id) {
                                        Declaration::Function { body, .. } => {
                                            (body.is_some() || self.options.in_type)
                                                && has_ternary_delimiter
                                        }
                                        _ => has_ternary_delimiter,
                                    };
                                    if should_accept {
                                        lambda_expression_id = Some(self.tree.insert(
                                            Expression::Declaration(lambda_id),
                                            self.get_span_from(&start),
                                        ));
                                    } else {
                                        self.restore(speculative_start, speculative_start_idx);
                                    }
                                } else {
                                    self.restore(speculative_start, speculative_start_idx);
                                }
                            } else {
                                let lambda_id =
                                    self.eat_function(&start, descriptor, false, false)?;
                                lambda_expression_id = Some(self.tree.insert(
                                    Expression::Declaration(lambda_id),
                                    self.get_span_from(&start),
                                ));
                            }
                        }

                        // tuple or parenthesized expression
                        if let Some(lambda_expression_id) = lambda_expression_id {
                            lambda_expression_id
                        } else {
                            self.bump(); // eat open parenthesis
                            self.eat_newlines_maybe()?;

                            // empty tuple or sequence when we immediately see a closing parenthesis
                            if self.peek_is(TokenType::CloseParenthesis) {
                                self.bump(); // eat closing parenthesis

                                // in Destack: empty tuple
                                if self.language.is_destack() {
                                    self.tree.insert(
                                        Expression::TupleExpression { elements: vec![] },
                                        self.get_span_from(&start),
                                    )
                                }
                                // in JS/TS: empty sequence expression
                                else {
                                    self.tree.insert(
                                        Expression::SequenceExpression {
                                            expressions: vec![],
                                        },
                                        self.get_span_from(&start),
                                    )
                                }
                            }
                            // tuple when we see a named element or top level comma
                            else if (self.language.is_destack()
                                && self.peek_is(TokenType::Identifier)
                                && self.peek_next_is(TokenType::Colon))
                                || has_top_level_comma
                            {
                                let tuple_elements = self
                                    .eat_sequence_literal_body(None, TokenType::CloseParenthesis)
                                    .for_node_type(NodeType::Expression)?;
                                self.eat_newlines_maybe()?;
                                self.eat_token(TokenType::CloseParenthesis)?;
                                self.tree.insert(
                                    Expression::TupleExpression {
                                        elements: tuple_elements,
                                    },
                                    self.get_span_from(&start),
                                )
                            }
                            // tuple or parenthesized expression for the remaining cases
                            else {
                                let inner_start = self.pos();
                                let mut inner_options = self.options.nested().in_parenthesis();
                                inner_options.allow_sequence_expression = true;
                                if self.options.in_type {
                                    inner_options = inner_options.in_type();
                                }
                                let expression_id = self.with_options(inner_options, |parser| {
                                    parser.eat_expression()
                                })?;
                                self.eat_newlines_maybe()?;
                                self.eat_token(TokenType::CloseParenthesis)?;
                                let inner_token_type = self.token_type_at(inner_start as usize);
                                match self.tree.get(expression_id) {
                                    // if it was a tuple starting here, expand it to cover the entire span
                                    //  (except if that tuple has its own parenthesis already when nesting)
                                    Expression::TupleExpression { .. }
                                        if inner_token_type != TokenType::OpenParenthesis =>
                                    {
                                        self.tree
                                            .set_span(expression_id, self.get_span_from(&start));
                                        expression_id
                                    }
                                    // if it was a sequence expression starting here, expand it to cover the entire span
                                    Expression::SequenceExpression { .. }
                                        if inner_token_type != TokenType::OpenParenthesis =>
                                    {
                                        self.tree
                                            .set_span(expression_id, self.get_span_from(&start));
                                        expression_id
                                    }
                                    // otherwise it was a manually parenthesized expression, wrap it
                                    _ => self.tree.insert(
                                        Expression::Parenthesized {
                                            expression: expression_id,
                                        },
                                        self.get_span_from(&start),
                                    ),
                                }
                            }
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
                        let right = self
                            .with_options(self.options.not_in_position(), |parser| {
                                parser.eat_expression()
                            })?;
                        let expression = Expression::PointerOf { mutability, right };
                        self.tree.insert(expression, self.get_span_from(&start))
                    }
                    // unary prefix operations
                    else if let Ok(operator) = self.peek_unary_prefix_operator() {
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
                        let right =
                            self.with_options(right_options, |parser| parser.eat_expression())?;
                        let expression = Expression::Unary { operator, right };
                        let expression_id =
                            self.tree.insert(expression, self.get_span_from(&start));
                        self.tree.set_main_span(expression_id, operator_span);
                        expression_id
                    }
                    // type unary operations
                    else if let Ok(operator) = self.peek_type_unary_prefix_operator() {
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
                        let right =
                            self.with_options(right_options, |parser| parser.eat_expression())?;
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
                        let right = self
                            .with_options(self.options.not_in_position(), |parser| {
                                parser.eat_expression()
                            })?;
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
                        let right = self
                            .with_options(self.options.not_in_position(), |parser| {
                                parser.eat_expression()
                            })?;
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
                        let elements = self
                            .with_options(self.options.not_in_position(), |parser| {
                                parser.eat_array_literal()
                            })?;
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
                        // prefer mapped types in type positions
                        if self.options.in_type {
                            let speculative_start = self.mark();
                            let speculative_start_idx = self.tree.next_id();
                            if let Ok(mapped_id) = self.eat_type_mapped_expression() {
                                mapped_id
                            } else {
                                self.restore(speculative_start, speculative_start_idx);
                                let properties = self.with_options(
                                    self.options.not_in_position().in_type(),
                                    |parser| parser.eat_object_literal(),
                                )?;
                                self.tree.insert(
                                    Expression::ObjectExpression {
                                        ty: None,
                                        properties,
                                    },
                                    self.get_span_from(&start),
                                )
                            }
                        }
                        // fall back to object literal
                        else {
                            let properties = self
                                .with_options(self.options.not_in_position(), |parser| {
                                    parser.eat_object_literal()
                                })?;
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
                    else if self.peek_block().is_ok() {
                        let block_id = self.eat_block()?;
                        self.tree
                            .insert(Expression::Block(block_id), self.get_span_from(&start))
                    }
                    // statically parameterized lambda: <T>(...) or <T,>(...)
                    // (also handles multiline in type context: `<\nT\n>(...) => ...`)
                    else if token_type == TokenType::LessThan
                        && self.can_start_generic_arrow_expression()
                    {
                        let function_id = self.eat_function(&start, descriptor, false, false)?;
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
                        self.with_options(self.options.not_in_position(), |parser| {
                            parser.eat_tree_literal()
                        })?
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

        self.eat_expression_continuation(&start, left_expression_id)
    }
}
