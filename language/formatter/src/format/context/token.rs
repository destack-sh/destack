use super::context::DestackFormatContext;
use destack_ast::{
    Comment, Expression, Keyword, LocalNodeId, Node, NodeTree, NodeTreeImpl, TokenSpan, TokenType,
};
use destack_source::Span;

/// Return whether one token type is horizontal or line whitespace.
fn token_type_is_whitespace(token_type: TokenType) -> bool {
    matches!(token_type, TokenType::Whitespace | TokenType::Newline)
}

/// Return whether one token type is a comment token.
fn token_type_is_comment(token_type: TokenType) -> bool {
    matches!(
        token_type,
        TokenType::LineComment
            | TokenType::BlockComment
            | TokenType::DocLineComment
            | TokenType::DocBlockComment
    )
}

/// Return whether one token type is formatter trivia.
fn token_type_is_trivia(token_type: TokenType) -> bool {
    token_type_is_whitespace(token_type) || token_type_is_comment(token_type)
}

/// Return the next non-whitespace token index at or after one start index.
fn next_non_whitespace_token_index(tokens: &[TokenSpan], mut token_index: usize) -> Option<usize> {
    while let Some(token) = tokens.get(token_index).copied() {
        if token_type_is_whitespace(token.token.ty) {
            token_index += 1;
            continue;
        }

        return Some(token_index);
    }

    None
}

/// Extend one span to include trailing tokens on the same line.
fn extend_span_to_line_end(tokens: &[TokenSpan], span: Span) -> Span {
    let mut end = span.end;
    let token_index = tokens.partition_point(|token| token.span.start < span.end);

    for token in tokens[token_index..].iter().copied() {
        match token.token.ty {
            TokenType::Whitespace => {
                end = token.span.end;
            }
            TokenType::Newline => {
                break;
            }
            _ => {
                end = token.span.end;
            }
        }
    }

    if end > span.end {
        Span::new(span.file, span.start, end)
    } else {
        span
    }
}

/// Extend one span to include a standalone trailing semicolon.
fn extend_span_with_trailing_statement_terminator(tokens: &[TokenSpan], span: Span) -> Span {
    let token_index = tokens.partition_point(|token| token.span.start < span.end);
    let Some(candidate_index) = next_non_whitespace_token_index(tokens, token_index) else {
        return span;
    };
    let Some(candidate) = tokens.get(candidate_index).copied() else {
        return span;
    };

    if candidate.token.ty != TokenType::Semicolon {
        return span;
    }

    let mut lookahead_index = candidate_index + 1;

    while let Some(token) = tokens.get(lookahead_index).copied() {
        match token.token.ty {
            TokenType::Whitespace => {}
            TokenType::Newline | TokenType::End => {
                return Span::new(span.file, span.start, candidate.span.end);
            }
            _ => {
                return span;
            }
        }

        lookahead_index += 1;
    }

    Span::new(span.file, span.start, candidate.span.end)
}

impl<'a> DestackFormatContext<'a> {
    /// Return all tokens across main and side streams sorted by source position.
    #[inline]
    pub(crate) fn all_tokens(&self) -> &[TokenSpan] {
        self.all_tokens_sorted.get_or_init(|| {
            let mut tokens: Vec<TokenSpan> = self
                .tokens
                .iter()
                .copied()
                .chain(self.side_tokens.iter().copied())
                .collect();

            tokens.sort_by_key(|token| token.span.start);
            tokens
        })
    }

    /// Return the first non-trivia token start for one node.
    pub fn node_token_start<T>(&self, node_id: LocalNodeId<T>) -> u32
    where
        T: Node + Clone,
        NodeTree: NodeTreeImpl<T>,
    {
        let node_span = self.span(node_id);

        self.first_non_trivia_token_in_span(node_span)
            .map_or(node_span.start, |token| token.span.start)
    }

    /// Return the first non-trivia token start for one expression.
    pub fn expression_token_start(&self, expression_id: LocalNodeId<Expression>) -> u32 {
        self.node_token_start(expression_id)
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

    /// Return the first non-trivia token between two offsets.
    pub fn first_non_trivia_token_between(&self, start: u32, end: u32) -> Option<TokenSpan> {
        if end <= start {
            return None;
        }

        let mut token_index = self
            .tokens
            .partition_point(|token| token.span.start < start);

        while let Some(token) = self.tokens.get(token_index).copied() {
            if token.span.start >= end {
                return None;
            }

            token_index += 1;

            if token_type_is_trivia(token.token.ty) {
                continue;
            }

            return Some(token);
        }

        None
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

            if token.token.ty == token_type {
                seen += 1;

                if seen == nth {
                    return Some(token.span.start);
                }
            }

            token_index += 1;
        }

        None
    }

    /// Return non-trivia tokens that intersect one span.
    pub fn non_trivia_tokens_in_span(&self, span: Span) -> Vec<TokenSpan> {
        if span.start >= span.end {
            return Vec::new();
        }

        let mut tokens_in_span = Vec::new();
        let mut token_index = self
            .tokens
            .partition_point(|token| token.span.end <= span.start);

        while let Some(token) = self.tokens.get(token_index).copied() {
            if token.span.start >= span.end {
                break;
            }

            token_index += 1;

            if token_type_is_trivia(token.token.ty) {
                continue;
            }

            tokens_in_span.push(token);
        }

        tokens_in_span
    }

    /// Return the comment token type at one exact comment span.
    pub fn comment_token_type_at_span(&self, comment_span: Span) -> Option<TokenType> {
        let comment_tokens = self.comment_tokens();
        let token_index =
            comment_tokens.partition_point(|token| token.span.start < comment_span.start);
        let comment_token = comment_tokens.get(token_index).copied()?;

        if comment_token.span != comment_span {
            return None;
        }

        Some(comment_token.token.ty)
    }

    /// Return comment tokens that intersect one span.
    pub fn comment_tokens_intersecting_span(&self, span: Span) -> Vec<TokenSpan> {
        self.comment_tokens()
            .iter()
            .copied()
            .filter(|token| span.intersects(token.span))
            .collect()
    }

    /// Return the nearest non-whitespace token before one span.
    pub fn previous_non_whitespace_token_before_span(&self, span: Span) -> Option<TokenSpan> {
        let tokens = self.tokens;
        let mut index = tokens.partition_point(|token| token.span.end <= span.start);

        while index > 0 {
            index -= 1;

            let token = tokens[index];
            if token_type_is_whitespace(token.token.ty) {
                continue;
            }

            return Some(token);
        }

        None
    }

    /// Return the nearest non-trivia token before one span.
    pub fn previous_non_trivia_token_before_span(&self, span: Span) -> Option<TokenSpan> {
        let tokens = self.tokens;
        let mut index = tokens.partition_point(|token| token.span.end <= span.start);

        while index > 0 {
            index -= 1;

            let token = tokens[index];
            if token_type_is_trivia(token.token.ty) {
                continue;
            }

            return Some(token);
        }

        None
    }

    /// Return the nearest non-whitespace token after one span.
    pub fn next_non_whitespace_token_after_span(&self, span: Span) -> Option<TokenSpan> {
        let tokens = self.tokens;
        let mut index = tokens.partition_point(|token| token.span.start < span.end);

        while let Some(token) = tokens.get(index).copied() {
            if token_type_is_whitespace(token.token.ty) {
                index += 1;
                continue;
            }

            return Some(token);
        }

        None
    }

    /// Return the nearest non-trivia token after one span.
    pub fn next_non_trivia_token_after_span(&self, span: Span) -> Option<TokenSpan> {
        let tokens = self.tokens;
        let mut index = tokens.partition_point(|token| token.span.start < span.end);

        while let Some(token) = tokens.get(index).copied() {
            if token_type_is_trivia(token.token.ty) {
                index += 1;
                continue;
            }

            return Some(token);
        }

        None
    }

    /// Return whether one span has a newline before the next non-whitespace token.
    pub fn span_has_newline_before_next_non_whitespace_token(&self, span: Span) -> bool {
        let Some(next_token) = self.next_non_whitespace_token_after_span(span) else {
            return false;
        };

        if next_token.span.file != span.file || next_token.span.start <= span.end {
            return false;
        }

        span.gap_to(next_token.span)
            .is_some_and(|between_span| self.has_newline(between_span))
    }

    /// Return the first non-trivia token that intersects one span.
    #[inline]
    pub fn first_non_trivia_token_in_span(&self, span: Span) -> Option<TokenSpan> {
        self.nth_non_trivia_token_in_span(span, 0)
    }

    /// Return the nth non-trivia token that intersects one span.
    pub fn nth_non_trivia_token_in_span(&self, span: Span, nth: usize) -> Option<TokenSpan> {
        let mut index = self
            .tokens
            .partition_point(|token| token.span.end <= span.start);
        let mut seen = 0usize;

        while let Some(token) = self.tokens.get(index).copied() {
            if token.span.start >= span.end {
                break;
            }

            index += 1;

            if token_type_is_trivia(token.token.ty) {
                continue;
            }

            if seen == nth {
                return Some(token);
            }

            seen += 1;
        }

        None
    }

    /// Return the last non-trivia token that intersects one span.
    pub fn last_non_trivia_token_in_span(&self, span: Span) -> Option<TokenSpan> {
        let mut index = self
            .tokens
            .partition_point(|token| token.span.start < span.end);

        while index > 0 {
            index -= 1;

            let token = self.tokens[index];
            if token.span.end <= span.start {
                break;
            }

            if token_type_is_trivia(token.token.ty) {
                continue;
            }

            return Some(token);
        }

        None
    }

    /// Return one literal token lexeme inside one span.
    #[inline]
    pub fn literal_lexeme_in_span(&self, span: Span) -> Option<&'a str> {
        let token = self.first_non_trivia_token_in_span(span)?;

        if token.token.ty != TokenType::Literal {
            return None;
        }

        Some(self.token_str(token))
    }

    /// Parse one identifier token as a language keyword.
    #[inline]
    pub fn token_keyword(&self, token: TokenSpan) -> Option<Keyword> {
        if token.token.ty != TokenType::Identifier {
            return None;
        }

        self.token_str(token).parse::<Keyword>().ok()
    }

    /// Return comments between the previous non-trivia token and one node body.
    pub fn comments_after_previous_non_trivia_token_for<T>(
        &self,
        node_id: LocalNodeId<T>,
    ) -> Vec<Comment>
    where
        T: Node + Clone,
        NodeTree: NodeTreeImpl<T>,
    {
        let node_span = self.span(node_id);
        let Some(previous_token) = self.previous_non_trivia_token_before_span(node_span) else {
            return Vec::new();
        };

        let node_start = self.node_token_start(node_id);

        self.comments()
            .comments_in_range(previous_token.span.end, node_start)
            .to_vec()
    }

    /// Return comments before the next non-trivia token after one span.
    pub fn comments_before_next_non_trivia_token_after_span(&self, span: Span) -> Vec<Comment> {
        let Some(next_token) = self.next_non_trivia_token_after_span(span) else {
            return Vec::new();
        };

        self.comments()
            .comments_in_range(span.end, next_token.span.start)
            .to_vec()
    }

    /// Get comment tokens sorted by source position.
    #[inline]
    pub fn comment_tokens(&self) -> &[TokenSpan] {
        self.comment_tokens_sorted.get_or_init(|| {
            self.all_tokens()
                .iter()
                .copied()
                .filter(|token| token_type_is_comment(token.token.ty))
                .collect()
        })
    }

    /// Return comment tokens that start after one position.
    #[inline]
    fn comment_tokens_after(&self, pos: u32) -> &[TokenSpan] {
        let comment_tokens = self.comment_tokens();
        let start_index = comment_tokens.partition_point(|token| token.span.end < pos);

        &comment_tokens[start_index..]
    }

    /// Return comment tokens that fall in one file-local range.
    #[inline]
    pub fn comment_tokens_in_range(&self, start: u32, end: u32) -> &[TokenSpan] {
        if start >= end {
            return &[];
        }

        let comment_tokens = self.comment_tokens_after(start);
        let end_index = comment_tokens.partition_point(|token| token.span.end <= end);

        &comment_tokens[..end_index]
    }

    /// Return end-of-line comment tokens after one position.
    pub fn end_of_line_comment_tokens_after(&self, mut pos: u32) -> &[TokenSpan] {
        let comment_tokens = self.comment_tokens_after(pos);
        let source_text = self.source_text();

        for (index, token) in comment_tokens.iter().enumerate() {
            if !source_text.all_bytes_match(pos, token.span.start, |byte| {
                matches!(byte, b'\t' | b' ' | b'=' | b':')
            }) {
                break;
            }

            if matches!(
                token.token.ty,
                TokenType::LineComment | TokenType::DocLineComment
            ) || self.has_newline(token.span)
            {
                return &comment_tokens[..=index];
            }

            pos = token.span.end;
        }

        &[]
    }

    /// Return whether one comment token is line-oriented.
    #[inline]
    pub fn comment_is_line(&self, token: TokenSpan) -> bool {
        matches!(
            token.token.ty,
            TokenType::LineComment | TokenType::DocLineComment
        )
    }

    /// Extend one span to include trailing same-line content and a standalone semicolon.
    #[inline]
    pub fn extend_span_with_trailing_line_tokens(&self, span: Span) -> Span {
        let tokens = self.all_tokens();
        let span = extend_span_to_line_end(tokens, span);
        extend_span_with_trailing_statement_terminator(tokens, span)
    }
}
