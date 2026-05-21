use crate::parse::scan::DelimiterDepth;
use crate::{ParseResult, Parser, ParserSpanStart};
use destack_dir::{Keyword, LocalNodeId, NodeType, TokenType, TypeExpression};
use destack_source::{NodeSpanBoundary, NodeSpanType, Span};

impl Parser {
    /// Parse type parentheses.
    ///
    /// Examples:
    /// ```ds
    /// (T)
    /// ()
    /// (first: string, second?: number)
    /// ```
    pub(super) fn eat_parenthesized_type(
        &mut self,
        start: &ParserSpanStart,
    ) -> ParseResult<LocalNodeId<TypeExpression>> {
        self.eat_token(TokenType::OpenParenthesis)?;

        // empty tuple
        if self.peek_is(TokenType::CloseParenthesis) {
            return Ok(self.eat_empty_parenthesized_type(start));
        }

        // tuple head
        if self.starts_type_tuple_head() {
            return self.eat_parenthesized_tuple_type(start);
        }

        // inner expression
        let inner = self.eat_type_expression_or_recover_missing(
            self.flags.nested().in_type(),
            NodeType::TypeExpression,
        )?;

        // tuple tail
        if self.peek_is(TokenType::Comma)
            || self.current_type_tuple_element_is_optional(TokenType::CloseParenthesis)
        {
            return self.eat_parenthesized_tuple_tail_type(start, inner);
        }

        // grouped type
        self.eat_close_token_or_recover_missing(
            TokenType::CloseParenthesis,
            NodeType::TypeExpression,
        )?;

        Ok(self.wrap_parenthesized_type(start, inner))
    }

    /// Eat an empty parenthesized tuple type.
    ///
    /// Examples:
    /// ```ds
    /// ()
    /// (() => void)
    /// Array<()>
    /// ```
    fn eat_empty_parenthesized_type(
        &mut self,
        start: &ParserSpanStart,
    ) -> LocalNodeId<TypeExpression> {
        self.bump();

        self.insert_node(
            TypeExpression::Tuple {
                elements: Vec::new(),
            },
            self.get_span_from(start),
        )
    }

    /// Eat a parenthesized tuple type with an explicit tuple head.
    ///
    /// Examples:
    /// ```ds
    /// (...items: string[])
    /// (readonly first: string)
    /// (name?: string, age: number)
    /// ```
    fn eat_parenthesized_tuple_type(
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

    /// Eat a parenthesized tuple type after the first element.
    ///
    /// Examples:
    /// ```ds
    /// (string,)
    /// (string, number)
    /// (string, ...boolean[])
    /// ```
    fn eat_parenthesized_tuple_tail_type(
        &mut self,
        start: &ParserSpanStart,
        first: LocalNodeId<TypeExpression>,
    ) -> ParseResult<LocalNodeId<TypeExpression>> {
        let elements = self.eat_type_tuple_tail(start, TokenType::CloseParenthesis, first)?;
        self.eat_close_token_or_recover_missing(
            TokenType::CloseParenthesis,
            NodeType::TypeExpression,
        )?;

        Ok(self.insert_node(
            TypeExpression::Tuple { elements },
            self.get_span_from(start),
        ))
    }

    /// Wrap or mark one parenthesized type.
    fn wrap_parenthesized_type(
        &mut self,
        start: &ParserSpanStart,
        inner: LocalNodeId<TypeExpression>,
    ) -> LocalNodeId<TypeExpression> {
        if self.preserves_parenthesized_wrappers() {
            return self.insert_node(
                TypeExpression::Parenthesized { expression: inner },
                self.get_span_from(start),
            );
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

    /// Return whether the current token starts a constructor type expression.
    pub(super) fn can_start_construct_type_expression(&mut self) -> bool {
        if self.current_keyword() == Some(Keyword::New) {
            return matches!(
                self.next_token_type(),
                TokenType::LessThan | TokenType::OpenParenthesis
            );
        }

        if self.keyword_at_offset(0) == Some(Keyword::Abstract)
            && self.keyword_at_offset(1) == Some(Keyword::New)
        {
            return matches!(
                self.token_type_at_offset(2),
                TokenType::LessThan | TokenType::OpenParenthesis
            );
        }

        false
    }

    /// Return whether the current parenthesis group is a function type head.
    pub(super) fn can_start_parenthesized_function_type(&mut self) -> bool {
        if self.peek_token_type() != TokenType::OpenParenthesis {
            return false;
        }

        self.lookahead(|parser| parser.current_parenthesis_is_function_type_head())
    }

    /// Return whether the current parenthesized type starts a function type.
    fn current_parenthesis_is_function_type_head(&mut self) -> bool {
        self.bump();

        if self.peek_is(TokenType::CloseParenthesis) {
            return self.close_parenthesized_type_head_has_function_follow();
        }

        if self.peek_is(TokenType::Spread) {
            return self.skip_parenthesized_type_head_to_function_follow();
        }

        if !self.skip_type_function_parameter_start() {
            return false;
        }

        if matches!(
            self.peek_token_type(),
            TokenType::Colon | TokenType::Maybe | TokenType::Assign | TokenType::Comma
        ) {
            return self.skip_parenthesized_type_head_to_function_follow();
        }

        if self.peek_is(TokenType::CloseParenthesis) {
            return self.close_parenthesized_type_head_has_function_follow();
        }

        false
    }

    /// Return whether a closed parenthesized type head is followed by function syntax.
    fn close_parenthesized_type_head_has_function_follow(&mut self) -> bool {
        self.bump();

        if matches!(self.peek_token_type(), TokenType::ArrowWide) {
            return !self.flags.is_in_arrow_return_type();
        }

        self.peek_token_type() == TokenType::Colon && !self.flags.is_in_type_conditional_right()
    }

    /// Skip a parenthesized type head and test for a function follow token.
    fn skip_parenthesized_type_head_to_function_follow(&mut self) -> bool {
        let mut depth = DelimiterDepth::default();

        while self.has_more_tokens() {
            let token_type = self.peek_token_type();

            // recover before rescanning later statements
            if self.current_token_is_statement_recovery_boundary(token_type) {
                return false;
            }

            if token_type == TokenType::CloseParenthesis && depth.is_top_level() {
                self.bump();

                return matches!(self.peek_token_type(), TokenType::ArrowWide)
                    || self.peek_token_type() == TokenType::Colon
                        && !self.flags.is_in_type_conditional_right();
            }

            if !depth.advance(token_type) {
                return false;
            }

            self.bump();
        }

        false
    }

    /// Skip the first token shape of a function type parameter.
    fn skip_type_function_parameter_start(&mut self) -> bool {
        if self.current_keyword() == Some(Keyword::This) || self.peek_is(TokenType::Identifier) {
            self.bump();
            return true;
        }

        if self.peek_is(TokenType::OpenBracket) {
            return self.skip_balanced_delimiter(TokenType::OpenBracket, TokenType::CloseBracket);
        }

        if self.peek_is(TokenType::OpenBrace) {
            return self.skip_balanced_delimiter(TokenType::OpenBrace, TokenType::CloseBrace);
        }

        false
    }

    /// Parse a type bracket primary.
    ///
    /// Examples:
    /// ```ds
    /// [string, number]
    /// [string; 4]
    /// [readonly name: string, ...rest: number[]]
    /// ```
    pub(super) fn eat_bracket_type(
        &mut self,
        start: &ParserSpanStart,
    ) -> ParseResult<LocalNodeId<TypeExpression>> {
        self.eat_token(TokenType::OpenBracket)?;

        // empty tuple
        if self.peek_is(TokenType::CloseBracket) {
            return Ok(self.eat_empty_array_tuple_type(start));
        }

        // tuple with explicit tuple element syntax
        if self.starts_type_tuple_head() {
            return self.eat_array_tuple_type(start);
        }

        // head element
        let element = self.eat_type_expression_or_recover_missing(
            self.flags.nested().in_type(),
            NodeType::TypeExpression,
        )?;

        // fixed array
        if self.peek_is(TokenType::Semicolon) {
            return self.eat_fixed_array_type(start, element);
        }

        // direct close: Destack uses slice syntax, TS keeps tuple syntax
        if self.peek_is(TokenType::CloseBracket) {
            self.bump();

            return self.eat_closed_single_bracket_type(start, element);
        }

        // missing close without tuple tail: recover to the boundary
        if !(self.peek_is(TokenType::Comma)
            || self.current_type_tuple_element_is_optional(TokenType::CloseBracket))
        {
            self.eat_close_token_or_recover_missing(
                TokenType::CloseBracket,
                NodeType::TypeExpression,
            )?;

            return self.eat_closed_single_bracket_type(start, element);
        }

        // tuple tail
        self.eat_array_tuple_tail_type(start, element)
    }

    /// Eat an empty array tuple type.
    ///
    /// Examples:
    /// ```ds
    /// []
    /// Array<[]>
    /// readonly []
    /// ```
    fn eat_empty_array_tuple_type(
        &mut self,
        start: &ParserSpanStart,
    ) -> LocalNodeId<TypeExpression> {
        self.bump();

        self.insert_node(
            TypeExpression::ArrayTuple {
                elements: Vec::new(),
            },
            self.get_span_from(start),
        )
    }

    /// Eat an array tuple type with an explicit tuple head.
    ///
    /// Examples:
    /// ```ds
    /// [name: string]
    /// [readonly name: string, age?: number]
    /// [...rest: string[]]
    /// ```
    fn eat_array_tuple_type(
        &mut self,
        start: &ParserSpanStart,
    ) -> ParseResult<LocalNodeId<TypeExpression>> {
        let elements = self.eat_type_tuple_elements_body(TokenType::CloseBracket)?;
        self.eat_close_token_or_recover_missing(TokenType::CloseBracket, NodeType::TypeExpression)?;

        Ok(self.insert_node(
            TypeExpression::ArrayTuple { elements },
            self.get_span_from(start),
        ))
    }

    /// Eat a fixed array type.
    ///
    /// Examples:
    /// ```ds
    /// [u8; 16]
    /// [string; count]
    /// [T; N + 1]
    /// ```
    fn eat_fixed_array_type(
        &mut self,
        start: &ParserSpanStart,
        element: LocalNodeId<TypeExpression>,
    ) -> ParseResult<LocalNodeId<TypeExpression>> {
        self.bump();
        let length = self.eat_expression_or_recover_missing(
            self.flags.nested().with_type(false),
            NodeType::Expression,
        )?;
        self.eat_close_token_or_recover_missing(TokenType::CloseBracket, NodeType::TypeExpression)?;

        Ok(self.insert_node(
            TypeExpression::FixedArray { element, length },
            self.get_span_from(start),
        ))
    }

    /// Eat a slice or single element array tuple after `]`.
    ///
    /// Examples:
    /// ```ds
    /// [T]
    /// [readonly T]
    /// [namespace.Value]
    /// ```
    fn eat_closed_single_bracket_type(
        &mut self,
        start: &ParserSpanStart,
        element: LocalNodeId<TypeExpression>,
    ) -> ParseResult<LocalNodeId<TypeExpression>> {
        if self.language.is_destack() {
            return Ok(
                self.insert_node(TypeExpression::Slice { element }, self.get_span_from(start))
            );
        }

        let elements = self.eat_type_tuple_tail(start, TokenType::CloseBracket, element)?;

        Ok(self.insert_node(
            TypeExpression::ArrayTuple { elements },
            self.get_span_from(start),
        ))
    }

    /// Eat an array tuple type after the first element.
    ///
    /// Examples:
    /// ```ds
    /// [string,]
    /// [string, number]
    /// [string, ...boolean[]]
    /// ```
    fn eat_array_tuple_tail_type(
        &mut self,
        start: &ParserSpanStart,
        first: LocalNodeId<TypeExpression>,
    ) -> ParseResult<LocalNodeId<TypeExpression>> {
        let elements = self.eat_type_tuple_tail(start, TokenType::CloseBracket, first)?;
        self.eat_close_token_or_recover_missing(TokenType::CloseBracket, NodeType::TypeExpression)?;

        Ok(self.insert_node(
            TypeExpression::ArrayTuple { elements },
            self.get_span_from(start),
        ))
    }
}
