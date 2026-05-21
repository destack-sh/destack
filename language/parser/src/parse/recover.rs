use crate::parse::flags::ParserFlags;
use crate::{ParseError, ParseResult, Parser, ParserSpanStart};
use destack_dir::{Expression, LocalNodeId, NodeType, TokenType, TypeExpression};
use destack_source::Span;

impl Parser {
    /// Insert one missing expression node at the current cursor position.
    pub(crate) fn insert_missing_expression_here(&mut self) -> LocalNodeId<Expression> {
        let anchor_span = self.anchor_span_here();
        let missing_span = Span::new(anchor_span.file, anchor_span.start, anchor_span.start);

        self.insert_node(Expression::Missing, missing_span)
    }

    /// Insert one missing type expression node at the current cursor position.
    pub(crate) fn insert_missing_type_expression_here(&mut self) -> LocalNodeId<TypeExpression> {
        let anchor_span = self.anchor_span_here();
        let missing_span = Span::new(anchor_span.file, anchor_span.start, anchor_span.start);

        self.insert_node(TypeExpression::Missing, missing_span)
    }

    /// Return the best local anchor span at the current cursor position.
    pub(crate) fn anchor_span_here(&mut self) -> Span {
        if let Ok(token) = self.peek() {
            token.span
        } else {
            self.eof_span()
        }
    }

    /// Report one unexpected node at the current cursor position.
    pub(crate) fn report_unexpected_for_here(&mut self, owner: NodeType) {
        let error = ParseError::unexpected_for(self.anchor_span_here(), owner);

        self.error(&error);
    }

    /// Recover one missing token at the current cursor position.
    pub(crate) fn recover_missing_token_here(
        &mut self,
        expected: TokenType,
        owner: NodeType,
        is_recoverable_boundary: bool,
    ) -> ParseResult<()> {
        if !is_recoverable_boundary {
            return Err(ParseError::expected(self.anchor_span_here(), expected));
        }

        self.report_unexpected_for_here(owner);
        Ok(())
    }

    /// Report one missing expression and insert the missing node.
    pub(crate) fn recover_missing_expression_here(
        &mut self,
        owner: NodeType,
    ) -> LocalNodeId<Expression> {
        self.report_unexpected_for_here(owner);
        self.insert_missing_expression_here()
    }

    /// Report one missing type expression and insert the missing node.
    pub(crate) fn recover_missing_type_expression_here(
        &mut self,
        owner: NodeType,
    ) -> LocalNodeId<TypeExpression> {
        self.report_unexpected_for_here(owner);
        self.insert_missing_type_expression_here()
    }

    /// Eat one type expression or recover one missing child at a type boundary.
    pub(crate) fn eat_type_expression_or_recover_missing(
        &mut self,
        flags: ParserFlags,
        owner: NodeType,
    ) -> ParseResult<LocalNodeId<TypeExpression>> {
        if self.is_type_expression_boundary() {
            return Ok(self.recover_missing_type_expression_here(owner));
        }

        self.eat_type_expression_in_flags(flags.in_type())
    }

    /// Eat one expression or recover one missing child at an expression boundary.
    pub(crate) fn eat_expression_or_recover_missing(
        &mut self,
        flags: ParserFlags,
        owner: NodeType,
    ) -> ParseResult<LocalNodeId<Expression>> {
        if Self::is_expression_slot_boundary_token(self.peek_token_type()) {
            return Ok(self.recover_missing_expression_here(owner));
        }

        self.eat_expression(flags)
    }

    /// Eat one close token or recover one missing close delimiter.
    pub(crate) fn eat_close_token_or_recover_missing(
        &mut self,
        expected: TokenType,
        owner: NodeType,
    ) -> ParseResult<()> {
        self.eat_close_token_or_recover_missing_with(expected, owner, |parser, token_type| {
            Self::is_close_delimiter_boundary_token(token_type)
                || parser.current_token_is_on_new_line()
                    && Self::token_can_start_recovered_statement_item(token_type)
        })
    }

    /// Eat one close token or recover one missing close delimiter with custom boundaries.
    pub(crate) fn eat_close_token_or_recover_missing_with(
        &mut self,
        expected: TokenType,
        owner: NodeType,
        is_recoverable_boundary: impl FnOnce(&mut Self, TokenType) -> bool,
    ) -> ParseResult<()> {
        if self.peek_is(expected) {
            self.bump();
            return Ok(());
        }

        let token_type = self.peek_token_type();
        let is_recoverable_boundary = is_recoverable_boundary(self, token_type);

        self.recover_missing_token_here(expected, owner, is_recoverable_boundary)
    }

    /// Eat one list close token or recover one missing delimiter in place.
    pub(crate) fn eat_list_close_token_or_recover_missing(
        &mut self,
        expected: TokenType,
        owner: NodeType,
    ) -> ParseResult<()> {
        if self.peek_is(expected) {
            self.bump();
            return Ok(());
        }

        self.report_unexpected_for_here(owner);
        Ok(())
    }

    /// Eat one type close token or recover one missing delimiter at a type boundary.
    pub(crate) fn eat_type_token_or_recover_missing(
        &mut self,
        expected: TokenType,
        owner: NodeType,
    ) -> ParseResult<()> {
        self.eat_close_token_or_recover_missing_with(expected, owner, |_, token_type| {
            Self::is_type_container_boundary_token(token_type)
        })
    }

    /// Attempt a parse with token recovery.
    pub fn with_token_recovery<T>(
        &mut self,
        start: &ParserSpanStart,
        parse: impl FnOnce(&mut Self) -> ParseResult<T>,
        default: T,
        bail: TokenType,
    ) -> T {
        match parse(self) {
            Ok(result) => result,
            Err(error) => {
                let _ = self.try_recover(start, bail, Some(error));
                default
            }
        }
    }

    /// Attempt a parse with statement recovery.
    pub fn with_statement_recovery<T>(
        &mut self,
        start: &ParserSpanStart,
        parse: impl FnOnce(&mut Self) -> ParseResult<T>,
        default: T,
    ) -> T {
        match parse(self) {
            Ok(result) => result,
            Err(error) => {
                let _ = self.try_recover_in_statement(start, Some(error));
                default
            }
        }
    }

    /// Recover until the expected token.
    pub fn try_recover(
        &mut self,
        start: &ParserSpanStart,
        recover: TokenType,
        error: Option<ParseError>,
    ) -> ParseResult<()> {
        while let Ok(token) = self.peek() {
            let token_type = token.token.ty;

            if token_type == TokenType::End {
                break;
            }

            if token_type == recover {
                let error = ParseError::from_source_maybe(self.get_span_from(start), error);
                self.error(&error);
                return Ok(());
            }

            self.bump();
        }

        let error = ParseError::from_source_maybe(self.get_span_from(start), error);
        self.error(&error);

        Err(error)
    }

    /// Recover within one list item until a separator or terminator boundary.
    pub fn try_recover_in_item_list(
        &mut self,
        start: &ParserSpanStart,
        terminator: TokenType,
        error: Option<ParseError>,
    ) -> ParseResult<()> {
        while let Ok(token) = self.peek() {
            let token_type = token.token.ty;

            if token_type == TokenType::End {
                break;
            }

            let is_new_line_boundary = start.is_before(token.span) && token.token.is_on_new_line;
            let is_boundary = is_new_line_boundary
                || self.token_matches_terminator(token_type, terminator)
                || Self::is_item_stop_token(token_type)
                || Self::is_close_delimiter_token(token_type);
            if is_boundary {
                let error = ParseError::from_source_maybe(self.get_span_from(start), error);
                self.error(&error);
                return Ok(());
            }

            self.bump();
        }

        let error = ParseError::from_source_maybe(self.get_span_from(start), error);
        self.error(&error);

        Err(error)
    }

    /// Recover within one statement until a statement boundary.
    pub fn try_recover_in_statement(
        &mut self,
        start: &ParserSpanStart,
        error: Option<ParseError>,
    ) -> ParseResult<()> {
        while let Ok(token) = self.peek() {
            let token_type = token.token.ty;

            if token_type == TokenType::End {
                break;
            }

            let is_new_line_boundary = start.is_before(token.span) && token.token.is_on_new_line;
            let is_boundary = is_new_line_boundary
                || Self::is_statement_stop_token(token_type)
                || token_type == TokenType::CloseBrace;
            if is_boundary {
                let error = ParseError::from_source_maybe(self.get_span_from(start), error);
                self.error(&error);
                return Ok(());
            }

            self.bump();
        }

        let error = ParseError::from_source_maybe(self.get_span_from(start), error);
        self.error(&error);

        Ok(())
    }

    /// Recover within one statement from an existing source span.
    pub fn try_recover_in_statement_from_span(
        &mut self,
        start_span: Span,
        error: Option<ParseError>,
    ) -> ParseResult<Span> {
        while let Ok(token) = self.peek() {
            let token_type = token.token.ty;

            if token_type == TokenType::End {
                break;
            }

            let is_new_line_boundary =
                start_span.start < token.span.start && token.token.is_on_new_line;
            let is_boundary = is_new_line_boundary
                || Self::is_statement_stop_token(token_type)
                || token_type == TokenType::CloseBrace;
            if is_boundary {
                let recovered_span = self.recovered_span_from(start_span);
                let error = ParseError::from_source_maybe(recovered_span, error);
                self.error(&error);
                return Ok(recovered_span);
            }

            self.bump();
        }

        let recovered_span = self.recovered_span_from(start_span);
        let error = ParseError::from_source_maybe(recovered_span, error);
        self.error(&error);

        Ok(recovered_span)
    }

    /// Return true when a recovered list item may continue parsing another item.
    pub(crate) fn can_continue_after_recovered_item(
        &mut self,
        terminator: TokenType,
        is_recovered_item: bool,
    ) -> bool {
        if !is_recovered_item {
            return false;
        }

        let token_type = self.peek_token_type();
        !self.token_matches_terminator(token_type, terminator)
            && !Self::is_close_delimiter_token(token_type)
            && token_type != TokenType::End
    }

    /// Return true when one token satisfies one recovery terminator.
    #[inline]
    fn token_matches_terminator(&self, token_type: TokenType, terminator: TokenType) -> bool {
        if terminator == TokenType::GreaterThan {
            return Self::starts_type_angle_close(token_type);
        }

        token_type == terminator
    }

    /// Recover within a property or member body until a boundary token.
    pub fn try_recover_in_body(
        &mut self,
        start: &ParserSpanStart,
        error: Option<ParseError>,
    ) -> ParseResult<()> {
        while let Ok(token) = self.peek() {
            let token_type = token.token.ty;

            if token_type == TokenType::End {
                break;
            }

            let is_new_line_boundary = start.is_before(token.span) && token.token.is_on_new_line;
            let is_boundary = is_new_line_boundary
                || token_type == TokenType::CloseBrace
                || Self::is_any_stop_token(token_type);
            if is_boundary {
                let error = ParseError::from_source_maybe(self.get_span_from(start), error);
                self.error(&error);
                return Ok(());
            }

            self.bump();
        }

        let error = ParseError::from_source_maybe(self.get_span_from(start), error);
        self.error(&error);

        Err(error)
    }

    /// Recover within a property or member body from an existing source span.
    pub fn try_recover_in_body_from_span(
        &mut self,
        start_span: Span,
        error: Option<ParseError>,
    ) -> ParseResult<Span> {
        while let Ok(token) = self.peek() {
            let token_type = token.token.ty;

            if token_type == TokenType::End {
                break;
            }

            let is_new_line_boundary =
                start_span.start < token.span.start && token.token.is_on_new_line;
            let is_boundary = is_new_line_boundary
                || token_type == TokenType::CloseBrace
                || Self::is_any_stop_token(token_type);
            if is_boundary {
                let recovered_span = self.recovered_span_from(start_span);
                let error = ParseError::from_source_maybe(recovered_span, error);
                self.error(&error);
                return Ok(recovered_span);
            }

            self.bump();
        }

        let recovered_span = self.recovered_span_from(start_span);
        let error = ParseError::from_source_maybe(recovered_span, error);
        self.error(&error);

        Err(error)
    }

    /// Return a recovered span from one source span start to the previous token.
    #[inline]
    fn recovered_span_from(&self, start_span: Span) -> Span {
        let end = self.prev_token_end().max(start_span.start);

        Span::new(start_span.file, start_span.start, end)
    }

    /// Eat the expected token with forward recovery.
    pub fn try_eat_token(&mut self, expected: TokenType, bail: TokenType) -> ParseResult<()> {
        if self.peek_is(expected) {
            self.bump();
            return Ok(());
        }

        let start = self.span_start();
        while let Ok(token) = self.peek()
            && token.token.ty != bail
        {
            if token.token.ty == TokenType::End {
                break;
            }

            if token.token.ty == expected {
                let error = ParseError::unexpected(self.get_span_from(&start));
                self.bump();
                self.error(&error);
                return Ok(());
            }

            self.bump();
        }

        let error = ParseError::unexpected(self.get_span_from(&start));
        self.error(&error);

        Err(error)
    }
}
