use crate::parse::lookahead::DelimiterDepth;
use crate::parse::{
    ExpressionPosition, ExpressionStop, TokenMode, TypeMemberContainerKind, TypePosition, TypeStop,
};
use crate::{ParseStart, Parser, ParserError, ParserResult};
use destack_dir::{
    Expression, Keyword, LocalNodeId, NodeType, TokenSpan, TokenType, TreeAttribute, TreeChild,
    TypeExpression,
};
use destack_source::ByteRange;

/// A source point where parsing can resume after damaged syntax.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum RecoveryPoint {
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
            Self::Declaration => {
                keyword == Keyword::Export || Self::is_declaration_keyword(keyword)
            }
            Self::TypeExpressionDeclaration => {
                keyword == Keyword::Export
                    || keyword == Keyword::Type
                    || keyword == Keyword::Newtype
                    || Self::is_outer_declaration_keyword(keyword)
            }
            Self::TypeMemberDeclaration(container_kind) => {
                keyword == Keyword::Export
                    || keyword == Keyword::Newtype
                    || (!container_kind.allows_associated_members() && keyword == Keyword::Type)
                    || Self::is_outer_declaration_keyword(keyword)
            }
        }
    }

    /// Return whether one keyword can modify this recovery point.
    fn accepts_modifier(self, keyword: Keyword) -> bool {
        match self {
            Self::TypeExpressionDeclaration | Self::TypeMemberDeclaration(_) => {
                Self::is_declaration_modifier_keyword(keyword)
            }
            _ => false,
        }
    }

    /// Return whether one following token is valid after this recovery point head.
    fn accepts_following_token(self, token_type: TokenType) -> bool {
        match self {
            Self::TypeExpressionDeclaration => {
                Self::is_declaration_keyword_follow_token(token_type)
            }
            _ => true,
        }
    }

    /// Return whether one token can follow a recovered declaration keyword.
    fn is_declaration_keyword_follow_token(token_type: TokenType) -> bool {
        matches!(token_type, TokenType::Identifier | TokenType::At)
    }

    /// Return whether one keyword starts a recoverable declaration.
    fn is_declaration_keyword(keyword: Keyword) -> bool {
        matches!(
            keyword,
            Keyword::Struct
                | Keyword::Class
                | Keyword::Enum
                | Keyword::Function
                | Keyword::Extension
                | Keyword::Interface
                | Keyword::Type
                | Keyword::Newtype
                | Keyword::Const
                | Keyword::Readonly
                | Keyword::Let
                | Keyword::Using
        ) || Self::is_declaration_modifier_keyword(keyword)
    }

    /// Return whether one keyword can modify a recoverable declaration.
    fn is_declaration_modifier_keyword(keyword: Keyword) -> bool {
        matches!(
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

    /// Return whether one keyword starts an outer declaration.
    fn is_outer_declaration_keyword(keyword: Keyword) -> bool {
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
    pub(crate) fn peek_semicolon_recovery_point(&self, point: RecoveryPoint) -> bool {
        if !self.peek_is(TokenType::Semicolon) {
            return false;
        }

        let next = self.peek_next_token();
        let following_token_type = self.peek_token_type_at(2);

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

        self.peek_keyword_at(1)
            .is_some_and(|keyword| point.accepts_keyword(keyword))
    }

    /// Return whether the current token starts one recovery point.
    pub(crate) fn peek_recovery_point(&self, point: RecoveryPoint) -> bool {
        let Some(keyword) = self.peek_keyword() else {
            return false;
        };

        let following_token_type = self.peek_token_type_at(1);

        // require recovery heads to begin a logical line
        if !self.peek_is_on_new_line() {
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

        self.peek_recovery_keyword_after_modifier(point, keyword)
    }

    /// Return whether one modifier is followed by a recovery keyword.
    fn peek_recovery_keyword_after_modifier(&self, point: RecoveryPoint, keyword: Keyword) -> bool {
        if !point.accepts_modifier(keyword) {
            return false;
        }

        self.peek_keyword_at(1)
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
        let start = self.peek_token().start();
        let missing_range = ByteRange { start, end: start };

        self.insert_node(Expression::Missing, missing_range)
    }

    /// Insert one missing type expression node at the current cursor position.
    pub(crate) fn insert_missing_type_expression_here(&mut self) -> LocalNodeId<TypeExpression> {
        let start = self.peek_token().start();
        let missing_range = ByteRange { start, end: start };

        self.insert_node(TypeExpression::Missing, missing_range)
    }

    /// Report one unexpected node at the current cursor position.
    pub(crate) fn report_unexpected_here(&mut self, owner: NodeType) {
        let error = ParserError::unexpected(self.peek_token().range()).in_node(owner);

        self.report_error(error);
    }

    /// Report one expected token at the current cursor position.
    pub(crate) fn report_expected_here(&mut self, expected: TokenType, owner: NodeType) {
        let error = ParserError::expected(self.peek_token_span(), expected).in_node(owner);
        self.report_error(error);
    }

    /// Eat one token range or report a missing token at the current position.
    pub(crate) fn eat_token_range_or_recover_missing(
        &mut self,
        expected: TokenType,
        owner: NodeType,
    ) -> ByteRange {
        if self.peek_is(expected) {
            let range = self.peek_token().range();
            self.bump();

            return range;
        }

        // anchor a missing token without claiming the following source token
        let start = self.peek_token().start();
        self.report_expected_here(expected, owner);

        ByteRange { start, end: start }
    }

    /// Report one missing expression and insert the missing node.
    pub(crate) fn recover_missing_expression_here(
        &mut self,
        owner: NodeType,
    ) -> LocalNodeId<Expression> {
        self.report_unexpected_here(owner);
        self.insert_missing_expression_here()
    }

    /// Report one missing type expression and insert the missing node.
    pub(crate) fn recover_missing_type_expression_here(
        &mut self,
        owner: NodeType,
    ) -> LocalNodeId<TypeExpression> {
        self.report_unexpected_here(owner);
        self.insert_missing_type_expression_here()
    }

    /// Recover one malformed tree attribute and preserve its list position.
    pub(crate) fn recover_tree_attribute(
        &mut self,
        start: &ParseStart,
        error: ParserError,
    ) -> LocalNodeId<TreeAttribute> {
        self.skip_damaged_tree_attribute();

        let recovered_range = self.range_since(start);
        let error = error.in_node(NodeType::TreeAttribute);
        self.report_error(error);

        self.insert_node(TreeAttribute::Error, recovered_range)
    }

    /// Recover one malformed tree child and preserve its list position.
    pub(crate) fn recover_tree_child(
        &mut self,
        start: &ParseStart,
        error: ParserError,
    ) -> LocalNodeId<TreeChild> {
        self.skip_damaged_tree_child();

        let recovered_range = self.range_since(start);
        self.insert_tree_error_child(recovered_range, error)
    }

    /// Recover one tree expression container after its opening brace was consumed.
    pub(crate) fn recover_tree_expression_child(
        &mut self,
        start: &ParseStart,
        error: ParserError,
        follow_mode: TokenMode,
    ) -> LocalNodeId<TreeChild> {
        self.skip_damaged_tree_expression(follow_mode);

        let recovered_range = self.range_since(start);
        self.insert_tree_error_child(recovered_range, error)
    }

    /// Insert one already consumed malformed tree child.
    pub(crate) fn insert_tree_error_child(
        &mut self,
        recovered_range: ByteRange,
        error: ParserError,
    ) -> LocalNodeId<TreeChild> {
        let error = error.in_node(NodeType::TreeChild);
        self.report_error(error);

        self.insert_node(TreeChild::Error, recovered_range)
    }

    /// Skip one damaged tree attribute without consuming the next attribute.
    fn skip_damaged_tree_attribute(&mut self) {
        // consume a malformed spread container as one attribute
        if self.peek_is(TokenType::OpenBrace) {
            self.bump_with_mode(TokenMode::Ordinary);
            self.skip_damaged_tree_expression(TokenMode::TreeTag);
            return;
        }

        // consume the malformed attribute head
        if self.peek_is(TokenType::End)
            || self.peek_tree_tag_close()
            || self.peek_tree_literal_close()
        {
            return;
        }
        self.bump_with_mode(TokenMode::TreeTag);

        // leave a following attribute untouched when the value is missing
        if !self.peek_is(TokenType::Assign) {
            return;
        }
        self.bump_with_mode(TokenMode::TreeAttributeValue);
        if self.peek_is(TokenType::Identifier)
            || self.peek_is(TokenType::Divide)
            || self.peek_tree_tag_close()
            || self.peek_is(TokenType::End)
        {
            return;
        }

        // consume one delimited or scalar attribute value
        if self.peek_is(TokenType::OpenBrace) {
            self.bump_with_mode(TokenMode::Ordinary);
            self.skip_damaged_tree_expression(TokenMode::TreeTag);
        } else {
            self.bump_with_mode(TokenMode::TreeTag);
        }
    }

    /// Skip one damaged tree child and resume in child mode.
    fn skip_damaged_tree_child(&mut self) {
        if self.peek_tree_literal_close() {
            return;
        }

        match self.peek_token_type() {
            TokenType::OpenBrace => {
                self.bump_with_mode(TokenMode::Ordinary);
                self.skip_damaged_tree_expression(TokenMode::TreeChild);
            }
            TokenType::LessThan => self.skip_damaged_tree_tag(),
            TokenType::End => {}
            _ => self.bump_with_mode(TokenMode::TreeChild),
        }
    }

    /// Skip one damaged tree expression after its opening brace was consumed.
    fn skip_damaged_tree_expression(&mut self, follow_mode: TokenMode) {
        let mut depth = 1;

        loop {
            let token_type = self.peek_token_type();

            // leave an enclosing closing tag for the tree parser
            if depth == 1 && self.peek_tree_literal_close() {
                return;
            }

            match token_type {
                TokenType::OpenBrace => {
                    depth += 1;
                    self.bump_with_mode(TokenMode::Ordinary);
                }
                TokenType::CloseBrace if depth == 1 => {
                    self.bump_with_mode(follow_mode);
                    return;
                }
                TokenType::CloseBrace => {
                    depth -= 1;
                    self.bump_with_mode(TokenMode::Ordinary);
                }
                TokenType::End => return,
                _ => self.bump_with_mode(TokenMode::Ordinary),
            }
        }
    }

    /// Skip one damaged tree tag head and resume in child mode.
    fn skip_damaged_tree_tag(&mut self) {
        self.bump_with_mode(TokenMode::TreeTag);

        loop {
            // leave an ancestor closing tag for the open tree stack
            if self.peek_tree_literal_close() {
                return;
            }

            if self.peek_tree_tag_close() {
                self.bump_with_mode(TokenMode::TreeChild);
                return;
            }

            if self.peek_is(TokenType::End) {
                return;
            }

            self.bump_with_mode(TokenMode::TreeTag);
        }
    }

    /// Parse one type or recover a missing child at a type boundary.
    pub(crate) fn parse_type_or_recover_missing(
        &mut self,
        position: TypePosition,
        stop: TypeStop,
        owner: NodeType,
    ) -> ParserResult<LocalNodeId<TypeExpression>> {
        if self.peek_type_expression_recovery_boundary() {
            return Ok(self.recover_missing_type_expression_here(owner));
        }

        self.parse_type(position, stop)
    }

    /// Parse one expression or recover a missing child at an expression boundary.
    pub(crate) fn parse_expression_or_recover_missing(
        &mut self,
        position: ExpressionPosition,
        stop: ExpressionStop,
        owner: NodeType,
    ) -> ParserResult<LocalNodeId<Expression>> {
        if self.peek_expression_slot_boundary() {
            return Ok(self.recover_missing_expression_here(owner));
        }

        self.parse_expression(position, stop)
    }

    /// Eat one close token or recover one missing close delimiter.
    pub(crate) fn eat_close_token_or_recover_missing(
        &mut self,
        expected: TokenType,
        owner: NodeType,
    ) -> ParserResult<()> {
        self.eat_close_token_or_recover_missing_with(expected, owner, |parser, token_type| {
            Self::is_close_delimiter_boundary_token(token_type)
                || parser.peek_is_on_new_line()
                    && Self::can_start_recovered_statement_item(token_type)
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

        // allow the caller to define the source boundary it owns
        let token_type = self.peek_token_type();
        let is_recoverable_boundary = is_recoverable_boundary(self, token_type);
        if !is_recoverable_boundary {
            return Err(ParserError::expected(self.peek_token().range(), expected));
        }

        self.report_expected_here(expected, owner);

        Ok(())
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

        self.report_expected_here(expected, owner);
    }

    /// Eat one type close token or recover one missing delimiter at a type boundary.
    pub(crate) fn eat_type_token_or_recover_missing(
        &mut self,
        expected: TokenType,
        owner: NodeType,
    ) -> ParserResult<()> {
        self.eat_close_token_or_recover_missing_with(expected, owner, |parser, token_type| {
            parser.is_type_token_recovery_boundary(token_type)
        })
    }

    /// Return whether type parsing can recover one missing close token here.
    fn is_type_token_recovery_boundary(&self, token_type: TokenType) -> bool {
        Self::is_type_container_boundary_token(token_type)
            || self.peek_recovery_point(RecoveryPoint::TypeExpressionDeclaration)
    }

    /// Recover until the expected token.
    pub fn recover_until(
        &mut self,
        start_range: ByteRange,
        recover: TokenType,
        error: ParserError,
    ) -> ByteRange {
        loop {
            let token = self.peek_token_span();
            let token_type = token.token.ty();

            if token_type == TokenType::End {
                break;
            }

            if token_type == recover {
                return self.report_recovery(start_range, error);
            }

            self.bump();
        }

        self.report_recovery(start_range, error)
    }

    /// Recover within one list item until a separator or terminator boundary.
    pub fn recover_list_item(
        &mut self,
        start_range: ByteRange,
        terminator: TokenType,
        error: ParserError,
    ) -> ByteRange {
        let mut depth = DelimiterDepth::list(terminator);

        loop {
            let token = self.peek_token_span();
            let token_type = token.token.ty();

            // stop at end of input
            if token_type == TokenType::End {
                break;
            }

            // stop before the next item or enclosing source boundary
            if Self::is_list_item_recovery_boundary(start_range, token, terminator, &depth) {
                return self.report_recovery(start_range, error);
            }

            // keep scanning within nested delimiters
            depth.advance(token_type);
            self.bump();
        }

        self.report_recovery(start_range, error)
    }

    /// Return whether one token ends recovery for the current list item.
    fn is_list_item_recovery_boundary(
        start_range: ByteRange,
        token: TokenSpan,
        terminator: TokenType,
        depth: &DelimiterDepth,
    ) -> bool {
        if !depth.is_top_level() {
            return false;
        }

        let token_type = token.token.ty();
        let is_new_line_boundary =
            start_range.start < token.span.start && token.token.is_on_new_line();

        is_new_line_boundary
            || Self::token_matches_terminator(token_type, terminator)
            || Self::is_item_stop_token(token_type)
            || Self::is_statement_stop_token(token_type)
            || Self::is_close_delimiter_token(token_type)
    }

    /// Recover within one statement until a statement boundary.
    pub fn recover_statement(&mut self, start_range: ByteRange, error: ParserError) -> ByteRange {
        loop {
            let token = self.peek_token_span();
            let token_type = token.token.ty();

            if token_type == TokenType::End {
                break;
            }

            let is_new_line_boundary =
                start_range.start < token.span.start && token.token.is_on_new_line();
            let is_boundary = is_new_line_boundary
                || Self::is_statement_stop_token(token_type)
                || token_type == TokenType::CloseBrace;
            if is_boundary {
                return self.report_recovery(start_range, error);
            }

            self.bump();
        }

        self.report_recovery(start_range, error)
    }

    /// Return true when a recovered list item may continue parsing another item.
    pub(crate) fn peek_recovered_list_continuation(&self, terminator: TokenType) -> bool {
        let token_type = self.peek_token_type();
        !Self::token_matches_terminator(token_type, terminator)
            && !Self::is_close_delimiter_token(token_type)
            && !Self::is_statement_stop_token(token_type)
            && token_type != TokenType::End
    }

    /// Return true when one token satisfies one recovery terminator.
    #[inline]
    fn token_matches_terminator(token_type: TokenType, terminator: TokenType) -> bool {
        if terminator == TokenType::GreaterThan {
            return Self::is_type_angle_close_start(token_type);
        }

        token_type == terminator
    }

    /// Recover within a property or member body until a boundary token.
    pub fn recover_body(&mut self, start_range: ByteRange, error: ParserError) -> ByteRange {
        loop {
            let token = self.peek_token_span();
            let token_type = token.token.ty();

            if token_type == TokenType::End {
                break;
            }

            let is_new_line_boundary =
                start_range.start < token.span.start && token.token.is_on_new_line();
            let is_boundary = is_new_line_boundary
                || token_type == TokenType::CloseBrace
                || Self::is_any_stop_token(token_type);
            if is_boundary {
                return self.report_recovery(start_range, error);
            }

            self.bump();
        }

        self.report_recovery(start_range, error)
    }

    /// Report one recovery and return its recovered byte range.
    fn report_recovery(&mut self, start_range: ByteRange, error: ParserError) -> ByteRange {
        let recovered_range = self.recovery_range(start_range);
        self.report_error(error);

        recovered_range
    }

    /// Return the recovered byte range ending at the previous token.
    #[inline]
    fn recovery_range(&self, start_range: ByteRange) -> ByteRange {
        let end = self.peek_previous_token_end().max(start_range.start);

        ByteRange {
            start: start_range.start,
            end,
        }
    }

    /// Eat the expected token, skipping damaged input before a bail token.
    pub fn eat_token_before(&mut self, expected: TokenType, bail: TokenType) -> ParserResult<()> {
        if self.peek_is(expected) {
            self.bump();
            return Ok(());
        }

        let start = self.mark_parse_start();
        loop {
            let token_type = self.peek_token_type();
            if matches!(token_type, TokenType::End) || token_type == bail {
                break;
            }

            if token_type == expected {
                let error = ParserError::unexpected(self.range_since(&start));
                self.bump();
                self.report_error(error);
                return Ok(());
            }

            self.bump();
        }

        let error = ParserError::unexpected(self.range_since(&start));
        self.report_error(error);

        Err(error)
    }
}
