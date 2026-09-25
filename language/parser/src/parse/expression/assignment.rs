use crate::parse::{ExpressionPosition, ExpressionStop};
use crate::{ParseStart, Parser, ParserError, ParserResult};
use tspp_dir::{
    AssignOperator, AssignPattern, AssignPatternField, Expression, LocalNodeId, Name, NodeType,
    OperatorPrecedence, TokenType, UnaryOperator,
};
use tspp_source::{ByteRange, NodeSpanRegion, NodeSpanType};

impl Parser {
    /// Parse one destructuring assignment directly into its assignment pattern.
    pub(in crate::parse::expression) fn parse_destructuring_assignment(
        &mut self,
        start: &ParseStart,
        position: ExpressionPosition,
        stop: ExpressionStop,
    ) -> ParserResult<LocalNodeId<Expression>> {
        // parse the assignment pattern and operator
        let left = self.parse_assignment_pattern(position, stop)?;
        let operator_range = self.eat_token(TokenType::Assign)?.range();

        // parse the complete right associative value
        let right_stop = stop
            .add(ExpressionStop::NEWLINE_CALL)
            .remove(ExpressionStop::CONDITIONAL_QUESTION);
        let right =
            self.parse_expression_at(position.right(), right_stop, OperatorPrecedence::Lowest)?;

        // build the assignment expression
        let expression = Expression::Assign {
            left,
            operator: AssignOperator::Assign,
            right,
        };
        let expression = self.insert_node(expression, self.range_since(start));
        self.tree.set_main_range(expression, operator_range);

        Ok(expression)
    }

    /// Parse one assignment-pattern source recursively.
    fn parse_assignment_pattern(
        &mut self,
        position: ExpressionPosition,
        stop: ExpressionStop,
    ) -> ParserResult<LocalNodeId<AssignPattern>> {
        self.with_recursive_descent(NodeType::AssignPattern, |parser| {
            parser.parse_assignment_pattern_after_descent(position, stop)
        })
    }

    /// Parse one assignment pattern after checking the recursion depth.
    fn parse_assignment_pattern_after_descent(
        &mut self,
        position: ExpressionPosition,
        stop: ExpressionStop,
    ) -> ParserResult<LocalNodeId<AssignPattern>> {
        let start = self.mark_parse_start();

        // parse recursive destructuring forms directly
        match self.peek_token_type() {
            TokenType::OpenBrace => self.parse_object_assignment_pattern(&start, position, stop),
            TokenType::OpenBracket => {
                self.parse_sequence_assignment_pattern(&start, position, stop)
            }
            TokenType::OpenParenthesis if self.peek_tuple_assignment_pattern() => {
                self.parse_tuple_assignment_pattern(&start, position, stop)
            }
            _ => {
                let expression = self.parse_expression_at(
                    position.nested(),
                    ExpressionStop::default(),
                    OperatorPrecedence::Assignment,
                )?;

                self.lower_assignment_pattern(expression)
            }
        }
    }

    /// Parse one object assignment pattern.
    fn parse_object_assignment_pattern(
        &mut self,
        start: &ParseStart,
        position: ExpressionPosition,
        stop: ExpressionStop,
    ) -> ParserResult<LocalNodeId<AssignPattern>> {
        self.eat_token(TokenType::OpenBrace)?;
        let mut fields = Vec::new();

        // parse fields through the closing brace
        while !self.peek_is(TokenType::CloseBrace) && self.has_more_tokens() {
            if self.peek_is(TokenType::Comma) {
                self.bump();

                continue;
            }

            let documentation = self.parse_documentation();
            let field_start = self.mark_parse_start();
            let (field, main_range) = self.parse_object_assignment_field(position, stop)?;
            let field = self.insert_node(field, self.range_since(&field_start));
            if let Some(main_range) = main_range {
                self.tree.set_main_range(field, main_range);
            }
            self.attach_documentation(field, documentation);
            fields.push(field);

            if self.peek_is(TokenType::Comma) {
                self.bump();
            } else if !self.peek_is_on_new_line() {
                break;
            }
        }

        self.eat_close_token_or_recover_missing(TokenType::CloseBrace, NodeType::AssignPattern)?;

        Ok(self.insert_node(AssignPattern::Object { fields }, self.range_since(start)))
    }

    /// Parse one object assignment field.
    fn parse_object_assignment_field(
        &mut self,
        position: ExpressionPosition,
        stop: ExpressionStop,
    ) -> ParserResult<(AssignPatternField, Option<ByteRange>)> {
        // rest field
        if self.peek_is(TokenType::Spread) {
            self.bump();
            let pattern = self.parse_assignment_pattern(position, stop)?;

            return Ok((
                AssignPatternField::Rest {
                    pattern: Some(pattern),
                },
                None,
            ));
        }

        // computed field
        if self.peek_is(TokenType::OpenBracket) {
            let start = self.mark_parse_start();
            self.bump();
            let key = self.parse_expression(position.nested(), ExpressionStop::default())?;
            self.eat_close_token_or_recover_missing(
                TokenType::CloseBracket,
                NodeType::AssignPattern,
            )?;
            let key_range = self.range_since(&start);
            self.eat_token(TokenType::Colon)?;
            let pattern = self.parse_assignment_pattern_default(position, stop)?;

            return Ok((
                AssignPatternField::Computed { key, pattern },
                Some(key_range),
            ));
        }

        // named field
        let (name, name_range) = self.eat_property_name_with_range()?;
        let (pattern, is_shorthand) = if self.eat_token_if(TokenType::Colon) {
            (
                self.parse_assignment_pattern_default(position, stop)?,
                false,
            )
        } else {
            let Name::Identifier(identifier) = name else {
                return Err(ParserError::unexpected(name_range));
            };
            let expression =
                self.insert_node(Expression::Identifier { name: identifier }, name_range);
            self.tree.set_main_range(expression, name_range);
            let pattern = self.lower_assignment_pattern(expression)?;
            let pattern = self.parse_assignment_pattern_default_after(pattern, position)?;

            (pattern, true)
        };

        Ok((
            AssignPatternField::Named {
                name,
                pattern,
                is_shorthand,
            },
            Some(name_range),
        ))
    }

    /// Parse one sequence assignment pattern.
    fn parse_sequence_assignment_pattern(
        &mut self,
        start: &ParseStart,
        position: ExpressionPosition,
        stop: ExpressionStop,
    ) -> ParserResult<LocalNodeId<AssignPattern>> {
        self.eat_token(TokenType::OpenBracket)?;
        let mut fields = Vec::new();

        // parse elements through the closing bracket
        while !self.peek_is(TokenType::CloseBracket) && self.has_more_tokens() {
            let field_start = self.mark_parse_start();
            let field = if self.peek_is(TokenType::Comma) {
                AssignPatternField::Elision
            } else if self.eat_token_if(TokenType::Spread) {
                let pattern = self.parse_assignment_pattern(position, stop)?;
                AssignPatternField::Rest {
                    pattern: Some(pattern),
                }
            } else {
                let pattern = self.parse_assignment_pattern_default(position, stop)?;
                AssignPatternField::Positional { pattern }
            };
            let field = self.insert_node(field, self.range_since(&field_start));
            fields.push(field);

            if self.peek_is(TokenType::Comma) {
                self.bump();
            } else if !self.peek_is_on_new_line() {
                break;
            }
        }

        self.eat_close_token_or_recover_missing(TokenType::CloseBracket, NodeType::AssignPattern)?;

        Ok(self.insert_node(AssignPattern::Sequence { fields }, self.range_since(start)))
    }

    /// Parse one tuple assignment pattern.
    fn parse_tuple_assignment_pattern(
        &mut self,
        start: &ParseStart,
        position: ExpressionPosition,
        stop: ExpressionStop,
    ) -> ParserResult<LocalNodeId<AssignPattern>> {
        self.eat_token(TokenType::OpenParenthesis)?;
        let mut fields = Vec::new();

        // parse positional fields through the closing parenthesis
        while !self.peek_is(TokenType::CloseParenthesis) && self.has_more_tokens() {
            let field_start = self.mark_parse_start();
            let pattern = self.parse_assignment_pattern_default(position, stop)?;
            let field = self.insert_node(
                AssignPatternField::Positional { pattern },
                self.range_since(&field_start),
            );
            fields.push(field);

            if self.peek_is(TokenType::Comma) {
                self.bump();
            } else if !self.peek_is_on_new_line() {
                break;
            }
        }

        self.eat_close_token_or_recover_missing(
            TokenType::CloseParenthesis,
            NodeType::AssignPattern,
        )?;

        Ok(self.insert_node(AssignPattern::Tuple { fields }, self.range_since(start)))
    }

    /// Parse one assignment field with an optional default value.
    fn parse_assignment_pattern_default(
        &mut self,
        position: ExpressionPosition,
        stop: ExpressionStop,
    ) -> ParserResult<LocalNodeId<AssignPattern>> {
        let pattern = self.parse_assignment_pattern(position, stop)?;

        self.parse_assignment_pattern_default_after(pattern, position)
    }

    /// Parse an optional default after one assignment field.
    fn parse_assignment_pattern_default_after(
        &mut self,
        pattern: LocalNodeId<AssignPattern>,
        position: ExpressionPosition,
    ) -> ParserResult<LocalNodeId<AssignPattern>> {
        if !self.peek_is(TokenType::Assign) {
            return Ok(pattern);
        }

        // parse and wrap the default value
        let start = self.tree.get_range(pattern).start;
        self.bump();
        let value = self.parse_expression(position.nested(), ExpressionStop::default())?;
        let end = self.tree.get_source_extent(value).range().end;

        Ok(self.insert_node(
            AssignPattern::Default { pattern, value },
            ByteRange { start, end },
        ))
    }

    /// Lower one value expression into an assignment target.
    pub(in crate::parse) fn lower_assignment_target(
        &mut self,
        expression: LocalNodeId<Expression>,
        operator: AssignOperator,
    ) -> ParserResult<LocalNodeId<AssignPattern>> {
        let mark = self.tree.mark();
        let target = self.lower_assignment_pattern(expression);
        let target = match target {
            Ok(target) => target,
            Err(error) => {
                self.tree.restore_to_mark(mark);

                return Err(error);
            }
        };

        // compound assignment requires one writable place
        if operator != AssignOperator::Assign
            && !matches!(self.tree.get(target), AssignPattern::Place { .. })
        {
            let range = self.tree.get_range(target);
            self.tree.restore_to_mark(mark);

            return Err(ParserError::invalid_assignment_target(range));
        }

        Ok(target)
    }

    /// Lower one assignment target at the current stack depth.
    fn lower_assignment_pattern(
        &mut self,
        expression: LocalNodeId<Expression>,
    ) -> ParserResult<LocalNodeId<AssignPattern>> {
        let has_parentheses = self
            .tree
            .get_side_range(
                expression,
                NodeSpanType::Region(NodeSpanRegion::Parentheses),
            )
            .is_some();

        // assertions must be parenthesized before assignment
        if matches!(
            self.tree.get(expression),
            Expression::As { .. } | Expression::Satisfies { .. }
        ) && !has_parentheses
        {
            return Err(ParserError::invalid_assignment_target(
                self.tree.get_range(expression),
            ));
        }

        // assignments cannot be hidden by parentheses
        if has_parentheses && matches!(self.tree.get(expression), Expression::Assign { .. }) {
            return Err(ParserError::invalid_assignment_target(
                self.tree.get_range(expression),
            ));
        }

        // lower default values into assignment patterns
        if let Expression::Assign {
            left,
            operator,
            right,
        } = self.tree.get(expression)
        {
            let left = *left;
            let operator = *operator;
            let right = *right;
            let range = self.tree.get_range(expression);

            if operator != AssignOperator::Assign {
                return Err(ParserError::invalid_assignment_target(range));
            }

            let pattern = AssignPattern::Default {
                pattern: left,
                value: right,
            };

            return Ok(self.insert_node(pattern, range));
        }

        // require one syntactic assignment place
        if !self.is_assignment_place(expression) {
            return Err(ParserError::invalid_assignment_target(
                self.tree.get_range(expression),
            ));
        }

        Ok(self.insert_node(
            AssignPattern::Place { expression },
            self.tree.get_range(expression),
        ))
    }

    /// Return whether one expression can syntactically denote an assignment place.
    fn is_assignment_place(&self, mut expression: LocalNodeId<Expression>) -> bool {
        loop {
            // accept direct place forms
            match self.tree.get(expression) {
                Expression::Identifier { .. } => return true,
                Expression::Member { .. } | Expression::Index { .. } => {
                    return !self.contains_optional_chain(expression);
                }
                Expression::As {
                    expression: left, ..
                }
                | Expression::Satisfies {
                    expression: left, ..
                } => expression = *left,
                Expression::Unary {
                    operator: UnaryOperator::Dereference,
                    ..
                } => return true,
                Expression::Must { left, .. } => expression = *left,
                _ => return false,
            }
        }
    }

    /// Return whether one expression contains optional chaining.
    fn contains_optional_chain(&self, mut expression: LocalNodeId<Expression>) -> bool {
        loop {
            // follow the left edge of the place expression
            match self.tree.get(expression) {
                Expression::Maybe { .. } => return true,
                Expression::Member { left, .. } | Expression::Index { left, .. } => {
                    expression = *left;
                }
                Expression::As {
                    expression: left, ..
                }
                | Expression::Satisfies {
                    expression: left, ..
                } => expression = *left,
                Expression::Must { left, .. } => expression = *left,
                _ => return false,
            }
        }
    }
}
