use crate::DestackFormatContext;
use destack_ast::{Annotation, LocalNodeId};
use destack_source::Span;

/// Return the first non-whitespace character before a span.
pub(crate) fn previous_non_whitespace_before_span(
    context: &DestackFormatContext<'_>,
    span: Span,
) -> Option<char> {
    if span.start == 0 {
        return None;
    }

    let head_span = Span::new(span.file, 0, span.start);
    let head_source = context.file.get_span_str(head_span)?;
    head_source
        .chars()
        .rev()
        .find(|character: &char| !character.is_whitespace())
}

/// Return the first non-whitespace character after a span.
pub(crate) fn next_non_whitespace_after_span(
    context: &DestackFormatContext<'_>,
    span: Span,
) -> Option<char> {
    if span.end >= context.file.len {
        return None;
    }

    let tail_span = Span::new(span.file, span.end, context.file.len);
    let tail_source = context.file.get_span_str(tail_span)?;
    tail_source
        .chars()
        .find(|character: &char| !character.is_whitespace())
}

/// Return the first non-whitespace character before an annotation span.
pub(crate) fn previous_non_whitespace_before_annotation(
    context: &DestackFormatContext<'_>,
    annotation_id: LocalNodeId<Annotation>,
) -> Option<char> {
    let span = context.get_span(annotation_id);
    previous_non_whitespace_before_span(context, span)
}

/// Return the first non-whitespace character after an annotation span.
pub(crate) fn next_non_whitespace_after_annotation(
    context: &DestackFormatContext<'_>,
    annotation_id: LocalNodeId<Annotation>,
) -> Option<char> {
    let span = context.get_span::<Annotation>(annotation_id);
    next_non_whitespace_after_span(context, span)
}
