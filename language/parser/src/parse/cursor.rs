use crate::{Lexer, ParserTriviaMode, TokenProbe, Tokenizer};
use destack_dir::{Comment, Token, TokenLiteral, TokenType};
use destack_source::{ByteRange, File};
use std::mem;

/// The tokenization mode used for the next parser-visible token.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub(crate) enum TokenMode {
    /// Use regular language tokenization.
    Normal,
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
    /// The source byte length used to synthesize repeated EOF tokens.
    source_len: u32,
    /// The ordinary semantic tokens in source order.
    tokens: Vec<Token>,
    /// The next ordinary semantic token index.
    index: usize,
    /// The current parser-visible token.
    current: Token,
    /// Whether the current parser-visible token is unchanged ordinary input.
    is_current_ordinary: bool,
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
    /// The retained non-semantic tokens in source order.
    side_tokens: Vec<Token>,
    /// The structured comments collected during tokenization.
    comments: Vec<Comment>,
    /// Contextual ranges whose ordinary trivia classification is invalid.
    contextual_ranges: Vec<ByteRange>,
    /// The end offset of the previous parser-visible token.
    previous_end: u32,
}

impl TokenCursor {
    /// Tokenize one file and create a cursor at its first visible token.
    pub(crate) fn new(file: &File, trivia_mode: ParserTriviaMode) -> Self {
        let mut lexer = Lexer::new(std::sync::Arc::new(file.clone()));
        lexer.set_trivia_mode(trivia_mode);
        lexer.lex_to_end();
        let comments = lexer.take_comments();
        let (tokens, side_tokens) = lexer.take_tokens();
        let current = tokens[0];
        #[cfg(test)]
        let consumed = Vec::with_capacity(tokens.len());

        Self {
            source_len: file.len,
            index: 1,
            current,
            is_current_ordinary: true,
            split: None,
            previous: Token::eof(0),
            #[cfg(test)]
            consumed,
            edits: Vec::new(),
            replacements: Vec::new(),
            tokens,
            side_tokens,
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

    /// Return the number of retained structured comments.
    #[inline]
    pub(crate) fn comment_count(&self) -> usize {
        self.comments.len()
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

    /// Create a disposable indexed ordinary-token probe at the parser cursor.
    #[inline]
    pub(crate) fn probe<'source>(&'source self, file: &'source File) -> TokenProbe<'source> {
        TokenProbe::new(file, &self.tokens, self.index, self.current, self.split)
    }

    /// Return one ordinary token relative to the parser cursor.
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

    /// Advance to the next regularly tokenized visible token.
    #[inline(always)]
    pub(crate) fn bump(&mut self) {
        self.consume_current();
        (self.current, self.is_current_ordinary) = self.read_ordinary();
    }

    /// Advance to the next visible token in one lexical mode.
    pub(crate) fn bump_with_mode(&mut self, file: &File, mode: TokenMode) {
        if mode == TokenMode::Normal {
            self.bump();

            return;
        }

        self.consume_current();
        (self.current, self.is_current_ordinary) = self.read_in_mode(file, mode);
    }

    /// Record the current token as consumed.
    #[inline(always)]
    fn consume_current(&mut self) {
        self.record_tree_text_range();

        let current = self.current;
        self.previous = current;
        self.previous_end = current.end();
        if !current.is(TokenType::End) {
            #[cfg(test)]
            self.consumed.push(current);

            if !self.is_current_ordinary {
                self.record_edit(current);
            }
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
        self.current = prefix;
        self.is_current_ordinary = false;
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
        let mut index = current.start() as usize + 1;
        let mut is_escaped = false;
        let mut is_character_class = false;

        // find the unescaped closing slash
        while index < source.len() {
            let Some(character) = source[index..].chars().next() else {
                break;
            };
            index += character.len_utf8();

            if is_escaped {
                is_escaped = false;
            } else if character == '\\' {
                is_escaped = true;
            } else if is_character_class && character == ']' {
                is_character_class = false;
            } else if !is_character_class && character == '[' {
                is_character_class = true;
            } else if !is_character_class && character == '/' {
                break;
            }
        }

        // consume ASCII flags
        let flags_start = index;
        while source
            .as_bytes()
            .get(index)
            .is_some_and(u8::is_ascii_alphabetic)
        {
            index += 1;
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
        self.current = token;
        self.is_current_ordinary = false;
        self.split = None;
        self.skip_ordinary_tokens_before(end);
        self.record_contextual_range(token);

        token
    }

    /// Record trivia coverage for one contextual tree-text token.
    fn record_tree_text_range(&mut self) {
        let current = self.current;
        let is_tree_text = current.is(TokenType::Literal)
            && matches!(
                current.literal(),
                Some(TokenLiteral::TreeString)
                    | Some(TokenLiteral::Character {
                        is_html_entity: true,
                        ..
                    })
            );
        if is_tree_text {
            self.record_contextual_range(current);
        }
    }

    /// Finish the parser-visible token stream and return both token buffers.
    pub(crate) fn take_tokens(&mut self) -> (Vec<Token>, Vec<Token>) {
        // retain the current contextual token and every pending split suffix
        if !self.is_current_ordinary && !self.current.is(TokenType::End) {
            self.record_edit(self.current);
        }
        if let Some(split) = self.split.take() {
            self.record_edit(split);
        }

        // finalize contextual trivia and materialize sparse token replacements
        self.record_tree_text_range();
        self.remove_contextual_trivia();
        let tokens = self.materialize_tokens();

        (tokens, mem::take(&mut self.side_tokens))
    }

    /// Return retained comments after removing contextually covered trivia.
    pub(crate) fn take_comments(&mut self) -> Vec<Comment> {
        self.remove_contextual_trivia();

        mem::take(&mut self.comments)
    }

    /// Read one visible token in one tokenization mode.
    fn read_in_mode(&mut self, file: &File, mode: TokenMode) -> (Token, bool) {
        let ordinary = self.split.unwrap_or_else(|| self.peek_ordinary());
        let start = ordinary.start();
        let is_on_new_line = ordinary.is_on_new_line();
        let token = match mode {
            TokenMode::Normal => return self.read_ordinary(),
            TokenMode::TreeTag => {
                self.split = None;
                Tokenizer::tree_tag_token(file, start, is_on_new_line)
            }
            TokenMode::TreeChild => {
                self.split = None;
                Tokenizer::tree_child_token(file, self.previous_end, false)
            }
            TokenMode::TreeAttributeValue => {
                let Some(token) =
                    Tokenizer::tree_attribute_value_token(file, start, is_on_new_line)
                else {
                    return self.read_ordinary();
                };
                self.split = None;

                token
            }
        };
        self.skip_ordinary_tokens_before(token.end());

        (token, false)
    }

    /// Read the next ordinary token or a pending split suffix.
    #[inline(always)]
    fn read_ordinary(&mut self) -> (Token, bool) {
        if let Some(split) = self.split.take() {
            return (split, false);
        }

        let token = self.peek_ordinary();
        if self.index < self.tokens.len() {
            self.index += 1;
        }

        (token, true)
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
            .unwrap_or_else(|| Token::eof(self.source_len))
    }

    /// Skip ordinary tokens covered by one contextual source range.
    fn skip_ordinary_tokens_before(&mut self, end: u32) {
        while self.peek_ordinary().start() < end {
            self.index += 1;
        }
    }

    /// Record one contextual range that invalidates ordinary trivia tokens.
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

    /// Remove ordinary trivia covered by contextual tokenization.
    fn remove_contextual_trivia(&mut self) {
        if self.contextual_ranges.is_empty() {
            return;
        }

        // remove covered side tokens in one ordered pass
        let mut range_index = 0;
        self.side_tokens.retain(|token| {
            !Self::is_range_covered(
                &self.contextual_ranges,
                &mut range_index,
                token.start(),
                token.end(),
            )
        });

        // remove covered structured comments in one ordered pass
        range_index = 0;
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
