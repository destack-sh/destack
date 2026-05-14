use super::argument::tree_argument_is_wrapped_in_braces;
use crate::DestackFormatContext;
use destack_dir::{Argument, Expression, LocalNodeId, ScalarLiteral};

/// Check whether a tree text child is whitespace-only.
pub(crate) fn tree_text_is_whitespace_only(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> Option<(bool, bool)> {
    let tree = context.tree;
    let strings = context.strings;
    let Argument::Positional { value, .. } = tree.get(argument_id) else {
        return None;
    };

    let has_annotation = context.has_annotation(argument_id) || context.has_annotation(*value);
    let wrapped_in_braces = tree_argument_is_wrapped_in_braces(context, argument_id);
    if wrapped_in_braces && has_annotation {
        return Some((false, false));
    }

    match tree.get(*value) {
        Expression::ScalarLiteral(ScalarLiteral::String(string_id)) => {
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
    }
}

/// Return whether one character is JSX whitespace.
#[inline]
pub(crate) fn is_jsx_whitespace_char(character: char) -> bool {
    matches!(character, ' ' | '\n' | '\r' | '\t')
}

/// Return whether source preserves an empty line between two tree child arguments.
pub(crate) fn tree_children_have_blank_line_between(
    context: &DestackFormatContext<'_>,
    previous_argument_id: LocalNodeId<Argument>,
    next_argument_id: LocalNodeId<Argument>,
) -> bool {
    let previous_span = context.span(previous_argument_id);
    let next_span = context.span(next_argument_id);
    let Some(between_span) = previous_span.gap_to(next_span) else {
        return false;
    };

    context.has_blank_line(between_span)
}
