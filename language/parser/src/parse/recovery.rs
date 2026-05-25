use crate::parse::flags::ParserFlags;
use crate::parse::{TypeMemberContainerKind, is_declaration_keyword};
use crate::{Parser, ParserError, ParserResult, ParserSpanStart};
use destack_dir::{Expression, Keyword, LocalNodeId, NodeType, TokenType, TypeExpression};
use destack_source::Span;

/// Delimiter depth while recovering one malformed list item.
#[derive(Debug, Default)]
struct RecoveryDelimiterDepth {
    /// The nested parenthesis depth.
    parenthesis: usize,
    /// The nested bracket depth.
    bracket: usize,
    /// The nested brace depth.
    brace: usize,
    /// The nested type angle depth.
    angle: usize,
    /// Whether angle brackets are item delimiters.
    tracks_angle: bool,
}

impl RecoveryDelimiterDepth {
    /// Create delimiter state for one list terminator.
    fn new(terminator: TokenType) -> Self {
        Self {
            tracks_angle: terminator == TokenType::GreaterThan,
            ..Self::default()
        }
    }

    /// Return whether recovery is scanning the list item itself.
    fn is_top_level(&self) -> bool {
        self.parenthesis == 0 && self.bracket == 0 && self.brace == 0 && self.angle == 0
    }

    /// Advance delimiter state after one consumed token.
    fn advance(&mut self, token_type: TokenType) {
        match token_type {
            TokenType::OpenParenthesis => self.parenthesis += 1,
            TokenType::CloseParenthesis => Self::close(&mut self.parenthesis),
            TokenType::OpenBracket => self.bracket += 1,
            TokenType::CloseBracket => Self::close(&mut self.bracket),
            TokenType::OpenBrace => self.brace += 1,
            TokenType::CloseBrace => Self::close(&mut self.brace),
            TokenType::LessThan if self.tracks_angle => self.angle += 1,
            TokenType::ShiftLeft if self.tracks_angle => self.angle += 2,
            TokenType::GreaterThan if self.tracks_angle => self.close_angle(1),
            TokenType::ShiftRight if self.tracks_angle => self.close_angle(2),
            TokenType::UnsignedShiftRight if self.tracks_angle => self.close_angle(3),
            _ => {}
        }
    }

    /// Close one delimiter level.
    fn close(depth: &mut usize) {
        *depth = depth.saturating_sub(1);
    }

    /// Close one or more angle levels.
    fn close_angle(&mut self, width: usize) {
        self.angle = self.angle.saturating_sub(width);
    }
}

/// A grammar point where parsing can resume after damaged syntax.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum RecoveryPoint {
    /// A statement or declaration.
    Statement,
    /// A declaration.
    Declaration,
    /// An outer declaration after a damaged type member.
    TypeMemberDeclaration(TypeMemberContainerKind),
}

impl RecoveryPoint {
    /// Return whether one keyword can start this recovery point.
    fn accepts_keyword(self, keyword: Keyword) -> bool {
        match self {
            Self::Statement => Self::statement_keyword(keyword),
            Self::Declaration => keyword == Keyword::Export || is_declaration_keyword(keyword),
            Self::TypeMemberDeclaration(container_kind) => {
                keyword == Keyword::Export
                    || (!container_kind.allows_associated_members() && keyword == Keyword::Type)
                    || Self::outer_declaration_keyword(keyword)
            }
        }
    }

    /// Return whether one keyword can modify this recovery point.
    fn accepts_modifier(self, keyword: Keyword) -> bool {
        matches!(self, Self::TypeMemberDeclaration(_))
            && matches!(
                keyword,
                Keyword::Declare
                    | Keyword::Abstract
                    | Keyword::Final
                    | Keyword::Override
                    | Keyword::Public
                    | Keyword::Protected
                    | Keyword::Private
                    | Keyword::Async
            )
    }

    /// Return whether one keyword starts a recovered statement.
    fn statement_keyword(keyword: Keyword) -> bool {
        is_declaration_keyword(keyword)
            || matches!(
                keyword,
                Keyword::If
                    | Keyword::For
                    | Keyword::While
                    | Keyword::Do
                    | Keyword::Switch
                    | Keyword::Return
                    | Keyword::Throw
                    | Keyword::Try
                    | Keyword::Break
                    | Keyword::Continue
                    | Keyword::Yield
            )
    }

    /// Return whether one keyword starts an outer declaration.
    fn outer_declaration_keyword(keyword: Keyword) -> bool {
        matches!(
            keyword,
            Keyword::Struct
                | Keyword::Class
                | Keyword::Enum
                | Keyword::Function
                | Keyword::Extension
                | Keyword::Interface
                | Keyword::Newtype
                | Keyword::Const
                | Keyword::Let
                | Keyword::Using
        )
    }
}

impl Parser {
    /// Return whether the current semicolon precedes one recovery point.
    pub(crate) fn current_semicolon_precedes_recovery_point(
        &mut self,
        token_type: TokenType,
        point: RecoveryPoint,
    ) -> bool {
        token_type == TokenType::Semicolon && self.semicolon_precedes_recovery_point(point)
    }

    /// Return whether the current token starts one recovery point.
    pub(crate) fn current_token_starts_recovery_point(&mut self, point: RecoveryPoint) -> bool {
        let Some(keyword) = self.current_keyword() else {
            return false;
        };
        let following_token_type = self.token_type_at_offset(1);

        if !self.current_token_is_on_new_line()
            || Self::token_continues_current_recovery_item(following_token_type)
        {
            return false;
        }

        if point.accepts_keyword(keyword) {
            return true;
        }

        point.accepts_modifier(keyword)
            && self
                .keyword_at_offset(1)
                .is_some_and(|keyword| point.accepts_keyword(keyword))
    }

    /// Return whether the current semicolon is followed by one recovery point.
    fn semicolon_precedes_recovery_point(&mut self, point: RecoveryPoint) -> bool {
        let next = self.next_token();
        let following_token_type = self.token_type_at_offset(2);

        next.token.is_on_new_line()
            && !Self::token_continues_current_recovery_item(following_token_type)
            && self
                .keyword_at_offset(1)
                .is_some_and(|keyword| point.accepts_keyword(keyword))
    }

    /// Return whether one following token keeps the current item ambiguous.
    fn token_continues_current_recovery_item(token_type: TokenType) -> bool {
        matches!(
            token_type,
            TokenType::Colon | TokenType::Maybe | TokenType::OpenParenthesis
        )
    }

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
        let error = ParserError::unexpected_for(self.anchor_span_here(), owner);

        self.error(&error);
    }

    /// Recover one missing token at the current cursor position.
    pub(crate) fn recover_missing_token_here(
        &mut self,
        expected: TokenType,
        owner: NodeType,
        is_recoverable_boundary: bool,
    ) -> ParserResult<()> {
        if !is_recoverable_boundary {
            return Err(ParserError::expected(self.anchor_span_here(), expected));
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
    ) -> ParserResult<LocalNodeId<TypeExpression>> {
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
    ) -> ParserResult<LocalNodeId<Expression>> {
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
    ) -> ParserResult<()> {
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
    ) -> ParserResult<()> {
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
    ) -> ParserResult<()> {
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
    ) -> ParserResult<()> {
        self.eat_close_token_or_recover_missing_with(expected, owner, |_, token_type| {
            Self::is_type_container_boundary_token(token_type)
        })
    }

    /// Attempt a parse with token recovery.
    pub fn with_token_recovery<T>(
        &mut self,
        start: &ParserSpanStart,
        parse: impl FnOnce(&mut Self) -> ParserResult<T>,
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
        parse: impl FnOnce(&mut Self) -> ParserResult<T>,
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
        error: Option<ParserError>,
    ) -> ParserResult<()> {
        while let Ok(token) = self.peek() {
            let token_type = token.token.ty();

            if token_type == TokenType::End {
                break;
            }

            if token_type == recover {
                let error = ParserError::from_source_maybe(self.get_span_from(start), error);
                self.error(&error);
                return Ok(());
            }

            self.bump();
        }

        let error = ParserError::from_source_maybe(self.get_span_from(start), error);
        self.error(&error);

        Err(error)
    }

    /// Recover within one list item until a separator or terminator boundary.
    pub fn try_recover_in_item_list(
        &mut self,
        start: &ParserSpanStart,
        terminator: TokenType,
        error: Option<ParserError>,
    ) -> ParserResult<()> {
        let mut depth = RecoveryDelimiterDepth::new(terminator);

        while let Ok(token) = self.peek() {
            let token_type = token.token.ty();

            if token_type == TokenType::End {
                break;
            }

            let is_new_line_boundary = start.is_before(token.span) && token.token.is_on_new_line();
            let is_boundary = depth.is_top_level()
                && (is_new_line_boundary
                    || self.token_matches_terminator(token_type, terminator)
                    || Self::is_item_stop_token(token_type)
                    || Self::is_close_delimiter_token(token_type));
            if is_boundary {
                let error = ParserError::from_source_maybe(self.get_span_from(start), error);
                self.error(&error);
                return Ok(());
            }

            depth.advance(token_type);
            self.bump();
        }

        let error = ParserError::from_source_maybe(self.get_span_from(start), error);
        self.error(&error);

        Err(error)
    }

    /// Recover within one statement until a statement boundary.
    pub fn try_recover_in_statement(
        &mut self,
        start: &ParserSpanStart,
        error: Option<ParserError>,
    ) -> ParserResult<()> {
        while let Ok(token) = self.peek() {
            let token_type = token.token.ty();

            if token_type == TokenType::End {
                break;
            }

            let is_new_line_boundary = start.is_before(token.span) && token.token.is_on_new_line();
            let is_boundary = is_new_line_boundary
                || Self::is_statement_stop_token(token_type)
                || token_type == TokenType::CloseBrace;
            if is_boundary {
                let error = ParserError::from_source_maybe(self.get_span_from(start), error);
                self.error(&error);
                return Ok(());
            }

            self.bump();
        }

        let error = ParserError::from_source_maybe(self.get_span_from(start), error);
        self.error(&error);

        Ok(())
    }

    /// Recover within one statement from an existing source span.
    pub fn try_recover_in_statement_from_span(
        &mut self,
        start_span: Span,
        error: Option<ParserError>,
    ) -> ParserResult<Span> {
        while let Ok(token) = self.peek() {
            let token_type = token.token.ty();

            if token_type == TokenType::End {
                break;
            }

            let is_new_line_boundary =
                start_span.start < token.span.start && token.token.is_on_new_line();
            let is_boundary = is_new_line_boundary
                || Self::is_statement_stop_token(token_type)
                || token_type == TokenType::CloseBrace;
            if is_boundary {
                let recovered_span = self.recovered_span_from(start_span);
                let error = ParserError::from_source_maybe(recovered_span, error);
                self.error(&error);
                return Ok(recovered_span);
            }

            self.bump();
        }

        let recovered_span = self.recovered_span_from(start_span);
        let error = ParserError::from_source_maybe(recovered_span, error);
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
        error: Option<ParserError>,
    ) -> ParserResult<()> {
        while let Ok(token) = self.peek() {
            let token_type = token.token.ty();

            if token_type == TokenType::End {
                break;
            }

            let is_new_line_boundary = start.is_before(token.span) && token.token.is_on_new_line();
            let is_boundary = is_new_line_boundary
                || token_type == TokenType::CloseBrace
                || Self::is_any_stop_token(token_type);
            if is_boundary {
                let error = ParserError::from_source_maybe(self.get_span_from(start), error);
                self.error(&error);
                return Ok(());
            }

            self.bump();
        }

        let error = ParserError::from_source_maybe(self.get_span_from(start), error);
        self.error(&error);

        Err(error)
    }

    /// Recover within a property or member body from an existing source span.
    pub fn try_recover_in_body_from_span(
        &mut self,
        start_span: Span,
        error: Option<ParserError>,
    ) -> ParserResult<Span> {
        while let Ok(token) = self.peek() {
            let token_type = token.token.ty();

            if token_type == TokenType::End {
                break;
            }

            let is_new_line_boundary =
                start_span.start < token.span.start && token.token.is_on_new_line();
            let is_boundary = is_new_line_boundary
                || token_type == TokenType::CloseBrace
                || Self::is_any_stop_token(token_type);
            if is_boundary {
                let recovered_span = self.recovered_span_from(start_span);
                let error = ParserError::from_source_maybe(recovered_span, error);
                self.error(&error);
                return Ok(recovered_span);
            }

            self.bump();
        }

        let recovered_span = self.recovered_span_from(start_span);
        let error = ParserError::from_source_maybe(recovered_span, error);
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
    pub fn try_eat_token(&mut self, expected: TokenType, bail: TokenType) -> ParserResult<()> {
        if self.peek_is(expected) {
            self.bump();
            return Ok(());
        }

        let start = self.span_start();
        while let Ok(token) = self.peek()
            && token.token.ty() != bail
        {
            if token.token.ty() == TokenType::End {
                break;
            }

            if token.token.ty() == expected {
                let error = ParserError::unexpected(self.get_span_from(&start));
                self.bump();
                self.error(&error);
                return Ok(());
            }

            self.bump();
        }

        let error = ParserError::unexpected(self.get_span_from(&start));
        self.error(&error);

        Err(error)
    }
}
