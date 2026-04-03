use crate::{Annotation, DestackFormatContext, DestackFormatter, FormatNode};
use destack_ast::{
    AnnotationPosition, Comment, Doc, DocStyle, LocalNodeId, Node, NodeTree, NodeTreeImpl,
    TokenType,
};
use destack_fir::format::{Format, FormatResult};
use destack_fir::prelude::{format_with, *};
use destack_fir::write;
use rustc_hash::FxHashSet;

/// One source-ordered prefix item.
#[derive(Debug, Copy, Clone)]
enum PrefixSequenceItem {
    /// One targeted comment trivia node.
    Comment(LocalNodeId<Comment>),
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
            Annotation::Doc { node, .. } => node.format(f),
            Annotation::Decorator { node, .. } => node.format(f),
        }
    }
}

/// Return whether one annotation is one multiline jsdoc block comment.
fn annotation_is_multiline_jsdoc_comment(
    context: &DestackFormatContext<'_>,
    annotation_id: LocalNodeId<Annotation>,
) -> bool {
    let annotation = context.annotation(annotation_id);
    let annotation_span = context.annotation_span(annotation_id);
    let is_multiline = context.has_newline(annotation_span);
    if !is_multiline {
        return false;
    }

    matches!(
        annotation,
        Annotation::Doc { node, .. } if context.tree.get::<Doc>(node).style == DocStyle::Star
    )
}

/// Return whether two adjacent annotations should be nestled as jsdoc comments.
fn should_nestle_adjacent_jsdoc_comments(
    context: &DestackFormatContext<'_>,
    current_annotation_id: LocalNodeId<Annotation>,
    next_annotation_id: LocalNodeId<Annotation>,
) -> bool {
    if !annotation_is_multiline_jsdoc_comment(context, current_annotation_id)
        || !annotation_is_multiline_jsdoc_comment(context, next_annotation_id)
    {
        return false;
    }

    let current_span = context.annotation_span(current_annotation_id);
    let next_span = context.annotation_span(next_annotation_id);
    current_span.end == next_span.start
}

/// Return whether one annotation is one own-line documentation comment.
fn annotation_is_own_line_comment(
    context: &DestackFormatContext<'_>,
    annotation_id: LocalNodeId<Annotation>,
) -> bool {
    matches!(context.annotation(annotation_id), Annotation::Doc { .. })
        && context.annotation_starts_on_own_line(annotation_id)
}

/// Return whether one annotation is one inline documentation comment.
fn annotation_is_inline_comment(
    context: &DestackFormatContext<'_>,
    annotation_id: LocalNodeId<Annotation>,
) -> bool {
    matches!(context.annotation(annotation_id), Annotation::Doc { .. })
        && !context.annotation_starts_on_own_line(annotation_id)
}

/// Return whether there is a blank line between two annotations.
fn annotations_have_blank_line_between(
    context: &DestackFormatContext<'_>,
    current_annotation_id: LocalNodeId<Annotation>,
    next_annotation_id: LocalNodeId<Annotation>,
) -> bool {
    let current_span = context.annotation_span(current_annotation_id);
    let next_span = context.annotation_span(next_annotation_id);
    current_span
        .gap_to(next_span)
        .is_some_and(|between_span| context.has_blank_line(between_span))
}

/// Return whether one annotation uses star comment syntax.
fn annotation_uses_star_comment_style(
    context: &DestackFormatContext<'_>,
    annotation: Annotation,
) -> bool {
    matches!(
        annotation,
        Annotation::Doc { node, .. }
            if context.tree.get::<Doc>(node).style == DocStyle::Star
    )
}

/// Return the source span for one prefix item.
fn prefix_sequence_item_span(
    context: &DestackFormatContext<'_>,
    item: PrefixSequenceItem,
) -> destack_source::Span {
    match item {
        PrefixSequenceItem::Comment(comment_id) => context.span(comment_id),
        PrefixSequenceItem::Annotation(annotation_id) => context.annotation_span(annotation_id),
    }
}

/// Format one source-ordered prefix item.
fn write_prefix_sequence_item<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    item: PrefixSequenceItem,
) -> FormatResult<()> {
    match item {
        PrefixSequenceItem::Comment(comment_id) => f
            .context()
            .tree
            .get::<Comment>(comment_id)
            .format_node(comment_id, f),
        PrefixSequenceItem::Annotation(annotation_id) => f
            .context()
            .annotation(annotation_id)
            .format_node(annotation_id, f),
    }
}

/// Return the first decorator start attached to one node.
fn first_decorator_span_start<'ast, T>(
    context: &DestackFormatContext<'ast>,
    node_id: LocalNodeId<T>,
) -> Option<u32>
where
    T: Node + Clone + 'ast,
    NodeTree: NodeTreeImpl<T>,
{
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
        })
}

/// Return token-after indexes that count as one node prefix seam.
fn prefix_comment_token_after_indexes<'ast, T>(
    context: &DestackFormatContext<'ast>,
    node_id: LocalNodeId<T>,
) -> FxHashSet<u32>
where
    T: Node + Clone + 'ast,
    NodeTree: NodeTreeImpl<T>,
{
    let mut token_after_indexes = FxHashSet::default();

    if let Some(token_index) = context.first_non_trivia_token_index_in_span(context.span(node_id)) {
        token_after_indexes.insert(token_index);
    }

    for annotation_id in context.annotation_ids(node_id).iter().copied() {
        if !matches!(
            context.annotation(annotation_id).position(),
            AnnotationPosition::BlockPrefix | AnnotationPosition::LinePrefix
        ) {
            continue;
        }

        if let Some(token_index) =
            context.first_non_trivia_token_index_in_span(context.annotation_span(annotation_id))
        {
            token_after_indexes.insert(token_index);
        }
    }

    token_after_indexes
}

/// Return whether one node is the outermost owner for one leading token.
fn node_owns_raw_prefix_comments<'ast, T>(
    context: &DestackFormatContext<'ast>,
    node_id: LocalNodeId<T>,
) -> bool
where
    T: Node + Clone + 'ast,
    NodeTree: NodeTreeImpl<T>,
{
    let Some(token_index) = context.first_non_trivia_token_index_in_span(context.span(node_id))
    else {
        return false;
    };

    let mut current_id = node_id.id;
    while let Some(parent_id) = context.parents.get_by_id(current_id) {
        let parent_span = context.tree.get_span_by_id(parent_id);
        if context.first_non_trivia_token_index_in_span(parent_span) == Some(token_index) {
            return false;
        }

        current_id = parent_id;
    }

    true
}

/// Return raw prefix comments for one node in source order.
pub(crate) fn raw_prefix_comment_nodes<'ast, T>(
    context: &DestackFormatContext<'ast>,
    node_id: LocalNodeId<T>,
) -> Vec<LocalNodeId<Comment>>
where
    T: Node + Clone + 'ast,
    NodeTree: NodeTreeImpl<T>,
{
    if !node_owns_raw_prefix_comments(context, node_id) {
        return Vec::new();
    }

    let token_after_indexes = prefix_comment_token_after_indexes(context, node_id);
    if token_after_indexes.is_empty() {
        return Vec::new();
    }

    let mut comment_ids = Vec::new();
    for trivia in context.tree.comment_trivia().iter().copied() {
        if !token_after_indexes.contains(&trivia.boundary.token_after) {
            continue;
        }

        if context.is_comment_owned(trivia.comment) {
            continue;
        }

        comment_ids.push(trivia.comment);
    }

    comment_ids
}

/// Push raw prefix comments for one node.
fn push_prefix_comment_items<'ast, T>(
    context: &DestackFormatContext<'ast>,
    node_id: LocalNodeId<T>,
    items: &mut Vec<PrefixSequenceItem>,
    include_comment: impl Fn(destack_source::Span) -> bool,
) where
    T: Node + Clone + 'ast,
    NodeTree: NodeTreeImpl<T>,
{
    for comment_id in raw_prefix_comment_nodes(context, node_id) {
        let comment_span = context.tree.get_span(comment_id);
        if include_comment(comment_span) {
            items.push(PrefixSequenceItem::Comment(comment_id));
        }
    }
}

/// Sort source-ordered prefix items in place.
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
                (PrefixSequenceItem::Comment(left_id), PrefixSequenceItem::Comment(right_id)) => {
                    left_id.id.cmp(&right_id.id)
                }
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
pub(crate) fn line_postfix_boundary_annotations<'ast, T>(
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
    let mut items = Vec::new();
    push_prefix_comment_items(context, node_id, &mut items, |_| true);

    for annotation_id in context.annotation_ids(node_id).iter().copied() {
        if matches!(
            context.annotation(annotation_id).position(),
            AnnotationPosition::BlockPrefix | AnnotationPosition::LinePrefix
        ) {
            items.push(PrefixSequenceItem::Annotation(annotation_id));
        }
    }

    sort_prefix_sequence_items(context, &mut items);

    format_with(move |f: &mut DestackFormatter<'ast, '_>| {
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
    })
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
    let mut items = Vec::new();
    let first_decorator_start = first_decorator_span_start(context, node_id);
    push_prefix_comment_items(context, node_id, &mut items, |comment_span| {
        first_decorator_start.is_some_and(|decorator_start| comment_span.end <= decorator_start)
    });

    for annotation_id in context.annotation_ids(node_id).iter().copied() {
        if matches!(
            context.annotation(annotation_id).position(),
            AnnotationPosition::BlockPrefix | AnnotationPosition::LinePrefix
        ) && !matches!(
            context.annotation(annotation_id),
            Annotation::Decorator { .. }
        ) {
            items.push(PrefixSequenceItem::Annotation(annotation_id));
        }
    }

    sort_prefix_sequence_items(context, &mut items);

    format_with(move |f: &mut DestackFormatter<'ast, '_>| {
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
    })
}

/// Format decorator prefix annotations for one node with grouped class-style seams.
pub(crate) fn decorator_prefix_annotations<'ast, T>(
    context: &DestackFormatContext<'ast>,
    node_id: LocalNodeId<T>,
) -> impl Format<DestackFormatContext<'ast>> + use<'ast, T>
where
    T: Node + Clone + 'ast,
    NodeTree: NodeTreeImpl<T>,
{
    let mut items = Vec::new();
    let first_decorator_start = first_decorator_span_start(context, node_id);
    push_prefix_comment_items(context, node_id, &mut items, |comment_span| {
        first_decorator_start.is_some_and(|decorator_start| comment_span.start >= decorator_start)
    });

    for annotation_id in context.annotation_ids(node_id).iter().copied() {
        if matches!(
            context.annotation(annotation_id).position(),
            AnnotationPosition::BlockPrefix | AnnotationPosition::LinePrefix
        ) && matches!(
            context.annotation(annotation_id),
            Annotation::Decorator { .. }
        ) {
            items.push(PrefixSequenceItem::Annotation(annotation_id));
        }
    }

    sort_prefix_sequence_items(context, &mut items);

    format_with(move |f: &mut DestackFormatter<'ast, '_>| {
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

                write!(f, [soft_line_break_or_space()])
            }))
            .should_expand(should_expand)]
        )
    })
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
pub(crate) fn postfix_annotations_without_line_postfix_boundary<'ast, T>(
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
pub(crate) fn infix_or_postfix_annotations_without_line_postfix_boundary<'ast, T>(
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

/// Write the leading seam before one annotation.
fn write_annotation_leading_seam<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    annotation_index: usize,
    position: AnnotationPosition,
    is_star_comment: bool,
    starts_on_own_line: bool,
) -> FormatResult<()> {
    let needs_leading_space = matches!(
        position,
        AnnotationPosition::LinePostfix | AnnotationPosition::LinePostfixBoundary
    ) && !starts_on_own_line;
    let needs_leading_break = !needs_leading_space
        && (annotation_index > 0
            || position != AnnotationPosition::LinePrefix
            || starts_on_own_line);

    // leading seam
    if needs_leading_space {
        write!(f, [space()])?;
    } else if needs_leading_break {
        write!(f, [hard_line_break()])?;
    } else if matches!(
        position,
        AnnotationPosition::BlockPrefix
            | AnnotationPosition::BlockInfix
            | AnnotationPosition::BlockPostfix
    ) && is_star_comment
        && !starts_on_own_line
    {
        write!(f, [space()])?;
    }

    Ok(())
}

/// Write the trailing seam after one annotation.
fn write_annotation_trailing_seam<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    position: AnnotationPosition,
    next_token_type: Option<TokenType>,
    next_annotation_is_inline_comment: bool,
    next_annotation_is_own_line_comment: bool,
    next_annotation_should_nestle_jsdoc_comment: bool,
    has_blank_line_before_next_annotation: bool,
    is_last_annotation: bool,
    should_write_trailing_break: bool,
) -> FormatResult<()> {
    // trailing seam
    if next_annotation_should_nestle_jsdoc_comment {
    } else if next_annotation_is_own_line_comment && has_blank_line_before_next_annotation {
        write!(f, [empty_line()])?;
    } else if next_annotation_is_inline_comment {
        write!(f, [space()])?;
    } else if !should_write_trailing_break && is_last_annotation {
    } else if matches!(position, AnnotationPosition::LinePostfixBoundary) {
        write!(f, [soft_line_break()])?;
    } else if next_token_type.is_none() || next_token_type == Some(TokenType::End) {
    } else {
        write!(f, [hard_line_break()])?;
    }

    Ok(())
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
        let is_star_comment = annotation_uses_star_comment_style(f.context(), annotation);
        let starts_on_own_line = f.context().annotation_starts_on_own_line(annotation_id);
        let next_token_type = f
            .context()
            .annotation_next_non_whitespace_token_type(annotation_id);
        let next_annotation_id = items.get(annotation_index + 1).copied();
        let next_annotation_is_inline_comment =
            next_annotation_id.is_some_and(|next_annotation_id| {
                annotation_is_inline_comment(f.context(), next_annotation_id)
            });
        let next_annotation_is_own_line_comment =
            next_annotation_id.is_some_and(|next_annotation_id| {
                annotation_is_own_line_comment(f.context(), next_annotation_id)
            });
        let next_annotation_should_nestle_jsdoc_comment =
            next_annotation_id.is_some_and(|next_annotation_id| {
                should_nestle_adjacent_jsdoc_comments(
                    f.context(),
                    annotation_id,
                    next_annotation_id,
                )
            });
        let has_blank_line_before_next_annotation =
            next_annotation_id.is_some_and(|next_annotation_id| {
                annotations_have_blank_line_between(f.context(), annotation_id, next_annotation_id)
            });
        let is_last_annotation = annotation_index + 1 == items.len();

        write_annotation_leading_seam(
            f,
            annotation_index,
            position,
            is_star_comment,
            starts_on_own_line,
        )?;
        f.context()
            .annotation(annotation_id)
            .format_node(annotation_id, f)?;
        write_annotation_trailing_seam(
            f,
            position,
            next_token_type,
            next_annotation_is_inline_comment,
            next_annotation_is_own_line_comment,
            next_annotation_should_nestle_jsdoc_comment,
            has_blank_line_before_next_annotation,
            is_last_annotation,
            should_write_trailing_break,
        )?;
    }

    Ok(())
}
