use crate::DestackFormatContext;
use destack_dir::{Expression, LocalNodeId, ScalarLiteral, TreeChild};

/// Check whether a tree text child is whitespace-only.
pub(crate) fn tree_text_is_whitespace_only(
    context: &DestackFormatContext<'_>,
    child_id: LocalNodeId<TreeChild>,
) -> Option<(bool, bool)> {
    let tree = context.tree;
    let strings = context.strings;

    match tree.get(child_id) {
        TreeChild::Text { value } => {
            if context.has_annotation(child_id) {
                return Some((false, false));
            }

            let content = strings.get(*value);
            let has_non_whitespace = content
                .chars()
                .any(|character| !is_jsx_whitespace_char(character));
            if has_non_whitespace {
                return Some((false, false));
            }

            let has_newline = content.contains(['\n', '\r']);
            Some((true, has_newline))
        }
        TreeChild::Expression { value } => match tree.get(*value) {
            Expression::ScalarLiteral(ScalarLiteral::String(string_id)) => {
                let has_annotation =
                    context.has_annotation(child_id) || context.has_annotation(*value);
                if has_annotation {
                    return Some((false, false));
                }

                let content = strings.get(*string_id);
                let has_non_whitespace = content
                    .chars()
                    .any(|character| !is_jsx_whitespace_char(character));
                if has_non_whitespace {
                    return Some((false, false));
                }

                let has_newline = content.contains(['\n', '\r']);
                Some((true, has_newline))
            }
            Expression::ScalarLiteral(ScalarLiteral::Character(value)) => {
                if !is_jsx_whitespace_char(*value) {
                    return Some((false, false));
                }

                let has_newline = matches!(value, '\n' | '\r');
                Some((true, has_newline))
            }
            _ => None,
        },
        TreeChild::Spread { .. } | TreeChild::Tree { .. } | TreeChild::Error => None,
    }
}

/// Return the raw text for one tree text child.
pub(crate) fn tree_text_child_text<'context>(
    context: &'context DestackFormatContext<'_>,
    child_id: LocalNodeId<TreeChild>,
) -> Option<&'context str> {
    let TreeChild::Text { value } = context.tree.get(child_id) else {
        return None;
    };

    Some(context.strings.get(*value))
}

/// Return whether one child is a braced string-space expression.
pub(crate) fn tree_child_is_jsx_space_expression(
    context: &DestackFormatContext<'_>,
    child_id: LocalNodeId<TreeChild>,
) -> bool {
    if context
        .comments()
        .has_comment_in_span(context.span(child_id))
    {
        return false;
    }

    let TreeChild::Expression { value } = context.tree.get(child_id) else {
        return false;
    };
    let Expression::ScalarLiteral(ScalarLiteral::String(string_id)) = context.tree.get(*value)
    else {
        return false;
    };

    context.strings.get(*string_id) == " "
}

/// Return whether source preserves an empty line between two tree children.
pub(crate) fn tree_children_have_blank_line_between(
    context: &DestackFormatContext<'_>,
    previous_child_id: LocalNodeId<TreeChild>,
    next_child_id: LocalNodeId<TreeChild>,
) -> bool {
    let previous_span = context.span(previous_child_id);
    let next_span = context.span(next_child_id);
    let Some(between_span) = previous_span.gap_to(next_span) else {
        return false;
    };

    context.has_blank_line(between_span)
}

/// Return whether one character is JSX whitespace.
#[inline]
pub(crate) fn is_jsx_whitespace_char(character: char) -> bool {
    matches!(character, ' ' | '\n' | '\r' | '\t')
}
