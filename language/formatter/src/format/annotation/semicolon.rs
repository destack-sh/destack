use crate::{Annotation, DestackFormatContext};
use destack_ast::{AnnotationPosition, LocalNodeId, TokenType};

/// Return whether one annotation needs continuation indentation for semicolon-guard seams.
pub(crate) fn annotation_needs_semicolon_guard_continuation_indent(
    ctx: &DestackFormatContext<'_>,
    annotation_id: LocalNodeId<Annotation>,
    position: AnnotationPosition,
    is_slash_comment: bool,
    starts_on_own_line: bool,
) -> bool {
    if position != AnnotationPosition::LinePostfixBoundary || !is_slash_comment {
        return false;
    }

    if !starts_on_own_line {
        return false;
    }

    if !ctx.annotation_starts_indented(annotation_id) {
        return false;
    }

    ctx.annotation_semicolon_guard_target_token_type(annotation_id)
        == Some(TokenType::OpenParenthesis)
}
