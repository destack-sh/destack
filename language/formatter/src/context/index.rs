use crate::file::comment_text_has_ignore_directive_marker;

use destack_dir::{TokenSpan, TokenType};
use destack_source::File;

/// Immutable source lookups for one formatter pass.
#[derive(Debug)]
pub struct FormatSourceIndex {
    /// Newline byte offsets in file text.
    newline_offsets: Vec<u32>,
    /// Comment tokens in source order.
    comment_tokens: Vec<TokenSpan>,
    /// Whether file text contains formatter ignore directive markers.
    has_ignore_directive_markers: bool,
}

impl FormatSourceIndex {
    /// Build source lookups for one parsed file.
    pub(crate) fn new(file: &File, side_tokens: &[TokenSpan]) -> Self {
        let newline_offsets = collect_newline_offsets(file);
        let comment_tokens = collect_comment_tokens(side_tokens);
        let has_ignore_directive_markers = comment_tokens
            .iter()
            .any(|token| comment_text_has_ignore_directive_marker(file.span_str(token.span)));

        Self {
            newline_offsets,
            comment_tokens,
            has_ignore_directive_markers,
        }
    }

    /// Return byte offsets of all newline characters in the source file.
    #[inline]
    pub(crate) fn newline_offsets(&self) -> &[u32] {
        &self.newline_offsets
    }

    /// Return comment tokens in source order.
    #[inline]
    pub(crate) fn comment_tokens(&self) -> &[TokenSpan] {
        &self.comment_tokens
    }

    /// Return whether file text contains formatter ignore directive markers.
    #[inline]
    pub(crate) fn has_ignore_directive_markers(&self) -> bool {
        self.has_ignore_directive_markers
    }
}

/// Collect all newline byte offsets in one source file.
fn collect_newline_offsets(file: &File) -> Vec<u32> {
    if let Some(line_start_offsets) = file.line_start_offsets() {
        return line_start_offsets
            .iter()
            .copied()
            .skip(1)
            .map(|line_start| line_start - 1)
            .collect();
    }

    file.text()
        .bytes()
        .enumerate()
        .filter_map(|(index, byte)| (byte == b'\n').then_some(index as u32))
        .collect()
}

/// Collect comment tokens from the side token stream.
fn collect_comment_tokens(side_tokens: &[TokenSpan]) -> Vec<TokenSpan> {
    side_tokens
        .iter()
        .copied()
        .filter(|token| is_comment_token_type(token.token.ty()))
        .collect()
}

/// Return whether one token type is a comment token.
fn is_comment_token_type(token_type: TokenType) -> bool {
    matches!(
        token_type,
        TokenType::LineComment
            | TokenType::BlockComment
            | TokenType::DocLineComment
            | TokenType::DocBlockComment
    )
}
