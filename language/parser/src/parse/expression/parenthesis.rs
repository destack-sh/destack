use crate::parse::scope::ExpressionScope;
use crate::{ParseResult, Parser, ParserSpanStart};
use destack_dir::{Argument, Expression, LocalNodeId, NodeType, TokenType};
use destack_source::{NodeSpanBoundary, NodeSpanType, Span};
use smallvec::SmallVec;

impl Parser {
    /// Parse value parentheses.
    ///
    /// Examples:
    /// ```ds
    /// (value)
    /// ()
    /// (first, second)
    /// ```
    pub(super) fn eat_parenthesized_value(
        &mut self,
        start: &ParserSpanStart,
    ) -> ParseResult<(LocalNodeId<Expression>, bool)> {
        // nested wrapper chain
        if let Some(expression) = self.eat_parenthesized_value_chain(start)? {
            return Ok(expression);
        }

        // open group
        self.eat_token(TokenType::OpenParenthesis)?;
        if self.peek_is(TokenType::CloseParenthesis) {
            return Ok((self.eat_empty_parenthesized_value(start), false));
        }

        // inner expression
        let inner = self.eat_expression(self.flags.nested().not_in_position())?;
        if self.peek_is(TokenType::Comma) {
            if self.language_uses_sequence_parentheses() {
                return self.eat_sequence_parenthesized_value(start, inner);
            }

            return self.eat_tuple_parenthesized_value(start, inner);
        }

        // grouped expression
        self.eat_close_token_or_recover_missing(TokenType::CloseParenthesis, NodeType::Expression)?;
        let id = self.wrap_parenthesized_value(start, inner);

        Ok((id, true))
    }

    /// Parse an empty parenthesized value.
    ///
    /// Examples:
    /// ```ds
    /// ()
    /// call(())
    /// (() => value)
    /// ```
    fn eat_empty_parenthesized_value(
        &mut self,
        start: &ParserSpanStart,
    ) -> LocalNodeId<Expression> {
        self.bump();

        self.insert_node(
            Expression::TupleExpression {
                elements: Vec::new(),
            },
            self.get_span_from(start),
        )
    }

    /// Return whether the language uses comma sequences in parentheses.
    fn language_uses_sequence_parentheses(&self) -> bool {
        self.language.is_javascript() || self.language.is_typescript()
    }

    /// Parse a parenthesized sequence expression.
    ///
    /// Examples:
    /// ```ds
    /// (first, second)
    /// (first, second, third)
    /// (first, call(second))
    /// ```
    fn eat_sequence_parenthesized_value(
        &mut self,
        start: &ParserSpanStart,
        first: LocalNodeId<Expression>,
    ) -> ParseResult<(LocalNodeId<Expression>, bool)> {
        let expressions = self.eat_parenthesized_sequence_values(first)?;
        self.eat_close_token_or_recover_missing(TokenType::CloseParenthesis, NodeType::Expression)?;

        let sequence = self.insert_node(
            Expression::SequenceExpression { expressions },
            self.get_span_from(start),
        );

        let expression = self.wrap_sequence_parenthesized_value(start, sequence);

        Ok((expression, true))
    }

    /// Parse sequence values after the first expression.
    ///
    /// Examples:
    /// ```ds
    /// , second
    /// , second, third
    /// , call(second)
    /// ```
    fn eat_parenthesized_sequence_values(
        &mut self,
        first: LocalNodeId<Expression>,
    ) -> ParseResult<Vec<LocalNodeId<Expression>>> {
        let mut expressions = vec![first];
        while self.peek_is(TokenType::Comma) {
            self.bump();
            let value = self.eat_expression(
                self.flags
                    .nested()
                    .not_in_position()
                    .not_in_sequence_expression(),
            )?;
            expressions.push(value);
        }

        Ok(expressions)
    }

    /// Wrap a sequence expression with explicit parenthesis ownership.
    fn wrap_sequence_parenthesized_value(
        &mut self,
        start: &ParserSpanStart,
        sequence: LocalNodeId<Expression>,
    ) -> LocalNodeId<Expression> {
        if !self.preserves_parenthesized_wrappers() {
            self.set_node_wrapper_span(sequence, self.get_span_from(start));

            return sequence;
        }

        let expression = self.insert_node(
            Expression::Parenthesized {
                expression: sequence,
            },
            self.get_span_from(start),
        );
        self.tree
            .set_head_span(expression, self.expression_head_span(sequence));

        expression
    }

    /// Parse a parenthesized tuple expression.
    ///
    /// Examples:
    /// ```ds
    /// (first, second)
    /// (first,)
    /// (first, ...rest)
    /// ```
    fn eat_tuple_parenthesized_value(
        &mut self,
        start: &ParserSpanStart,
        first: LocalNodeId<Expression>,
    ) -> ParseResult<(LocalNodeId<Expression>, bool)> {
        let elements = self.eat_parenthesized_tuple_elements(first)?;
        self.eat_close_token_or_recover_missing(TokenType::CloseParenthesis, NodeType::Expression)?;

        let expression = self.insert_node(
            Expression::TupleExpression { elements },
            self.get_span_from(start),
        );

        Ok((expression, false))
    }

    /// Parse tuple elements after the first expression.
    ///
    /// Examples:
    /// ```ds
    /// , second
    /// , second, third
    /// , ...rest
    /// ```
    fn eat_parenthesized_tuple_elements(
        &mut self,
        first: LocalNodeId<Expression>,
    ) -> ParseResult<Vec<LocalNodeId<Argument>>> {
        let mut elements = vec![self.insert_node(
            Argument::Positional { value: first },
            self.tree.get_span(first),
        )];

        while self.peek_is(TokenType::Comma) {
            self.bump();
            if self.peek_is(TokenType::CloseParenthesis) {
                break;
            }

            let value = self.eat_expression(self.flags.nested().not_in_position())?;
            let element =
                self.insert_node(Argument::Positional { value }, self.tree.get_span(value));
            elements.push(element);
        }

        Ok(elements)
    }

    /// Parse a run of nested value parentheses without recursive descent.
    ///
    /// Examples:
    /// ```ds
    /// (((value)))
    /// ((left) + right)
    /// (((condition ? yes : no)))
    /// ```
    fn eat_parenthesized_value_chain(
        &mut self,
        start: &ParserSpanStart,
    ) -> ParseResult<Option<(LocalNodeId<Expression>, bool)>> {
        if self.next_token_type() != TokenType::OpenParenthesis {
            return Ok(None);
        }

        let checkpoint = self.checkpoint();
        let mark = self.tree.next_id();
        match self.eat_parenthesized_value_chain_at_cursor(start) {
            Ok(expression) => Ok(Some(expression)),
            Err(_) if self.parenthesized_chain_may_be_arrow_head() => {
                self.restore(checkpoint, mark);

                Ok(None)
            }
            Err(error) => Err(error),
        }
    }

    /// Parse one known nested parenthesis chain.
    ///
    /// Examples:
    /// ```ds
    /// ((value))
    /// ((left) + right)
    /// (((call())))
    /// ```
    fn eat_parenthesized_value_chain_at_cursor(
        &mut self,
        start: &ParserSpanStart,
    ) -> ParseResult<(LocalNodeId<Expression>, bool)> {
        let mut wrappers = SmallVec::<[ParserSpanStart; 4]>::new();
        while self.peek_is(TokenType::OpenParenthesis) {
            wrappers.push(self.span_start());
            self.bump();
        }

        let mut left = self.eat_parenthesized_chain_head(&mut wrappers)?;

        while let Some(wrapper_start) = wrappers.pop() {
            left = self.eat_parenthesized_chain_continuation(start, left)?;

            if self.peek_is(TokenType::Comma) {
                let (sequence, _) = if self.language_uses_sequence_parentheses() {
                    self.eat_sequence_parenthesized_value(&wrapper_start, left)?
                } else {
                    self.eat_tuple_parenthesized_value(&wrapper_start, left)?
                };
                left = sequence;
            } else {
                self.eat_close_token_or_recover_missing(
                    TokenType::CloseParenthesis,
                    NodeType::Expression,
                )?;
                left = self.wrap_parenthesized_value(&wrapper_start, left);
            }
        }

        Ok((left, true))
    }

    /// Parse the innermost value owned by a parenthesized chain.
    ///
    /// Examples:
    /// ```ds
    /// value
    /// first, second
    /// ()
    /// ```
    fn eat_parenthesized_chain_head(
        &mut self,
        wrappers: &mut SmallVec<[ParserSpanStart; 4]>,
    ) -> ParseResult<LocalNodeId<Expression>> {
        let Some(wrapper_start) = wrappers.pop() else {
            return self.eat_expression(self.flags.nested().not_in_position());
        };

        // empty innermost group
        if self.peek_is(TokenType::CloseParenthesis) {
            return Ok(self.eat_empty_parenthesized_value(&wrapper_start));
        }

        // sequence or tuple innermost group
        let inner = self.eat_expression(self.flags.nested().not_in_position())?;
        if self.peek_is(TokenType::Comma) {
            let (inner, _) = if self.language_uses_sequence_parentheses() {
                self.eat_sequence_parenthesized_value(&wrapper_start, inner)?
            } else {
                self.eat_tuple_parenthesized_value(&wrapper_start, inner)?
            };

            return Ok(inner);
        }

        // grouped innermost expression
        self.eat_close_token_or_recover_missing(TokenType::CloseParenthesis, NodeType::Expression)?;

        Ok(self.wrap_parenthesized_value(&wrapper_start, inner))
    }

    /// Parse expression continuations before the next wrapper close.
    ///
    /// Examples:
    /// ```ds
    /// .member
    /// + right
    /// as Type
    /// ```
    fn eat_parenthesized_chain_continuation(
        &mut self,
        start: &ParserSpanStart,
        left: LocalNodeId<Expression>,
    ) -> ParseResult<LocalNodeId<Expression>> {
        let flags = self
            .flags
            .nested()
            .not_in_position()
            .not_in_sequence_expression();
        let scope = ExpressionScope::from_flags(flags);

        let (left, _) = self.eat_postfix(start, left, true, scope)?;
        let left = self.eat_binary_rest(start, left, scope)?;
        let left = self.eat_conditional_rest(start, left, scope)?;

        self.eat_assignment_rest(start, left, scope)
    }

    /// Return whether a failed parenthesized chain may be a parenthesized arrow head.
    fn parenthesized_chain_may_be_arrow_head(&mut self) -> bool {
        matches!(
            self.peek_token_type(),
            TokenType::ArrowWide | TokenType::Colon
        )
    }

    /// Wrap one expression in a parenthesized value node or span.
    pub(in crate::parse::expression) fn wrap_parenthesized_value(
        &mut self,
        start: &ParserSpanStart,
        inner: LocalNodeId<Expression>,
    ) -> LocalNodeId<Expression> {
        if self.preserves_parenthesized_wrappers() {
            let id = self.insert_node(
                Expression::Parenthesized { expression: inner },
                self.get_span_from(start),
            );
            self.tree
                .set_head_span(id, self.expression_head_span(inner));

            return id;
        }

        let inner_span = self.tree.get_span(inner);
        let leading_span = Span::new(inner_span.file, start.token_end(), inner_span.start);
        if leading_span.start < leading_span.end {
            self.tree.set_side_span(
                inner,
                NodeSpanType::Boundary(NodeSpanBoundary::Leading),
                leading_span,
            );
        }
        self.set_node_wrapper_span(inner, self.get_span_from(start));

        inner
    }
}
