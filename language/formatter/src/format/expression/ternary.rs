use super::*;
use destack_fir::{format_args, write};

/// Return the value expression for an argument.
pub(super) fn get_argument_value(
    tree: &NodeTree,
    argument_id: LocalNodeId<Argument>,
) -> Option<LocalNodeId<Expression>> {
    match tree.get(argument_id) {
        Argument::Positional { value, .. } => Some(*value),
        _ => None,
    }
}

/// Collect ternary chain into a flat list of (condition, then) pairs plus final else.
/// Collect nested ternary branches into a linear chain.
#[allow(clippy::type_complexity)]
pub(super) fn collect_ternary_chain(
    tree: &NodeTree,
    node_id: LocalNodeId<Expression>,
) -> (
    Vec<(LocalNodeId<Expression>, LocalNodeId<Expression>)>,
    Option<LocalNodeId<Expression>>,
) {
    let mut branches = Vec::new();
    let mut current = node_id;

    loop {
        let Expression::If {
            kind: IfKind::Ternary,
            condition,
            then_expression,
            else_expression,
            ..
        } = tree.get(current)
        else {
            break;
        };

        let condition_id = match condition {
            IfCondition::Expression { condition } => *condition,
            IfCondition::Let { .. } => break,
        };
        branches.push((condition_id, *then_expression));

        // check if else is another ternary
        let Some(else_id) = else_expression else {
            return (branches, None);
        };

        if matches!(
            tree.get(*else_id),
            Expression::If {
                kind: IfKind::Ternary,
                ..
            }
        ) {
            current = *else_id;
        } else {
            return (branches, Some(*else_id));
        }
    }

    (branches, None)
}

/// Collect trailing boundary comments that belong after `catch (<pattern>)`.
pub(super) fn collect_catch_pattern_trailing_boundary_comments(
    context: &DestackFormatContext<'_>,
    pattern_id: LocalNodeId<Pattern>,
) -> Vec<String> {
    let Some(annotations) = context.get_annotations(pattern_id) else {
        return Vec::new();
    };

    let pattern_span = context.get_span(pattern_id);
    let mut comments: Vec<(u32, String)> = Vec::new();

    for annotation_id in annotations {
        let Annotation::Comment { node, position } = context.get_annotation(annotation_id) else {
            continue;
        };
        if !matches!(
            position,
            AnnotationPosition::LinePostfix | AnnotationPosition::LinePostfixBoundary
        ) {
            continue;
        }

        let comment = context.tree.get::<destack_ast::Comment>(node);
        if comment.style != destack_ast::CommentStyle::Star {
            continue;
        }

        let annotation_span = context.get_annotation_span(annotation_id);
        if annotation_span.start <= pattern_span.end {
            continue;
        }

        let annotation_text = context.get_span_str(annotation_span).trim().to_string();
        if annotation_text.is_empty() {
            continue;
        }
        comments.push((annotation_span.start, annotation_text));
    }

    comments.sort_by_key(|(start, _)| *start);
    comments.into_iter().map(|(_, text)| text).collect()
}

/// Return whether a ternary expression appears in statement position.
pub(super) fn ternary_requires_terminator(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let Some((parent_id, parent_type)) = context.get_parent(node_id) else {
        return false;
    };

    if parent_type == NodeType::Block {
        return true;
    }

    if parent_type == NodeType::Expression {
        let parent_id = LocalNodeId::<Expression>::new(parent_id);
        if let Expression::Statement(inner_id) = context.tree.get(parent_id)
            && inner_id.id == node_id.id
        {
            return false;
        }
    }

    false
}

/// Return whether a ternary branch expression is tree-like and prefers compact separators.
pub(super) fn ternary_branch_is_tree_like(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let expression_id = transparent_inner_expression(context, expression_id);
    matches!(
        context.tree.get(expression_id),
        Expression::TreeExpression { .. }
    )
}

/// Return whether ternary formatting can use compact tree separators.
fn ternary_should_use_compact_tree_layout(
    context: &DestackFormatContext<'_>,
    branches: &[(LocalNodeId<Expression>, LocalNodeId<Expression>)],
    final_else: Option<LocalNodeId<Expression>>,
) -> bool {
    let has_branch_prefix_annotations = branches
        .iter()
        .any(|(_, then_expr)| context.has_prefix_annotation(*then_expr))
        || final_else.is_some_and(|final_else_id| context.has_prefix_annotation(final_else_id));

    !has_branch_prefix_annotations
        && (branches
            .iter()
            .any(|(_, then_expr)| ternary_branch_is_tree_like(context, *then_expr))
            || final_else
                .is_some_and(|final_else_id| ternary_branch_is_tree_like(context, final_else_id)))
}

/// Format one single-branch ternary body including `?` and `:` separators.
fn format_single_ternary_branch<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    condition: LocalNodeId<Expression>,
    then_expr: LocalNodeId<Expression>,
    final_else: Option<LocalNodeId<Expression>>,
    use_compact_tree_layout: bool,
) -> FormatResult<()> {
    // compact single-branch tree ternary
    if use_compact_tree_layout {
        write!(
            f,
            [group(&format_args![
                condition,
                space(),
                token("?"),
                space(),
                then_expr,
                space(),
                token(":"),
                space(),
                final_else
            ])]
        )?;
        return Ok(());
    }

    // expanded single-branch ternary
    write!(
        f,
        [group(&format_args![
            condition,
            indent(&format_args![
                soft_line_break_or_space(),
                token("?"),
                space(),
                then_expr,
                soft_line_break_or_space(),
                token(":"),
                space(),
                indent(&format_args![final_else])
            ]),
        ])]
    )?;

    Ok(())
}

/// Format nested ternary branches with shared separator and indentation policy.
fn format_nested_ternary_branches<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    branches: &[(LocalNodeId<Expression>, LocalNodeId<Expression>)],
    final_else: Option<LocalNodeId<Expression>>,
    use_compact_tree_layout: bool,
) -> FormatResult<()> {
    write!(
        f,
        [group(&format_with(|f| {
            // write each branch using shared separator comment policy
            for (condition, then_expr) in branches.iter() {
                write!(f, [condition])?;

                // compact nested ternary branch
                if use_compact_tree_layout {
                    write!(
                        f,
                        [
                            space(),
                            token("?"),
                            space(),
                            then_expr,
                            space(),
                            token(":"),
                            space(),
                        ]
                    )?;
                    continue;
                }

                // expanded nested ternary branch
                write!(
                    f,
                    [indent(&format_args![
                        soft_line_break_or_space(),
                        token("?"),
                        space(),
                        then_expr,
                        soft_line_break_or_space(),
                        token(":"),
                        space(),
                    ])]
                )?;
            }

            write!(f, [final_else])
        }))]
    )?;

    Ok(())
}

/// Format a ternary expression with Prettier-style breaking.
/// Nested ternaries get progressive indentation when they break.
pub(super) fn format_ternary(
    f: &mut DestackFormatter<'_, '_>,
    node_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let tree = f.context().tree;

    // collect flattened ternary branches
    let (branches, final_else) = collect_ternary_chain(tree, node_id);

    // decide compact vs expanded separator policy
    let use_compact_tree_layout =
        ternary_should_use_compact_tree_layout(f.context(), &branches, final_else);

    // format one-branch ternary
    if branches.len() == 1 {
        let (condition, then_expr) = branches[0];
        format_single_ternary_branch(f, condition, then_expr, final_else, use_compact_tree_layout)?;
    }
    // format nested ternary chains
    else {
        format_nested_ternary_branches(f, &branches, final_else, use_compact_tree_layout)?;
    }

    // statement-position ternaries keep explicit terminators
    if ternary_requires_terminator(f.context(), node_id) {
        write!(f, [token(";")])?;
    }

    Ok(())
}
