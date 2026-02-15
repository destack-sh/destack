use super::*;
use crate::scan::next_non_whitespace_after_span;
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

/// Return statement-level prefix comment text grouped by ternary branch boundary.
///
/// The first vector contains comments before `?` for each branch.
/// The second vector contains comments before `:` for each branch.
pub(super) fn collect_statement_ternary_boundary_prefix_comments(
    context: &DestackFormatContext<'_>,
    ternary_id: LocalNodeId<Expression>,
    branches: &[(LocalNodeId<Expression>, LocalNodeId<Expression>)],
    final_else: Option<LocalNodeId<Expression>>,
) -> (Vec<Vec<String>>, Vec<Vec<String>>) {
    let mut question_comments: Vec<Vec<(u32, String)>> = vec![Vec::new(); branches.len()];
    let mut colon_comments: Vec<Vec<(u32, String)>> = vec![Vec::new(); branches.len()];

    let Some((parent_id, parent_type)) = context.get_parent(ternary_id) else {
        return (Vec::new(), Vec::new());
    };
    if parent_type != NodeType::Expression {
        return (Vec::new(), Vec::new());
    }

    let statement_id = LocalNodeId::<Expression>::new(parent_id);
    let Expression::Statement(inner_id) = context.tree.get(statement_id) else {
        return (Vec::new(), Vec::new());
    };
    if *inner_id != ternary_id {
        return (Vec::new(), Vec::new());
    }

    let Some(statement_annotations) = context.get_annotations(statement_id) else {
        return (Vec::new(), Vec::new());
    };

    for annotation_id in statement_annotations {
        let Annotation::Comment { node, position } = context.tree.get::<Annotation>(annotation_id)
        else {
            continue;
        };
        if *position != AnnotationPosition::LinePrefix {
            continue;
        }

        let comment = context.tree.get::<destack_ast::Comment>(*node);
        if comment.style != destack_ast::CommentStyle::Star {
            continue;
        }

        let annotation_span = context.get_span::<Annotation>(annotation_id);
        let annotation_text = context.get_span_str(annotation_span).trim().to_string();
        if annotation_text.is_empty() {
            continue;
        }

        for (branch_index, (condition_id, then_id)) in branches.iter().enumerate() {
            let condition_span = context.get_span(*condition_id);
            let then_span = context.get_span(*then_id);

            if annotation_span.start >= condition_span.end && annotation_span.end <= then_span.start
            {
                question_comments[branch_index].push((annotation_span.start, annotation_text));
                break;
            }

            let else_start = if branch_index + 1 < branches.len() {
                context.get_span(branches[branch_index + 1].0).start
            } else {
                final_else
                    .map(|else_id| context.get_span(else_id).start)
                    .unwrap_or(u32::MAX)
            };

            if annotation_span.start >= then_span.end && annotation_span.end <= else_start {
                colon_comments[branch_index].push((annotation_span.start, annotation_text));
                break;
            }
        }
    }

    let question_comments = question_comments
        .into_iter()
        .map(|mut comments| {
            comments.sort_by_key(|(start, _)| *start);
            comments
                .into_iter()
                .map(|(_, text)| text)
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let colon_comments = colon_comments
        .into_iter()
        .map(|mut comments| {
            comments.sort_by_key(|(start, _)| *start);
            comments
                .into_iter()
                .map(|(_, text)| text)
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();

    (question_comments, colon_comments)
}

/// Write inline block comments captured around ternary separators.
pub(super) fn write_ternary_separator_comments<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    comments: &[String],
) -> FormatResult<()> {
    for comment in comments {
        write!(f, [text(comment.as_str()), space()])?;
    }
    Ok(())
}

/// Write line comments captured around ternary `:` separators.
pub(super) fn write_ternary_colon_line_comments<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    comments: &[String],
) -> FormatResult<()> {
    for (comment_index, comment) in comments.iter().enumerate() {
        if comment_index > 0 {
            write!(f, [hard_line_break()])?;
        }
        write!(f, [text(comment.as_str())])?;
    }

    Ok(())
}

/// Collect slash comments that appear after `:` and before the else branch.
pub(super) fn collect_ternary_colon_line_comments(
    context: &DestackFormatContext<'_>,
    branches: &[(LocalNodeId<Expression>, LocalNodeId<Expression>)],
    final_else: Option<LocalNodeId<Expression>>,
) -> Vec<Vec<String>> {
    let mut colon_line_comments: Vec<Vec<(u32, String)>> = vec![Vec::new(); branches.len()];
    let comment_tokens = context.comment_tokens();

    for comment_token in comment_tokens.iter().copied() {
        if comment_token.token.ty != TokenType::LineComment {
            continue;
        }

        if !line_comment_follows_colon_on_same_line(context, comment_token.span) {
            continue;
        }

        let comment_source = context.get_token_str(comment_token).trim().to_string();
        if comment_source.is_empty() {
            continue;
        }

        for (branch_index, (_, then_id)) in branches.iter().enumerate() {
            let then_span = context.get_span(*then_id);
            let else_expression_id = if branch_index + 1 < branches.len() {
                Some(branches[branch_index + 1].0)
            } else {
                final_else
            };
            let else_start = if branch_index + 1 < branches.len() {
                context.get_span(branches[branch_index + 1].0).start
            } else {
                final_else
                    .map(|else_id| context.get_span(else_id).start)
                    .unwrap_or(u32::MAX)
            };

            if comment_token.span.start >= then_span.end && comment_token.span.end <= else_start {
                if else_expression_id.is_some_and(|else_id| {
                    expression_has_matching_line_prefix_comment_span(
                        context,
                        else_id,
                        comment_token.span,
                    )
                }) {
                    break;
                }
                colon_line_comments[branch_index].push((comment_token.span.start, comment_source));
                break;
            }
        }
    }

    colon_line_comments
        .into_iter()
        .map(|mut comments| {
            comments.sort_by_key(|(start, _)| *start);
            comments
                .into_iter()
                .map(|(_, comment)| comment)
                .collect::<Vec<_>>()
        })
        .collect()
}

/// Return whether one expression already owns a line-prefix slash comment at `comment_span`.
fn expression_has_matching_line_prefix_comment_span(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
    comment_span: Span,
) -> bool {
    let Some(annotation_ids) = context.get_annotations(expression_id) else {
        return false;
    };

    annotation_ids.into_iter().any(|annotation_id| {
        let Annotation::Comment { node, position } = context.tree.get::<Annotation>(annotation_id)
        else {
            return false;
        };
        if *position != AnnotationPosition::LinePrefix {
            return false;
        }
        let comment = context.tree.get::<destack_ast::Comment>(*node);
        if comment.style != destack_ast::CommentStyle::Slash {
            return false;
        }
        let annotation_span = context.get_span::<Annotation>(annotation_id);
        annotation_span.start == comment_span.start && annotation_span.end == comment_span.end
    })
}

/// Return whether a line comment appears after `:` on the same source line.
fn line_comment_follows_colon_on_same_line(
    context: &DestackFormatContext<'_>,
    comment_span: Span,
) -> bool {
    let Some((line_index, _)) = context.file.get_position(comment_span.start) else {
        return false;
    };
    let Some(line_span) = context.file.get_line_span(line_index) else {
        return false;
    };

    let line_prefix_span = Span::new(comment_span.file, line_span.start, comment_span.start);
    let line_prefix_source = context.get_span_str(line_prefix_span);
    line_prefix_source.trim_end().ends_with(':')
}

/// Return the source start offset for the else side of a ternary branch.
pub(super) fn ternary_else_start_for_branch(
    context: &DestackFormatContext<'_>,
    branches: &[(LocalNodeId<Expression>, LocalNodeId<Expression>)],
    final_else: Option<LocalNodeId<Expression>>,
    branch_index: usize,
) -> u32 {
    if branch_index + 1 < branches.len() {
        return context.get_span(branches[branch_index + 1].0).start;
    }

    final_else
        .map(|else_id| context.get_span(else_id).start)
        .unwrap_or(u32::MAX)
}

/// Return whether source has a block comment boundary before `:` for a ternary branch.
pub(super) fn ternary_then_has_boundary_comment_before_colon(
    context: &DestackFormatContext<'_>,
    then_expression_id: LocalNodeId<Expression>,
    else_start: u32,
) -> bool {
    let then_span = context.get_span(then_expression_id);

    context
        .comment_tokens()
        .iter()
        .copied()
        .any(|comment_token| {
            if !matches!(
                comment_token.token.ty,
                TokenType::BlockComment | TokenType::DocBlockComment
            ) {
                return false;
            }
            if comment_token.span.start < then_span.end || comment_token.span.end > else_start {
                return false;
            }
            next_non_whitespace_after_span(context, comment_token.span) == Some(':')
        })
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
        let Annotation::Comment { node, position } = context.tree.get::<Annotation>(annotation_id)
        else {
            continue;
        };
        if !matches!(
            position,
            AnnotationPosition::LinePostfix | AnnotationPosition::LinePostfixBoundary
        ) {
            continue;
        }

        let comment = context.tree.get::<destack_ast::Comment>(*node);
        if comment.style != destack_ast::CommentStyle::Star {
            continue;
        }

        let annotation_span = context.get_span::<Annotation>(annotation_id);
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
    question_comments: &[Vec<String>],
    colon_comments: &[Vec<String>],
    colon_line_comments: &[Vec<String>],
) -> bool {
    let has_separator_comments = question_comments
        .iter()
        .any(|comments| !comments.is_empty())
        || colon_comments.iter().any(|comments| !comments.is_empty());
    let has_colon_line_comments = colon_line_comments.iter().any(|comments| !comments.is_empty());
    let has_branch_prefix_annotations = branches.iter().any(|(_, then_expr)| {
        context.has_prefix_annotation(*then_expr)
    }) || final_else.is_some_and(|final_else_id| context.has_prefix_annotation(final_else_id));

    !has_separator_comments
        && !has_colon_line_comments
        && !has_branch_prefix_annotations
        && (branches
            .iter()
            .any(|(_, then_expr)| ternary_branch_is_tree_like(context, *then_expr))
            || final_else
                .is_some_and(|final_else_id| ternary_branch_is_tree_like(context, final_else_id)))
}

/// Write question-mark separator comments for one ternary branch.
fn write_ternary_question_comments<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    question_comments: &[Vec<String>],
    branch_index: usize,
) -> FormatResult<()> {
    if let Some(comments) = question_comments.get(branch_index) {
        write_ternary_separator_comments(f, comments)?;
    }

    Ok(())
}

/// Write colon separator comments for one ternary branch.
fn write_ternary_colon_comments_at_index<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    colon_comments: &[Vec<String>],
    colon_line_comments: &[Vec<String>],
    branch_index: usize,
) -> FormatResult<()> {
    if let Some(comments) = colon_comments.get(branch_index) {
        write_ternary_separator_comments(f, comments)?;
    }

    if let Some(comments) = colon_line_comments.get(branch_index) {
        write_ternary_colon_line_comments(f, comments)?;
        if !comments.is_empty() {
            write!(f, [hard_line_break()])?;
        }
    }

    Ok(())
}

/// Format one single-branch ternary body including `?` and `:` separators.
fn format_single_ternary_branch<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    condition: LocalNodeId<Expression>,
    then_expr: LocalNodeId<Expression>,
    final_else: Option<LocalNodeId<Expression>>,
    use_compact_tree_layout: bool,
    then_has_boundary_comment_before_colon: bool,
    question_comments: &[Vec<String>],
    colon_comments: &[Vec<String>],
    colon_line_comments: &[Vec<String>],
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
                format_with(|f| write_ternary_question_comments(f, question_comments, 0)),
                then_expr,
                format_with(|f| {
                    if !then_has_boundary_comment_before_colon {
                        write!(f, [space()])?;
                    }
                    Ok(())
                }),
                token(":"),
                space(),
                format_with(|f| {
                    write_ternary_colon_comments_at_index(f, colon_comments, colon_line_comments, 0)
                }),
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
                format_with(|f| write_ternary_question_comments(f, question_comments, 0)),
                then_expr,
                format_with(|f| {
                    if then_has_boundary_comment_before_colon {
                        write!(f, [soft_line_break()])?;
                    } else {
                        write!(f, [soft_line_break_or_space()])?;
                    }
                    Ok(())
                }),
                token(":"),
                space(),
                format_with(|f| {
                    write_ternary_colon_comments_at_index(f, colon_comments, colon_line_comments, 0)
                }),
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
    question_comments: &[Vec<String>],
    colon_comments: &[Vec<String>],
    colon_line_comments: &[Vec<String>],
) -> FormatResult<()> {
    write!(
        f,
        [group(&format_with(|f| {
            // write each branch using shared separator comment policy
            for (branch_index, (condition, then_expr)) in branches.iter().enumerate() {
                let branch_else_start =
                    ternary_else_start_for_branch(f.context(), branches, final_else, branch_index);
                let branch_then_has_boundary_comment_before_colon =
                    ternary_then_has_boundary_comment_before_colon(
                        f.context(),
                        *then_expr,
                        branch_else_start,
                    );
                write!(f, [condition])?;

                // compact nested ternary branch
                if use_compact_tree_layout {
                    write!(
                        f,
                        [
                            space(),
                            token("?"),
                            space(),
                            format_with(|f| {
                                write_ternary_question_comments(f, question_comments, branch_index)
                            }),
                            then_expr,
                            format_with(|f| {
                                if !branch_then_has_boundary_comment_before_colon {
                                    write!(f, [space()])?;
                                }
                                Ok(())
                            }),
                            token(":"),
                            space(),
                            format_with(|f| {
                                write_ternary_colon_comments_at_index(
                                    f,
                                    colon_comments,
                                    colon_line_comments,
                                    branch_index,
                                )
                            }),
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
                        format_with(|f| {
                            write_ternary_question_comments(f, question_comments, branch_index)
                        }),
                        then_expr,
                        format_with(|f| {
                            if branch_then_has_boundary_comment_before_colon {
                                write!(f, [soft_line_break()])?;
                            } else {
                                write!(f, [soft_line_break_or_space()])?;
                            }
                            Ok(())
                        }),
                        token(":"),
                        space(),
                        format_with(|f| {
                            write_ternary_colon_comments_at_index(
                                f,
                                colon_comments,
                                colon_line_comments,
                                branch_index,
                            )
                        }),
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

    // collect separator comments for `?` and `:` boundaries
    let (question_comments, colon_comments) = collect_statement_ternary_boundary_prefix_comments(
        f.context(),
        node_id,
        &branches,
        final_else,
    );
    let colon_line_comments =
        collect_ternary_colon_line_comments(f.context(), &branches, final_else);

    // decide compact vs expanded separator policy
    let use_compact_tree_layout = ternary_should_use_compact_tree_layout(
        f.context(),
        &branches,
        final_else,
        &question_comments,
        &colon_comments,
        &colon_line_comments,
    );

    // format one-branch ternary
    if branches.len() == 1 {
        let (condition, then_expr) = branches[0];
        let first_else_start = ternary_else_start_for_branch(f.context(), &branches, final_else, 0);
        let then_has_boundary_comment_before_colon = ternary_then_has_boundary_comment_before_colon(
            f.context(),
            then_expr,
            first_else_start,
        );
        format_single_ternary_branch(
            f,
            condition,
            then_expr,
            final_else,
            use_compact_tree_layout,
            then_has_boundary_comment_before_colon,
            &question_comments,
            &colon_comments,
            &colon_line_comments,
        )?;
    }
    // format nested ternary chains
    else {
        format_nested_ternary_branches(
            f,
            &branches,
            final_else,
            use_compact_tree_layout,
            &question_comments,
            &colon_comments,
            &colon_line_comments,
        )?;
    }

    // statement-position ternaries keep explicit terminators
    if ternary_requires_terminator(f.context(), node_id) {
        write!(f, [token(";")])?;
    }

    Ok(())
}
