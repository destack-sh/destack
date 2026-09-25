use super::context::TsppFormatContext;
use tspp_dir::{
    Comment, Expression, Keyword, LocalNodeId, Node, TokenSpan, TokenType, Tree, TreeStore,
};
use tspp_source::Span;

impl<'a> TsppFormatContext<'a> {
    /// Return one expression's complete statement source extent.
    pub fn expression_statement_extent(&self, node_id: LocalNodeId<Expression>) -> Span {
        let mut expression_id = node_id;
        let mut span = self.tree.get_source_extent(expression_id);

        // include source wrappers owned by a postfix expression head
        while let Expression::Member { left, .. }
        | Expression::Index { left, .. }
        | Expression::Instantiation { left, .. }
        | Expression::Call { left, .. }
        | Expression::Maybe { left, .. }
        | Expression::Must { left, .. } = self.tree.get(expression_id)
        {
            span = span.merge(self.tree.get_source_extent(*left));
            expression_id = *left;
        }

        let mut start = span.start;

        // include one leading statement protection semicolon
        start = self
            .previous_token_before_span(Span::new(span.file, start, span.end))
            .filter(|token| token.token.is(TokenType::Semicolon) && token.token.is_on_new_line())
            .map_or(start, |token| token.span.start);

        // include a terminator unless it protects following syntax on the same line
        let Some(semicolon) = self
            .next_token_after_span(span)
            .filter(|token| token.token.is(TokenType::Semicolon))
        else {
            return Span::new(span.file, start, span.end);
        };
        let next_token = self.next_token_after_span(semicolon.span);
        let protects_following_statement = semicolon.token.is_on_new_line()
            && next_token.is_some_and(|token| !token.token.is_on_new_line());
        if protects_following_statement {
            return Span::new(span.file, start, span.end);
        }

        Span::new(span.file, start, semicolon.span.end)
    }

    /// Return the first token start for one node.
    pub fn node_token_start<T>(&self, node_id: LocalNodeId<T>) -> u32
    where
        T: Node + Clone,
        Tree: TreeStore<T>,
    {
        let node_span = self.span(node_id);

        self.first_token_in_span(node_span)
            .map_or(node_span.start, |token| token.span.start)
    }

    /// Return the last token end for one node.
    pub fn node_token_end<T>(&self, node_id: LocalNodeId<T>) -> u32
    where
        T: Node + Clone,
        Tree: TreeStore<T>,
    {
        let node_span = self.span(node_id);

        self.last_token_in_span(node_span)
            .map_or(node_span.end, |token| token.span.end)
    }

    /// Return the first token start for one expression.
    pub fn expression_token_start(&self, expression_id: LocalNodeId<Expression>) -> u32 {
        self.tree
            .get_head_span(expression_id)
            .map_or_else(|| self.node_token_start(expression_id), |span| span.start)
    }

    /// Return the token immediately before one token that starts at the given offset.
    pub fn token_before_token_start(&self, token_start: u32) -> Option<TokenSpan> {
        let tokens = self.tokens;
        let token_index = tokens.partition_point(|token| token.span.start < token_start);
        let token = tokens.get(token_index).copied()?;

        if token.span.start != token_start {
            return None;
        }

        token_index
            .checked_sub(1)
            .and_then(|previous_index| tokens.get(previous_index).copied())
    }

    /// Return the first token between two offsets.
    pub fn first_token_between(&self, start: u32, end: u32) -> Option<TokenSpan> {
        if end <= start {
            return None;
        }

        let token_index = self
            .tokens
            .partition_point(|token| token.span.start < start);
        let token = self.tokens.get(token_index).copied()?;

        (token.span.start < end).then_some(token)
    }

    /// Return the start offset of the nth token of one type inside one span.
    pub fn nth_token_type_start_in_span(
        &self,
        span: Span,
        token_type: TokenType,
        nth: usize,
    ) -> Option<u32> {
        if nth == 0 || span.start >= span.end {
            return None;
        }

        let mut token_index = self
            .tokens
            .partition_point(|token| token.span.start < span.start);
        let mut seen = 0usize;

        while let Some(token) = self.tokens.get(token_index).copied() {
            if token.span.start >= span.end {
                return None;
            }

            if token.token.ty() == token_type {
                seen += 1;

                if seen == nth {
                    return Some(token.span.start);
                }
            }

            token_index += 1;
        }

        None
    }

    /// Return tokens that intersect one span.
    pub fn tokens_in_span(&self, span: Span) -> &[TokenSpan] {
        if span.start >= span.end {
            return &[];
        }

        let start_index = self
            .tokens
            .partition_point(|token| token.span.end <= span.start);
        let remaining = &self.tokens[start_index..];
        let end_index = remaining.partition_point(|token| token.span.start < span.end);

        &remaining[..end_index]
    }

    /// Return source comments that intersect one span.
    pub fn source_comments_intersecting_span(&self, span: Span) -> &[Comment] {
        let comments = self.source_comments();
        let start_index = comments.partition_point(|comment| comment.span.end <= span.start);
        let remaining = &comments[start_index..];
        let end_index = remaining.partition_point(|comment| comment.span.start < span.end);

        &remaining[..end_index]
    }

    /// Return the nearest token before one span.
    pub fn previous_token_before_span(&self, span: Span) -> Option<TokenSpan> {
        let tokens = self.tokens;
        let index = tokens.partition_point(|token| token.span.end <= span.start);

        index
            .checked_sub(1)
            .and_then(|index| tokens.get(index).copied())
    }

    /// Return the nearest token after one span.
    pub fn next_token_after_span(&self, span: Span) -> Option<TokenSpan> {
        let tokens = self.tokens;
        let index = tokens.partition_point(|token| token.span.start < span.end);

        tokens.get(index).copied()
    }

    /// Return whether one span has a newline before the next token.
    pub fn has_newline_before_next_token(&self, span: Span) -> bool {
        let Some(next_token) = self.next_token_after_span(span) else {
            return false;
        };

        if next_token.span.file != span.file || next_token.span.start <= span.end {
            return false;
        }

        span.gap_to(next_token.span)
            .is_some_and(|between_span| self.has_newline(between_span))
    }

    /// Return the first token that intersects one span.
    #[inline]
    pub fn first_token_in_span(&self, span: Span) -> Option<TokenSpan> {
        self.nth_token_in_span(span, 0)
    }

    /// Return the zero-based nth token that intersects one span.
    pub fn nth_token_in_span(&self, span: Span, nth: usize) -> Option<TokenSpan> {
        let start_index = self
            .tokens
            .partition_point(|token| token.span.end <= span.start);
        let token = self.tokens.get(start_index + nth).copied()?;

        (token.span.start < span.end).then_some(token)
    }

    /// Return the last token that intersects one span.
    pub fn last_token_in_span(&self, span: Span) -> Option<TokenSpan> {
        let mut index = self
            .tokens
            .partition_point(|token| token.span.start < span.end);
        index = index.checked_sub(1)?;
        let token = self.tokens[index];

        (token.span.end > span.start).then_some(token)
    }

    /// Return one literal token lexeme inside one span.
    #[inline]
    pub fn literal_lexeme_in_span(&self, span: Span) -> Option<&'a str> {
        let token = self.first_token_in_span(span)?;

        if token.token.ty() != TokenType::Literal {
            return None;
        }

        Some(self.token_str(token))
    }

    /// Parse one identifier token as a language keyword.
    #[inline]
    pub fn token_keyword(&self, token: TokenSpan) -> Option<Keyword> {
        if token.token.ty() != TokenType::Identifier {
            return None;
        }

        self.token_str(token).parse::<Keyword>().ok()
    }

    /// Return comments between the previous token and one node body.
    pub fn comments_after_previous_token<T>(&self, node_id: LocalNodeId<T>) -> Vec<Comment>
    where
        T: Node + Clone,
        Tree: TreeStore<T>,
    {
        let node_span = self.span(node_id);
        let Some(previous_token) = self.previous_token_before_span(node_span) else {
            return Vec::new();
        };

        let node_start = self.node_token_start(node_id);

        self.comments()
            .comments_in_range(previous_token.span.end, node_start)
            .to_vec()
    }

    /// Return comments before the next token after one span.
    pub fn comments_before_next_token_after_span(&self, span: Span) -> Vec<Comment> {
        let Some(next_token) = self.next_token_after_span(span) else {
            return Vec::new();
        };

        self.comments()
            .comments_in_range(span.end, next_token.span.start)
            .to_vec()
    }

    /// Return comments in source order.
    #[inline]
    pub fn source_comments(&self) -> &[Comment] {
        self.comments().source_comments()
    }

    /// Return source comments that start after one position.
    #[inline]
    fn source_comments_after(&self, pos: u32) -> &[Comment] {
        let comments = self.source_comments();
        let start_index = comments.partition_point(|comment| comment.span.end <= pos);

        &comments[start_index..]
    }

    /// Return source comments that fall in one file-local range.
    #[inline]
    pub fn source_comments_in_range(&self, start: u32, end: u32) -> &[Comment] {
        if start >= end {
            return &[];
        }

        let comments = self.source_comments_after(start);
        let end_index = comments.partition_point(|comment| comment.span.end <= end);

        &comments[..end_index]
    }

    /// Return end-of-line comments after one position.
    pub fn source_end_of_line_comments_after(&self, mut pos: u32) -> &[Comment] {
        let comments = self.source_comments_after(pos);
        let source_text = self.source_text();

        for (index, comment) in comments.iter().enumerate() {
            if !source_text.all_bytes_match(pos, comment.span.start, |byte| {
                matches!(byte, b'\t' | b' ' | b'=' | b':')
            }) {
                break;
            }

            if comment.is_line() || self.has_newline(comment.span) {
                return &comments[..=index];
            }

            pos = comment.span.end;
        }

        &[]
    }

    /// Extend one span to the end of its physical source line.
    pub fn extend_span_with_trailing_line_tokens(&self, span: Span) -> Span {
        let Some((line_index, _)) = self.file.get_position(span.end) else {
            return span;
        };

        let Some(line_span) = self.file.get_line_span(line_index) else {
            return span;
        };

        Span::new(span.file, span.start, line_span.end)
    }
}
