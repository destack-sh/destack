use super::trivia::format_comment;
use crate::{Decorator, DestackFormatContext, DestackFormatter, FormatNode};
use destack_ast::{
    Comment, DecoratorPosition, LocalNodeId, Node, NodeTree, NodeTreeImpl, TokenType,
};
use destack_fir::format::{Format, FormatResult};
use destack_fir::prelude::{format_with, *};
use destack_fir::write;
use destack_source::NodeSpanType;

/// One source-ordered prefix item.
#[derive(Debug, Copy, Clone)]
enum PrefixSequenceItem {
    /// One targeted comment trivia node.
    Comment(Comment),
    /// One semantic annotation node.
    Decorator(LocalNodeId<Decorator>),
}

/// Return prefix comments for one node in source order.
pub(crate) fn prefix_comment_nodes<'ast, T>(
    context: &DestackFormatContext<'ast>,
    node_id: LocalNodeId<T>,
) -> Vec<Comment>
where
    T: Node + Clone + 'ast,
    NodeTree: NodeTreeImpl<T>,
{
    let token_start = context.node_token_start(node_id);

    if let Some(leading_span) = context.tree.get_side_span(node_id, NodeSpanType::Leading) {
        return context
            .comments()
            .comments_in_range(leading_span.start, token_start)
            .iter()
            .copied()
            .filter(|comment| comment.is_leading() || comment.preceded_by_newline())
            .collect();
    }

    context
        .comments()
        .comments_before(token_start)
        .iter()
        .copied()
        .filter(|comment| comment.is_leading() || comment.preceded_by_newline())
        .collect()
}

/// Return prefix comments that are not physically inside decorator spans.
fn prefix_comment_nodes_outside_decorators<'ast, T>(
    context: &DestackFormatContext<'ast>,
    node_id: LocalNodeId<T>,
) -> Vec<Comment>
where
    T: Node + Clone + 'ast,
    NodeTree: NodeTreeImpl<T>,
{
    prefix_comment_nodes(context, node_id)
        .into_iter()
        .filter(|comment| !comment_is_inside_decorator_span(context, node_id, *comment))
        .collect()
}

/// Return prefix comments for one node that start at or after one offset.
fn prefix_comments_after_offset<'ast, T>(
    context: &DestackFormatContext<'ast>,
    node_id: LocalNodeId<T>,
    start_offset: u32,
) -> Vec<Comment>
where
    T: Node + Clone + 'ast,
    NodeTree: NodeTreeImpl<T>,
{
    prefix_comment_nodes_outside_decorators(context, node_id)
        .into_iter()
        .filter(|comment| comment.span.start >= start_offset)
        .collect()
}

/// Format one prepared annotation sequence.
pub(crate) fn write_annotation_sequence<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    items: &[LocalNodeId<Decorator>],
) -> FormatResult<()> {
    write_annotation_sequence_with_trailing_break(f, items, true)
}

/// Format inline prefix annotations without introducing extra line breaks.
pub(crate) fn write_inline_prefix_annotations<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    items: &[LocalNodeId<Decorator>],
) -> FormatResult<()> {
    let mut wrote_annotation = false;

    for annotation_id in items.iter().copied() {
        if wrote_annotation {
            write!(f, [space()])?;
        }

        let annotation = f.context().annotation(annotation_id).clone();
        annotation.format_node(annotation_id, f)?;
        wrote_annotation = true;
    }

    Ok(())
}

/// Format one vertical prefix annotation block.
pub(crate) fn write_vertical_prefix_annotations<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    items: &[LocalNodeId<Decorator>],
) -> FormatResult<()> {
    for annotation_id in items.iter().copied() {
        let annotation = f.context().annotation(annotation_id).clone();
        annotation.format_node(annotation_id, f)?;
        write!(f, [hard_line_break()])?;
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
        if context.annotation(annotation_id).position == DecoratorPosition::BlockInfix {
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
    let comments = prefix_comment_nodes_outside_decorators(context, node_id);
    let annotation_ids = prefix_annotation_ids(context, node_id);

    prefix_sequence(context, comments, annotation_ids)
}

/// Format prefix annotations for one node without leading comments.
pub(crate) fn prefix_annotations_without_comments<'ast, T>(
    context: &DestackFormatContext<'ast>,
    node_id: LocalNodeId<T>,
) -> impl Format<DestackFormatContext<'ast>> + use<'ast, T>
where
    T: Node + Clone + 'ast,
    NodeTree: NodeTreeImpl<T>,
{
    prefix_sequence(context, Vec::new(), prefix_annotation_ids(context, node_id))
}

/// Format prefix annotations for one node after one prefix comment cutoff.
pub(crate) fn prefix_annotations_after_offset<'ast, T>(
    context: &DestackFormatContext<'ast>,
    node_id: LocalNodeId<T>,
    start_offset: u32,
) -> impl Format<DestackFormatContext<'ast>> + use<'ast, T>
where
    T: Node + Clone + 'ast,
    NodeTree: NodeTreeImpl<T>,
{
    let comments = prefix_comments_after_offset(context, node_id, start_offset);
    let annotation_ids = prefix_annotation_ids(context, node_id);

    prefix_sequence(context, comments, annotation_ids)
}

/// Format one prefix sequence for one node.
fn prefix_sequence<'ast>(
    context: &DestackFormatContext<'ast>,
    comments: Vec<Comment>,
    annotation_ids: Vec<LocalNodeId<Decorator>>,
) -> impl Format<DestackFormatContext<'ast>> + use<'ast> {
    let items = collect_prefix_sequence_items(context, comments, annotation_ids);

    format_with(move |f: &mut DestackFormatter<'ast, '_>| write_prefix_sequence_items(f, &items))
}

/// Return prefix decorator ids for one node.
fn prefix_annotation_ids<'ast, T>(
    context: &DestackFormatContext<'ast>,
    node_id: LocalNodeId<T>,
) -> Vec<LocalNodeId<Decorator>>
where
    T: Node + Clone + 'ast,
    NodeTree: NodeTreeImpl<T>,
{
    let mut annotation_ids = Vec::new();

    for annotation_id in context.annotation_ids(node_id).iter().copied() {
        let position = context.annotation(annotation_id).position;

        if matches!(
            position,
            DecoratorPosition::BlockPrefix | DecoratorPosition::LinePrefix
        ) {
            annotation_ids.push(annotation_id);
        }
    }

    annotation_ids
}

/// Return whether one comment lies inside any decorator annotation span for the node.
fn comment_is_inside_decorator_span<'ast, T>(
    context: &DestackFormatContext<'ast>,
    node_id: LocalNodeId<T>,
    comment: Comment,
) -> bool
where
    T: Node + Clone + 'ast,
    NodeTree: NodeTreeImpl<T>,
{
    context
        .annotation_ids(node_id)
        .iter()
        .copied()
        .any(|annotation_id| {
            let decorator_span = context.annotation_span(annotation_id);
            comment.span.start >= decorator_span.start && comment.span.end <= decorator_span.end
        })
}

/// Format prefix comments that appear before the decorator block for one node.
pub(crate) fn prefix_comments_before_decorators<'ast, T>(
    context: &DestackFormatContext<'ast>,
    node_id: LocalNodeId<T>,
) -> impl Format<DestackFormatContext<'ast>> + use<'ast, T>
where
    T: Node + Clone + 'ast,
    NodeTree: NodeTreeImpl<T>,
{
    let first_decorator_start = context
        .annotation_ids(node_id)
        .iter()
        .copied()
        .map(|annotation_id| context.annotation_span(annotation_id).start)
        .next();
    let comments = prefix_comment_nodes(context, node_id)
        .into_iter()
        .filter(|comment| {
            first_decorator_start.is_none_or(|decorator_start| comment.span.end <= decorator_start)
        })
        .collect();

    let items = collect_prefix_comment_items(comments);

    format_with(move |f: &mut DestackFormatter<'ast, '_>| write_prefix_sequence_items(f, &items))
}

/// Format decorator prefix annotations for one node as a vertical prefix block.
pub(crate) fn decorator_prefix_annotations<'ast, T>(
    context: &DestackFormatContext<'ast>,
    node_id: LocalNodeId<T>,
) -> impl Format<DestackFormatContext<'ast>> + use<'ast, T>
where
    T: Node + Clone + 'ast,
    NodeTree: NodeTreeImpl<T>,
{
    let first_decorator_start = context
        .annotation_ids(node_id)
        .iter()
        .copied()
        .map(|annotation_id| context.annotation_span(annotation_id).start)
        .next();
    let comments = prefix_comment_nodes(context, node_id)
        .into_iter()
        .filter(|comment| {
            first_decorator_start
                .is_some_and(|decorator_start| comment.span.start >= decorator_start)
                && !comment_is_inside_decorator_span(context, node_id, *comment)
        })
        .collect();
    let annotation_ids = prefix_annotation_ids(context, node_id);
    let items = collect_prefix_sequence_items(context, comments, annotation_ids);

    format_with(move |f: &mut DestackFormatter<'ast, '_>| {
        write_decorator_prefix_sequence_items(f, &items)
    })
}

/// Collect one prefix item sequence.
fn collect_prefix_sequence_items<'ast>(
    context: &DestackFormatContext<'ast>,
    comments: Vec<Comment>,
    annotation_ids: Vec<LocalNodeId<Decorator>>,
) -> Vec<PrefixSequenceItem> {
    let mut items = collect_prefix_comment_items(comments);

    for annotation_id in annotation_ids {
        items.push(PrefixSequenceItem::Decorator(annotation_id));
    }

    sort_prefix_sequence_items(context, &mut items);

    items
}

/// Collect one prefix comment item sequence.
fn collect_prefix_comment_items(comments: Vec<Comment>) -> Vec<PrefixSequenceItem> {
    let mut items = Vec::new();

    for comment in comments {
        items.push(PrefixSequenceItem::Comment(comment));
    }

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
                    PrefixSequenceItem::Decorator(left_id),
                    PrefixSequenceItem::Decorator(right_id),
                ) => left_id.id.cmp(&right_id.id),
                (PrefixSequenceItem::Comment(_), PrefixSequenceItem::Decorator(_)) => {
                    std::cmp::Ordering::Less
                }
                (PrefixSequenceItem::Decorator(_), PrefixSequenceItem::Comment(_)) => {
                    std::cmp::Ordering::Greater
                }
            })
    });
}

/// Return the span for one prefix sequence item.
fn prefix_sequence_item_span(context: &DestackFormatContext<'_>, item: PrefixSequenceItem) -> Span {
    match item {
        PrefixSequenceItem::Comment(comment) => comment.span,
        PrefixSequenceItem::Decorator(annotation_id) => context.annotation_span(annotation_id),
    }
}

/// Write one prefix item.
fn write_prefix_sequence_item<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    item: PrefixSequenceItem,
) -> FormatResult<()> {
    match item {
        PrefixSequenceItem::Comment(comment) => format_comment(f, comment),
        PrefixSequenceItem::Decorator(annotation_id) => {
            let annotation = f.context().annotation(annotation_id).clone();
            annotation.format_node(annotation_id, f)
        }
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
        let lines_after = f.context().source_text().lines_after(item_span.end);

        if lines_after > 1 {
            write!(f, [empty_line()])?;
        } else if lines_after == 1 {
            write!(f, [hard_line_break()])?;
        } else {
            write!(f, [space()])?;
        }
    }

    Ok(())
}

/// Write one vertical decorator prefix sequence.
fn write_decorator_prefix_sequence_items<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    items: &[PrefixSequenceItem],
) -> FormatResult<()> {
    if items.is_empty() {
        return Ok(());
    }

    for item in items.iter().copied() {
        write_prefix_sequence_item(f, item)?;
        write!(f, [hard_line_break()])?;
    }

    Ok(())
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
            context.annotation(annotation_id).position,
            DecoratorPosition::BlockPostfix | DecoratorPosition::LinePostfix
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
            context.annotation(annotation_id).position,
            DecoratorPosition::BlockInfix
                | DecoratorPosition::BlockPostfix
                | DecoratorPosition::LinePostfix
        ) {
            items.push(annotation_id);
        }
    }

    format_with(move |f: &mut DestackFormatter<'ast, '_>| write_annotation_sequence(f, &items))
}

/// Format one prepared annotation sequence.
fn write_annotation_sequence_with_trailing_break<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    items: &[LocalNodeId<Decorator>],
    should_write_trailing_break: bool,
) -> FormatResult<()> {
    if items.is_empty() {
        return Ok(());
    }

    for (annotation_index, annotation_id) in items.iter().copied().enumerate() {
        let annotation = f.context().annotation(annotation_id);
        let position = annotation.position;
        let starts_on_own_line = f.context().annotation_starts_on_own_line(annotation_id);
        let next_token_type = f
            .context()
            .annotation_next_non_whitespace_token_type(annotation_id);
        let is_last_annotation = annotation_index + 1 == items.len();

        let needs_leading_space = position == DecoratorPosition::LinePostfix && !starts_on_own_line;
        let needs_leading_break = !needs_leading_space
            && (annotation_index > 0
                || position != DecoratorPosition::LinePrefix
                || starts_on_own_line);
        if needs_leading_space {
            write!(f, [space()])?;
        } else if needs_leading_break {
            write!(f, [hard_line_break()])?;
        }

        let annotation = f.context().annotation(annotation_id).clone();
        annotation.format_node(annotation_id, f)?;

        let should_write_trailing_break = should_write_trailing_break
            || !is_last_annotation
            || matches!(next_token_type, Some(token_type) if token_type != TokenType::End);

        if should_write_trailing_break {
            write!(f, [hard_line_break()])?;
        }
    }

    Ok(())
}
