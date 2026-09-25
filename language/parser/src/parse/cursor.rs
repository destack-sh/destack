use crate::{CommentRetention, Lexer, Tokenizer};
use std::mem;
use std::sync::Arc;
use tspp_dir::{Comment, Keyword, Token, TokenLiteral, TokenType, is_identifier_continue};
use tspp_source::{ByteRange, File};

use super::TokenProbe;

/// The tokenization mode used for the next parser-visible token.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub(crate) enum TokenMode {
    /// Use ordinary language tokenization.
    Ordinary,
    /// Use tree tag tokenization.
    TreeTag,
    /// Use tree child tokenization.
    TreeChild,
    /// Use tree attribute value tokenization.
    TreeAttributeValue,
}

/// One parser-visible replacement of ordinary source tokens.
struct TokenEdit {
    /// The ordinary source range replaced by this edit.
    range: ByteRange,
    /// The first replacement token in the cursor replacement buffer.
    replacement_start: u32,
    /// The number of replacement tokens in the cursor replacement buffer.
    replacement_len: u32,
}

/// An indexed contextual token cursor over one source file.
pub(crate) struct TokenCursor {
    /// The parsed source file.
    file: Arc<File>,
    /// The active comment retention mode.
    comment_retention: CommentRetention,
    /// The ordinary semantic tokens in source order.
    tokens: Vec<Token>,
    /// The next ordinary semantic token index.
    index: usize,
    /// The current parser-visible token.
    current: Token,
    /// Whether the parser produced the current token contextually.
    is_current_contextual: bool,
    /// The unconsumed suffix of one split compound token.
    split: Option<Token>,
    /// The last consumed parser-visible token.
    previous: Token,
    /// The parser-visible tokens consumed so far in parser tests.
    #[cfg(test)]
    consumed: Vec<Token>,
    /// The sparse parser-visible changes to the ordinary token stream.
    edits: Vec<TokenEdit>,
    /// The dense parser-visible tokens referenced by the edits.
    replacements: Vec<Token>,
    /// The structured comments collected during tokenization.
    comments: Vec<Comment>,
    /// Contextual ranges that replace ordinarily recognized comments.
    contextual_ranges: Vec<ByteRange>,
    /// The end offset of the previous parser-visible token.
    previous_end: u32,
}

impl TokenCursor {
    /// Tokenize one file and create a cursor at its first visible token.
    pub(crate) fn new(file: Arc<File>, comment_retention: CommentRetention) -> Self {
        let mut lexer = Lexer::new(file.clone());
        lexer.set_comment_retention(comment_retention);
        lexer.lex_to_end();
        let comments = lexer.take_comments();
        let tokens = lexer.take_tokens();
        let current = tokens[0];
        #[cfg(test)]
        let consumed = Vec::with_capacity(tokens.len());

        Self {
            file,
            comment_retention,
            index: 1,
            current,
            is_current_contextual: false,
            split: None,
            previous: Token::eof(0),
            #[cfg(test)]
            consumed,
            edits: Vec::new(),
            replacements: Vec::new(),
            tokens,
            comments,
            contextual_ranges: Vec::new(),
            previous_end: 0,
        }
    }

    /// Return the number of ordinary semantic tokens.
    #[inline]
    pub(crate) fn token_count(&self) -> usize {
        self.tokens.len()
    }

    /// Return retained comments in source order.
    #[inline]
    pub(crate) fn comments(&self) -> &[Comment] {
        &self.comments
    }

    /// Return the current visible token.
    #[inline(always)]
    pub(crate) const fn peek(&self) -> Token {
        self.current
    }

    /// Return the last consumed visible token.
    #[inline(always)]
    pub(crate) const fn peek_previous(&self) -> Token {
        self.previous
    }

    /// Return the end offset of the last consumed visible token.
    #[inline(always)]
    pub(crate) const fn previous_end(&self) -> u32 {
        self.previous_end
    }

    /// Return the parser-visible tokens consumed so far.
    #[cfg(test)]
    #[inline]
    pub(crate) fn consumed(&self) -> &[Token] {
        &self.consumed
    }

    /// Create a disposable lookahead probe at the current token.
    #[inline]
    pub(crate) fn probe<'source>(&'source self, file: &'source File) -> TokenProbe<'source> {
        TokenProbe::new(file, self)
    }

    /// Return one parser-visible token relative to the current token.
    #[inline(always)]
    pub(crate) fn peek_token_at(&self, offset: usize) -> Token {
        if offset == 0 {
            return self.current;
        }

        if let Some(split) = self.split {
            if offset == 1 {
                return split;
            }

            return self.peek_at_index(self.index + offset - 2);
        }

        self.peek_at_index(self.index + offset - 1)
    }

    /// Classify one parser-visible identifier token as a keyword.
    #[inline]
    pub(crate) fn classify_keyword(&self, file: &File, token: Token) -> Option<Keyword> {
        // reject non-identifier tokens
        if !token.is(TokenType::Identifier) {
            return None;
        }

        // use the packed keyword classification when available
        if let Some(keyword) = token.classified_keyword() {
            return keyword;
        }

        // classify contextually produced identifiers from source text
        crate::classify_keyword(file.span_str(token.span(file.id)))
    }

    /// Return whether one parser-visible token is the given identifier.
    #[inline]
    pub(crate) fn identifier_is(&self, file: &File, token: Token, expected: &str) -> bool {
        token.is(TokenType::Identifier) && file.span_str(token.span(file.id)) == expected
    }

    /// Advance to the next ordinarily tokenized visible token.
    #[inline(always)]
    pub(crate) fn bump(&mut self) {
        self.consume_current();
        self.read_ordinary();
    }

    /// Advance to the next visible token in one lexical mode.
    pub(crate) fn bump_with_mode(&mut self, file: &File, mode: TokenMode) {
        if mode == TokenMode::Ordinary {
            self.bump();

            return;
        }

        self.consume_current();
        self.read_in_mode(file, mode);
    }

    /// Record the current token as consumed.
    #[inline(always)]
    fn consume_current(&mut self) {
        let current = self.current;

        // record only parser-visible replacements of ordinary tokenization
        if self.is_current_contextual && !current.is(TokenType::End) {
            self.record_current_contextual_range();
            self.record_edit(current);
        }

        // advance the parser-visible source boundary
        self.previous = current;
        self.previous_end = current.end();
        if !current.is(TokenType::End) {
            #[cfg(test)]
            self.consumed.push(current);
        }
    }

    /// Replace the current compound token with one token prefix.
    pub(crate) fn split(&mut self, token_type: TokenType, prefix_len: u32) -> bool {
        let current = self.current;
        debug_assert!(self.split.is_none());
        debug_assert!(prefix_len > 0 && prefix_len < current.len());

        let remainder_type = match current.ty() {
            TokenType::LogicalAnd => TokenType::ElementwiseAnd,
            TokenType::ShiftLeft => TokenType::LessThan,
            TokenType::LessThanOrEqual => TokenType::Assign,
            TokenType::ShiftLeftAssign => TokenType::LessThanOrEqual,
            TokenType::ShiftRight => TokenType::GreaterThan,
            TokenType::UnsignedShiftRight => TokenType::ShiftRight,
            TokenType::GreaterThanOrEqual => TokenType::Assign,
            TokenType::ShiftRightAssign => TokenType::GreaterThanOrEqual,
            TokenType::UnsignedShiftRightAssign => TokenType::ShiftRightAssign,
            _ => return false,
        };

        let prefix = Token::simple(token_type, current.start(), prefix_len)
            .with_on_new_line(current.is_on_new_line());
        let remainder_start = current.start() + prefix_len;
        let remainder_len = current.len() - prefix_len;
        self.set_contextual(prefix);
        self.split = Some(Token::simple(
            remainder_type,
            remainder_start,
            remainder_len,
        ));

        true
    }

    /// Replace the current division token with one regex literal token.
    pub(crate) fn reclassify_regex(&mut self, source: &str) -> Token {
        let current = self.current;
        let bytes = source.as_bytes();
        let mut index = current.start() as usize + 1;
        let mut is_escaped = false;
        let mut is_character_class = false;

        // find the unescaped closing slash
        while index < bytes.len() {
            let byte = bytes[index];
            index += 1;

            if is_escaped {
                is_escaped = false;
            } else if byte == b'\\' {
                is_escaped = true;
            } else if is_character_class && byte == b']' {
                is_character_class = false;
            } else if !is_character_class && byte == b'[' {
                is_character_class = true;
            } else if !is_character_class && byte == b'/' {
                break;
            }
        }

        // consume authored flag identifier characters
        let flags_start = index;
        while let Some(character) = source[index..].chars().next() {
            if !is_identifier_continue(character) {
                break;
            }

            index += character.len_utf8();
        }

        let end = index as u32;
        let token = Token::new(
            TokenType::Literal,
            current.start(),
            end - current.start(),
            Some(TokenLiteral::RegexString {
                has_flags: index > flags_start,
            }),
        )
        .with_on_new_line(current.is_on_new_line());
        self.set_contextual(token);

        token
    }

    /// Record comment coverage for the current contextual literal.
    fn record_current_contextual_range(&mut self) {
        let current = self.current;
        let covers_comments = current.is(TokenType::Literal)
            && matches!(
                current.literal(),
                Some(TokenLiteral::RegexString { .. })
                    | Some(TokenLiteral::TreeString)
                    | Some(TokenLiteral::Character {
                        is_html_entity: true,
                        ..
                    })
            );
        if covers_comments {
            self.record_contextual_range(current);
        }
    }

    /// Finish and return the parser-visible semantic token stream.
    pub(crate) fn take_tokens(&mut self) -> Vec<Token> {
        // retain the current contextual token and every pending split suffix
        if self.is_current_contextual && !self.current.is(TokenType::End) {
            self.record_edit(self.current);
        }
        if let Some(split) = self.split.take() {
            self.record_edit(split);
        }

        // remove contextual comments and materialize sparse token replacements
        self.record_current_contextual_range();
        self.remove_contextual_comments();

        self.materialize_tokens()
    }

    /// Return retained comments after removing contextually covered comments.
    pub(crate) fn take_comments(&mut self) -> Vec<Comment> {
        self.finalize_comments();

        mem::take(&mut self.comments)
    }

    /// Remove comments replaced by contextual tokenization.
    pub(crate) fn finalize_comments(&mut self) {
        self.remove_contextual_comments();
    }

    /// Read one visible token in one tokenization mode.
    fn read_in_mode(&mut self, file: &File, mode: TokenMode) {
        let ordinary = self.split.unwrap_or_else(|| self.peek_ordinary());
        let start = ordinary.start();
        let is_on_new_line = ordinary.is_on_new_line();
        let token = match mode {
            TokenMode::Ordinary => return self.read_ordinary(),
            TokenMode::TreeTag => Tokenizer::tree_tag_token(file, start, is_on_new_line),
            TokenMode::TreeChild => Tokenizer::tree_child_token(file, self.previous_end, false),
            TokenMode::TreeAttributeValue => {
                let Some(token) =
                    Tokenizer::tree_attribute_value_token(file, start, is_on_new_line)
                else {
                    return self.read_ordinary();
                };

                token
            }
        };

        self.set_contextual(token);
    }

    /// Read the next ordinary token or a pending split suffix.
    #[inline(always)]
    fn read_ordinary(&mut self) {
        if let Some(split) = self.split.take() {
            self.current = split;
            self.is_current_contextual = true;

            return;
        }

        self.current = self.peek_ordinary();
        self.is_current_contextual = false;
        if self.index < self.tokens.len() {
            self.index += 1;
        }
    }

    /// Install one contextual token and skip the ordinary tokens it replaces.
    fn set_contextual(&mut self, token: Token) {
        self.current = token;
        self.is_current_contextual = true;
        self.split = None;
        self.skip_ordinary_tokens_before(token.end());
        self.retokenize_masked_comment_suffix(token);
    }

    /// Retokenize an ordinary suffix when contextual text masks a comment opener.
    fn retokenize_masked_comment_suffix(&mut self, token: Token) {
        if !self.masks_comment_suffix(token) {
            return;
        }

        // lex through the first unaffected ordinary token
        let synchronization = self.peek_ordinary();
        let lexer = Lexer::resume(self.file.clone(), token, self.comment_retention);
        let (tokens, comments) = lexer.lex_through(synchronization.start());
        debug_assert_eq!(
            tokens.last().map(|token| token.start()),
            Some(synchronization.start()),
            "contextual retokenization must reach the ordinary token stream"
        );

        // replace the damaged ordinary interval and its comments
        self.tokens.splice(self.index..self.index + 1, tokens);
        let comment_start = self
            .comments
            .partition_point(|comment| comment.span.end <= token.start());
        let comment_end = self
            .comments
            .partition_point(|comment| comment.span.start < synchronization.end());
        self.comments.splice(comment_start..comment_end, comments);
    }

    /// Return whether one contextual token starts a comment that ends beyond it.
    fn masks_comment_suffix(&self, token: Token) -> bool {
        let text = self.file.span_str(token.span(self.file.id));
        let bytes = text.as_bytes();
        let mut index = 0;

        // inspect each ordinary comment opener in the contextual range
        while index + 1 < bytes.len() {
            if bytes[index] != b'/' {
                index += 1;
                continue;
            }

            // line comments remain contained when the contextual token includes their newline
            if bytes[index + 1] == b'/' {
                let suffix = &bytes[index + 2..];
                let Some(end) = suffix.iter().position(|byte| matches!(byte, b'\n' | b'\r')) else {
                    return true;
                };

                index += end + 3;
                continue;
            }

            // block comments remain contained when their terminator is inside the token
            if bytes[index + 1] == b'*' {
                let suffix = &bytes[index + 2..];
                let Some(end) = suffix.windows(2).position(|bytes| bytes == b"*/") else {
                    return true;
                };

                index += end + 4;
                continue;
            }

            index += 1;
        }

        false
    }

    /// Return the next ordinary token without advancing.
    #[inline(always)]
    fn peek_ordinary(&self) -> Token {
        self.peek_at_index(self.index)
    }

    /// Return one indexed token, repeating EOF beyond the stream.
    #[inline(always)]
    fn peek_at_index(&self, index: usize) -> Token {
        self.tokens
            .get(index)
            .copied()
            .unwrap_or_else(|| Token::eof(self.file.len))
    }

    /// Skip ordinary tokens covered by one contextual source range.
    fn skip_ordinary_tokens_before(&mut self, end: u32) {
        while self.peek_ordinary().start() < end {
            self.index += 1;
        }
    }

    /// Record one contextual range that replaces ordinary comment recognition.
    fn record_contextual_range(&mut self, token: Token) {
        let range = token.range();

        // merge touching contextual ranges while parsing forward
        if let Some(previous) = self.contextual_ranges.last_mut()
            && range.start <= previous.end
        {
            previous.end = previous.end.max(range.end);

            return;
        }

        self.contextual_ranges.push(range);
    }

    /// Record one parser-visible token that differs from ordinary tokenization.
    fn record_edit(&mut self, token: Token) {
        let range = token.range();

        // combine adjacent replacement tokens into one edit
        if let Some(edit) = self.edits.last_mut()
            && edit.range.end == range.start
        {
            edit.range.end = range.end;
            edit.replacement_len += 1;
            self.replacements.push(token);

            return;
        }

        // append one disjoint replacement edit
        debug_assert!(
            self.edits
                .last()
                .is_none_or(|edit| edit.range.end <= range.start),
            "token edits must remain in source order"
        );
        let replacement_start = self.replacements.len() as u32;
        self.replacements.push(token);
        self.edits.push(TokenEdit {
            range,
            replacement_start,
            replacement_len: 1,
        });
    }

    /// Materialize parser-visible tokens from ordinary tokens and sparse edits.
    fn materialize_tokens(&mut self) -> Vec<Token> {
        if self.edits.is_empty() {
            return mem::take(&mut self.tokens);
        }

        // take the dense input buffers before rebuilding the visible stream
        let ordinary = mem::take(&mut self.tokens);
        let edits = mem::take(&mut self.edits);
        let replacements = mem::take(&mut self.replacements);
        let mut tokens = Vec::with_capacity(ordinary.len() + replacements.len());
        let mut ordinary_index = 0;

        // transcribe ordinary runs and parser-visible replacements in source order
        for edit in edits {
            // retain the ordinary run before this replacement
            while ordinary
                .get(ordinary_index)
                .is_some_and(|token| token.end() <= edit.range.start)
            {
                tokens.push(ordinary[ordinary_index]);
                ordinary_index += 1;
            }

            // discard ordinary tokens covered by the replacement
            while ordinary
                .get(ordinary_index)
                .is_some_and(|token| token.start() < edit.range.end)
            {
                ordinary_index += 1;
            }

            // append the replacement token range
            let replacement_start = edit.replacement_start as usize;
            let replacement_end = replacement_start + edit.replacement_len as usize;
            tokens.extend_from_slice(&replacements[replacement_start..replacement_end]);
        }

        // retain the final ordinary run
        tokens.extend_from_slice(&ordinary[ordinary_index..]);

        tokens
    }

    /// Remove comments covered by contextual tokenization.
    fn remove_contextual_comments(&mut self) {
        if self.contextual_ranges.is_empty() {
            return;
        }

        // remove covered structured comments in one ordered pass
        let mut range_index = 0;
        self.comments.retain(|comment| {
            !Self::is_range_covered(
                &self.contextual_ranges,
                &mut range_index,
                comment.span.start,
                comment.span.end,
            )
        });

        self.contextual_ranges.clear();
    }

    /// Return whether one ordered contextual range covers a source range.
    fn is_range_covered(
        ranges: &[ByteRange],
        range_index: &mut usize,
        start: u32,
        end: u32,
    ) -> bool {
        while ranges
            .get(*range_index)
            .is_some_and(|range| range.end <= start)
        {
            *range_index += 1;
        }

        ranges
            .get(*range_index)
            .is_some_and(|range| range.start <= start && end <= range.end)
    }
}
