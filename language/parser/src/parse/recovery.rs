use crate::parse::flags::ParserFlags;
use crate::parse::mode::ContextualLexMode;
use crate::parse::scan::DelimiterDepth;
use crate::parse::{
    TypeMemberContainerKind, is_declaration_keyword, is_declaration_modifier_keyword,
};
use crate::{Parser, ParserError, ParserResult, ParserSpanStart};
use destack_dir::{
    Expression, Keyword, LocalNodeId, NodeType, TokenSpan, TokenType, TreeAttribute, TreeChild,
    TypeExpression,
};
use destack_source::Span;

/// A grammar point where parsing can resume after damaged syntax.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum RecoveryPoint {
    /// A statement or declaration.
    Statement,
    /// A declaration.
    Declaration,
    /// An outer declaration after a damaged type expression.
    TypeExpressionDeclaration,
    /// An outer declaration after a damaged type member.
    TypeMemberDeclaration(TypeMemberContainerKind),
}

impl RecoveryPoint {
    /// Return whether one keyword can start this recovery point.
    fn accepts_keyword(self, keyword: Keyword) -> bool {
        match self {
            Self::Statement => Self::statement_keyword(keyword),
            Self::Declaration => keyword == Keyword::Export || is_declaration_keyword(keyword),
            Self::TypeExpressionDeclaration => {
                keyword == Keyword::Export
                    || keyword == Keyword::Type
                    || keyword == Keyword::Newtype
                    || Self::outer_declaration_keyword(keyword)
            }
            Self::TypeMemberDeclaration(container_kind) => {
                keyword == Keyword::Export
                    || keyword == Keyword::Newtype
                    || (!container_kind.allows_associated_members() && keyword == Keyword::Type)
                    || Self::outer_declaration_keyword(keyword)
            }
        }
    }

    /// Return whether one keyword can modify this recovery point.
    fn accepts_modifier(self, keyword: Keyword) -> bool {
        match self {
            Self::TypeExpressionDeclaration | Self::TypeMemberDeclaration(_) => {
                is_declaration_modifier_keyword(keyword)
            }
            _ => false,
        }
    }

    /// Return whether one following token is valid after this recovery point head.
    fn accepts_following_token(self, token_type: TokenType) -> bool {
        match self {
            Self::TypeExpressionDeclaration => Self::declaration_keyword_follow_token(token_type),
            _ => true,
        }
    }

    /// Return whether one token can follow a recovered declaration keyword.
    fn declaration_keyword_follow_token(token_type: TokenType) -> bool {
        matches!(token_type, TokenType::Identifier | TokenType::At)
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
    pub(crate) fn semicolon_precedes_recovery_point(&mut self, point: RecoveryPoint) -> bool {
        if !self.peek_is(TokenType::Semicolon) {
            return false;
        }

        let next = self.next_token();
        let following_token_type = self.token_type_at_offset(2);

        // require the recovery point to start after a line boundary
        if !next.is_on_new_line() {
            return false;
        }

        // stay in the current item when the next token still binds to it
        if Self::token_continues_current_recovery_item(following_token_type) {
            return false;
        }

        // keep contextual recovery on a real declaration shaped head
        if !point.accepts_following_token(following_token_type) {
            return false;
        }

        self.keyword_at_offset(1)
            .is_some_and(|keyword| point.accepts_keyword(keyword))
    }

    /// Return whether the current token starts one recovery point.
    pub(crate) fn current_token_starts_recovery_point(&mut self, point: RecoveryPoint) -> bool {
        let Some(keyword) = self.current_keyword() else {
            return false;
        };

        let following_token_type = self.token_type_at_offset(1);

        // require recovery heads to begin a logical line
        if !self.current_token_is_on_new_line() {
            return false;
        }

        // stay in the current item when the next token still binds to it
        if Self::token_continues_current_recovery_item(following_token_type) {
            return false;
        }

        // keep contextual recovery on a real declaration shaped head
        if !point.accepts_following_token(following_token_type) {
            return false;
        }

        // accept direct recovery keywords
        if point.accepts_keyword(keyword) {
            return true;
        }

        self.modifier_precedes_recovery_keyword(point, keyword)
    }

    /// Return whether one modifier is followed by a recovery keyword.
    fn modifier_precedes_recovery_keyword(
        &mut self,
        point: RecoveryPoint,
        keyword: Keyword,
    ) -> bool {
        if !point.accepts_modifier(keyword) {
            return false;
        }

        self.keyword_at_offset(1)
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
    pub(crate) fn anchor_span_here(&self) -> Span {
        self.peek().span
    }

    /// Report one unexpected node at the current cursor position.
    pub(crate) fn report_unexpected_for_here(&mut self, owner: NodeType) {
        let error = ParserError::unexpected_for(self.anchor_span_here(), owner);

        self.report_error(&error);
    }

    /// Report one expected token at the current cursor position.
    fn report_expected_for_here(&mut self, expected: TokenType, owner: NodeType) {
        let error = ParserError::expected_for(self.peek(), expected, owner);
        self.report_error(&error);
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

        self.report_expected_for_here(expected, owner);

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

    /// Recover one malformed tree attribute and preserve its list position.
    pub(crate) fn recover_tree_attribute(
        &mut self,
        start: &ParserSpanStart,
        error: ParserError,
    ) -> LocalNodeId<TreeAttribute> {
        self.skip_damaged_tree_attribute();

        let recovered_span = self.get_span_from(start);
        let error = error.for_node_type(NodeType::TreeAttribute);
        self.report_error(&error);

        self.insert_node(TreeAttribute::Error, recovered_span)
    }

    /// Recover one malformed tree child and preserve its list position.
    pub(crate) fn recover_tree_child(
        &mut self,
        start: &ParserSpanStart,
        error: ParserError,
    ) -> LocalNodeId<TreeChild> {
        self.skip_damaged_tree_child();

        let recovered_span = self.get_span_from(start);
        let error = error.for_node_type(NodeType::TreeChild);
        self.report_error(&error);

        self.insert_node(TreeChild::Error, recovered_span)
    }

    /// Skip one damaged tree attribute without consuming the next attribute.
    fn skip_damaged_tree_attribute(&mut self) {
        // consume a malformed spread container as one attribute
        if self.peek_is(TokenType::OpenBrace) {
            self.skip_damaged_tree_braces(ContextualLexMode::TreeTag);
            return;
        }

        // consume the malformed attribute head
        if self.peek_is(TokenType::End)
            || self.peek_starts_tree_tag_close()
            || self.peek_starts_tree_literal_close()
        {
            return;
        }
        self.bump_with_contextual_lex_mode(ContextualLexMode::TreeTag);

        // leave a following attribute untouched when the value is missing
        if !self.peek_is(TokenType::Assign) {
            return;
        }
        self.bump_with_contextual_lex_mode(ContextualLexMode::TreeAttributeValue);
        if self.peek_is(TokenType::Identifier)
            || self.peek_is(TokenType::Divide)
            || self.peek_starts_tree_tag_close()
            || self.peek_is(TokenType::End)
        {
            self.set_tree_tag_follow();
            return;
        }

        // consume one delimited or scalar attribute value
        if self.peek_is(TokenType::OpenBrace) {
            self.skip_damaged_tree_braces(ContextualLexMode::TreeTag);
        } else {
            self.bump_with_contextual_lex_mode(ContextualLexMode::TreeTag);
        }
    }

    /// Skip one damaged tree child and resume in child mode.
    fn skip_damaged_tree_child(&mut self) {
        match self.peek_token_type() {
            TokenType::OpenBrace => self.skip_damaged_tree_braces(ContextualLexMode::TreeChild),
            TokenType::LessThan => self.skip_damaged_tree_tag(),
            TokenType::End => {}
            _ => self.bump_with_contextual_lex_mode(ContextualLexMode::TreeChild),
        }
    }

    /// Skip one damaged tree expression container.
    fn skip_damaged_tree_braces(&mut self, follow_mode: ContextualLexMode) {
        let mut depth = 0usize;

        loop {
            let token_type = self.peek_token_type();

            // leave an enclosing closing tag for the tree parser
            if depth == 1 && self.peek_starts_tree_literal_close() {
                return;
            }

            match token_type {
                TokenType::OpenBrace => {
                    depth += 1;
                    self.bump_with_contextual_lex_mode(ContextualLexMode::Normal);
                }
                TokenType::CloseBrace if depth == 1 => {
                    self.bump_with_contextual_lex_mode(follow_mode);
                    return;
                }
                TokenType::CloseBrace => {
                    depth -= 1;
                    self.bump_with_contextual_lex_mode(ContextualLexMode::Normal);
                }
                TokenType::End => return,
                _ => self.bump_with_contextual_lex_mode(ContextualLexMode::Normal),
            }
        }
    }

    /// Skip one damaged tree tag head and resume in child mode.
    fn skip_damaged_tree_tag(&mut self) {
        self.bump_with_contextual_lex_mode(ContextualLexMode::TreeTag);

        loop {
            // leave an ancestor closing tag for the open tree stack
            if self.peek_starts_tree_literal_close() {
                return;
            }

            if self.peek_starts_tree_tag_close() {
                self.bump_with_contextual_lex_mode(ContextualLexMode::TreeChild);
                return;
            }

            if self.peek_is(TokenType::End) {
                return;
            }

            self.bump_with_contextual_lex_mode(ContextualLexMode::TreeTag);
        }
    }

    /// Eat one type expression or recover one missing child at a type boundary.
    pub(crate) fn eat_type_expression_or_recover_missing(
        &mut self,
        flags: ParserFlags,
        owner: NodeType,
    ) -> ParserResult<LocalNodeId<TypeExpression>> {
        if self.is_type_expression_recovery_boundary() {
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

        // allow the caller to define the grammar boundary it owns
        let token_type = self.peek_token_type();
        let is_recoverable_boundary = is_recoverable_boundary(self, token_type);

        self.recover_missing_token_here(expected, owner, is_recoverable_boundary)
    }

    /// Eat one list close token or recover one missing delimiter in place.
    pub(crate) fn eat_list_close_token_or_recover_missing(
        &mut self,
        expected: TokenType,
        owner: NodeType,
    ) {
        if self.peek_is(expected) {
            self.bump();
            return;
        }

        self.report_expected_for_here(expected, owner);
    }

    /// Eat one type close token or recover one missing delimiter at a type boundary.
    pub(crate) fn eat_type_token_or_recover_missing(
        &mut self,
        expected: TokenType,
        owner: NodeType,
    ) -> ParserResult<()> {
        self.eat_close_token_or_recover_missing_with(expected, owner, |parser, token_type| {
            parser.type_token_recovery_boundary(token_type)
        })
    }

    /// Return whether type parsing can recover one missing close token here.
    fn type_token_recovery_boundary(&mut self, token_type: TokenType) -> bool {
        Self::is_type_container_boundary_token(token_type)
            || self.current_token_starts_recovery_point(RecoveryPoint::TypeExpressionDeclaration)
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
                let start_span = self.get_span_from(start);
                self.recover_until(start_span, bail, Some(error));
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
                let start_span = self.get_span_from(start);
                self.recover_statement(start_span, Some(error));
                default
            }
        }
    }

    /// Recover until the expected token.
    pub fn recover_until(
        &mut self,
        start_span: Span,
        recover: TokenType,
        error: Option<ParserError>,
    ) -> Span {
        loop {
            let token = self.peek();
            let token_type = token.token.ty();

            if token_type == TokenType::End {
                break;
            }

            if token_type == recover {
                return self.complete_recovery(start_span, error);
            }

            self.bump();
        }

        self.complete_recovery(start_span, error)
    }

    /// Recover within one list item until a separator or terminator boundary.
    pub fn recover_list_item(
        &mut self,
        start_span: Span,
        terminator: TokenType,
        error: Option<ParserError>,
    ) -> Span {
        let mut depth = DelimiterDepth::for_list_terminator(terminator);

        loop {
            let token = self.peek();
            let token_type = token.token.ty();

            // stop at end of input
            if token_type == TokenType::End {
                break;
            }

            // stop before the next item or enclosing grammar boundary
            if self.item_list_recovery_boundary(start_span, token, terminator, &depth) {
                return self.complete_recovery(start_span, error);
            }

            // keep scanning within nested delimiters
            depth.advance(token_type);
            self.bump();
        }

        self.complete_recovery(start_span, error)
    }

    /// Return whether one token ends recovery for the current list item.
    fn item_list_recovery_boundary(
        &self,
        start_span: Span,
        token: TokenSpan,
        terminator: TokenType,
        depth: &DelimiterDepth,
    ) -> bool {
        if !depth.is_top_level() {
            return false;
        }

        let token_type = token.token.ty();
        let is_new_line_boundary =
            start_span.start < token.span.start && token.token.is_on_new_line();

        is_new_line_boundary
            || self.token_matches_terminator(token_type, terminator)
            || Self::is_item_stop_token(token_type)
            || Self::is_statement_stop_token(token_type)
            || Self::is_close_delimiter_token(token_type)
    }

    /// Recover within one statement until a statement boundary.
    pub fn recover_statement(&mut self, start_span: Span, error: Option<ParserError>) -> Span {
        loop {
            let token = self.peek();
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
                return self.complete_recovery(start_span, error);
            }

            self.bump();
        }

        self.complete_recovery(start_span, error)
    }

    /// Return true when a recovered list item may continue parsing another item.
    pub(crate) fn can_continue_after_recovered_item(&mut self, terminator: TokenType) -> bool {
        let token_type = self.peek_token_type();
        !self.token_matches_terminator(token_type, terminator)
            && !Self::is_close_delimiter_token(token_type)
            && !Self::is_statement_stop_token(token_type)
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
    pub fn recover_body(&mut self, start_span: Span, error: Option<ParserError>) -> Span {
        loop {
            let token = self.peek();
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
                return self.complete_recovery(start_span, error);
            }

            self.bump();
        }

        self.complete_recovery(start_span, error)
    }

    /// Complete one recovery and return its recovered span.
    fn complete_recovery(&mut self, start_span: Span, error: Option<ParserError>) -> Span {
        let recovered_span = self.recovered_span_from(start_span);
        let error = error.unwrap_or_else(|| ParserError::unexpected(recovered_span));
        self.report_error(&error);

        recovered_span
    }

    /// Return one source span from a recovery start to the previous token.
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
        loop {
            let token_type = self.peek_token_type();
            if matches!(token_type, TokenType::End) || token_type == bail {
                break;
            }

            if token_type == expected {
                let error = ParserError::unexpected(self.get_span_from(&start));
                self.bump();
                self.report_error(&error);
                return Ok(());
            }

            self.bump();
        }

        let error = ParserError::unexpected(self.get_span_from(&start));
        self.report_error(&error);

        Err(error)
    }
}
