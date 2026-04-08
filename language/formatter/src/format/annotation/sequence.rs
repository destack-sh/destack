use super::trivia::format_raw_comment;
use crate::{Annotation, DestackFormatContext, DestackFormatter, FormatNode};
use destack_ast::{
    AnnotationPosition, Comment, LocalNodeId, Node, NodeTree, NodeTreeImpl, TokenType,
};
use destack_fir::format::{Format, FormatResult};
use destack_fir::prelude::{format_with, *};
use destack_fir::write;
use destack_source::Span;

/// One source-ordered prefix item.
#[derive(Debug, Copy, Clone)]
enum PrefixSequenceItem {
    /// One targeted comment trivia node.
    Comment(Comment),
    /// One semantic annotation node.
    Annotation(LocalNodeId<Annotation>),
}

impl<'ast> FormatNode<'ast, Annotation> for Annotation {
    /// Format one annotation wrapper node.
    fn format_node(
        &self,
        _node_id: LocalNodeId<Annotation>,
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        match self {
            Annotation::Decorator { node, .. } => node.format(f),
        }
    }
}

/// The prefix item subset to format.
#[derive(Debug, Copy, Clone)]
enum PrefixSequenceKind {
    /// Format comments and all prefix annotations.
    All,
    /// Format comments and only non-decorator prefix annotations.
    WithoutDecorators,
    /// Format comments and only decorator prefix annotations.
    DecoratorsOnly,
}

/// Return raw prefix comments for one node in source order.
pub(crate) fn raw_prefix_comment_nodes<'ast, T>(
    context: &DestackFormatContext<'ast>,
    node_id: LocalNodeId<T>,
) -> Vec<Comment>
where
    T: Node + Clone + 'ast,
    NodeTree: NodeTreeImpl<T>,
{
    let owner_span = context.span(node_id);

    context
        .comments()
        .comments_before(owner_span.start)
        .iter()
        .copied()
        .filter(|comment: &Comment| comment.is_leading() && comment.attached_to == owner_span.start)
        .collect()
}

/// Return raw prefix comments for one node that start at or after one offset.
pub(crate) fn raw_prefix_comments_after_offset<'ast, T>(
    context: &DestackFormatContext<'ast>,
    node_id: LocalNodeId<T>,
    start_offset: u32,
) -> Vec<Comment>
where
    T: Node + Clone + 'ast,
    NodeTree: NodeTreeImpl<T>,
{
    raw_prefix_comment_nodes(context, node_id)
        .into_iter()
        .filter(|comment| comment.span.start >= start_offset)
        .collect()
}

/// Format one prepared annotation sequence.
pub(crate) fn write_annotation_sequence<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    items: &[LocalNodeId<Annotation>],
) -> FormatResult<()> {
    write_annotation_sequence_with_trailing_break(f, items, true)
}

/// Format one prepared annotation sequence without a trailing break.
pub(crate) fn write_annotation_sequence_without_trailing_break<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    items: &[LocalNodeId<Annotation>],
) -> FormatResult<()> {
    write_annotation_sequence_with_trailing_break(f, items, false)
}

/// Format inline prefix annotations without introducing formatter-owned line breaks.
pub(crate) fn write_inline_prefix_annotations<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    items: &[LocalNodeId<Annotation>],
) -> FormatResult<()> {
    let mut wrote_annotation = false;

    for annotation_id in items.iter().copied() {
        if wrote_annotation {
            write!(f, [space()])?;
        }

        f.context()
            .annotation(annotation_id)
            .format_node(annotation_id, f)?;
        wrote_annotation = true;
    }

    Ok(())
}

/// Format block infix annotations for one node.
pub(crate) fn block_infix_annotations<'ast, T>(
    context: &DestackFormatContext<'ast>,
    node_id: LocalNodeId<T>,
) -> impl Format<DestackFormatContext<'ast>> + use<'ast, T>
where
    T: Node + Clone + 'ast,
    NodeTree: NodeTreeImpl<T>,
{
    let mut items = Vec::new();
    for annotation_id in context.annotation_ids(node_id).iter().copied() {
        if context.annotation(annotation_id).position() == AnnotationPosition::BlockInfix {
            items.push(annotation_id);
        }
    }

    format_with(move |f: &mut DestackFormatter<'ast, '_>| write_annotation_sequence(f, &items))
}

/// Format line postfix boundary annotations for one node.
pub(crate) fn line_suffix_boundary_annotations<'ast, T>(
    context: &DestackFormatContext<'ast>,
    node_id: LocalNodeId<T>,
) -> impl Format<DestackFormatContext<'ast>> + use<'ast, T>
where
    T: Node + Clone + 'ast,
    NodeTree: NodeTreeImpl<T>,
{
    let mut items = Vec::new();
    for annotation_id in context.annotation_ids(node_id).iter().copied() {
        if context.annotation(annotation_id).position() == AnnotationPosition::LinePostfixBoundary {
            items.push(annotation_id);
        }
    }

    format_with(move |f: &mut DestackFormatter<'ast, '_>| write_annotation_sequence(f, &items))
}

/// Format prefix annotations for one node.
pub(crate) fn prefix_annotations<'ast, T>(
    context: &DestackFormatContext<'ast>,
    node_id: LocalNodeId<T>,
) -> impl Format<DestackFormatContext<'ast>> + use<'ast, T>
where
    T: Node + Clone + 'ast,
    NodeTree: NodeTreeImpl<T>,
{
    prefix_sequence(
        context,
        node_id,
        raw_prefix_comment_nodes(context, node_id),
        PrefixSequenceKind::All,
    )
}

/// Format prefix annotations for one node after the raw prefix comment boundary.
pub(crate) fn prefix_annotations_after_offset<'ast, T>(
    context: &DestackFormatContext<'ast>,
    node_id: LocalNodeId<T>,
    start_offset: u32,
) -> impl Format<DestackFormatContext<'ast>> + use<'ast, T>
where
    T: Node + Clone + 'ast,
    NodeTree: NodeTreeImpl<T>,
{
    prefix_sequence(
        context,
        node_id,
        raw_prefix_comments_after_offset(context, node_id, start_offset),
        PrefixSequenceKind::All,
    )
}

/// Format one prefix sequence for one node.
fn prefix_sequence<'ast, T>(
    context: &DestackFormatContext<'ast>,
    node_id: LocalNodeId<T>,
    raw_comments: Vec<Comment>,
    kind: PrefixSequenceKind,
) -> impl Format<DestackFormatContext<'ast>> + use<'ast, T>
where
    T: Node + Clone + 'ast,
    NodeTree: NodeTreeImpl<T>,
{
    let items = collect_prefix_sequence_items(context, node_id, raw_comments, kind);

    format_with(move |f: &mut DestackFormatter<'ast, '_>| write_prefix_sequence_items(f, &items))
}

/// Format prefix annotations for one node without decorator items.
pub(crate) fn prefix_annotations_without_decorators<'ast, T>(
    context: &DestackFormatContext<'ast>,
    node_id: LocalNodeId<T>,
) -> impl Format<DestackFormatContext<'ast>> + use<'ast, T>
where
    T: Node + Clone + 'ast,
    NodeTree: NodeTreeImpl<T>,
{
    let first_decorator_start =
        context
            .annotation_ids(node_id)
            .iter()
            .copied()
            .find_map(|annotation_id| {
                matches!(
                    context.annotation(annotation_id),
                    Annotation::Decorator { .. }
                )
                .then(|| context.annotation_span(annotation_id).start)
            });
    let comments = raw_prefix_comment_nodes(context, node_id)
        .into_iter()
        .filter(|comment| {
            first_decorator_start.is_none_or(|decorator_start| comment.span.end <= decorator_start)
        })
        .collect();

    prefix_sequence(
        context,
        node_id,
        comments,
        PrefixSequenceKind::WithoutDecorators,
    )
}

/// Format decorator prefix annotations for one node with grouped class-style spacing.
pub(crate) fn decorator_prefix_annotations<'ast, T>(
    context: &DestackFormatContext<'ast>,
    node_id: LocalNodeId<T>,
) -> impl Format<DestackFormatContext<'ast>> + use<'ast, T>
where
    T: Node + Clone + 'ast,
    NodeTree: NodeTreeImpl<T>,
{
    let owner_span = context.span(node_id);
    let first_decorator_start =
        context
            .annotation_ids(node_id)
            .iter()
            .copied()
            .find_map(|annotation_id| {
                matches!(
                    context.annotation(annotation_id),
                    Annotation::Decorator { .. }
                )
                .then(|| context.annotation_span(annotation_id).start)
            });
    let comments = context
        .comments()
        .comments_before(owner_span.start)
        .iter()
        .copied()
        .filter(|comment| comment.is_leading())
        .filter(|comment| {
            first_decorator_start.is_some_and(|decorator_start| {
                comment.attached_to >= decorator_start && comment.attached_to <= owner_span.start
            })
        })
        .collect();

    let items = collect_prefix_sequence_items(
        context,
        node_id,
        comments,
        PrefixSequenceKind::DecoratorsOnly,
    );

    format_with(move |f: &mut DestackFormatter<'ast, '_>| {
        write_grouped_prefix_sequence_items(f, &items)
    })
}

/// Collect one prefix item sequence for one node.
fn collect_prefix_sequence_items<'ast, T>(
    context: &DestackFormatContext<'ast>,
    node_id: LocalNodeId<T>,
    raw_comments: Vec<Comment>,
    kind: PrefixSequenceKind,
) -> Vec<PrefixSequenceItem>
where
    T: Node + Clone + 'ast,
    NodeTree: NodeTreeImpl<T>,
{
    let mut items = Vec::new();

    for comment in raw_comments {
        items.push(PrefixSequenceItem::Comment(comment));
    }

    for annotation_id in context.annotation_ids(node_id).iter().copied() {
        if !matches!(
            context.annotation(annotation_id).position(),
            AnnotationPosition::BlockPrefix | AnnotationPosition::LinePrefix
        ) {
            continue;
        }

        let is_decorator = matches!(
            context.annotation(annotation_id),
            Annotation::Decorator { .. }
        );
        let should_include = match kind {
            PrefixSequenceKind::All => true,
            PrefixSequenceKind::WithoutDecorators => !is_decorator,
            PrefixSequenceKind::DecoratorsOnly => is_decorator,
        };
        if should_include {
            items.push(PrefixSequenceItem::Annotation(annotation_id));
        }
    }

    sort_prefix_sequence_items(context, &mut items);

    items
}

/// Sort one prefix item sequence into source order.
fn sort_prefix_sequence_items(
    context: &DestackFormatContext<'_>,
    items: &mut [PrefixSequenceItem],
) {
    items.sort_by(|left, right| {
        let left_span = prefix_sequence_item_span(context, *left);
        let right_span = prefix_sequence_item_span(context, *right);

        left_span
            .start
            .cmp(&right_span.start)
            .then(left_span.end.cmp(&right_span.end))
            .then_with(|| match (*left, *right) {
                (
                    PrefixSequenceItem::Comment(left_comment),
                    PrefixSequenceItem::Comment(right_comment),
                ) => left_comment
                    .span
                    .start
                    .cmp(&right_comment.span.start)
                    .then(left_comment.span.end.cmp(&right_comment.span.end)),
                (
                    PrefixSequenceItem::Annotation(left_id),
                    PrefixSequenceItem::Annotation(right_id),
                ) => left_id.id.cmp(&right_id.id),
                (PrefixSequenceItem::Comment(_), PrefixSequenceItem::Annotation(_)) => {
                    std::cmp::Ordering::Less
                }
                (PrefixSequenceItem::Annotation(_), PrefixSequenceItem::Comment(_)) => {
                    std::cmp::Ordering::Greater
                }
            })
    });
}

/// Return the span for one prefix sequence item.
fn prefix_sequence_item_span(context: &DestackFormatContext<'_>, item: PrefixSequenceItem) -> Span {
    match item {
        PrefixSequenceItem::Comment(comment) => comment.span,
        PrefixSequenceItem::Annotation(annotation_id) => context.annotation_span(annotation_id),
    }
}

/// Write one prefix item.
fn write_prefix_sequence_item<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    item: PrefixSequenceItem,
) -> FormatResult<()> {
    match item {
        PrefixSequenceItem::Comment(comment) => format_raw_comment(f, comment),
        PrefixSequenceItem::Annotation(annotation_id) => f
            .context()
            .annotation(annotation_id)
            .format_node(annotation_id, f),
    }
}

/// Write one plain prefix item sequence.
fn write_prefix_sequence_items<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    items: &[PrefixSequenceItem],
) -> FormatResult<()> {
    for item in items.iter().copied() {
        write_prefix_sequence_item(f, item)?;

        let item_span = prefix_sequence_item_span(f.context(), item);
        if f.context()
            .span_has_newline_before_next_non_whitespace_token(item_span)
        {
            write!(f, [hard_line_break()])?;
        } else {
            write!(f, [space()])?;
        }
    }

    Ok(())
}

/// Write one grouped decorator prefix sequence.
fn write_grouped_prefix_sequence_items<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    items: &[PrefixSequenceItem],
) -> FormatResult<()> {
    if items.is_empty() {
        return Ok(());
    }

    let should_expand = items.iter().copied().any(|item| {
        let item_span = prefix_sequence_item_span(f.context(), item);
        f.context()
            .span_has_newline_before_next_non_whitespace_token(item_span)
    });

    write!(
        f,
        [group(&format_with(|f: &mut DestackFormatter<'ast, '_>| {
            let separator = soft_line_break_or_space();
            let mut join = f.join_with(&separator);

            for item in items.iter().copied() {
                join.entry(&format_with(move |f: &mut DestackFormatter<'ast, '_>| {
                    write_prefix_sequence_item(f, item)
                }));
            }
            join.finish()?;

            let last_item = items[items.len() - 1];
            let last_item_span = prefix_sequence_item_span(f.context(), last_item);
            if f.context()
                .span_has_newline_before_next_non_whitespace_token(last_item_span)
            {
                write!(f, [hard_line_break()])
            } else {
                write!(f, [space()])
            }
        }))
        .should_expand(should_expand)]
    )
}

/// Format postfix annotations for one node.
pub(crate) fn postfix_annotations<'ast, T>(
    context: &DestackFormatContext<'ast>,
    node_id: LocalNodeId<T>,
) -> impl Format<DestackFormatContext<'ast>> + use<'ast, T>
where
    T: Node + Clone + 'ast,
    NodeTree: NodeTreeImpl<T>,
{
    let mut items = Vec::new();
    for annotation_id in context.annotation_ids(node_id).iter().copied() {
        if matches!(
            context.annotation(annotation_id).position(),
            AnnotationPosition::BlockPostfix
                | AnnotationPosition::LinePostfix
                | AnnotationPosition::LinePostfixBoundary
        ) {
            items.push(annotation_id);
        }
    }

    format_with(move |f: &mut DestackFormatter<'ast, '_>| write_annotation_sequence(f, &items))
}

/// Format postfix annotations for one node without line postfix boundary items.
pub(crate) fn postfix_annotations_without_line_suffix_boundary<'ast, T>(
    context: &DestackFormatContext<'ast>,
    node_id: LocalNodeId<T>,
) -> impl Format<DestackFormatContext<'ast>> + use<'ast, T>
where
    T: Node + Clone + 'ast,
    NodeTree: NodeTreeImpl<T>,
{
    let mut items = Vec::new();
    for annotation_id in context.annotation_ids(node_id).iter().copied() {
        if matches!(
            context.annotation(annotation_id).position(),
            AnnotationPosition::BlockPostfix | AnnotationPosition::LinePostfix
        ) {
            items.push(annotation_id);
        }
    }

    format_with(move |f: &mut DestackFormatter<'ast, '_>| write_annotation_sequence(f, &items))
}

/// Format infix or postfix annotations for one node.
pub(crate) fn infix_or_postfix_annotations<'ast, T>(
    context: &DestackFormatContext<'ast>,
    node_id: LocalNodeId<T>,
) -> impl Format<DestackFormatContext<'ast>> + use<'ast, T>
where
    T: Node + Clone + 'ast,
    NodeTree: NodeTreeImpl<T>,
{
    let mut items = Vec::new();
    for annotation_id in context.annotation_ids(node_id).iter().copied() {
        if matches!(
            context.annotation(annotation_id).position(),
            AnnotationPosition::BlockInfix
                | AnnotationPosition::BlockPostfix
                | AnnotationPosition::LinePostfix
                | AnnotationPosition::LinePostfixBoundary
        ) {
            items.push(annotation_id);
        }
    }

    format_with(move |f: &mut DestackFormatter<'ast, '_>| write_annotation_sequence(f, &items))
}

/// Format infix or postfix annotations for one node without line postfix boundary items.
pub(crate) fn infix_or_postfix_annotations_without_line_suffix_boundary<'ast, T>(
    context: &DestackFormatContext<'ast>,
    node_id: LocalNodeId<T>,
) -> impl Format<DestackFormatContext<'ast>> + use<'ast, T>
where
    T: Node + Clone + 'ast,
    NodeTree: NodeTreeImpl<T>,
{
    let mut items = Vec::new();
    for annotation_id in context.annotation_ids(node_id).iter().copied() {
        if matches!(
            context.annotation(annotation_id).position(),
            AnnotationPosition::BlockInfix
                | AnnotationPosition::BlockPostfix
                | AnnotationPosition::LinePostfix
        ) {
            items.push(annotation_id);
        }
    }

    format_with(move |f: &mut DestackFormatter<'ast, '_>| write_annotation_sequence(f, &items))
}

/// Format one prepared annotation sequence.
fn write_annotation_sequence_with_trailing_break<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    items: &[LocalNodeId<Annotation>],
    should_write_trailing_break: bool,
) -> FormatResult<()> {
    if items.is_empty() {
        return Ok(());
    }

    for (annotation_index, annotation_id) in items.iter().copied().enumerate() {
        let annotation = f.context().annotation(annotation_id);
        let position = annotation.position();
        let starts_on_own_line = f.context().annotation_starts_on_own_line(annotation_id);
        let next_token_type = f
            .context()
            .annotation_next_non_whitespace_token_type(annotation_id);
        let is_last_annotation = annotation_index + 1 == items.len();

        let needs_leading_space = matches!(
            position,
            AnnotationPosition::LinePostfix | AnnotationPosition::LinePostfixBoundary
        ) && !starts_on_own_line;
        let needs_leading_break = !needs_leading_space
            && (annotation_index > 0
                || position != AnnotationPosition::LinePrefix
                || starts_on_own_line);
        if needs_leading_space {
            write!(f, [space()])?;
        } else if needs_leading_break {
            write!(f, [hard_line_break()])?;
        }

        f.context()
            .annotation(annotation_id)
            .format_node(annotation_id, f)?;

        if !should_write_trailing_break && is_last_annotation {
        } else if matches!(position, AnnotationPosition::LinePostfixBoundary) {
            write!(f, [soft_line_break()])?;
        } else if next_token_type.is_none() || next_token_type == Some(TokenType::End) {
        } else {
            write!(f, [hard_line_break()])?;
        }
    }

    Ok(())
}
