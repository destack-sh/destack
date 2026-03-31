use crate::{Annotation, DestackFormatContext, DestackFormatter, FormatNode};
use destack_ast::{
    AnnotationPosition, Doc, DocStyle, LocalNodeId, Node, NodeTree, NodeTreeImpl, TokenType,
};
use destack_fir::format::{Format, FormatResult};
use destack_fir::prelude::{format_with, *};
use destack_fir::write;

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
    for annotation_id in context.annotation_ids(node_id).iter().copied() {
        if matches!(
            context.annotation(annotation_id).position(),
            AnnotationPosition::BlockPrefix | AnnotationPosition::LinePrefix
        ) {
            items.push(annotation_id);
        }
    }

    format_with(move |f: &mut DestackFormatter<'ast, '_>| write_annotation_sequence(f, &items))
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
