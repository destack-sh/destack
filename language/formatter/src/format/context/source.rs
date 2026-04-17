use super::comment::Comments;
use super::context::{
    DestackFormatContext, NODE_BOOL_STATE_FALSE, NODE_BOOL_STATE_TRUE, TYPE_CONTEXT_STATE_FALSE,
    TYPE_CONTEXT_STATE_TRUE, TYPE_CONTEXT_STATE_UNKNOWN,
};
use std::borrow::Cow;
use std::cell::Cell;
use std::ops::Deref;

use destack_ast::{
    Comment, Expression, Keyword, LocalNodeId, Node, NodeTree, NodeTreeImpl, NodeType, TokenSpan,
    TokenType, normalize_comment_payload,
};
use destack_source::{File, NodeSpanType, SourcePartKey, Span};
use rustc_hash::FxHashMap;
use smallvec::SmallVec;

/// Source text wrapper for formatter byte and span queries.
#[derive(Debug, Clone, Copy)]
pub struct SourceText<'a> {
    /// The underlying source text.
    text: &'a str,
}

impl Deref for SourceText<'_> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.text
    }
}

impl<'a> SourceText<'a> {
    /// Create one source text wrapper.
    pub fn new(text: &'a str) -> Self {
        Self { text }
    }

    /// Return the text for one span.
    pub fn text_for(&self, span: &Span) -> &'a str {
        &self.text[span.start as usize..span.end as usize]
    }

    /// Return the slice from one offset.
    pub fn slice_from(&self, position: u32) -> &'a str {
        &self.text[position as usize..]
    }

    /// Return the slice up to one offset.
    pub fn slice_to(&self, position: u32) -> &'a str {
        &self.text[..position as usize]
    }

    /// Return the slice inside one byte range.
    pub fn slice_range(&self, start: u32, end: u32) -> &'a str {
        &self.text[start as usize..end as usize]
    }

    /// Return bytes from one offset to the end.
    pub fn bytes_from(&self, position: u32) -> impl Iterator<Item = u8> + '_ {
        self.text.as_bytes()[position as usize..].iter().copied()
    }

    /// Return bytes before one offset in reverse.
    pub fn bytes_to(&self, position: u32) -> impl Iterator<Item = u8> + '_ {
        self.text.as_bytes()[..position as usize]
            .iter()
            .copied()
            .rev()
    }

    /// Return bytes inside one byte range.
    pub fn bytes_range(&self, start: u32, end: u32) -> &'a [u8] {
        &self.text.as_bytes()[start as usize..end as usize]
    }

    /// Return whether the next non-whitespace byte matches one value.
    pub fn next_non_whitespace_byte_is(&self, position: u32, expected_byte: u8) -> bool {
        self.bytes_from(position)
            .find(|byte| !byte.is_ascii_whitespace())
            .is_some_and(|byte| byte == expected_byte)
    }

    /// Return the byte at one position.
    pub fn byte_at(&self, position: u32) -> Option<u8> {
        self.text.as_bytes().get(position as usize).copied()
    }

    /// Return whether one span contains a newline.
    pub fn contains_newline(&self, span: Span) -> bool {
        self.slice_range(span.start, span.end)
            .bytes()
            .any(|byte| matches!(byte, b'\n' | b'\r'))
    }

    /// Return whether horizontal trivia before one offset contains a newline.
    pub fn has_newline_before(&self, position: u32) -> bool {
        for byte in self.bytes_to(position) {
            match byte {
                b'\n' | b'\r' => return true,
                b' ' | b'\t' => {}
                _ => return false,
            }
        }

        false
    }

    /// Return whether horizontal trivia after one offset contains a newline.
    pub fn has_newline_after(&self, position: u32) -> bool {
        for byte in self.bytes_from(position) {
            match byte {
                b'\n' | b'\r' => return true,
                b' ' | b'\t' => {}
                _ => return false,
            }
        }

        false
    }

    /// Return whether a newline appears after one opening brace, scanning through comments.
    pub fn has_newline_after_opening_brace(&self, position: u32) -> bool {
        let mut iter = self.bytes_from(position + 1).peekable();

        while let Some(byte) = iter.next() {
            match byte {
                b'\n' | b'\r' => return true,
                b' ' | b'\t' => {}
                b'/' => match iter.peek() {
                    Some(&b'/') => {
                        iter.next();
                        return iter.any(|byte| matches!(byte, b'\n' | b'\r'));
                    }
                    Some(&b'*') => {
                        iter.next();

                        while let Some(byte) = iter.next() {
                            if matches!(byte, b'\n' | b'\r') {
                                return true;
                            }

                            if byte == b'*' && matches!(iter.peek(), Some(&b'/')) {
                                iter.next();
                                break;
                            }
                        }
                    }
                    _ => return false,
                },
                _ => return false,
            }
        }

        false
    }

    /// Return whether one byte range contains one byte value.
    pub fn bytes_contain(&self, start: u32, end: u32, byte: u8) -> bool {
        self.bytes_range(start, end).contains(&byte)
    }

    /// Return whether all bytes in one range match one predicate.
    pub fn all_bytes_match<F>(&self, start: u32, end: u32, predicate: F) -> bool
    where
        F: Fn(u8) -> bool,
    {
        self.bytes_range(start, end)
            .iter()
            .all(|byte| predicate(*byte))
    }

    /// Return the character width of one span.
    pub fn span_width(&self, span: Span) -> usize {
        self.text_for(&span).chars().count()
    }

    /// Return the number of consecutive line breaks after one offset.
    pub fn lines_after(&self, end: u32) -> usize {
        let mut count = 0usize;
        let mut chars = self.slice_from(end).chars().peekable();

        while let Some(current) = chars.next() {
            if is_single_line_whitespace(current) {
                continue;
            }

            if is_line_terminator(current) {
                count += 1;

                if current == '\r' && chars.peek() == Some(&'\n') {
                    chars.next();
                }

                continue;
            }

            return count;
        }

        0
    }

    /// Return the number of line breaks before one span.
    pub fn get_lines_before(&self, span: Span, comments: &Comments<'_>) -> usize {
        let mut start = span.start;
        let comments = comments.unprinted_comments();

        // skip leading comments and semicolons that conceptually belong to the node
        if let Some(comment) = comments.first()
            && comment.span.end <= start
        {
            start = comment.span.start;
        } else if start != 0 && matches!(self.byte_at(start - 1), Some(b';')) {
            start -= 1;
        }

        let mut count = 0usize;
        let mut following_source = self.bytes_from(span.end);
        let mut chars = self.slice_to(start).chars().rev().peekable();

        while let Some(current) = chars.next() {
            if is_single_line_whitespace(current) {
                continue;
            }

            if current == '(' {
                for byte in following_source.by_ref() {
                    if byte.is_ascii_whitespace() {
                        continue;
                    }

                    if byte == b')' {
                        break;
                    }

                    return count;
                }

                count = 0;
                continue;
            }

            if !is_line_terminator(current) {
                return count;
            }

            count += 1;

            if current == '\n' && chars.peek() == Some(&'\r') {
                chars.next();
            }
        }

        0
    }
}

/// Return whether one character is a line terminator.
fn is_line_terminator(current: char) -> bool {
    matches!(current, '\n' | '\r' | '\u{2028}' | '\u{2029}')
}

/// Return whether one character is horizontal whitespace.
fn is_single_line_whitespace(current: char) -> bool {
    current.is_whitespace() && !is_line_terminator(current)
}

/// Build one keyword map for identifier tokens across main and side streams.
pub(crate) fn token_keyword_map(
    file: &File,
    tokens: &[TokenSpan],
    side_tokens: &[TokenSpan],
) -> FxHashMap<Span, Option<Keyword>> {
    let mut token_keyword_by_span = FxHashMap::default();

    for token in tokens.iter().copied().chain(side_tokens.iter().copied()) {
        if token.token.ty != TokenType::Identifier {
            continue;
        }

        let keyword = file.span_str(token.span).parse::<Keyword>().ok();
        token_keyword_by_span.insert(token.span, keyword);
    }

    token_keyword_by_span
}

/// Return whether a token contributes non-whitespace content.
#[inline]
fn token_has_non_whitespace_content(token_type: TokenType) -> bool {
    !matches!(
        token_type,
        TokenType::Whitespace | TokenType::Newline | TokenType::End
    )
}

/// Return whether one token stream contains non-whitespace content inside one span.
fn token_stream_has_non_whitespace_content(tokens: &[TokenSpan], span: Span) -> bool {
    if span.start >= span.end {
        return false;
    }

    let mut index = tokens.partition_point(|token| token.span.end <= span.start);
    while let Some(token) = tokens.get(index).copied() {
        if token.span.start >= span.end {
            break;
        }

        if token_has_non_whitespace_content(token.token.ty) {
            return true;
        }

        index += 1;
    }

    false
}

/// Return the next non-whitespace token index at or after one start index.
fn next_non_whitespace_token_index(tokens: &[TokenSpan], mut token_index: usize) -> Option<usize> {
    while let Some(token) = tokens.get(token_index).copied() {
        if matches!(token.token.ty, TokenType::Whitespace | TokenType::Newline) {
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
    for token in tokens.iter().copied() {
        if token.span.start < span.end {
            continue;
        }

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

    /// Return the source text wrapper for this file.
    pub fn source_text(&self) -> SourceText<'a> {
        SourceText::new(self.file.text())
    }

    /// Return whether all bytes in one file-local range match one predicate.
    fn file_range_bytes_match(
        &self,
        start: u32,
        end: u32,
        mut predicate: impl FnMut(u8) -> bool,
    ) -> bool {
        if start >= end {
            return true;
        }

        let Ok(start) = usize::try_from(start) else {
            return false;
        };
        let Ok(end) = usize::try_from(end) else {
            return false;
        };
        let Some(bytes) = self.file.text().as_bytes().get(start..end) else {
            return false;
        };

        bytes.iter().copied().all(&mut predicate)
    }

    /// Return whether one file-local range contains one byte value.
    #[inline]
    pub fn range_contains_byte(&self, start: u32, end: u32, byte: u8) -> bool {
        !self.file_range_bytes_match(start, end, |current| current != byte)
    }

    /// Return whether one file-local range contains only horizontal whitespace.
    #[inline]
    pub fn range_contains_only_horizontal_whitespace(&self, start: u32, end: u32) -> bool {
        self.file_range_bytes_match(start, end, |current| matches!(current, b' ' | b'\t'))
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
            if matches!(
                token.token.ty,
                TokenType::Whitespace
                    | TokenType::Newline
                    | TokenType::LineComment
                    | TokenType::BlockComment
                    | TokenType::DocLineComment
                    | TokenType::DocBlockComment
            ) {
                continue;
            }

            return Some(token);
        }

        None
    }

    /// Return whether one span contains at least one token of the given type.
    pub fn span_has_token_type(&self, span: Span, token_type: TokenType) -> bool {
        if span.start >= span.end {
            return false;
        }

        let mut token_index = self
            .tokens
            .partition_point(|token| token.span.start < span.start);
        while let Some(token) = self.tokens.get(token_index).copied() {
            if token.span.start >= span.end {
                return false;
            }

            if token.token.ty == token_type {
                return true;
            }

            token_index += 1;
        }

        false
    }

    /// Return the start offset of the Nth token of the given type within one span.
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
            if matches!(
                token.token.ty,
                TokenType::Whitespace
                    | TokenType::Newline
                    | TokenType::LineComment
                    | TokenType::BlockComment
                    | TokenType::DocLineComment
                    | TokenType::DocBlockComment
            ) {
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
            if matches!(token.token.ty, TokenType::Whitespace | TokenType::Newline) {
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
            if matches!(
                token.token.ty,
                TokenType::Whitespace
                    | TokenType::Newline
                    | TokenType::LineComment
                    | TokenType::BlockComment
                    | TokenType::DocLineComment
                    | TokenType::DocBlockComment
            ) {
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
            if matches!(token.token.ty, TokenType::Whitespace | TokenType::Newline) {
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
            if matches!(
                token.token.ty,
                TokenType::Whitespace
                    | TokenType::Newline
                    | TokenType::LineComment
                    | TokenType::BlockComment
                    | TokenType::DocLineComment
                    | TokenType::DocBlockComment
            ) {
                index += 1;
                continue;
            }

            return Some(token);
        }

        None
    }

    /// Return the nearest non-trivia token type after one span.
    pub fn next_non_trivia_token_type_after_span(&self, span: Span) -> Option<TokenType> {
        self.next_non_trivia_token_after_span(span)
            .map(|token| token.token.ty)
    }

    /// Return whether one span has a newline before its next non-whitespace token.
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

    /// Return the first non-trivia token index that intersects one span.
    pub fn first_non_trivia_token_index_in_span(&self, span: Span) -> Option<u32> {
        let mut index = self
            .tokens
            .partition_point(|token| token.span.end <= span.start);

        while let Some(token) = self.tokens.get(index).copied() {
            if token.span.start >= span.end {
                break;
            }

            if matches!(
                token.token.ty,
                TokenType::Whitespace
                    | TokenType::Newline
                    | TokenType::LineComment
                    | TokenType::BlockComment
                    | TokenType::DocLineComment
                    | TokenType::DocBlockComment
            ) {
                index += 1;
                continue;
            }

            return Some(index as u32);
        }

        None
    }

    /// Return the Nth non-trivia token that intersects one span.
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
            if matches!(
                token.token.ty,
                TokenType::Whitespace
                    | TokenType::Newline
                    | TokenType::LineComment
                    | TokenType::BlockComment
                    | TokenType::DocLineComment
                    | TokenType::DocBlockComment
            ) {
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

            if matches!(
                token.token.ty,
                TokenType::Whitespace
                    | TokenType::Newline
                    | TokenType::LineComment
                    | TokenType::BlockComment
                    | TokenType::DocLineComment
                    | TokenType::DocBlockComment
            ) {
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

    /// Return byte offsets of all newline characters in the source file.
    #[inline]
    fn newline_offsets(&self) -> &[u32] {
        self.newline_offsets.get_or_init(|| {
            self.file
                .text()
                .bytes()
                .enumerate()
                .filter_map(|(index, byte)| (byte == b'\n').then_some(index as u32))
                .collect()
        })
    }

    pub fn has_ignore_directive_markers(&self) -> bool {
        self.has_ignore_directive_markers
    }

    /// Return whether this file may contain template literals.
    #[inline]
    pub fn has_template_literal_markers(&self) -> bool {
        self.has_template_literal_markers
    }

    /// Mark that file-level ignore was applied.
    #[inline]
    pub fn mark_file_ignore_applied(&self) {
        self.file_ignore_applied.set(true);
    }

    /// Return whether file-level ignore was applied.
    #[inline]
    pub fn file_ignore_applied(&self) -> bool {
        self.file_ignore_applied.get()
    }

    /// Gets the str source backing a Span.
    #[inline]
    pub fn span_str(&self, span: Span) -> &'a str {
        {
            let cache = self.span_text_by_span.borrow();
            if let Some(span_str) = cache.get(&span) {
                return span_str;
            }
        }

        let span_str = self.file.span_str(span);
        self.span_text_by_span.borrow_mut().insert(span, span_str);
        span_str
    }

    /// Gets the str source backing a TokenSpan.
    #[inline]
    pub fn token_str(&self, token: TokenSpan) -> &'a str {
        self.span_str(token.span)
    }

    /// Parse one identifier token as a language keyword.
    #[inline]
    pub fn token_keyword(&self, token: TokenSpan) -> Option<Keyword> {
        if token.token.ty != TokenType::Identifier {
            return None;
        }

        {
            let cache = self.token_keyword_by_span.borrow();
            if let Some(keyword) = cache.get(&token.span) {
                return *keyword;
            }
        }

        let keyword = self.token_str(token).parse::<Keyword>().ok();
        self.token_keyword_by_span
            .borrow_mut()
            .insert(token.span, keyword);
        keyword
    }

    /// Get one normalized comment payload string.
    #[inline]
    pub fn comment_text(&self, comment: Comment) -> Cow<'a, str> {
        let comment_source = self.comment_raw_text(comment);
        normalize_comment_payload(comment_source)
    }

    /// Get one raw comment text slice.
    #[inline]
    pub fn comment_raw_text(&self, comment: Comment) -> &'a str {
        self.span_str(comment.span)
    }

    /// Return whether one raw comment is a doc comment token.
    #[inline]
    pub fn comment_is_doc(&self, comment: Comment) -> bool {
        matches!(
            self.comment_token_type_at_span(comment.span),
            Some(TokenType::DocLineComment | TokenType::DocBlockComment)
        )
    }

    /// Return whether one raw comment is a line doc comment token.
    #[inline]
    pub fn comment_is_doc_line(&self, comment: Comment) -> bool {
        self.comment_token_type_at_span(comment.span) == Some(TokenType::DocLineComment)
    }

    /// Return whether one raw comment is a block doc comment token.
    #[inline]
    pub fn comment_is_doc_block(&self, comment: Comment) -> bool {
        self.comment_token_type_at_span(comment.span) == Some(TokenType::DocBlockComment)
    }

    /// Return leading raw comments attached to one node head.
    pub fn raw_prefix_comments_for<T>(&self, node_id: LocalNodeId<T>) -> Vec<Comment>
    where
        T: Node + Clone,
        NodeTree: NodeTreeImpl<T>,
    {
        let leading_separator_is_structural = self
            .tree
            .get_side_span(node_id, NodeSpanType::Leading)
            .and_then(|leading_span| self.first_non_trivia_token_in_span(leading_span))
            .is_some_and(|token| {
                matches!(
                    token.token.ty,
                    TokenType::ElementwiseOr | TokenType::ElementwiseAnd
                )
            });
        let head_start = self
            .tree
            .get_head_span(node_id)
            .or_else(|| self.tree.get_main_span(node_id))
            .unwrap_or_else(|| self.span(node_id))
            .start;

        let mut comments = if leading_separator_is_structural {
            Vec::new()
        } else {
            self.tree
                .comments()
                .iter()
                .copied()
                .filter(|comment| {
                    comment.attached_part == SourcePartKey::new(node_id.id, NodeSpanType::Leading)
                })
                .collect::<Vec<_>>()
        };

        comments.extend(
            self.tree
                .comments()
                .iter()
                .copied()
                .filter(|comment| {
                    comment.attached_part == SourcePartKey::new(node_id.id, NodeSpanType::Enclosing)
                })
                .filter(|comment| comment.span.end <= head_start),
        );

        comments.sort_by_key(|comment| (comment.span.start, comment.span.end));
        comments
    }

    /// Return raw comments fully contained in one boundary range.
    pub(crate) fn raw_boundary_comments_in_range(&self, start: u32, end: u32) -> Vec<Comment> {
        let mut comments = self
            .tree
            .comments()
            .iter()
            .copied()
            .filter(|comment| comment.span.start >= start && comment.span.end <= end)
            .collect::<Vec<_>>();
        comments.sort_by_key(|comment| (comment.span.start, comment.span.end));
        comments
    }

    /// Return raw comments between the previous non-trivia token and one expression body.
    pub fn raw_comments_after_previous_non_trivia_token_for(
        &self,
        expression_id: LocalNodeId<Expression>,
    ) -> Vec<Comment> {
        let expression_span = self.span(expression_id);
        let Some(previous_token) = self.previous_non_trivia_token_before_span(expression_span)
        else {
            return Vec::new();
        };
        let expression_start = self.expression_token_start(expression_id);
        self.raw_boundary_comments_in_range(previous_token.span.end, expression_start)
    }

    /// Return raw comments before the next non-trivia token after one span.
    pub fn raw_comments_before_next_non_trivia_token_after_span(&self, span: Span) -> Vec<Comment> {
        let Some(next_token) = self.next_non_trivia_token_after_span(span) else {
            return Vec::new();
        };

        self.raw_boundary_comments_in_range(span.end, next_token.span.start)
    }

    /// Return raw comments inside one trailing span for one node.
    pub fn raw_comments_in_trailing_for<T>(&self, node_id: LocalNodeId<T>) -> Vec<Comment>
    where
        T: Node + Clone,
        NodeTree: NodeTreeImpl<T>,
    {
        let Some(trailing_span) = self.tree.get_side_span(node_id, NodeSpanType::Trailing) else {
            return Vec::new();
        };
        let owner = SourcePartKey::new(node_id.id, NodeSpanType::Trailing);

        self.tree
            .comments()
            .iter()
            .copied()
            .filter(|comment| comment.attached_part == owner)
            .filter(|comment| comment.span.file == trailing_span.file)
            .collect()
    }

    /// Return raw comments after the previous non-trivia token and before one expression body.
    pub fn raw_comments_after_previous_non_trivia_token_before_expression(
        &self,
        expression_id: LocalNodeId<Expression>,
    ) -> Vec<Comment> {
        self.raw_comments_after_previous_non_trivia_token_for(expression_id)
    }

    /// Return the first non-trivia token start for one type expression.
    pub fn type_expression_token_start<T>(&self, node_id: LocalNodeId<T>) -> u32
    where
        T: Node + Clone,
        NodeTree: NodeTreeImpl<T>,
    {
        self.node_token_start(node_id)
    }

    /// Return raw comments that belong before one explicit type-position expression.
    pub fn raw_type_position_comments_for<T>(&self, node_id: LocalNodeId<T>) -> Vec<Comment>
    where
        T: Node + Clone,
        NodeTree: NodeTreeImpl<T>,
    {
        if let Some(leading_span) = self.tree.get_side_span(node_id, NodeSpanType::Leading)
            && self
                .first_non_trivia_token_in_span(leading_span)
                .is_some_and(|token| {
                    matches!(
                        token.token.ty,
                        TokenType::ElementwiseOr | TokenType::ElementwiseAnd
                    )
                })
        {
            if let Some(start) = self.type_expression_leading_comment_start(node_id) {
                let separator_start = leading_span.start;
                if start < separator_start {
                    return self.raw_boundary_comments_in_range(start, separator_start);
                }
            }

            return Vec::new();
        }

        if let Some(start) = self.type_expression_leading_comment_start(node_id) {
            let node_start = self.type_expression_token_start(node_id);
            if start < node_start {
                return self.raw_boundary_comments_in_range(start, node_start);
            }
        }

        self.raw_prefix_comments_for(node_id)
    }

    /// Return leading raw doc comments attached to one node head.
    pub fn raw_prefix_doc_comments_for<T>(&self, node_id: LocalNodeId<T>) -> Vec<Comment>
    where
        T: Node + Clone,
        NodeTree: NodeTreeImpl<T>,
    {
        self.raw_prefix_comments_for(node_id)
            .into_iter()
            .filter(|comment| self.comment_is_doc(*comment))
            .collect()
    }

    /// Return end-of-line raw doc comments after one offset.
    pub fn end_of_line_raw_doc_comments_after(&self, pos: u32) -> Vec<Comment> {
        self.end_of_line_raw_comments_after(pos)
            .into_iter()
            .filter(|comment| self.comment_is_doc(*comment))
            .collect()
    }

    /// Get the source position for one byte offset.
    #[inline]
    pub fn source_position(&self, offset: u32) -> Option<(u32, u32)> {
        self.file.get_position(offset)
    }

    /// Get the source span for one line index.
    #[inline]
    pub fn source_line_span(&self, line_index: u32) -> Option<Span> {
        self.file.get_line_span(line_index)
    }

    /// Return whether one line prefix has only whitespace trivia.
    #[inline]
    pub fn line_prefix_is_whitespace(&self, offset: u32) -> bool {
        let Some((line_index, _)) = self.source_position(offset) else {
            return false;
        };
        let Some(line_span) = self.source_line_span(line_index) else {
            return false;
        };
        if line_span.start >= offset {
            return true;
        }

        let prefix_span = Span::new(line_span.file, line_span.start, offset);
        !self.has_non_whitespace_content(prefix_span)
    }

    /// Get the source line distance between two byte offsets.
    #[inline]
    pub fn source_line_distance(&self, start_offset: u32, end_offset: u32) -> Option<u32> {
        let (start_line, _) = self.source_position(start_offset)?;
        let (end_line, _) = self.source_position(end_offset)?;
        end_line.checked_sub(start_line)
    }

    /// Get the raw line prefix string before one byte offset.
    #[inline]
    pub fn line_prefix_text(&self, offset: u32) -> Option<&'a str> {
        let (line_index, column) = self.source_position(offset)?;
        let line_span = self.source_line_span(line_index)?;
        let line_text = self.span_str(line_span);
        line_text.get(..column as usize)
    }

    /// Get comment tokens sorted by source position.
    #[inline]
    pub fn comment_tokens(&self) -> &[TokenSpan] {
        self.comment_tokens_sorted.get_or_init(|| {
            let mut tokens: Vec<TokenSpan> = self
                .tokens
                .iter()
                .copied()
                .chain(self.side_tokens.iter().copied())
                .filter(|token| {
                    matches!(
                        token.token.ty,
                        TokenType::LineComment
                            | TokenType::BlockComment
                            | TokenType::DocLineComment
                            | TokenType::DocBlockComment
                    )
                })
                .collect();
            tokens.sort_by_key(|token| token.span.start);
            tokens
        })
    }

    /// Return comment tokens that end before or at one position.
    #[inline]
    pub fn comment_tokens_before(&self, pos: u32) -> &[TokenSpan] {
        let comment_tokens = self.comment_tokens();
        let end_index = comment_tokens.partition_point(|token| token.span.end <= pos);
        &comment_tokens[..end_index]
    }

    /// Return comment tokens that start after one position.
    #[inline]
    pub fn comment_tokens_after(&self, pos: u32) -> &[TokenSpan] {
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

    /// Return comments that fall in one file-local range.
    #[inline]
    pub fn comments_in_range(&self, start: u32, end: u32) -> &[TokenSpan] {
        self.comment_tokens_in_range(start, end)
    }

    /// Return end-of-line comment tokens after one position.
    pub fn end_of_line_comment_tokens_after(&self, mut pos: u32) -> &[TokenSpan] {
        let comment_tokens = self.comment_tokens_after(pos);

        for (index, token) in comment_tokens.iter().enumerate() {
            if !self.file_range_bytes_match(pos, token.span.start, |byte| {
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

    /// Return end-of-line raw comments after one position.
    pub fn end_of_line_raw_comments_after(&self, mut pos: u32) -> Vec<Comment> {
        let mut comments = Vec::new();

        for comment in self.comments().comments_after(pos).iter().copied() {
            if !self.file_range_bytes_match(pos, comment.span.start, |byte| {
                matches!(byte, b'\t' | b' ' | b'=' | b':' | b';')
            }) {
                break;
            }

            comments.push(comment);
            if comment.is_line() || self.comment_followed_by_newline_token_span(comment.span) {
                break;
            }

            pos = comment.span.end;
        }

        comments
    }

    /// Return whether one comment token starts on its own line.
    #[inline]
    pub fn comment_starts_on_own_line(&self, token: TokenSpan) -> bool {
        self.span_starts_on_own_line(token.span)
    }

    /// Return whether one comment token is line-oriented.
    #[inline]
    pub fn comment_is_line(&self, token: TokenSpan) -> bool {
        matches!(
            token.token.ty,
            TokenType::LineComment | TokenType::DocLineComment
        )
    }

    /// Return whether one comment token is block-oriented.
    #[inline]
    pub fn comment_is_block(&self, token: TokenSpan) -> bool {
        matches!(
            token.token.ty,
            TokenType::BlockComment | TokenType::DocBlockComment
        )
    }

    /// Return whether one comment token is followed by a newline before the next token.
    #[inline]
    pub fn comment_followed_by_newline(&self, token: TokenSpan) -> bool {
        self.span_has_newline_before_next_non_whitespace_token(token.span)
    }

    /// Return whether one comment span is followed by a newline before the next token.
    #[inline]
    fn comment_followed_by_newline_token_span(&self, comment_span: Span) -> bool {
        self.span_has_newline_before_next_non_whitespace_token(comment_span)
    }

    /// Extend one span to include trailing same-line content and standalone semicolons.
    #[inline]
    pub fn extend_span_with_trailing_line_tokens(&self, span: Span) -> Span {
        let mut tokens: Vec<TokenSpan> = self
            .tokens
            .iter()
            .copied()
            .chain(self.side_tokens.iter().copied())
            .collect();
        tokens.sort_by_key(|token| token.span.start);
        let span = extend_span_to_line_end(&tokens, span);
        extend_span_with_trailing_statement_terminator(&tokens, span)
    }

    /// Return whether one node span contains a newline.
    #[inline]
    pub fn node_has_newline<T>(&self, node_id: LocalNodeId<T>) -> bool
    where
        T: Node,
        NodeTree: NodeTreeImpl<T>,
    {
        let node_index = node_id.id as usize;
        let cached = self.node_has_newline_by_node_id[node_index].get();
        if cached == NODE_BOOL_STATE_TRUE {
            return true;
        }
        if cached == NODE_BOOL_STATE_FALSE {
            return false;
        }

        let has_newline = self.has_newline(self.span(node_id));
        self.node_has_newline_by_node_id[node_index].set(if has_newline {
            NODE_BOOL_STATE_TRUE
        } else {
            NODE_BOOL_STATE_FALSE
        });

        has_newline
    }

    /// Get a Node from the tree.
    #[inline]
    pub fn node<T>(&self, node_id: LocalNodeId<T>) -> &T
    where
        T: Node,
        NodeTree: NodeTreeImpl<T>,
    {
        self.tree.get(node_id)
    }

    /// Get a Node from the tree.
    #[inline]
    pub fn node_type<T>(&self, node_id: LocalNodeId<T>) -> NodeType
    where
        T: Node,
        NodeTree: NodeTreeImpl<T>,
    {
        self.tree.get_node_type(node_id.id)
    }

    /// Get a parent node id and its type from the tree.
    #[inline]
    pub fn parent<T>(&self, node_id: LocalNodeId<T>) -> Option<(u32, NodeType)>
    where
        T: Node,
        NodeTree: NodeTreeImpl<T>,
    {
        let parent_id = self.parents.get(node_id);
        if let Some(parent_id) = parent_id {
            let parent_type = self.tree.get_node_type(parent_id);
            Some((parent_id, parent_type))
        } else {
            None
        }
    }

    /// Get a parent node id and its type from the tree.
    #[inline]
    pub fn parent_by_id(&self, node_id: u32) -> Option<(u32, NodeType)> {
        let parent_id = self.parents.get_by_id(node_id);
        if let Some(parent_id) = parent_id {
            let parent_type = self.tree.get_node_type(parent_id);
            Some((parent_id, parent_type))
        } else {
            None
        }
    }

    /// Get all ancestors of a node.
    #[inline]
    pub fn ancestors<T>(&self, node_id: LocalNodeId<T>) -> Vec<(u32, NodeType)>
    where
        T: Node,
        NodeTree: NodeTreeImpl<T>,
    {
        self.parents
            .get_ancestors(node_id)
            .into_iter()
            .map(|parent_id| {
                let parent_type = self.tree.get_node_type(parent_id);
                (parent_id, parent_type)
            })
            .collect()
    }

    /// Return the transparent inner expression for one expression node.
    #[inline]
    pub fn transparent_inner_expression(
        &self,
        node_id: LocalNodeId<Expression>,
    ) -> LocalNodeId<Expression> {
        let node_index = node_id.id as usize;

        if let Some(inner_expression_id) = self
            .transparent_inner_expression_by_node_id
            .get(node_index)
            .and_then(Cell::get)
        {
            return inner_expression_id;
        }

        let mut current_id = node_id;
        let mut visited_expression_indices: SmallVec<[usize; 8]> = SmallVec::new();

        loop {
            let current_index = current_id.id as usize;
            visited_expression_indices.push(current_index);

            if self.has_annotation(current_id) {
                break;
            }

            let next_id = match self.tree.get(current_id) {
                Expression::Await { expression }
                | Expression::AwaitMaybe { expression }
                | Expression::Parenthesized { expression } => Some(*expression),
                _ => None,
            };

            let Some(next_id) = next_id else {
                break;
            };
            current_id = next_id;
        }

        for expression_index in visited_expression_indices {
            if let Some(state_cell) = self
                .transparent_inner_expression_by_node_id
                .get(expression_index)
            {
                state_cell.set(Some(current_id));
            }
        }

        current_id
    }

    /// Return whether one expression appears in template-literal interpolation.
    #[inline]
    pub fn expression_is_in_template_literal_interpolation(
        &self,
        node_id: LocalNodeId<Expression>,
    ) -> bool {
        if !self.has_template_literal_markers() {
            return false;
        }

        let mut current_id = node_id.id;
        let mut visited_expression_indices: SmallVec<[usize; 8]> = SmallVec::new();

        let has_template_interpolation_ancestor = loop {
            let current_index = current_id as usize;
            let state = self
                .expression_template_interpolation_by_node_id
                .get(current_index)
                .map(Cell::get)
                .unwrap_or(TYPE_CONTEXT_STATE_UNKNOWN);
            if state == TYPE_CONTEXT_STATE_TRUE {
                break true;
            }
            if state == TYPE_CONTEXT_STATE_FALSE {
                break false;
            }

            visited_expression_indices.push(current_index);

            let Some(parent_id) = self.parents.get_by_id(current_id) else {
                break false;
            };

            let is_template_parent = self.tree.get_node_type(parent_id) == NodeType::Expression
                && matches!(
                    self.tree.get(LocalNodeId::<Expression>::new(parent_id)),
                    Expression::TemplateExpression { .. }
                );
            if is_template_parent {
                break true;
            }

            current_id = parent_id;
        };

        let state = if has_template_interpolation_ancestor {
            TYPE_CONTEXT_STATE_TRUE
        } else {
            TYPE_CONTEXT_STATE_FALSE
        };
        for expression_index in visited_expression_indices {
            if let Some(state_cell) = self
                .expression_template_interpolation_by_node_id
                .get(expression_index)
            {
                state_cell.set(state);
            }
        }

        has_template_interpolation_ancestor
    }

    /// Return whether one expression has a type-conditional ancestor.
    #[inline]
    pub fn expression_has_type_conditional_ancestor(
        &self,
        node_id: LocalNodeId<Expression>,
    ) -> bool {
        let mut current_id = node_id.id;
        let mut visited_expression_indices: SmallVec<[usize; 8]> = SmallVec::new();

        let has_type_conditional_ancestor = loop {
            let current_index = current_id as usize;
            let state = self
                .expression_type_conditional_ancestor_by_node_id
                .get(current_index)
                .map(Cell::get)
                .unwrap_or(TYPE_CONTEXT_STATE_UNKNOWN);
            if state == TYPE_CONTEXT_STATE_TRUE {
                break true;
            }
            if state == TYPE_CONTEXT_STATE_FALSE {
                break false;
            }

            visited_expression_indices.push(current_index);

            let Some(parent_id) = self.parents.get_by_id(current_id) else {
                break false;
            };

            let is_type_conditional_parent = self.tree.get_node_type(parent_id)
                == NodeType::Expression
                && matches!(
                    self.tree.get(LocalNodeId::<Expression>::new(parent_id)),
                    Expression::Type { .. }
                );
            if is_type_conditional_parent {
                break true;
            }

            current_id = parent_id;
        };

        let state = if has_type_conditional_ancestor {
            TYPE_CONTEXT_STATE_TRUE
        } else {
            TYPE_CONTEXT_STATE_FALSE
        };
        for expression_index in visited_expression_indices {
            if let Some(state_cell) = self
                .expression_type_conditional_ancestor_by_node_id
                .get(expression_index)
            {
                state_cell.set(state);
            }
        }

        has_type_conditional_ancestor
    }

    /// Get a Span from the tree.
    #[inline]
    pub fn span<T>(&self, node_id: LocalNodeId<T>) -> Span
    where
        T: Node,
        NodeTree: NodeTreeImpl<T>,
    {
        self.tree.get_span(node_id)
    }

    /// Get a Span from the tree.
    #[inline]
    pub fn span_by_id(&self, node_id: u32) -> Span {
        self.source_map.get(node_id)
    }

    /// Whether the given span has a newline.
    #[inline]
    pub fn has_newline(&self, span: Span) -> bool {
        if span.start >= span.end {
            return false;
        }

        {
            let cache = self.span_has_newline_by_span.borrow();
            if let Some(has_newline) = cache.get(&span) {
                return *has_newline;
            }
        }

        let newline_offsets = self.newline_offsets();
        let newline_index = newline_offsets.partition_point(|offset| *offset < span.start);
        let has_newline = newline_offsets
            .get(newline_index)
            .is_some_and(|offset| *offset < span.end);
        self.span_has_newline_by_span
            .borrow_mut()
            .insert(span, has_newline);
        has_newline
    }

    /// Whether the given span contains one explicit blank line in trivia.
    #[inline]
    pub fn has_blank_line(&self, span: Span) -> bool {
        if span.start >= span.end || !self.has_newline(span) {
            return false;
        }

        let newline_offsets = self.newline_offsets();
        let mut newline_index = newline_offsets.partition_point(|offset| *offset < span.start);
        let Some(mut previous_newline_offset) = newline_offsets.get(newline_index).copied() else {
            return false;
        };
        if previous_newline_offset >= span.end {
            return false;
        }

        newline_index += 1;
        while let Some(newline_offset) = newline_offsets.get(newline_index).copied() {
            if newline_offset >= span.end {
                break;
            }

            let line_span = Span::new(
                span.file,
                previous_newline_offset.saturating_add(1),
                newline_offset,
            );
            let line_has_content = token_stream_has_non_whitespace_content(self.tokens, line_span)
                || token_stream_has_non_whitespace_content(self.side_tokens, line_span);
            if !line_has_content {
                return true;
            }

            previous_newline_offset = newline_offset;
            newline_index += 1;
        }

        false
    }

    /// Whether the given span contains non-whitespace token content.
    #[inline]
    pub fn has_non_whitespace_content(&self, span: Span) -> bool {
        token_stream_has_non_whitespace_content(self.tokens, span)
            || token_stream_has_non_whitespace_content(self.side_tokens, span)
    }

    /// Whether the given span starts on a line with only leading whitespace.
    #[inline]
    pub fn span_starts_on_own_line(&self, span: Span) -> bool {
        if span.start == 0 {
            return true;
        }

        let newline_offsets = self.newline_offsets();
        let newline_index = newline_offsets.partition_point(|offset| *offset < span.start);
        let line_start = newline_index
            .checked_sub(1)
            .and_then(|index| newline_offsets.get(index).copied())
            .map_or(0, |offset| offset.saturating_add(1));
        if line_start >= span.start {
            return true;
        }

        let prefix_span = Span::new(span.file, line_start, span.start);
        !self.has_non_whitespace_content(prefix_span)
    }

    /// Return whether one span contains an own-line or multiline comment span.
    pub fn has_own_line_or_multiline_comment(&self, span: Span) -> bool {
        let first_relevant_index = self
            .comment_spans
            .partition_point(|comment_span| comment_span.end <= span.start);

        for comment_span in &self.comment_spans[first_relevant_index..] {
            if comment_span.file != span.file {
                continue;
            }

            if comment_span.start >= span.end {
                break;
            }

            if comment_span.end <= span.start {
                continue;
            }

            let is_multiline = !self
                .file
                .is_same_line(comment_span.start, comment_span.end.saturating_sub(1));
            if is_multiline || self.span_starts_on_own_line(*comment_span) {
                return true;
            }
        }

        false
    }

    /// Whether the given node is at a line start.
    /// (With no other semantic spans between it and the previous newline / start).
    pub fn is_at_line_start(&self, node_id: u32) -> bool {
        // find the token starting the node's span
        let span = self.span_by_id(node_id);
        let Some(token_idx) = self
            .tokens
            .iter()
            .position(|token| token.span.start == span.start)
        else {
            return false; // not found
        };
        if token_idx == 0 {
            return true;
        }

        // walk backward from the previous token:
        // only side-span trivia is allowed before a newline / file start
        let mut previous_token_idx = token_idx - 1;
        loop {
            let Some(previous_token) = self.tokens.get(previous_token_idx) else {
                return true;
            };
            if previous_token.token.ty == TokenType::Newline {
                return true;
            }
            if !self.side_span.contains(&previous_token.span) {
                return false;
            }
            if previous_token_idx == 0 {
                return true;
            }
            previous_token_idx -= 1;
        }
    }
}
