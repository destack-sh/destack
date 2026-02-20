use crate::analysis::scan::{
    next_non_whitespace_token_after_annotation, previous_non_whitespace_token_before_span,
};
use crate::analysis::{
    argument_has_source_separator_line_comment_annotation,
    call_arguments_preserve_blank_line_between, write_plain_call_argument,
};
use crate::expression::{
    Annotation, AnnotationPosition, Argument, DestackFormatContext, DestackFormatter, Expression,
    FormatResult, LocalNodeId, Span, TokenType, argument_value_id, block_indent, empty_line,
    format_with, group, hard_line_break, space, token,
};
use destack_ast::{Comment, CommentStyle, TokenSpan};
use destack_fir::format::Buffer;
use destack_fir::write;

/// Store one separator line comment and whether it started on its own source line.
pub(super) struct SeparatorLineCommentSource {
    /// The separator comment node ids in source order.
    comment_ids: Vec<LocalNodeId<Comment>>,
    /// Whether the comment starts on its own line after the separator comma.
    is_own_line: bool,
}

/// Return one source comma token that owns one separator slash comment seam.
fn separator_line_comment_preceding_comma(
    context: &DestackFormatContext<'_>,
    annotation_span: Span,
) -> Option<TokenSpan> {
    let mut previous_token = previous_non_whitespace_token_before_span(context, annotation_span);

    while let Some(token) = previous_token {
        if token.token.ty == TokenType::Comma {
            return Some(token);
        }

        if matches!(
            token.token.ty,
            TokenType::LineComment | TokenType::DocLineComment
        ) {
            previous_token = previous_non_whitespace_token_before_span(context, token.span);
            continue;
        }

        return None;
    }

    None
}

/// Return one separator slash comment annotation source payload.
fn separator_line_comment_annotation_source(
    context: &DestackFormatContext<'_>,
    annotation_id: LocalNodeId<Annotation>,
) -> Option<(LocalNodeId<Comment>, bool)> {
    let Annotation::Comment { node, position } = context.annotation(annotation_id) else {
        return None;
    };

    if !matches!(
        position,
        AnnotationPosition::LinePrefix
            | AnnotationPosition::LinePostfix
            | AnnotationPosition::LinePostfixBoundary
            | AnnotationPosition::BlockPostfix
    ) {
        return None;
    }

    let comment = context.tree.get::<Comment>(node);
    if comment.style != CommentStyle::Slash {
        return None;
    }

    let annotation_span = context.annotation_span(annotation_id);
    let preceding_comma = separator_line_comment_preceding_comma(context, annotation_span);
    let following_token = next_non_whitespace_token_after_annotation(context, annotation_id);
    let following_separator =
        following_token.is_some_and(|token| token.token.ty == TokenType::Comma);
    let following_close_brace =
        following_token.is_some_and(|token| token.token.ty == TokenType::CloseBrace);
    if preceding_comma.is_none() && !following_separator {
        return None;
    }
    if preceding_comma.is_some() && following_close_brace && !following_separator {
        return None;
    }

    let is_own_line = if let Some(separator_token) = preceding_comma {
        let before_comment_span = Span::new(
            annotation_span.file,
            separator_token.span.end,
            annotation_span.start,
        );
        context.has_newline(before_comment_span)
    } else if following_separator {
        let separator_start = next_non_whitespace_token_after_annotation(context, annotation_id)
            .expect("separator token should exist when checked")
            .span
            .start;
        let after_comment_span =
            Span::new(annotation_span.file, annotation_span.end, separator_start);
        context.has_newline(after_comment_span)
    } else {
        false
    };

    Some((node, is_own_line))
}

/// Return one separator comment source from one annotation list and filter.
fn separator_line_comment_source_from_annotations<F>(
    context: &DestackFormatContext<'_>,
    annotations: &[LocalNodeId<Annotation>],
    mut annotation_allowed: F,
) -> Option<SeparatorLineCommentSource>
where
    F: FnMut(LocalNodeId<Annotation>) -> bool,
{
    for (index, annotation_id) in annotations.iter().copied().enumerate() {
        if !annotation_allowed(annotation_id) {
            continue;
        }

        let Some((comment_id, is_own_line)) =
            separator_line_comment_annotation_source(context, annotation_id)
        else {
            continue;
        };

        let mut comment_ids = vec![comment_id];
        for next_annotation_id in annotations.iter().skip(index + 1).copied() {
            if !annotation_allowed(next_annotation_id) {
                break;
            }

            if matches!(
                context.annotation(next_annotation_id),
                Annotation::Blank { .. }
            ) {
                continue;
            }

            let Some((next_comment_id, _)) =
                separator_line_comment_annotation_source(context, next_annotation_id)
            else {
                break;
            };

            comment_ids.push(next_comment_id);
        }

        return Some(SeparatorLineCommentSource {
            comment_ids,
            is_own_line,
        });
    }

    None
}

/// Return one separator line comment source attached to one argument.
pub(super) fn single_argument_separator_line_comment_source(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    argument_id: LocalNodeId<Argument>,
) -> Option<SeparatorLineCommentSource> {
    let resolve_from_annotations = |annotations: &[LocalNodeId<Annotation>]| {
        separator_line_comment_source_from_annotations(context, annotations, |_| true)
    };

    if let Some(annotations) = context.annotations(argument_id)
        && let Some(comment_source) = resolve_from_annotations(&annotations)
    {
        return Some(comment_source);
    }

    let value_id = argument_value_id(context.tree, argument_id);
    let value_annotation_source = context
        .annotations(value_id)
        .and_then(|annotations| resolve_from_annotations(&annotations));
    if value_annotation_source.is_some() {
        return value_annotation_source;
    }

    let argument_span = context.span(argument_id);
    let value_span = context.span(value_id);
    let seam_start = value_span.end;
    let call_span = context.span(call_node_id);
    if value_span.file != call_span.file || seam_start >= call_span.end {
        return None;
    }

    let call_annotation_source = context.annotations(call_node_id).and_then(|annotations| {
        separator_line_comment_source_from_annotations(context, &annotations, |annotation_id| {
            let annotation_span = context.annotation_span(annotation_id);
            annotation_span.file == argument_span.file
                && annotation_span.start >= seam_start
                && annotation_span.end <= call_span.end
        })
    });
    if call_annotation_source.is_some() {
        return call_annotation_source;
    }

    None
}

/// Write one separator line comment after one argument comma.
pub(super) fn write_separator_line_comment_after_comma<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    separator_comment_source: &SeparatorLineCommentSource,
) -> FormatResult<()> {
    let Some(first_comment_id) = separator_comment_source.comment_ids.first().copied() else {
        return Ok(());
    };

    if separator_comment_source.is_own_line {
        write!(f, [token(","), hard_line_break(), first_comment_id])?;
        for comment_id in separator_comment_source.comment_ids.iter().skip(1) {
            write!(f, [hard_line_break(), comment_id])?;
        }
    } else {
        write!(f, [token(","), space(), first_comment_id])?;
        for comment_id in separator_comment_source.comment_ids.iter().skip(1) {
            write!(f, [hard_line_break(), comment_id])?;
        }
    }

    Ok(())
}

/// Format one single plain argument with a separator line comment seam.
pub(super) fn format_single_plain_argument_with_separator_line_comment<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    argument_id: LocalNodeId<Argument>,
    separator_comment_source: &SeparatorLineCommentSource,
) -> FormatResult<()> {
    write!(f, [token("("), hard_line_break()])?;
    write!(
        f,
        [block_indent(&format_with(
            |f: &mut DestackFormatter<'ast, '_>| {
                write_plain_call_argument(f, argument_id)?;
                write_separator_line_comment_after_comma(f, separator_comment_source)?;
                Ok(())
            }
        ))]
    )?;
    write!(f, [hard_line_break(), token(")")])?;

    Ok(())
}

/// Return whether one annotation is a separator line comment.
fn annotation_is_separator_line_comment(
    context: &DestackFormatContext<'_>,
    annotation_id: LocalNodeId<Annotation>,
) -> bool {
    separator_line_comment_annotation_source(context, annotation_id).is_some()
}

/// Return whether an argument has non-separator postfix or infix annotations.
fn argument_has_non_separator_postfix_or_infix_annotation(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    context
        .visit_annotations(argument_id, |annotations| {
            annotations.iter().any(|annotation_id| {
                let position = context.annotation(*annotation_id).position();
                let is_postfix_or_infix = matches!(
                    position,
                    AnnotationPosition::BlockInfix
                        | AnnotationPosition::BlockPostfix
                        | AnnotationPosition::LinePostfix
                        | AnnotationPosition::LinePostfixBoundary
                );
                is_postfix_or_infix
                    && !matches!(context.annotation(*annotation_id), Annotation::Blank { .. })
                    && !annotation_is_separator_line_comment(context, *annotation_id)
            })
        })
        .unwrap_or(false)
}

/// Write one argument without separator line comments that are emitted at list level.
pub(super) fn write_argument_without_separator_line_comment<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    argument_id: LocalNodeId<Argument>,
) -> FormatResult<bool> {
    if !argument_has_source_separator_line_comment_annotation(f.context(), argument_id) {
        return Ok(false);
    }
    if argument_has_non_separator_postfix_or_infix_annotation(f.context(), argument_id) {
        return Ok(false);
    }

    let Argument::Positional {
        modifiers: None,
        value,
    } = f.context().tree.get(argument_id)
    else {
        return Ok(false);
    };

    write!(
        f,
        [
            f.context().any_prefix_annotations(argument_id),
            group(value)
        ]
    )?;
    Ok(true)
}

/// Return whether the last argument can be rendered without separator line comment annotations.
fn last_argument_can_render_without_separator_line_comment(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    if !argument_has_source_separator_line_comment_annotation(context, argument_id) {
        return false;
    }
    if argument_has_non_separator_postfix_or_infix_annotation(context, argument_id) {
        return false;
    }

    matches!(
        context.tree.get(argument_id),
        Argument::Positional {
            modifiers: None,
            ..
        }
    )
}

/// Format a multiline call list with a trailing separator line comment after the last argument.
pub(super) fn format_multiline_call_argument_list_with_last_separator_line_comment<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
) -> FormatResult<bool> {
    let Some(last_argument_id) = dynamic_arguments.last().copied() else {
        return Ok(false);
    };
    let Some(comment_source) =
        single_argument_separator_line_comment_source(f.context(), call_node_id, last_argument_id)
    else {
        return Ok(false);
    };
    if !last_argument_can_render_without_separator_line_comment(f.context(), last_argument_id) {
        return Ok(false);
    }

    write!(f, [token("("), hard_line_break()])?;
    write!(
        f,
        [block_indent(&format_with(
            |f: &mut DestackFormatter<'ast, '_>| {
                for (index, argument_id) in dynamic_arguments.iter().enumerate() {
                    if index > 0 {
                        let left_argument_id = dynamic_arguments[index - 1];
                        if call_arguments_preserve_blank_line_between(
                            f.context(),
                            left_argument_id,
                            *argument_id,
                        ) {
                            write!(f, [empty_line()])?;
                        } else {
                            write!(f, [hard_line_break()])?;
                        }
                    }

                    let is_last_argument = index + 1 == dynamic_arguments.len();
                    if is_last_argument {
                        write_argument_without_separator_line_comment(f, *argument_id)?;
                        write_separator_line_comment_after_comma(f, &comment_source)?;
                    } else {
                        write!(f, [group(argument_id), token(",")])?;
                    }
                }
                Ok(())
            }
        ))]
    )?;
    write!(f, [hard_line_break(), token(")")])?;

    Ok(true)
}
