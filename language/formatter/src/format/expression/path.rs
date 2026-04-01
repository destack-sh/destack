use crate::format::chain::{argument_value_id_if_present, transparent_inner_expression};
use crate::format::operator::{
    format_static_argument_list, leading_raw_type_position_comment_nodes,
};
use crate::{Annotation, DestackFormatContext, DestackFormatter, FormatNode};
use destack_ast::{AnnotationPosition, Argument, Expression, LocalNodeId, Path, TokenType};
use destack_fir::format::{Buffer, FormatResult};
use destack_fir::prelude::{format_with, hard_line_break, indent, token};
use destack_fir::write;
use smallvec::SmallVec;

/// Return whether a static argument should stay inline in a path.
fn is_simple_static_argument(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    if context.has_annotation(argument_id) {
        return false;
    }

    let Some(value_id) = argument_value_id_if_present(context.tree, argument_id) else {
        return false;
    };
    if !leading_raw_type_position_comment_nodes(context, value_id).is_empty() {
        return false;
    }

    let value = context.tree.get(value_id);

    super::is_trivial_expression(context.tree, value)
}

/// Build per-dot boundary annotation buckets for one path expression.
pub(super) fn path_boundary_annotations_by_dot_seam(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
    segment_count: usize,
) -> Option<Vec<SmallVec<[LocalNodeId<Annotation>; 2]>>> {
    if segment_count <= 1 {
        return None;
    }

    let expression_span = context.span(expression_id);
    let mut dot_starts = Vec::with_capacity(segment_count.saturating_sub(1));
    for seam_index in 1..segment_count {
        let dot_start =
            context.nth_token_type_start_in_span(expression_span, TokenType::Dot, seam_index)?;
        dot_starts.push(dot_start);
    }

    let mut buckets =
        vec![SmallVec::<[LocalNodeId<Annotation>; 2]>::new(); segment_count.saturating_sub(1)];
    let mut has_boundary_annotations = false;
    for annotation_id in context.annotation_ids(expression_id) {
        let annotation = context.annotation(*annotation_id);
        if annotation.position() != AnnotationPosition::LinePostfixBoundary {
            continue;
        }

        let Some(next_token) = context.annotation_next_non_whitespace_token(*annotation_id) else {
            continue;
        };
        if next_token.token.ty != TokenType::Dot {
            continue;
        }

        let Some(bucket_index) = dot_starts
            .iter()
            .position(|dot_start| *dot_start == next_token.span.start)
        else {
            continue;
        };

        buckets[bucket_index].push(*annotation_id);
        has_boundary_annotations = true;
    }

    if !has_boundary_annotations {
        return None;
    }

    Some(buckets)
}

/// Return whether one primary expression already renders boundary annotations inline.
pub(crate) fn primary_expression_skips_boundary_annotations(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
    expression: &Expression,
) -> bool {
    let Expression::Path { path, .. } = expression else {
        return false;
    };

    path_boundary_annotations_by_dot_seam(context, expression_id, path.segments.len()).is_some()
}

/// Format one path expression with seam-targeted boundary annotations.
fn format_path_with_boundary_annotations<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    path: &Path,
    boundary_annotation_buckets: &[SmallVec<[LocalNodeId<Annotation>; 2]>],
) -> FormatResult<()> {
    let Some(first_segment) = path.segments.first().copied() else {
        return Ok(());
    };
    write!(f, [first_segment])?;

    for (segment_index, segment) in path.segments.iter().copied().enumerate().skip(1) {
        let bucket_index = segment_index.saturating_sub(1);
        let seam_annotations = boundary_annotation_buckets
            .get(bucket_index)
            .map_or(&[][..], SmallVec::as_slice);

        if seam_annotations.is_empty() {
            write!(f, [token("."), segment])?;
            continue;
        }

        write!(
            f,
            [indent(&format_with(
                |f: &mut DestackFormatter<'ast, '_>| {
                    for annotation_id in seam_annotations {
                        let annotation = f.context().annotation(*annotation_id);
                        write!(f, [hard_line_break()])?;
                        annotation.format_node(*annotation_id, f)?;
                    }

                    write!(f, [hard_line_break(), token("."), segment])?;
                    Ok(())
                }
            ))]
        )?;
    }

    Ok(())
}

/// Format one path expression and its static arguments.
pub(crate) fn format_path_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    path: &Path,
    static_arguments: &Option<Vec<LocalNodeId<Argument>>>,
) -> FormatResult<()> {
    let tree = f.context().tree;
    let boundary_annotation_buckets =
        path_boundary_annotations_by_dot_seam(f.context(), node_id, path.segments.len());

    if let Some(boundary_annotation_buckets) = boundary_annotation_buckets {
        format_path_with_boundary_annotations(f, path, &boundary_annotation_buckets)?;
    } else {
        write!(f, [path])?;
    }

    if let Some(static_arguments) = static_arguments
        && !static_arguments.is_empty()
    {
        let is_single_simple = static_arguments.len() == 1 && {
            let argument_id = static_arguments[0];
            if let Some(argument_value_id) = argument_value_id_if_present(tree, argument_id) {
                let argument_value_id =
                    transparent_inner_expression(f.context(), argument_value_id);
                let is_index_like = matches!(
                    tree.get(argument_value_id),
                    Expression::Index { .. } | Expression::TypeIndex { .. }
                );
                !is_index_like && is_simple_static_argument(f.context(), argument_id)
            } else {
                false
            }
        };

        if is_single_simple {
            write!(f, [token("<"), static_arguments[0], token(">")])?;
        } else {
            format_static_argument_list(f, static_arguments)?;
        }
    }

    Ok(())
}
