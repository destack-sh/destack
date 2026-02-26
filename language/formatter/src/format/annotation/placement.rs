use super::boundary::{CommentSeamContext, CommentSeamData};

/// One normalized placement class for one comment seam.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CommentPlacement {
    /// One own-line comment with one leading newline before the comment token.
    OwnLine,
    /// One end-of-line comment that terminates one line or file tail.
    EndOfLine,
    /// One remaining inline comment between two semantic tokens.
    Remaining,
}

/// Classify one comment seam into one normalized placement class.
pub(crate) fn classify_comment_placement(
    context: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
) -> CommentPlacement {
    if seam.has_leading_newline {
        return CommentPlacement::OwnLine;
    }

    if seam.has_trailing_newline || context.token_after.is_none() {
        return CommentPlacement::EndOfLine;
    }

    CommentPlacement::Remaining
}
