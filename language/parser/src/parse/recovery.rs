use crate::parse::lookahead::DelimiterDepth;
use crate::parse::{
    DECLARATION_START_TOKENS, ExpressionPosition, ExpressionStop, TokenMode, TypePosition, TypeStop,
};
use crate::{ParseStart, Parser, ParserError, ParserResult, TokenProbe};
use tspp_dir::{
    Expression, Keyword, LocalNodeId, NodeType, TokenSpan, TokenType, TreeAttribute, TreeChild,
    TypeExpression,
};
use tspp_source::ByteRange;

/// The declaration form allowed to remain inside the current parser container.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DeclarationNesting {
    /// No declaration may remain nested.
    None,
    /// An unexported declaration may remain as an expression.
    Expression,
    /// An associated declaration may remain as a member.
    Member,
}

impl DeclarationNesting {
    /// Return whether a line-leading token probe starts a declaration boundary.
    fn matches(self, probe: &mut TokenProbe<'_>) -> bool {
        // scan declaration prefixes without consuming parser state
        let Some(is_exported) = probe.scan_declaration_prefixes() else {
            return false;
        };
        if self == Self::Expression && !is_exported {
            return false;
        }

        // recognize ambient global blocks
        if probe.peek_identifier_is("global") {
            probe.bump();

            return probe.peek_token_type() == TokenType::OpenBrace;
        }

        let Some(keyword) = probe.peek_keyword() else {
            return false;
        };
        if !self.is_boundary_keyword(keyword) {
            return false;
        }

        // classify the declaration noun and its required continuation
        probe.bump();
        match keyword {
            Keyword::Async => {
                let is_function = probe.peek_keyword() == Some(Keyword::Function)
                    && !probe.peek_token().is_on_new_line();
                if !is_function {
                    return false;
                }
                probe.bump();

                Self::scan_function_name(probe)
            }
            Keyword::Function => Self::scan_function_name(probe),
            Keyword::Struct | Keyword::Enum | Keyword::Interface => matches!(
                probe.peek_token_type(),
                TokenType::Identifier | TokenType::Literal | TokenType::OpenBrace
            ),
            Keyword::Class => {
                matches!(
                    probe.peek_token_type(),
                    TokenType::Identifier | TokenType::Literal | TokenType::OpenBrace
                ) || probe.peek_keyword() == Some(Keyword::Extends)
            }
            Keyword::Extension => {
                probe.peek_keyword() == Some(Keyword::Of)
                    || matches!(
                        probe.peek_token_type(),
                        TokenType::Identifier | TokenType::LessThan | TokenType::ShiftLeft
                    )
            }
            Keyword::Newtype => {
                probe.peek_keyword() == Some(Keyword::Interface)
                    || probe.peek_token_type() == TokenType::Identifier
            }
            Keyword::Type | Keyword::Readonly => probe.peek_token_type() == TokenType::Identifier,
            Keyword::Const => {
                probe.peek_keyword() == Some(Keyword::Function)
                    || DECLARATION_START_TOKENS.contains(&probe.peek_token_type())
            }
            Keyword::Let | Keyword::Using => {
                DECLARATION_START_TOKENS.contains(&probe.peek_token_type())
            }
            _ => false,
        }
    }

    /// Advance past one function name.
    fn scan_function_name(probe: &mut TokenProbe<'_>) -> bool {
        if probe.peek_token_type() == TokenType::Multiply {
            probe.bump();
        }

        probe.peek_token_type() == TokenType::Identifier
    }

    /// Return whether one keyword starts a declaration boundary under this nesting.
    fn is_boundary_keyword(self, keyword: Keyword) -> bool {
        match self {
            Self::None | Self::Expression => {
                Self::is_member_boundary_keyword(keyword)
                    || matches!(keyword, Keyword::Type | Keyword::Readonly | Keyword::Const)
            }
            Self::Member => Self::is_member_boundary_keyword(keyword),
        }
    }

    /// Return whether one keyword starts a declaration outside a member list.
    fn is_member_boundary_keyword(keyword: Keyword) -> bool {
        matches!(
            keyword,
            Keyword::Struct
                | Keyword::Class
                | Keyword::Enum
                | Keyword::Function
                | Keyword::Async
                | Keyword::Extension
                | Keyword::Interface
                | Keyword::Newtype
                | Keyword::Let
                | Keyword::Using
        )
    }
}

impl Parser {
    /// Return whether the current semicolon precedes a declaration boundary.
    pub(crate) fn peek_semicolon_declaration_boundary(&self, nesting: DeclarationNesting) -> bool {
        let mut probe = self.cursor.probe(&self.file);
        if probe.peek_token_type() != TokenType::Semicolon {
            return false;
        }
        probe.bump();
        if !probe.peek_token().is_on_new_line() {
            return false;
        }

        nesting.matches(&mut probe)
    }

    /// Return whether the current token starts a declaration boundary.
    pub(crate) fn peek_declaration_boundary(&self, nesting: DeclarationNesting) -> bool {
        if !self.peek_is_on_new_line() {
            return false;
        }

        let mut probe = self.cursor.probe(&self.file);

        nesting.matches(&mut probe)
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
            || self.peek_declaration_boundary(DeclarationNesting::None)
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
            let is_declaration_boundary = self.peek_declaration_boundary(DeclarationNesting::None);
            let is_boundary = is_declaration_boundary
                || is_new_line_boundary
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
