use crate::{CommentRetention, ParserError, ParserResult, PatternMarker};
use core::fmt::{self, Debug};
use std::sync::Arc;
use tspp_core::{LocalStringPool, StringId, ensure_sufficient_stack};
use tspp_dir::{
    BlockContext, BlockForm, Comment, Expression, Keyword, LocalNodeId, Node, NodeType, Token,
    TokenLiteral, TokenSpan, TokenType, Tree, TreeCapacity, TreeStore,
};
use tspp_source::{
    ByteRange, Diagnostic, DiagnosticCollection, File, FileId, LanguageType, NodeSpanBoundary,
    NodeSpanRegion, NodeSpanType, Span,
};

use super::cursor::TokenCursor;
use super::{FunctionKeywords, TokenMode};

/// Recursive descents between nested stack checks.
const STACK_CHECK_INTERVAL: u16 = 8;
/// Maximum nested recursive parser descent before reporting malformed input.
const MAX_RECURSIVE_DESCENT_DEPTH: u16 = 2048;

/// The form of one parsed source file.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SourceForm {
    /// A TS++ program.
    #[default]
    Program,
    /// A structural pattern.
    Pattern,
}

/// Configuration for parsing one source file.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ParseOptions {
    /// The parsed source form.
    pub form: SourceForm,
    /// The source comments to retain.
    pub comment_retention: CommentRetention,
}

/// The completed parse of one source file.
pub struct Parse {
    /// The parsed source file.
    pub file: Arc<File>,
    /// The parsed root expressions.
    pub roots: Vec<LocalNodeId<Expression>>,
    /// The completed DIR tree.
    pub tree: Tree,

    /// The strings interned while parsing.
    pub strings: LocalStringPool,
    /// The semantic tokens.
    pub tokens: Vec<Token>,
    /// The retained source comments.
    pub comments: Vec<Comment>,

    /// The distinct parser errors.
    pub errors: Vec<ParserError>,
}

impl Debug for Parse {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Parse")
            .field("file", &self.file.path)
            .field("roots", &self.roots.len())
            .field("tokens", &self.tokens.len())
            .field("comments", &self.comments.len())
            .field("errors", &self.errors.len())
            .finish()
    }
}

impl Parse {
    /// Take the semantic tokens as source spans.
    pub fn take_token_spans(&mut self) -> Vec<TokenSpan> {
        // attach the parsed file to every source-local token
        let file_id = self.file.id;

        std::mem::take(&mut self.tokens)
            .into_iter()
            .map(|token| TokenSpan::new(token, file_id))
            .collect()
    }

    /// Create source diagnostics from parser errors.
    pub fn diagnostics(&self) -> DiagnosticCollection {
        diagnostics(&self.file, &self.errors)
    }
}

/// An indexed parser for one source file.
pub struct Parser {
    /// The source file.
    pub file: Arc<File>,
    /// The source file ID.
    pub file_id: FileId,
    /// The forward token cursor.
    pub(super) cursor: TokenCursor,
    /// The next retained comment to inspect for documentation.
    pub(super) next_documentation_comment: usize,
    /// The current nested recursive descent depth.
    recursive_descent_depth: u16,
    /// The parsed source form.
    form: SourceForm,
    /// The active interpretation of function-sensitive keywords.
    pub(super) keywords: FunctionKeywords,

    /// The DIR tree.
    pub tree: Tree,
    /// The strings interned locally during this parse.
    pub strings: LocalStringPool,
    /// Whether the source is an ambient declaration file.
    pub(crate) is_ambient: bool,
    /// The distinct parser errors encountered so far.
    pub errors: Vec<ParserError>,
}

impl Debug for Parser {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Parser")
    }
}

impl Parser {
    /// Return one parser token as a full token span.
    #[inline(always)]
    fn token_span(&self, token: Token) -> TokenSpan {
        TokenSpan::new(token, self.file_id)
    }

    /// Return true when the current token starts after a line break.
    #[inline]
    pub(crate) fn peek_is_on_new_line(&self) -> bool {
        self.cursor.peek().is_on_new_line()
    }

    /// Return true when one relative token starts after a line break.
    #[inline]
    pub(crate) fn peek_token_at_is_on_new_line(&self, offset: usize) -> bool {
        self.peek_token_at(offset).is_on_new_line()
    }

    /// Return true when comments appear between the previous token and current token.
    pub(crate) fn peek_token_has_leading_comment(&self) -> bool {
        let previous_end = self.cursor.previous_end();
        let current_start = self.cursor.peek().start();

        self.contains_comment_in_source_range(previous_end, current_start)
    }

    /// Return true when comments appear before one peeked token.
    pub(crate) fn contains_comment_before_token(
        &self,
        previous_end: u32,
        token: TokenSpan,
    ) -> bool {
        self.contains_comment_in_source_range(previous_end, token.span.start)
    }

    /// Return true when one source range contains a line or block comment.
    fn contains_comment_in_source_range(&self, start: u32, end: u32) -> bool {
        let bytes = self.file.text().as_bytes();
        let mut offset = start as usize;
        let end = end as usize;

        // scan trivia bytes between visible tokens
        while offset + 1 < end {
            if bytes[offset] == b'/' && matches!(bytes[offset + 1], b'/' | b'*') {
                return true;
            }

            offset += 1;
        }

        false
    }

    /// Run one parser descent under the shared recursion limit.
    #[inline(always)]
    pub(crate) fn with_recursive_descent<T>(
        &mut self,
        owner: NodeType,
        parse: impl FnOnce(&mut Self) -> ParserResult<T>,
    ) -> ParserResult<T> {
        // bound malformed recursive input
        if self.recursive_descent_depth >= MAX_RECURSIVE_DESCENT_DEPTH {
            return Err(ParserError::unexpected(self.peek_token_span()).in_node(owner));
        }

        // periodically allow nested calls to grow onto another stack
        self.recursive_descent_depth += 1;
        let is_stack_check = self
            .recursive_descent_depth
            .is_multiple_of(STACK_CHECK_INTERVAL);
        let result = if is_stack_check {
            ensure_sufficient_stack(|| parse(self))
        } else {
            parse(self)
        };

        // restore the caller depth
        self.recursive_descent_depth -= 1;

        result
    }

    /// Create a parser for one source file and DIR tree.
    pub fn new(
        file: Arc<File>,
        language_type: LanguageType,
        mut tree: Tree,
        options: ParseOptions,
    ) -> Self {
        // lex the source and reserve the expected node storage
        let cursor = TokenCursor::new(file.clone(), options.comment_retention);
        tree.reserve(TreeCapacity::nodes(cursor.token_count()));

        // initialize source-local parser state
        let file_id = file.id;
        tree.begin_source_file(file_id);
        Self {
            file,
            file_id,
            cursor,
            next_documentation_comment: 0,
            recursive_descent_depth: 0,
            form: options.form,
            keywords: FunctionKeywords::default(),
            is_ambient: language_type.is_declaration(),
            tree,
            strings: LocalStringPool::new(),
            errors: Vec::new(),
        }
    }

    /// Parse with one keyword interpretation and restore the enclosing interpretation.
    pub(crate) fn with_keywords<T>(
        &mut self,
        keywords: FunctionKeywords,
        parse: impl FnOnce(&mut Self) -> ParserResult<T>,
    ) -> ParserResult<T> {
        let enclosing = std::mem::replace(&mut self.keywords, keywords);
        let result = parse(self);

        self.keywords = enclosing;

        result
    }

    /// Return consumed semantic tokens for parser tests.
    #[cfg(test)]
    #[inline]
    pub(crate) fn consumed_tokens(&self) -> Vec<TokenSpan> {
        self.cursor
            .consumed()
            .iter()
            .copied()
            .map(|token| TokenSpan::new(token, self.file_id))
            .collect()
    }

    /// Eat a tree opening `<`.
    #[inline]
    pub(crate) fn eat_tree_opening_angle(&mut self) -> ParserResult<()> {
        if !self.peek_is(TokenType::LessThan) {
            return Err(ParserError::expected(
                self.peek_token().range(),
                TokenType::LessThan,
            ));
        }

        self.bump_with_mode(TokenMode::TreeTag);

        Ok(())
    }

    /// Bump the current token and read the next one in a contextual lexer mode.
    #[inline]
    pub(crate) fn bump_with_mode(&mut self, mode: TokenMode) {
        self.cursor.bump_with_mode(&self.file, mode);
    }

    /// Re-lex the current token as a generic `<`.
    #[inline]
    pub(crate) fn re_lex_generic_angle_open(&mut self) -> bool {
        let token_type = self.cursor.peek().ty();
        if token_type == TokenType::LessThan {
            return true;
        }

        if !matches!(
            token_type,
            TokenType::ShiftLeft | TokenType::LessThanOrEqual | TokenType::ShiftLeftAssign
        ) {
            return false;
        }

        self.cursor.split(TokenType::LessThan, 1)
    }

    /// Eat one reference prefix operator.
    #[inline]
    pub(crate) fn eat_reference_prefix_operator(&mut self) -> ParserResult<TokenSpan> {
        if !self.re_lex_reference_prefix_operator() {
            return Err(ParserError::unexpected(self.peek_token_span()));
        }

        let token = self.peek_token_span();
        self.bump();

        Ok(token)
    }

    /// Re-lex the current token as one reference prefix operator.
    #[inline]
    fn re_lex_reference_prefix_operator(&mut self) -> bool {
        let token_type = self.cursor.peek().ty();
        if matches!(
            token_type,
            TokenType::ElementwiseAnd | TokenType::ElementwiseXor
        ) {
            return true;
        }

        if token_type != TokenType::LogicalAnd {
            return false;
        }

        self.cursor.split(TokenType::ElementwiseAnd, 1)
    }

    /// Eat one angle-close token with optional contextual follow mode.
    #[inline]
    fn eat_angle_close(&mut self, follow_mode: Option<TokenMode>) -> ParserResult<()> {
        let token_type = self.cursor.peek().ty();
        if token_type == TokenType::GreaterThan {
            self.bump_after_angle_close(follow_mode);

            return Ok(());
        }

        if !matches!(
            token_type,
            TokenType::ShiftRight
                | TokenType::UnsignedShiftRight
                | TokenType::GreaterThanOrEqual
                | TokenType::ShiftRightAssign
                | TokenType::UnsignedShiftRightAssign
        ) {
            return Err(ParserError::unexpected(self.peek_token_span()));
        }

        if !self.cursor.split(TokenType::GreaterThan, 1) {
            return Err(ParserError::unexpected(self.peek_token_span()));
        }

        self.bump_after_angle_close(follow_mode);

        Ok(())
    }

    /// Advance after consuming one angle-close token.
    #[inline]
    fn bump_after_angle_close(&mut self, follow_mode: Option<TokenMode>) {
        if let Some(follow_mode) = follow_mode {
            self.bump_with_mode(follow_mode);
        } else {
            self.bump();
        }
    }

    /// Re-lex the current `/` or `/=` token as a regex literal.
    #[inline]
    pub(crate) fn re_lex_regex(&mut self) -> bool {
        let current = self.cursor.peek();
        let token_type = current.ty();
        if token_type == TokenType::Literal
            && matches!(current.literal(), Some(TokenLiteral::RegexString { .. }))
        {
            return true;
        }

        if !matches!(token_type, TokenType::Divide | TokenType::DivideAssign) {
            return false;
        }

        self.cursor.reclassify_regex(self.file.text());

        true
    }

    /// Eat one typed angle-close token.
    #[inline]
    pub(crate) fn eat_type_angle_close(&mut self) -> ParserResult<()> {
        self.eat_angle_close(None)
    }

    /// Eat one expression-position typed angle-close token.
    #[inline]
    pub(crate) fn eat_expression_type_angle_close(&mut self) -> ParserResult<()> {
        if !Self::is_expression_type_angle_close_start(self.peek_token_type()) {
            return Err(ParserError::unexpected(self.peek_token_span()));
        }

        self.eat_type_angle_close()
    }

    /// Return true when one token can begin a type-angle close sequence.
    #[inline]
    pub(crate) const fn is_type_angle_close_start(token_type: TokenType) -> bool {
        matches!(
            token_type,
            TokenType::GreaterThan
                | TokenType::ShiftRight
                | TokenType::UnsignedShiftRight
                | TokenType::GreaterThanOrEqual
                | TokenType::ShiftRightAssign
                | TokenType::UnsignedShiftRightAssign
        )
    }

    /// Return true when one token can begin an expression-position type-angle close.
    #[inline]
    pub(crate) const fn is_expression_type_angle_close_start(token_type: TokenType) -> bool {
        matches!(
            token_type,
            TokenType::GreaterThan | TokenType::ShiftRight | TokenType::UnsignedShiftRight
        )
    }

    /// Return true when the current token can begin a type-angle close sequence.
    #[inline]
    pub(crate) fn peek_type_angle_close(&self) -> bool {
        Self::is_type_angle_close_start(self.peek_token_type())
    }

    /// Return true when the current token can begin an expression-position type-angle close.
    #[inline]
    pub(crate) fn peek_expression_type_angle_close(&self) -> bool {
        Self::is_expression_type_angle_close_start(self.peek_token_type())
    }

    /// Return true when the current token can begin one `>` in tree tag syntax.
    #[inline]
    pub(crate) fn peek_tree_tag_close(&self) -> bool {
        Self::is_type_angle_close_start(self.peek_token_type())
    }

    /// Return owned semantic tokens after lexing to EOF.
    pub fn take_tokens(&mut self) -> Vec<Token> {
        self.cursor.take_tokens()
    }

    /// Return owned semantic token spans after lexing to EOF.
    pub fn take_token_spans(&mut self) -> Vec<TokenSpan> {
        let file_id = self.file_id;
        self.take_tokens()
            .into_iter()
            .map(|token| TokenSpan::new(token, file_id))
            .collect()
    }

    /// Return one parser-visible token relative to the current token.
    #[inline(always)]
    pub(crate) fn peek_token_at(&self, offset: usize) -> Token {
        self.cursor.peek_token_at(offset)
    }

    /// Return one parser-visible token type relative to the current token.
    #[inline(always)]
    pub(crate) fn peek_token_type_at(&self, offset: usize) -> TokenType {
        self.peek_token_at(offset).ty()
    }

    /// Return one parser-visible token as a keyword.
    #[inline]
    pub(crate) fn token_keyword(&self, token: Token) -> Option<Keyword> {
        self.cursor.classify_keyword(&self.file, token)
    }

    /// Return one parser-visible token as a keyword relative to the current token.
    #[inline]
    pub(crate) fn peek_keyword_at(&self, offset: usize) -> Option<Keyword> {
        let token = self.peek_token_at(offset);

        self.token_keyword(token)
    }

    /// Return the next parser-visible token.
    #[inline(always)]
    pub(crate) fn peek_next_token(&self) -> Token {
        self.peek_token_at(1)
    }

    /// Return the next parser-visible token type.
    #[inline(always)]
    pub(crate) fn peek_next_token_type(&self) -> TokenType {
        self.peek_token_type_at(1)
    }

    /// Return the next parser-visible token as a keyword.
    #[inline(always)]
    pub(crate) fn peek_next_keyword(&self) -> Option<Keyword> {
        self.peek_keyword_at(1)
    }

    /// Return whether the current tokens start a tree closing tag.
    #[inline]
    pub(crate) fn peek_tree_literal_close(&self) -> bool {
        self.peek_is(TokenType::LessThan) && self.peek_next_token_type() == TokenType::Divide
    }

    /// Parse root expressions as an implicit namespace.
    ///
    /// Examples:
    /// ```tspp
    /// const answer = 42;
    /// export function read() -> int { return answer; }
    /// ```
    pub fn parse_roots(&mut self) -> Vec<LocalNodeId<Expression>> {
        let start = self.mark_parse_start();
        let expressions = self.parse_block_body(BlockForm::Implicit, BlockContext::Statement);

        match expressions {
            Ok(expressions) => expressions,
            Err(error) => {
                let range = self.range_since(&start);
                self.recover_until(range, TokenType::End, error);

                Vec::new()
            }
        }
    }

    /// Parse the complete source file.
    pub fn parse(mut self) -> Parse {
        // parse the implicit file namespace
        let roots = self.parse_roots();

        // finish retained source metadata
        self.finalize_comments();
        let tokens = self.cursor.take_tokens();
        let comments = self.cursor.take_comments();

        Parse {
            file: self.file,
            roots,
            tree: self.tree,

            strings: self.strings,
            tokens,
            comments,

            errors: self.errors,
        }
    }

    /// Parse the complete source file while retaining parser state for tests.
    #[cfg(test)]
    pub(crate) fn parse_in_place(&mut self) -> Vec<LocalNodeId<Expression>> {
        let roots = self.parse_roots();
        self.finalize_comments();

        roots
    }

    /// Finalize retained comments after contextual tokenization.
    pub fn finalize_comments(&mut self) {
        self.cursor.finalize_comments();
        self.finalize_documentation();
    }

    /// Return retained source comments.
    pub fn comments(&self) -> &[Comment] {
        self.cursor.comments()
    }

    /// Take retained source comments.
    pub fn take_comments(&mut self) -> Vec<Comment> {
        self.finalize_comments();

        self.cursor.take_comments()
    }

    /// Report one parser error unless an identical error was already reported.
    #[cold]
    #[inline(never)]
    pub(crate) fn report_error(&mut self, error: ParserError) {
        if !self.errors.contains(&error) {
            self.errors.push(error);
        }
    }

    /// Create source diagnostics from parser errors.
    pub fn diagnostics(&self) -> DiagnosticCollection {
        diagnostics(&self.file, &self.errors)
    }

    /// Create one source diagnostic from one parser error.
    pub fn diagnostic(&self, error: &ParserError) -> Diagnostic {
        let blob = self.file.blob();

        error.to_diagnostic(blob, self.file_id)
    }

    /// Mark the current token range as the start of a parse operation.
    #[inline(always)]
    pub fn mark_parse_start(&self) -> ParseStart {
        ParseStart {
            range: self.cursor.peek().range(),
        }
    }

    /// Insert a node into the DIR tree.
    #[inline]
    pub(crate) fn insert_node<T>(&mut self, node: T, range: ByteRange) -> LocalNodeId<T>
    where
        T: Node,
        Tree: TreeStore<T>,
    {
        self.tree.insert_parsed(node, range)
    }

    /// Record one node's defining keyword as its main and keyword ranges.
    pub(crate) fn set_node_keyword_range<T>(&mut self, node_id: LocalNodeId<T>, range: ByteRange)
    where
        T: Node,
    {
        self.tree.set_main_range(node_id, range);
        self.tree.set_side_range(
            node_id,
            NodeSpanType::Region(NodeSpanRegion::Keyword),
            range,
        );
    }

    /// Attach one child-owned leading boundary range.
    pub(crate) fn set_node_leading_range<T>(&mut self, node_id: LocalNodeId<T>, boundary_start: u32)
    where
        T: Node,
    {
        let node_range = self.tree.get_range(node_id);
        if boundary_start >= node_range.start {
            return;
        }

        let leading_range = ByteRange {
            start: boundary_start,
            end: node_range.start,
        };
        self.tree.set_side_range(
            node_id,
            NodeSpanType::Boundary(NodeSpanBoundary::Leading),
            leading_range,
        );
    }

    /// Attach one child-owned trailing boundary range.
    pub(crate) fn set_node_trailing_range<T>(&mut self, node_id: LocalNodeId<T>, boundary_end: u32)
    where
        T: Node,
    {
        let node_range = self.tree.get_range(node_id);
        if boundary_end <= node_range.end {
            return;
        }

        let trailing_range = ByteRange {
            start: node_range.end,
            end: boundary_end,
        };
        self.tree.set_side_range(
            node_id,
            NodeSpanType::Boundary(NodeSpanBoundary::Trailing),
            trailing_range,
        );
    }

    /// Extend one file-local source region owned by a node.
    pub(crate) fn extend_node_region_range<T>(
        &mut self,
        node_id: LocalNodeId<T>,
        region: NodeSpanRegion,
        range: ByteRange,
    ) where
        T: Node,
    {
        let node_id = node_id.id;
        let span_type = NodeSpanType::Region(region);
        let range = self
            .tree
            .get_side_range_by_id(node_id, span_type)
            .map_or(range, |existing| ByteRange {
                start: existing.start.min(range.start),
                end: existing.end.max(range.end),
            });

        self.tree.set_side_range_by_id(node_id, span_type, range);
    }

    /// Record written parentheses around one canonical node.
    pub(crate) fn record_parentheses<T>(&mut self, start: &ParseStart, node_id: LocalNodeId<T>)
    where
        T: Node,
    {
        let node_range = self.tree.get_range(node_id);
        let leading_range = ByteRange {
            start: start.token_end(),
            end: node_range.start,
        };
        if leading_range.start < leading_range.end {
            self.tree.set_side_range(
                node_id,
                NodeSpanType::Boundary(NodeSpanBoundary::Leading),
                leading_range,
            );
        }

        self.extend_node_region_range(
            node_id,
            NodeSpanRegion::Parentheses,
            self.range_since(start),
        );
    }

    /// Return the file-local byte range since one parse start.
    #[inline(always)]
    pub fn range_since(&self, start: &ParseStart) -> ByteRange {
        let start = start.range.start;
        let end = self.cursor.previous_end().max(start);
        ByteRange { start, end }
    }

    /// Return the source text backing one span.
    #[inline]
    pub fn span_str(&self, span: Span) -> &str {
        debug_assert_eq!(span.file, self.file_id, "span must belong to parser file");

        &self.file.text()[span.start as usize..span.end as usize]
    }

    /// Intern the source text backing one file-local byte range.
    #[inline]
    pub(crate) fn intern_range(&mut self, range: ByteRange) -> StringId {
        let text = &self.file.text()[range.start as usize..range.end as usize];

        self.strings.intern(text)
    }

    /// Return the source text backing one file-local byte range.
    #[inline]
    pub fn range_str(&self, range: ByteRange) -> &str {
        &self.file.text()[range.start as usize..range.end as usize]
    }

    /// Return the source text backing one token span.
    #[inline]
    pub fn token_span_str(&self, token: TokenSpan) -> &str {
        self.span_str(token.span)
    }

    /// Return the source text backing one token.
    #[inline]
    pub fn token_str(&self, token: Token) -> &str {
        self.range_str(token.range())
    }

    /// Return the source text backing the current token.
    #[inline]
    pub(crate) fn peek_token_str(&self) -> &str {
        self.token_str(self.cursor.peek())
    }

    /// Return the current token.
    #[inline]
    pub(crate) const fn peek_token(&self) -> Token {
        self.cursor.peek()
    }

    /// Return the current token as a keyword.
    #[inline]
    pub(crate) fn peek_keyword(&self) -> Option<Keyword> {
        self.peek_keyword_at(0)
    }

    /// Return the previous token.
    #[inline]
    pub fn peek_previous_token(&self) -> Option<TokenSpan> {
        (self.cursor.previous_end() > 0).then(|| self.token_span(self.cursor.peek_previous()))
    }

    /// Return the end offset of the previously consumed semantic token.
    #[inline]
    pub(crate) fn peek_previous_token_end(&self) -> u32 {
        self.cursor.previous_end()
    }

    /// Return the current token.
    #[inline]
    pub fn peek_token_span(&self) -> TokenSpan {
        self.token_span(self.cursor.peek())
    }

    /// Return the current token type.
    #[inline]
    pub fn peek_token_type(&self) -> TokenType {
        self.cursor.peek().ty()
    }

    /// Return whether the current token is a repeated Pattern placeholder.
    #[inline]
    pub(crate) fn peek_repeated_pattern_marker(&self) -> bool {
        if self.form != SourceForm::Pattern {
            return false;
        }

        matches!(
            PatternMarker::parse(self.peek_token_str()),
            Some(PatternMarker::Nodes { .. })
        )
    }

    /// Return whether the current token matches the given type.
    #[inline]
    pub fn peek_is(&self, token_type: TokenType) -> bool {
        debug_assert!(
            token_type.is_semantic(),
            "peek_is requires semantic token type"
        );

        self.cursor.peek().is(token_type)
    }

    /// Return true when more tokens remain before EOF.
    #[inline]
    pub fn has_more_tokens(&self) -> bool {
        self.peek_token_type() != TokenType::End
    }

    /// Eat and return the current token.
    #[inline]
    pub fn eat(&mut self) -> TokenSpan {
        let token = self.token_span(self.cursor.peek());
        self.bump();

        token
    }

    /// Advance to the next token.
    #[inline]
    pub fn bump(&mut self) {
        self.cursor.bump();
    }

    /// Require the current token to have one type.
    #[inline]
    pub fn require_token(&self, token_type: TokenType) -> ParserResult<Token> {
        debug_assert!(
            token_type.is_semantic(),
            "require_token requires semantic token type"
        );
        let current = self.peek_token();
        if current.is(token_type) {
            Ok(current)
        } else {
            Err(ParserError::expected(current, token_type))
        }
    }

    /// Eat the current token and require one type.
    #[inline]
    pub fn eat_token(&mut self, token_type: TokenType) -> ParserResult<Token> {
        debug_assert!(
            token_type.is_semantic(),
            "eat_token requires semantic token type"
        );
        let current = self.require_token(token_type)?;
        self.bump();

        Ok(current)
    }

    /// Eat the current token when it has the requested type.
    #[inline]
    pub fn eat_token_if(&mut self, token_type: TokenType) -> bool {
        if self.peek_is(token_type) {
            self.bump();

            true
        } else {
            false
        }
    }

    /// Eat one tree tag close token and advance in the requested mode.
    #[inline]
    pub(crate) fn eat_tree_tag_close(&mut self, follow_mode: TokenMode) -> ParserResult<()> {
        self.eat_angle_close(Some(follow_mode))
    }
}

/// One token range recorded at the start of a parse operation.
#[derive(Debug, Copy, Clone)]
pub struct ParseStart {
    /// The current token range at parse start time.
    range: ByteRange,
}

impl ParseStart {
    /// Return the end of the token that started this parse operation.
    #[inline]
    pub(crate) fn token_end(&self) -> u32 {
        self.range.end
    }
}

/// Create source diagnostics from parser errors.
fn diagnostics(file: &File, errors: &[ParserError]) -> DiagnosticCollection {
    // skip diagnostic allocation for successful parses
    if errors.is_empty() {
        return DiagnosticCollection::new();
    }

    // convert every parser failure against the authored source
    let blob = file.blob();
    let diagnostics = errors
        .iter()
        .map(|error| error.to_diagnostic(blob, file.id))
        .collect();

    DiagnosticCollection::from_diagnostics(diagnostics)
}
