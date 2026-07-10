use crate::parse::scope::ExpressionScope;
use crate::{Parser, ParserResult, ParserSpanStart};
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
    ) -> ParserResult<(LocalNodeId<Expression>, bool)> {
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
    ) -> ParserResult<(LocalNodeId<Expression>, bool)> {
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
    ) -> ParserResult<Vec<LocalNodeId<Argument>>> {
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
    ) -> ParserResult<Option<(LocalNodeId<Expression>, bool)>> {
        if self.next_token_type() != TokenType::OpenParenthesis {
            return Ok(None);
        }

        // let nested lambda heads parse as ordinary inner expressions
        if self.parenthesized_lambda_head_starts_at(1) {
            return Ok(None);
        }

        let checkpoint = self.checkpoint();
        match self.eat_parenthesized_value_chain_at_cursor(start) {
            Ok(expression) => Ok(Some(expression)),
            Err(_) if self.parenthesized_chain_may_be_arrow_head() => {
                self.restore(checkpoint);

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
    ) -> ParserResult<(LocalNodeId<Expression>, bool)> {
        let mut wrappers = SmallVec::<[ParserSpanStart; 4]>::new();
        while self.peek_is(TokenType::OpenParenthesis) {
            wrappers.push(self.span_start());
            self.bump();
        }

        let mut left = self.eat_parenthesized_chain_head(&mut wrappers)?;

        while let Some(wrapper_start) = wrappers.pop() {
            left = self.eat_parenthesized_chain_continuation(start, left)?;

            if self.peek_is(TokenType::Comma) {
                let (tuple, _) = self.eat_tuple_parenthesized_value(&wrapper_start, left)?;
                left = tuple;
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
    ) -> ParserResult<LocalNodeId<Expression>> {
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
            let (inner, _) = self.eat_tuple_parenthesized_value(&wrapper_start, inner)?;

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
    ) -> ParserResult<LocalNodeId<Expression>> {
        let flags = self.flags.nested().not_in_position();
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
