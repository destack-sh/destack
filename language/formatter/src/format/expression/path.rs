use crate::format::operator::format_generic_argument_list;
use crate::{Decorator, DestackFormatContext, DestackFormatter, FormatNode};
use destack_ast::{DecoratorPosition, Expression, GenericArgument, LocalNodeId, Path, TokenType};
use destack_fir::format::{Buffer, FormatResult};
use destack_fir::prelude::{format_with, hard_line_break, indent, token};
use destack_fir::write;
use smallvec::SmallVec;

/// Build per-dot boundary annotation buckets for one path expression.
pub(super) fn path_boundary_annotations_by_dot(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
    segment_count: usize,
) -> Option<Vec<SmallVec<[LocalNodeId<Decorator>; 2]>>> {
    if segment_count <= 1 {
        return None;
    }

    let expression_span = context.span(expression_id);
    let mut dot_starts = Vec::with_capacity(segment_count.saturating_sub(1));
    for dot_index in 1..segment_count {
        let dot_start =
            context.nth_token_type_start_in_span(expression_span, TokenType::Dot, dot_index)?;
        dot_starts.push(dot_start);
    }

    let mut buckets =
        vec![SmallVec::<[LocalNodeId<Decorator>; 2]>::new(); segment_count.saturating_sub(1)];
    let mut has_boundary_annotations = false;
    for annotation_id in context.annotation_ids(expression_id) {
        let annotation = context.annotation(*annotation_id);
        if annotation.position != DecoratorPosition::LinePostfixBoundary {
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
    let Expression::QualifiedReference { path, .. } = expression else {
        return false;
    };

    path_boundary_annotations_by_dot(context, expression_id, path.segments.len()).is_some()
}

/// Format one path expression with boundary-targeted annotations.
fn format_path_with_boundary_annotations<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    path: &Path,
    boundary_annotation_buckets: &[SmallVec<[LocalNodeId<Decorator>; 2]>],
) -> FormatResult<()> {
    let Some(first_segment) = path.segments.first().copied() else {
        return Ok(());
    };
    write!(f, [first_segment])?;

    for (segment_index, segment) in path.segments.iter().copied().enumerate().skip(1) {
        let bucket_index = segment_index.saturating_sub(1);
        let dot_annotations = boundary_annotation_buckets
            .get(bucket_index)
            .map_or(&[][..], SmallVec::as_slice);

        if dot_annotations.is_empty() {
            write!(f, [token("."), segment])?;
            continue;
        }

        write!(
            f,
            [indent(&format_with(
                |f: &mut DestackFormatter<'ast, '_>| {
                    for annotation_id in dot_annotations {
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

/// Format one path expression and its generic arguments.
pub(crate) fn format_path_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    path: &Path,
    generic_arguments: &[LocalNodeId<GenericArgument>],
) -> FormatResult<()> {
    let boundary_annotation_buckets =
        path_boundary_annotations_by_dot(f.context(), node_id, path.segments.len());

    if let Some(boundary_annotation_buckets) = boundary_annotation_buckets {
        format_path_with_boundary_annotations(f, path, &boundary_annotation_buckets)?;
    } else {
        write!(f, [path])?;
    }

    if !generic_arguments.is_empty() {
        format_generic_argument_list(f, generic_arguments)?;
    }

    Ok(())
}
