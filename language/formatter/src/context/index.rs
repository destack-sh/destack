use crate::file::comment_text_has_ignore_directive_marker;
use destack_dir::{TokenSpan, TokenType};
use destack_source::File;

/// Immutable source lookups shared by cloned formatter contexts.
#[derive(Debug)]
pub struct FormatSourceIndex {
    /// Newline byte offsets in file text.
    newline_offsets: Vec<u32>,
    /// Comment tokens sorted by source position.
    comment_tokens: Vec<TokenSpan>,
    /// Tokens across main and side streams sorted by source position.
    all_tokens: Vec<TokenSpan>,
    /// Whether file text contains formatter ignore directive markers.
    has_ignore_directive_markers: bool,
}

impl FormatSourceIndex {
    /// Build source lookups for one parsed file.
    pub(crate) fn new(file: &File, tokens: &[TokenSpan], side_tokens: &[TokenSpan]) -> Self {
        let newline_offsets = collect_newline_offsets(file.text());
        let all_tokens = merge_tokens_by_start(tokens, side_tokens);
        let comment_tokens = collect_comment_tokens(&all_tokens);
        let has_ignore_directive_markers = comment_tokens
            .iter()
            .any(|token| comment_text_has_ignore_directive_marker(file.span_str(token.span)));

        Self {
            newline_offsets,
            comment_tokens,
            all_tokens,
            has_ignore_directive_markers,
        }
    }

    /// Return byte offsets of all newline characters in the source file.
    #[inline]
    pub(crate) fn newline_offsets(&self) -> &[u32] {
        &self.newline_offsets
    }

    /// Return comment tokens sorted by source position.
    #[inline]
    pub(crate) fn comment_tokens(&self) -> &[TokenSpan] {
        &self.comment_tokens
    }

    /// Return all tokens across main and side streams sorted by source position.
    #[inline]
    pub(crate) fn all_tokens(&self) -> &[TokenSpan] {
        &self.all_tokens
    }

    /// Return whether file text contains formatter ignore directive markers.
    #[inline]
    pub(crate) fn has_ignore_directive_markers(&self) -> bool {
        self.has_ignore_directive_markers
    }
}

/// Collect all newline byte offsets in one source text.
fn collect_newline_offsets(text: &str) -> Vec<u32> {
    text.bytes()
        .enumerate()
        .filter_map(|(index, byte)| (byte == b'\n').then_some(index as u32))
        .collect()
}

/// Collect comment tokens from a sorted token stream.
fn collect_comment_tokens(tokens: &[TokenSpan]) -> Vec<TokenSpan> {
    tokens
        .iter()
        .copied()
        .filter(|token| token_type_is_comment(token.token.ty))
        .collect()
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

/// Merge main and side token streams by source start.
fn merge_tokens_by_start(tokens: &[TokenSpan], side_tokens: &[TokenSpan]) -> Vec<TokenSpan> {
    let mut merged = Vec::with_capacity(tokens.len() + side_tokens.len());
    let mut token_index = 0usize;
    let mut side_token_index = 0usize;

    // merge sorted streams
    while token_index < tokens.len() && side_token_index < side_tokens.len() {
        let token = tokens[token_index];
        let side_token = side_tokens[side_token_index];

        if token.span.start <= side_token.span.start {
            merged.push(token);
            token_index += 1;
        } else {
            merged.push(side_token);
            side_token_index += 1;
        }
    }

    // append the remaining tail
    merged.extend_from_slice(&tokens[token_index..]);
    merged.extend_from_slice(&side_tokens[side_token_index..]);

    merged
}
