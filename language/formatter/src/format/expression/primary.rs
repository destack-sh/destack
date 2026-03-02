use crate::format::analysis::timing;
use crate::format::collection::{collection_nodes_have_annotations, collection_range_is_inline};
use crate::format::directive::{
    FormatterDirective, FormatterDirectiveKind, FormatterDirectivePosition, directive_for_node,
};
use crate::format::expression::{
    Argument, BinaryOperator, DestackFormatContext, DestackFormatter, Expression, FormatResult,
    HugOptions, Keyword, LocalNodeId, NodeType, ParenthesizedDropMode, SeparatorLineCommentSource,
    TokenType, TypeModifier, TypePredicateSubject,
    argument_can_render_without_separator_line_comment, argument_value_id,
    array_elements_are_fill_candidates, array_has_only_boundary_comments, block_indent,
    format_boundary_comment_array, format_expression, format_fill_array, format_hugged,
    format_scalar_literal, format_static_argument_list, format_struct_literal,
    format_template_literal, format_type_index_expression, format_type_template_literal,
    format_with, group, hard_line_break, indent, is_assignment_left_target, is_call_like_argument,
    is_complex_argument, is_expression_breakable, is_simple_static_argument, is_trivial_argument,
    line_postfix_boundary, list_like, parenthesized_boundary_comments,
    parenthesized_has_leading_inner_comments, parenthesized_has_leading_inner_trivia,
    sequence_expression_needs_parens, should_drop_parenthesized,
    should_force_multiline_mapped_type, should_hoist_parenthesized_inner_cast_prefix_comments,
    single_argument_separator_line_comment_source, soft_block_indent, soft_line_break,
    soft_line_break_or_space, space, token, transparent_inner_expression,
    tree_literal_should_break, write_argument_without_separator_line_comment,
    write_separator_line_comment_after_comma,
};
use crate::format::tree::format_tree_literal_expression;
use crate::{Annotation, FormatNode};
use destack_ast as ast;
use destack_ast::{AnnotationPosition, NodeTree};
use destack_fir::format::{Buffer, Format};
use destack_fir::{format_args, write};
use smallvec::SmallVec;

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
    context.visit_annotations(expression_id, |annotation_ids| {
        for annotation_id in annotation_ids {
            let annotation = context.annotation(*annotation_id);
            if annotation.position() != AnnotationPosition::LinePostfixBoundary {
                continue;
            }

            let Some(next_token) = context.annotation_next_non_whitespace_token(*annotation_id)
            else {
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
    });

    if !has_boundary_annotations {
        return None;
    }

    Some(buckets)
}

/// Format one path expression with seam-targeted boundary annotations.
fn format_path_with_boundary_annotations<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    path: &ast::Path,
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

/// Format an array literal primary expression.
pub(crate) fn format_primary_array_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    elements_ids: &[LocalNodeId<Argument>],
) -> FormatResult<()> {
    let _timing = f
        .context()
        .timing_scope(timing::FORMAT_EXPRESSION_PRIMARY_ARRAY);

    if elements_ids.is_empty() {
        if f.context().has_infix_annotation(node_id) {
            write!(
                f,
                [group(&format_args![
                    token("["),
                    block_indent(&f.context().block_infix_annotations(node_id)),
                    hard_line_break(),
                    token("]")
                ])]
            )?;
        } else {
            write!(f, [token("[]")])?;
        }

        return Ok(());
    }

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
            let annotation_cache = f.context().argument_annotation_cache(element_id);
            annotation_cache.has_line_comment
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

    let should_expand_by_structure = array_should_expand_by_structure(tree, elements_ids);
    let should_expand = (should_expand_for_annotations && !can_keep_inline_boundary_comment_array)
        || has_complex_elements
        || has_single_non_trivial_multiline_element
        || has_multiline_non_inline_multi_element
        || should_expand_by_structure;
    let should_use_fill_layout =
        !has_annotations && array_elements_are_fill_candidates(tree, elements_ids);

    if format_array_with_last_separator_line_comment(f, node_id, elements_ids)? {
        // formatter-owned trailing separator comments around close brackets need
        // explicit comma-before-comment emission to stay source-idempotent
    } else if can_keep_inline_boundary_comment_array {
        format_boundary_comment_array(f, elements_ids)?;
    } else if should_use_fill_layout {
        format_fill_array(f, elements_ids)?;
    } else {
        write!(
            f,
            [list_like("[", "]", ",", elements_ids)
                .as_collection()
                .should_expand(should_expand)]
        )?;
    }

    Ok(())
}

/// Return one trailing separator comment source on the last array element.
fn array_last_separator_line_comment_source(
    context: &DestackFormatContext<'_>,
    array_node_id: LocalNodeId<Expression>,
    elements_ids: &[LocalNodeId<Argument>],
) -> Option<(LocalNodeId<Argument>, SeparatorLineCommentSource)> {
    let last_argument_id = elements_ids.last().copied()?;
    if !argument_can_render_without_separator_line_comment(context, last_argument_id) {
        return None;
    }

    let comment_source =
        single_argument_separator_line_comment_source(context, array_node_id, last_argument_id)?;

    Some((last_argument_id, comment_source))
}

/// Format one array with the last separator line comment emitted after the trailing comma.
fn format_array_with_last_separator_line_comment<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    array_node_id: LocalNodeId<Expression>,
    elements_ids: &[LocalNodeId<Argument>],
) -> FormatResult<bool> {
    let Some((last_argument_id, comment_source)) =
        array_last_separator_line_comment_source(f.context(), array_node_id, elements_ids)
    else {
        return Ok(false);
    };

    if comment_source.is_own_line {
        write!(
            f,
            [group(&format_args![
                token("["),
                soft_block_indent(&format_with(
                    |f: &mut DestackFormatter<'ast, '_>| -> FormatResult<()> {
                        for (index, argument_id) in elements_ids.iter().copied().enumerate() {
                            if index > 0 {
                                write!(f, [token(","), space()])?;
                            }

                            if argument_id == last_argument_id {
                                if !write_argument_without_separator_line_comment(f, argument_id)? {
                                    write!(f, [argument_id])?;
                                }
                                write_separator_line_comment_after_comma(f, &comment_source)?;
                            } else {
                                write!(f, [argument_id])?;
                            }
                        }

                        Ok(())
                    }
                )),
                token("]")
            ])]
        )?;
    } else {
        write!(f, [token("["), hard_line_break()])?;
        write!(
            f,
            [block_indent(&format_with(
                |f: &mut DestackFormatter<'ast, '_>| -> FormatResult<()> {
                    for (index, argument_id) in elements_ids.iter().copied().enumerate() {
                        if index > 0 {
                            write!(f, [hard_line_break()])?;
                        }

                        if argument_id == last_argument_id {
                            if !write_argument_without_separator_line_comment(f, argument_id)? {
                                write!(f, [argument_id])?;
                            }
                            write_separator_line_comment_after_comma(f, &comment_source)?;
                        } else {
                            write!(f, [argument_id, token(",")])?;
                        }
                    }

                    Ok(())
                }
            ))]
        )?;
        write!(f, [hard_line_break(), token("]")])?;
    }

    Ok(true)
}

/// Format a tuple literal primary expression.
pub(crate) fn format_primary_tuple_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    elements_ids: &[LocalNodeId<Argument>],
) -> FormatResult<()> {
    let _timing = f
        .context()
        .timing_scope(timing::FORMAT_EXPRESSION_PRIMARY_TUPLE);

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

/// Return whether one array should expand for nested array or object structure.
fn array_should_expand_by_structure(tree: &NodeTree, elements: &[LocalNodeId<Argument>]) -> bool {
    if elements.len() < 2 {
        return false;
    }

    #[derive(Clone, Copy, PartialEq, Eq)]
    enum ArrayChildKind {
        Array,
        Object,
    }

    let mut child_kind = None;
    for element_id in elements {
        let Argument::Positional { value, .. } = tree.get(*element_id) else {
            return false;
        };

        let value_id = transparent_inner_expression_from_tree(tree, *value);
        match tree.get(value_id) {
            Expression::ArrayExpression {
                elements: nested_elements,
            } => {
                if nested_elements.len() < 2 {
                    return false;
                }

                if child_kind.is_some_and(|kind| kind != ArrayChildKind::Array) {
                    return false;
                }

                child_kind = Some(ArrayChildKind::Array);
            }
            Expression::ObjectExpression { properties, .. } => {
                if properties.len() < 2 {
                    return false;
                }

                if child_kind.is_some_and(|kind| kind != ArrayChildKind::Object) {
                    return false;
                }

                child_kind = Some(ArrayChildKind::Object);
            }
            _ => {
                return false;
            }
        }
    }

    true
}

/// Return one expression id with transparent parenthesized wrappers removed.
fn transparent_inner_expression_from_tree(
    tree: &NodeTree,
    expression_id: LocalNodeId<Expression>,
) -> LocalNodeId<Expression> {
    let mut expression_id = expression_id;
    while let Expression::Parenthesized { expression } = tree.get(expression_id) {
        expression_id = *expression;
    }

    expression_id
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

/// Format primary expression variants.
pub(crate) fn format_primary_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    expression: &Expression,
) -> FormatResult<bool> {
    let tree = f.context().tree;

    match expression {
        // path
        Expression::Path {
            path,
            static_arguments,
        } => {
            let _timing = f
                .context()
                .timing_scope(timing::FORMAT_EXPRESSION_PRIMARY_PATH);
            let boundary_annotation_buckets =
                path_boundary_annotations_by_dot_seam(f.context(), node_id, path.segments.len());
            if let Some(boundary_annotation_buckets) = boundary_annotation_buckets {
                format_path_with_boundary_annotations(f, path, &boundary_annotation_buckets)?;
            } else {
                write!(f, [path])?;
            }

            // static arguments
            if let Some(static_arguments) = static_arguments
                && !static_arguments.is_empty()
            {
                let is_single_simple = static_arguments.len() == 1 && {
                    let argument_id = static_arguments[0];
                    let argument_value_id = argument_value_id(tree, argument_id);
                    let argument_value_id =
                        transparent_inner_expression(f.context(), argument_value_id);
                    let is_index_like = matches!(
                        tree.get(argument_value_id),
                        Expression::Index { .. } | Expression::TypeIndex { .. }
                    );
                    !is_index_like && is_simple_static_argument(f.context(), argument_id)
                };
                if is_single_simple {
                    write!(f, [token("<"), static_arguments[0], token(">"),])?;
                } else {
                    format_static_argument_list(f, static_arguments)?;
                }
            }
        }

        // private identifier
        Expression::PrivateIdentifier { name } => {
            write!(f, [token("#"), *name])?;
        }

        // this
        Expression::This => {
            write!(f, [Keyword::This])?;
        }

        // super
        Expression::Super => {
            write!(f, [Keyword::Super])?;
        }

        // scalar literal
        Expression::ScalarLiteral(node) => {
            format_scalar_literal(node, tree.get_span(node_id), f)?;
        }

        // template literal
        Expression::TemplateExpression { value } => {
            format_template_literal(value, tree.get_span(node_id), f)?;
        }

        // tagged template literal
        Expression::TaggedTemplateExpression { tag, value } => {
            write!(f, [tag, f.context().block_infix_annotations(node_id)])?;
            format_template_literal(value, tree.get_span(node_id), f)?;
        }

        // type template literal
        Expression::TypeTemplateLiteral { strings, spans } => {
            format_type_template_literal(strings, spans, f)?;
        }

        // type literal
        Expression::TypeLiteral(node) => node.format(f)?,

        // type import
        Expression::TypeImport {
            target: _,
            arguments,
            qualifier,
            static_arguments,
        } => {
            write!(f, [Keyword::Import, list_like("(", ")", ",", arguments)])?;
            if let Some(qualifier) = qualifier {
                write!(f, [token("."), qualifier])?;
            }
            if let Some(static_arguments) = static_arguments {
                format_static_argument_list(f, static_arguments)?;
            }
        }

        // type infer
        Expression::TypeInfer { name, constraint } => {
            write!(f, [Keyword::Infer, space(), *name])?;
            if let Some(constraint) = constraint {
                write!(f, [space(), Keyword::Extends, space(), *constraint])?;
            }
        }

        // type predicate
        Expression::TypePredicate {
            asserts,
            subject,
            target,
        } => {
            if *asserts {
                write!(f, [Keyword::Asserts, space()])?;
            }
            match subject {
                TypePredicateSubject::Identifier(name) => {
                    write!(f, [*name])?;
                }
                TypePredicateSubject::This => {
                    write!(f, [Keyword::This])?;
                }
            }
            if let Some(target) = target {
                write!(f, [space(), Keyword::Is, space(), *target])?;
            }
        }

        // type conditional
        Expression::TypeConditional {
            left,
            right,
            then_type,
            else_type,
        } => {
            let _timing = f
                .context()
                .timing_scope(timing::FORMAT_EXPRESSION_PRIMARY_TYPE_CONDITIONAL);
            let conditional_tail = format_with(|f| {
                write!(
                    f,
                    [
                        soft_line_break_or_space(),
                        token("?"),
                        space(),
                        then_type,
                        soft_line_break_or_space(),
                        token(":"),
                        space(),
                        else_type
                    ]
                )
            });
            let should_double_indent_tail = f
                .context()
                .expression_is_in_template_literal_interpolation(node_id)
                && f.context()
                    .expression_has_type_conditional_ancestor(node_id);
            if should_double_indent_tail {
                write!(
                    f,
                    [group(&format_args![
                        left,
                        space(),
                        Keyword::Extends,
                        space(),
                        right,
                        indent(&indent(&conditional_tail))
                    ])]
                )?;
            } else {
                write!(
                    f,
                    [group(&format_args![
                        left,
                        space(),
                        Keyword::Extends,
                        space(),
                        right,
                        indent(&conditional_tail)
                    ])]
                )?;
            }
        }

        // type mapped
        Expression::TypeMapped {
            parameter,
            modifiers,
            value,
        } => {
            let _timing = f
                .context()
                .timing_scope(timing::FORMAT_EXPRESSION_PRIMARY_TYPE_MAPPED);
            let include_space = f.context().options.bracket_spacing;
            let break_parameter_clause = f.context().has_annotation(parameter.constraint)
                || f.context().node_has_newline(parameter.constraint)
                || is_expression_breakable(tree, tree.get(parameter.constraint));
            let inline_separator = if include_space {
                soft_line_break_or_space()
            } else {
                soft_line_break()
            };
            let field_terminator = ";";

            let format_parameter_clause = |f: &mut DestackFormatter<'ast, '_>,
                                           break_between_name_and_in: bool|
             -> FormatResult<()> {
                write!(f, [token("["), parameter.name])?;

                if break_between_name_and_in {
                    write!(
                        f,
                        [indent(&format_args![
                            hard_line_break(),
                            Keyword::In,
                            space(),
                            parameter.constraint
                        ])]
                    )?;
                } else {
                    write!(f, [space(), Keyword::In, space(), parameter.constraint])?;
                }

                if let Some(key_remap) = parameter.key_remap {
                    write!(f, [space(), Keyword::As, space(), key_remap])?;
                }

                write!(f, [token("]")])
            };

            let inner_multiline = format_with(|f| {
                match modifiers.readonly {
                    TypeModifier::Add => write!(f, [token("readonly"), space()])?,
                    TypeModifier::Remove => {
                        write!(f, [token("-readonly"), space()])?;
                    }
                    TypeModifier::None => {}
                }

                format_parameter_clause(f, break_parameter_clause)?;

                match modifiers.optional {
                    TypeModifier::Add => write!(f, [token("?")])?,
                    TypeModifier::Remove => {
                        write!(f, [token("-?")])?;
                    }
                    TypeModifier::None => {}
                }

                write!(f, [token(":"), space()])?;

                let has_value_postfix_annotations = f.context().has_postfix_annotation(*value);
                if f.context().options.language_type.is_typescript()
                    && has_value_postfix_annotations
                {
                    let value_expression = f.context().tree.get(*value);
                    write!(f, [f.context().any_prefix_annotations(*value)])?;
                    format_expression(
                        f,
                        *value,
                        value_expression,
                        directive_for_node(f.context(), *value),
                    )?;
                    write!(f, [token(field_terminator)])?;
                    write!(f, [f.context().any_infix_or_postfix_annotations(*value)])
                } else {
                    write!(f, [*value, token(field_terminator)])
                }
            });

            let inner_flat = format_with(|f| {
                match modifiers.readonly {
                    TypeModifier::Add => write!(f, [token("readonly"), space()])?,
                    TypeModifier::Remove => {
                        write!(f, [token("-readonly"), space()])?;
                    }
                    TypeModifier::None => {}
                }

                format_parameter_clause(f, false)?;

                match modifiers.optional {
                    TypeModifier::Add => write!(f, [token("?")])?,
                    TypeModifier::Remove => {
                        write!(f, [token("-?")])?;
                    }
                    TypeModifier::None => {}
                }

                write!(f, [token(":"), space(), *value])
            });

            let mapped_multiline = format_with(|f| {
                write!(
                    f,
                    [
                        token("{"),
                        hard_line_break(),
                        block_indent(&inner_multiline),
                        hard_line_break(),
                        token("}")
                    ]
                )
            });

            let mapped_flat = format_with(|f| {
                write!(
                    f,
                    [
                        token("{"),
                        indent(&format_args![inline_separator, inner_flat]),
                        inline_separator,
                        token("}")
                    ]
                )
            });

            let force_multiline = should_force_multiline_mapped_type(f.context(), node_id, *value);
            if force_multiline {
                write!(f, [mapped_multiline])?;
            } else {
                write!(f, [group(&mapped_flat)])?;
            }
        }

        // type index
        Expression::TypeIndex { left, index } => {
            format_type_index_expression(f, *left, *index)?;
        }

        // array literal
        Expression::ArrayExpression {
            elements: elements_ids,
        } => {
            format_primary_array_expression(f, node_id, elements_ids)?;
        }

        // tuple literal
        Expression::TupleExpression {
            elements: elements_ids,
        } => {
            format_primary_tuple_expression(f, node_id, elements_ids)?;
        }

        // sequence expression (JS/TS comma operator)
        Expression::SequenceExpression { expressions } => {
            if expressions.is_empty() {
                write!(f, [token("()")])?;
            } else {
                let format_sequence = format_with(|f| {
                    let joiner_separator = format_with(|f| {
                        write!(
                            f,
                            [
                                token(","),
                                line_postfix_boundary(),
                                soft_line_break_or_space()
                            ]
                        )
                    });
                    let mut joiner = f.join_with(joiner_separator);
                    joiner.entries(expressions);
                    joiner.finish()
                });
                if sequence_expression_needs_parens(f.context(), node_id) {
                    write!(
                        f,
                        [group(&format_args![
                            token("("),
                            format_sequence,
                            token(")")
                        ])]
                    )?;
                } else {
                    write!(f, [group(&format_sequence)])?;
                }
            }
        }

        // struct literal
        Expression::ObjectExpression { ty, properties } => {
            let _timing = f
                .context()
                .timing_scope(timing::FORMAT_EXPRESSION_PRIMARY_OBJECT);
            format_struct_literal(f, node_id, ty, properties)?;
        }

        // tree literal
        Expression::TreeExpression {
            left,
            arguments,
            elements,
        } => {
            let _timing = f
                .context()
                .timing_scope(timing::FORMAT_EXPRESSION_PRIMARY_TREE);
            format_tree_literal_expression(f, node_id, left, arguments, elements)?;
        }

        // parenthesized
        Expression::Parenthesized { expression } => {
            format_primary_parenthesized_expression(f, node_id, *expression)?;
        }
        _ => return Ok(false),
    }

    Ok(true)
}

const PAREN_ASSIGNMENT_OBJECT_EXPAND_MIN_PROPERTIES: usize = 3;
const PAREN_ASSIGNMENT_ARRAY_EXPAND_MIN_ELEMENTS: usize = 4;

/// Format a parenthesized primary expression.
pub(crate) fn format_primary_parenthesized_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    expression_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let _timing = f
        .context()
        .timing_scope(timing::FORMAT_EXPRESSION_PRIMARY_PARENTHESES);

    let tree = f.context().tree;
    let expression = &expression_id;
    let inner_expression = tree.get(expression_id);
    let should_drop_parentheses = should_drop_parenthesized(
        f.context(),
        node_id,
        expression_id,
        ParenthesizedDropMode::ExpressionWrapper,
    );

    if should_drop_parentheses {
        write!(f, [expression_id])?;
    } else {
        let has_parenthesized_leading_inner_trivia =
            parenthesized_has_leading_inner_trivia(f.context(), node_id, expression_id);
        let has_parenthesized_leading_inner_comments =
            parenthesized_has_leading_inner_comments(f.context(), node_id, expression_id);
        let parenthesized_starts_with_type_operator =
            parenthesized_inner_starts_with_type_operator(f.context(), node_id, expression_id);
        let inner_expression_has_comments =
            f.context().has_comment(f.context().span(expression_id));
        let union_or_intersection_member_count =
            expression_union_or_intersection_member_count(f.context().tree, expression_id);
        let has_inner_decorator_prefix_annotation =
            expression_has_effective_decorator_prefix_annotation(f.context(), expression_id);
        let parent_is_postfix_continuation =
            expression_parent_is_postfix_continuation(f.context(), node_id);
        let is_in_assignment_value_context =
            expression_is_in_assignment_value_context(f.context(), node_id);
        let should_expand_assignment_target = match inner_expression {
            // prefer expanded destructuring targets once they become moderately wide
            Expression::ObjectExpression { properties, .. } => {
                properties.len() >= PAREN_ASSIGNMENT_OBJECT_EXPAND_MIN_PROPERTIES
                    && is_assignment_left_target(f.context(), expression_id)
            }
            Expression::ArrayExpression { elements } => {
                elements.len() >= PAREN_ASSIGNMENT_ARRAY_EXPAND_MIN_ELEMENTS
                    && is_assignment_left_target(f.context(), expression_id)
            }
            _ => false,
        };

        if should_expand_assignment_target {
            write!(
                f,
                [group(&format_args![
                    token("("),
                    group(expression).should_expand(true),
                    token(")")
                ])
                .should_expand(true)]
            )?;
        } else if let Expression::TreeExpression {
            arguments,
            elements,
            ..
        } = inner_expression
        {
            let tree_should_break = tree_literal_should_break(f.context(), arguments, elements)
                || f.context().node_has_newline(expression_id);
            if is_call_like_argument(f.context(), node_id) {
                write!(f, [expression_id])?;
            } else if has_parenthesized_leading_inner_trivia || tree_should_break {
                write!(
                    f,
                    [
                        token("("),
                        block_indent(&group(expression).should_expand(true)),
                        hard_line_break(),
                        token(")")
                    ]
                )?;
            } else {
                write!(f, [token("("), soft_block_indent(&expression), token(")")])?;
            }
        } else if !is_in_assignment_value_context
            && should_hoist_parenthesized_inner_cast_prefix_comments(
                f.context(),
                node_id,
                expression_id,
            )
        {
            let inner_directive = directive_for_node(f.context(), expression_id);
            let format_inner_without_prefix = format_with(|f| {
                format_expression(
                    f,
                    expression_id,
                    f.context().tree.get(expression_id),
                    inner_directive,
                )?;
                if !matches!(
                    inner_directive,
                    Some(FormatterDirective {
                        kind: FormatterDirectiveKind::IgnoreFormat,
                        position: FormatterDirectivePosition::Postfix { .. },
                    })
                ) {
                    write!(
                        f,
                        [f.context().any_infix_or_postfix_annotations(expression_id)]
                    )?;
                }
                Ok(())
            });
            write!(f, [f.context().any_prefix_annotations(expression_id)])?;
            write!(
                f,
                [group(&format_args![
                    token("("),
                    format_inner_without_prefix,
                    token(")")
                ])]
            )?;
        } else if has_inner_decorator_prefix_annotation
            || (is_in_assignment_value_context && has_parenthesized_leading_inner_trivia)
        {
            write!(
                f,
                [
                    token("("),
                    block_indent(&group(expression).should_expand(true)),
                    hard_line_break(),
                    token(")")
                ]
            )?;
        } else if has_parenthesized_leading_inner_trivia
            && f.context().has_postfix_annotation(expression_id)
            && !expression_is_ternary_branch(f.context(), node_id)
        {
            write!(
                f,
                [
                    token("("),
                    block_indent(&group(expression).should_expand(true)),
                    hard_line_break(),
                    token(")")
                ]
            )?;
        } else if (parenthesized_starts_with_type_operator && inner_expression_has_comments)
            || (expression_is_union_or_intersection_binary(inner_expression)
                && (expression_has_effective_prefix_annotation(f.context(), expression_id)
                    || has_parenthesized_leading_inner_comments
                    || inner_expression_has_comments))
            || union_or_intersection_member_count > 2
        {
            write!(f, [token("("), soft_block_indent(&expression), token(")")])?;
        } else if matches!(inner_expression, Expression::TypeConditional { .. }) {
            write!(f, [token("("), soft_block_indent(&expression), token(")")])?;
        } else if has_parenthesized_leading_inner_trivia
            && expression_has_effective_prefix_annotation(f.context(), expression_id)
        {
            write!(f, [token("("), soft_block_indent(&expression), token(")")])?;
        } else if parent_is_postfix_continuation
            && has_parenthesized_leading_inner_trivia
            && expression_is_await_like(inner_expression)
        {
            write!(
                f,
                [
                    token("("),
                    block_indent(&group(expression).should_expand(true)),
                    hard_line_break(),
                    token(")")
                ]
            )?;
        } else {
            // prefer one canonical wrapper layout for ordinary parenthesized expressions
            write!(f, [token("("), expression, token(")")])?;
        }

        let boundary_comments =
            parenthesized_boundary_comments(f.context(), node_id, expression_id);
        for comment_id in boundary_comments {
            write!(f, [space(), comment_id])?;
        }
    }

    Ok(())
}

/// Return whether one expression is await-like.
fn expression_is_await_like(expression: &Expression) -> bool {
    matches!(
        expression,
        Expression::Await { .. } | Expression::AwaitMaybe { .. } | Expression::Comptime { .. }
    )
}

/// Return whether one expression is a union or intersection binary expression.
fn expression_is_union_or_intersection_binary(expression: &Expression) -> bool {
    matches!(
        expression,
        Expression::Binary {
            operator: BinaryOperator::ElementwiseOr | BinaryOperator::ElementwiseAnd,
            ..
        }
    )
}

/// Return the member count for one union or intersection binary chain.
fn expression_union_or_intersection_member_count(
    tree: &NodeTree,
    expression_id: LocalNodeId<Expression>,
) -> usize {
    match tree.get(expression_id) {
        Expression::Binary {
            operator: BinaryOperator::ElementwiseOr | BinaryOperator::ElementwiseAnd,
            left,
            right,
        } => {
            expression_union_or_intersection_member_count(tree, *left)
                + expression_union_or_intersection_member_count(tree, *right)
        }
        _ => 1,
    }
}

/// Return whether one parenthesized inner expression starts with a type operator token.
fn parenthesized_inner_starts_with_type_operator(
    context: &DestackFormatContext<'_>,
    parenthesized_id: LocalNodeId<Expression>,
    inner_expression_id: LocalNodeId<Expression>,
) -> bool {
    let parenthesized_span = context.span(parenthesized_id);
    let inner_span = context.span(inner_expression_id);
    let mut search_start = parenthesized_span.start.saturating_add(1);
    if search_start >= inner_span.end || parenthesized_span.file != inner_span.file {
        return false;
    }

    loop {
        let Some(token) = context.first_non_trivia_token_between(search_start, inner_span.end)
        else {
            return false;
        };

        if token.token.ty == ast::TokenType::OpenParenthesis {
            if token.span.end <= search_start {
                return false;
            }
            search_start = token.span.end;
            continue;
        }

        return matches!(
            token.token.ty,
            ast::TokenType::ElementwiseOr | ast::TokenType::ElementwiseAnd
        );
    }
}

/// Return whether one expression starts with one effective prefix annotation.
fn expression_has_effective_prefix_annotation(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let mut current_id = expression_id;

    loop {
        if context.has_prefix_annotation(current_id) {
            return true;
        }

        let next_id = match context.tree.get(current_id) {
            Expression::Statement(expression) | Expression::Parenthesized { expression } => {
                Some(*expression)
            }
            Expression::Binary { left, .. } | Expression::TypeBinary { left, .. } => Some(*left),
            _ => None,
        };
        let Some(next_id) = next_id else {
            return false;
        };
        current_id = next_id;
    }
}

/// Return whether one parenthesized expression continues into a postfix chain parent.
fn expression_parent_is_postfix_continuation(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    context
        .parent(node_id)
        .is_some_and(|(parent_id, parent_type)| {
            if parent_type != NodeType::Expression {
                return false;
            }

            matches!(
                context.tree.get(LocalNodeId::<Expression>::new(parent_id)),
                Expression::Member { .. }
                    | Expression::PrivateMember { .. }
                    | Expression::Index { .. }
                    | Expression::Call { .. }
                    | Expression::New { .. }
                    | Expression::Must { .. }
                    | Expression::Maybe { .. }
            )
        })
}

/// Return whether one expression is a ternary consequent or alternate branch.
fn expression_is_ternary_branch(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let Some((parent_id, parent_type)) = context.parent(node_id) else {
        return false;
    };
    if parent_type != NodeType::Expression {
        return false;
    }

    let parent_id = LocalNodeId::<Expression>::new(parent_id);
    let Expression::If {
        kind: ast::IfKind::Ternary,
        then_expression,
        else_expression,
        ..
    } = context.tree.get(parent_id)
    else {
        return false;
    };

    *then_expression == node_id
        || else_expression.is_some_and(|expression_id| expression_id == node_id)
}

/// Return whether a parenthesized expression is one assignment or declarator value.
fn expression_is_in_assignment_value_context(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let mut current_id = node_id;

    loop {
        let Some((parent_id, parent_type)) = context.parent(current_id) else {
            return false;
        };

        match parent_type {
            NodeType::Expression => {
                let parent_id = LocalNodeId::<Expression>::new(parent_id);
                let parent_expression = context.tree.get(parent_id);
                if matches!(
                    parent_expression,
                    Expression::Assign { right, .. } if *right == current_id
                ) {
                    return true;
                }

                if let Expression::If {
                    kind: ast::IfKind::Ternary,
                    condition,
                    then_expression,
                    else_expression,
                } = parent_expression
                {
                    let is_ternary_test = matches!(
                        condition,
                        ast::IfCondition::Expression { condition }
                            if *condition == current_id
                    );
                    let is_ternary_branch = *then_expression == current_id
                        || else_expression
                            .is_some_and(|else_expression| else_expression == current_id);

                    if is_ternary_branch {
                        return false;
                    }

                    if !is_ternary_test {
                        return false;
                    }
                }

                current_id = parent_id;
            }
            NodeType::Declarator => return true,
            _ => return false,
        }
    }
}

/// Return whether an expression or wrapped declaration has decorator prefix annotations.
fn expression_has_effective_decorator_prefix_annotation(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let expression_has_decorator = context.visit_annotations(expression_id, |annotations| {
        annotations.iter().any(|annotation_id| {
            matches!(
                context.annotation(*annotation_id),
                Annotation::Decorator {
                    position: AnnotationPosition::BlockPrefix | AnnotationPosition::LinePrefix,
                    ..
                }
            )
        })
    });
    if expression_has_decorator.unwrap_or(false) {
        return true;
    }

    let Expression::Declaration(declaration_id) = context.tree.get(expression_id) else {
        return false;
    };
    context
        .visit_annotations(declaration_id.clone(), |annotations| {
            annotations.iter().any(|annotation_id| {
                matches!(
                    context.annotation(*annotation_id),
                    Annotation::Decorator {
                        position: AnnotationPosition::BlockPrefix | AnnotationPosition::LinePrefix,
                        ..
                    }
                )
            })
        })
        .unwrap_or(false)
}
