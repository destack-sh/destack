use ast::TokenType;
use destack_ast as ast;

use super::seam::{CommentSeamFacts, CommentSeamKeyword};
use super::token::is_open_delimiter_token;

/// Compact fallback facts derived from seam facts.
#[derive(Debug, Clone, Copy)]
pub(super) struct CommentFallbackFacts {
    /// Whether comment has one leading newline.
    pub(super) has_leading_newline: bool,
    /// Whether comment has one trailing newline.
    pub(super) has_trailing_newline: bool,
    /// Whether comment is one multiline star block.
    pub(super) comment_is_multiline_star: bool,
    /// Whether token after seam is `else`.
    pub(super) token_after_is_else: bool,
    /// Whether token before seam is one open delimiter.
    pub(super) token_before_is_open_delimiter: bool,
    /// Whether token after seam is `<`.
    pub(super) token_after_is_less_than: bool,
    /// Whether token after seam prefers left ownership.
    pub(super) token_after_prefers_left: bool,
    /// Whether default trailing behavior should bind right.
    pub(super) seam_binds_right: bool,
}

impl CommentFallbackFacts {
    /// Build fallback facts from seam facts.
    pub(super) fn from_seam_facts(facts: CommentSeamFacts) -> Self {
        Self {
            has_leading_newline: facts.has_leading_newline,
            has_trailing_newline: facts.has_trailing_newline,
            comment_is_multiline_star: facts.comment_is_multiline_star,
            token_after_is_else: facts.token_after_is_keyword(CommentSeamKeyword::Else),
            token_before_is_open_delimiter: facts
                .token_before_type
                .is_some_and(is_open_delimiter_token),
            token_after_is_less_than: facts.token_after_is(TokenType::LessThan),
            token_after_prefers_left: facts.token_after_prefers_left,
            seam_binds_right: facts.seam_binds_right,
        }
    }
}
