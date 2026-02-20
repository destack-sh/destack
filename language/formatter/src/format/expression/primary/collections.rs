use crate::analysis::timing::tags;
use crate::collection::{collection_nodes_have_annotations, collection_range_is_inline};
use crate::expression::{
    Argument, DestackFormatContext, DestackFormatter, Expression, FormatResult, HugOptions,
    LocalNodeId, array_elements_are_fill_candidates, array_has_only_boundary_comments,
    format_boundary_comment_array, format_hugged, is_complex_argument, is_trivial_argument,
    list_like, space, token, transparent_inner_expression,
};
use destack_fir::format::Buffer;
use destack_fir::write;

/// Format an array literal primary expression.
pub(super) fn format_primary_array_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    elements_ids: &[LocalNodeId<Argument>],
) -> FormatResult<()> {
    let _timing = f
        .context()
        .timing_scope(tags::FORMAT_EXPRESSION_PRIMARY_ARRAY);

    // try hugged format for single object or array elements
    if !format_hugged(f, elements_ids, HugOptions::ARRAY, None, false)? {
        if array_has_sparse_holes(f.context(), elements_ids) {
            let span = f.context().span(node_id);
            let has_newline_in_source = f.context().has_newline(span);
            let has_sparse_annotations = f.context().has_infix_annotation(node_id)
                || collection_nodes_have_annotations(f.context(), elements_ids);

            if has_newline_in_source || has_sparse_annotations {
                write!(
                    f,
                    [list_like("[", "]", ",", elements_ids)
                        .as_collection()
                        .should_expand(has_newline_in_source)]
                )?;
            } else {
                format_sparse_array_literal(f, elements_ids)?;
            }

            return Ok(());
        }

        let span = f.context().span(node_id);
        let has_newline_in_source = f.context().has_newline(span);
        let has_annotations = f.context().has_infix_annotation(node_id)
            || collection_nodes_have_annotations(f.context(), elements_ids);

        let mut elements_are_inline_in_source = true;
        if has_annotations || (has_newline_in_source && elements_ids.len() > 1) {
            elements_are_inline_in_source = collection_range_is_inline(f.context(), elements_ids);
        }

        let mut should_expand_for_annotations = false;
        let mut can_keep_inline_boundary_comment_array = false;

        // annotation sensitive expansion checks
        if has_annotations {
            let has_line_comment_annotations = elements_ids.iter().copied().any(|element_id| {
                let annotation_profile = f.context().ensure_argument_annotation_facts(element_id);
                annotation_profile.has_line_comment
            });

            can_keep_inline_boundary_comment_array = elements_are_inline_in_source
                && array_elements_are_fill_candidates(f.context().tree, elements_ids)
                && array_has_only_boundary_comments(f.context(), span, elements_ids);
            should_expand_for_annotations =
                has_line_comment_annotations || !elements_are_inline_in_source;
        }

        let tree = f.context().tree;
        let has_complex_elements = elements_ids.len() > 1
            && elements_ids
                .iter()
                .copied()
                .any(|element_id| is_complex_argument(tree, tree.get(element_id)));
        let has_single_non_trivial_multiline_element = elements_ids.len() == 1
            && has_newline_in_source
            && !is_trivial_argument(tree, tree.get(elements_ids[0]));
        let has_multiline_non_inline_multi_element =
            has_newline_in_source && elements_ids.len() > 1 && !elements_are_inline_in_source;

        let should_expand = (should_expand_for_annotations
            && !can_keep_inline_boundary_comment_array)
            || has_complex_elements
            || has_single_non_trivial_multiline_element
            || has_multiline_non_inline_multi_element;

        if can_keep_inline_boundary_comment_array {
            format_boundary_comment_array(f, elements_ids)?;
        } else {
            write!(
                f,
                [list_like("[", "]", ",", elements_ids)
                    .as_collection()
                    .should_expand(should_expand)]
            )?;
        }
    }

    Ok(())
}

/// Format a tuple literal primary expression.
pub(super) fn format_primary_tuple_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    elements_ids: &[LocalNodeId<Argument>],
) -> FormatResult<()> {
    let _timing = f
        .context()
        .timing_scope(tags::FORMAT_EXPRESSION_PRIMARY_TUPLE);

    if elements_ids.is_empty() {
        write!(f, [token("()")])?;
    } else if !format_hugged(f, elements_ids, HugOptions::TUPLE, None, false)? {
        // not a single huggable element: use regular formatting
        let span = f.context().span(node_id);

        // check for annotations that require expansion
        let has_annotations = f.context().has_infix_annotation(node_id)
            || collection_nodes_have_annotations(f.context(), elements_ids);
        let tree = f.context().tree;
        let has_complex_elements = elements_ids.len() > 1
            && elements_ids
                .iter()
                .copied()
                .any(|element_id| is_complex_argument(tree, tree.get(element_id)));
        let should_expand = has_annotations
            || has_complex_elements
            || (f.context().has_newline(span) && elements_ids.len() > 1);

        // trailing comma disambiguates tuples from parenthesized expressions
        write!(
            f,
            [list_like("(", ")", ",", elements_ids)
                .as_collection()
                .force_trailing_separator()
                .should_expand(should_expand)]
        )?;
    }

    Ok(())
}

/// Return whether an array literal contains sparse elision slots.
fn array_has_sparse_holes(
    context: &DestackFormatContext<'_>,
    elements: &[LocalNodeId<Argument>],
) -> bool {
    elements
        .iter()
        .copied()
        .any(|argument_id| argument_is_sparse_hole(context, argument_id))
}

/// Return whether an array element argument is a sparse hole.
fn argument_is_sparse_hole(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    let Argument::Positional { value, .. } = context.tree.get(argument_id) else {
        return false;
    };
    let value_id = transparent_inner_expression(context, *value);
    matches!(context.tree.get(value_id), Expression::Stub)
}

/// Format sparse arrays while preserving elision comma count.
fn format_sparse_array_literal<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    elements: &[LocalNodeId<Argument>],
) -> FormatResult<()> {
    write!(f, [token("[")])?;

    let mut expects_element = true;
    for (index, argument_id) in elements.iter().copied().enumerate() {
        let is_hole = argument_is_sparse_hole(f.context(), argument_id);

        if !expects_element {
            write!(f, [token(",")])?;
            if !is_hole {
                write!(f, [space()])?;
            }
            expects_element = true;
        }

        if is_hole {
            write!(f, [token(",")])?;
            let next_is_non_hole = elements
                .get(index + 1)
                .is_some_and(|next_id| !argument_is_sparse_hole(f.context(), *next_id));
            if next_is_non_hole {
                write!(f, [space()])?;
            }
            continue;
        }

        write!(f, [argument_id])?;
        expects_element = false;
    }

    write!(f, [token("]")])?;
    Ok(())
}
