use super::*;
use destack_fir::{format_args, write};

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
    let comment_tokens = collect_comment_tokens(context);

    for comment_token in comment_tokens {
        if comment_token.token.ty != TokenType::LineComment {
            continue;
        }

        if previous_non_whitespace_before_span(context, comment_token.span) != Some(':') {
            continue;
        }

        let comment_source = context.get_token_str(comment_token).trim().to_string();
        if comment_source.is_empty() {
            continue;
        }

        for (branch_index, (_, then_id)) in branches.iter().enumerate() {
            let then_span = context.get_span(*then_id);
            let else_start = if branch_index + 1 < branches.len() {
                context.get_span(branches[branch_index + 1].0).start
            } else {
                final_else
                    .map(|else_id| context.get_span(else_id).start)
                    .unwrap_or(u32::MAX)
            };

            if comment_token.span.start >= then_span.end && comment_token.span.end <= else_start {
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

    collect_comment_tokens(context)
        .into_iter()
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

/// Return the first non-whitespace character after a span.
pub(super) fn next_non_whitespace_after_span(
    context: &DestackFormatContext<'_>,
    span: Span,
) -> Option<char> {
    if span.end >= context.file.len {
        return None;
    }

    let tail_span = Span::new(span.file, span.end, context.file.len);
    let tail_source = context.file.get_span_str(tail_span)?;
    tail_source
        .chars()
        .find(|character: &char| !character.is_whitespace())
}

/// Collect postfix star comments from an inner expression that should render after `)`.
pub(super) fn collect_parenthesized_boundary_comments(
    context: &DestackFormatContext<'_>,
    parenthesized_id: LocalNodeId<Expression>,
    inner_expression_id: LocalNodeId<Expression>,
) -> Vec<String> {
    if matches!(
        context.tree.get(inner_expression_id),
        Expression::TreeExpression { .. }
    ) {
        return Vec::new();
    }

    let parenthesized_span = context.get_span(parenthesized_id);
    let inner_span = context.get_span(inner_expression_id);
    let mut comments: Vec<(u32, String)> = Vec::new();

    for comment_token in collect_comment_tokens(context) {
        if !matches!(
            comment_token.token.ty,
            TokenType::BlockComment | TokenType::DocBlockComment
        ) {
            continue;
        }
        if comment_token.span.start < inner_span.end
            || comment_token.span.end > parenthesized_span.end
        {
            continue;
        }

        let comment_source = context.get_token_str(comment_token).trim().to_string();
        if comment_source.is_empty() {
            continue;
        }
        if next_non_whitespace_after_span(context, comment_token.span) != Some(')') {
            continue;
        }

        comments.push((comment_token.span.start, comment_source));
    }

    comments.sort_by_key(|(start, _)| *start);
    comments.into_iter().map(|(_, source)| source).collect()
}

/// Return whether source contains leading trivia between `(` and the inner expression.
pub(super) fn parenthesized_has_leading_inner_trivia(
    context: &DestackFormatContext<'_>,
    parenthesized_id: LocalNodeId<Expression>,
    inner_expression_id: LocalNodeId<Expression>,
) -> bool {
    let parenthesized_span = context.get_span(parenthesized_id);
    let inner_span = context.get_span(inner_expression_id);

    let leading_start = parenthesized_span.start.saturating_add(1);
    if leading_start >= inner_span.start || parenthesized_span.file != inner_span.file {
        return false;
    }

    let leading_span = Span::new(parenthesized_span.file, leading_start, inner_span.start);
    let leading_source = context.get_span_str(leading_span);
    leading_source.contains('\n') || leading_source.contains("/*") || leading_source.contains("//")
}

/// Return whether source contains leading comments between `(` and the inner expression.
pub(super) fn parenthesized_has_leading_inner_comments(
    context: &DestackFormatContext<'_>,
    parenthesized_id: LocalNodeId<Expression>,
    inner_expression_id: LocalNodeId<Expression>,
) -> bool {
    let parenthesized_span = context.get_span(parenthesized_id);
    let inner_span = context.get_span(inner_expression_id);

    let leading_start = parenthesized_span.start.saturating_add(1);
    if leading_start >= inner_span.start || parenthesized_span.file != inner_span.file {
        return false;
    }

    let leading_span = Span::new(parenthesized_span.file, leading_start, inner_span.start);
    let leading_source = context.get_span_str(leading_span);
    leading_source.contains("/*") || leading_source.contains("//")
}

/// Return whether source for a member expression includes optional chaining syntax.
pub(super) fn member_expression_source_has_optional_chain(
    context: &DestackFormatContext<'_>,
    member_id: LocalNodeId<Expression>,
) -> bool {
    let member_span = context.get_span(member_id);
    context.get_span_str(member_span).contains("?.")
}

/// Decide whether a parenthesized expression can be unwrapped in member object position.
pub(super) fn should_unwrap_parenthesized_member_object(
    context: &DestackFormatContext<'_>,
    parenthesized_id: LocalNodeId<Expression>,
    inner_expression_id: LocalNodeId<Expression>,
) -> bool {
    if context.has_annotation(parenthesized_id) || context.has_annotation(inner_expression_id) {
        return false;
    }

    if parenthesized_has_leading_inner_comments(context, parenthesized_id, inner_expression_id) {
        return false;
    }

    !needs_parens_in_postfix_position(context.tree, inner_expression_id)
}

/// Return whether a member object should keep parentheses as a `new` callee.
pub(super) fn member_object_prefers_new_callee_parentheses(
    context: &DestackFormatContext<'_>,
    object_id: LocalNodeId<Expression>,
) -> bool {
    let mut current_id = object_id;

    while let Expression::Parenthesized { expression } = context.tree.get(current_id) {
        if context.has_annotation(current_id)
            || parenthesized_has_leading_inner_trivia(context, current_id, *expression)
        {
            return false;
        }
        current_id = *expression;
    }

    matches!(
        context.tree.get(current_id),
        Expression::Call { .. } | Expression::Instantiation { .. }
    )
}

/// Return whether a member object is simple enough for `new a.b()` style callee formatting.
pub(super) fn is_simple_new_member_object(
    tree: &NodeTree,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    match tree.get(expression_id) {
        Expression::Path { .. }
        | Expression::This
        | Expression::Super
        | Expression::PrivateIdentifier { .. } => true,
        Expression::Member { left, .. } | Expression::PrivateMember { left, .. } => {
            is_simple_new_member_object(tree, *left)
        }
        Expression::Parenthesized { expression } => is_simple_new_member_object(tree, *expression),
        _ => false,
    }
}

/// Decide whether `new (<member>)()` can unwrap outer parentheses.
pub(super) fn should_unwrap_parenthesized_new_member_callee(
    context: &DestackFormatContext<'_>,
    parenthesized_id: LocalNodeId<Expression>,
    inner_expression_id: LocalNodeId<Expression>,
) -> bool {
    if context.has_annotation(parenthesized_id) || context.has_annotation(inner_expression_id) {
        return false;
    }

    if parenthesized_has_leading_inner_trivia(context, parenthesized_id, inner_expression_id) {
        return false;
    }

    match context.tree.get(inner_expression_id) {
        Expression::Path { .. } => true,
        Expression::Member { left, .. } | Expression::PrivateMember { left, .. } => {
            if member_expression_source_has_optional_chain(context, inner_expression_id) {
                return false;
            }

            is_simple_new_member_object(context.tree, *left)
        }
        _ => false,
    }
}

/// Remove one surrounding pair of parentheses from text when present.
pub(super) fn strip_one_wrapping_parentheses(source: &str) -> &str {
    let trimmed = source.trim();
    if trimmed.len() < 2 || !trimmed.starts_with('(') || !trimmed.ends_with(')') {
        return trimmed;
    }
    trimmed[1..trimmed.len() - 1].trim()
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

/// Format a ternary expression with Prettier-style breaking.
/// Nested ternaries get progressive indentation when they break.
pub(super) fn format_ternary(
    f: &mut DestackFormatter<'_, '_>,
    node_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let tree = f.context().tree;
    let (branches, final_else) = collect_ternary_chain(tree, node_id);
    let (question_comments, colon_comments) = collect_statement_ternary_boundary_prefix_comments(
        f.context(),
        node_id,
        &branches,
        final_else,
    );
    let colon_line_comments =
        collect_ternary_colon_line_comments(f.context(), &branches, final_else);
    let has_separator_comments = question_comments
        .iter()
        .any(|comments| !comments.is_empty())
        || colon_comments.iter().any(|comments| !comments.is_empty());
    let use_compact_tree_layout = !has_separator_comments
        && (branches
            .iter()
            .any(|(_, then_expr)| ternary_branch_is_tree_like(f.context(), *then_expr))
            || final_else.is_some_and(|final_else_id| {
                ternary_branch_is_tree_like(f.context(), final_else_id)
            }));

    if branches.len() == 1 {
        // simple ternary
        let (condition, then_expr) = branches[0];
        let first_else_start = ternary_else_start_for_branch(f.context(), &branches, final_else, 0);
        let then_has_boundary_comment_before_colon = ternary_then_has_boundary_comment_before_colon(
            f.context(),
            then_expr,
            first_else_start,
        );
        if use_compact_tree_layout {
            write!(
                f,
                [group(&format_args![
                    condition,
                    space(),
                    token("?"),
                    space(),
                    format_with(|f| {
                        if let Some(comments) = question_comments.first() {
                            write_ternary_separator_comments(f, comments)?;
                        }
                        Ok(())
                    }),
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
                        if let Some(comments) = colon_comments.first() {
                            write_ternary_separator_comments(f, comments)?;
                        }
                        if let Some(comments) = colon_line_comments.first() {
                            write_ternary_colon_line_comments(f, comments)?;
                            if !comments.is_empty() {
                                write!(f, [hard_line_break()])?;
                            }
                        }
                        Ok(())
                    }),
                    final_else
                ])]
            )?;
        } else {
            write!(
                f,
                [group(&format_args![
                    condition,
                    indent(&format_args![
                        soft_line_break_or_space(),
                        token("?"),
                        space(),
                        format_with(|f| {
                            if let Some(comments) = question_comments.first() {
                                write_ternary_separator_comments(f, comments)?;
                            }
                            Ok(())
                        }),
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
                            if let Some(comments) = colon_comments.first() {
                                write_ternary_separator_comments(f, comments)?;
                            }
                            if let Some(comments) = colon_line_comments.first() {
                                write_ternary_colon_line_comments(f, comments)?;
                                if !comments.is_empty() {
                                    write!(f, [hard_line_break()])?;
                                }
                            }
                            Ok(())
                        }),
                        indent(&format_args![final_else])
                    ]),
                ])]
            )?;
        }
    } else {
        // nested ternary chain: all branches at same indent level
        write!(
            f,
            [group(&format_with(|f| {
                for (branch_index, (condition, then_expr)) in branches.iter().enumerate() {
                    let branch_else_start = ternary_else_start_for_branch(
                        f.context(),
                        &branches,
                        final_else,
                        branch_index,
                    );
                    let branch_then_has_boundary_comment_before_colon =
                        ternary_then_has_boundary_comment_before_colon(
                            f.context(),
                            *then_expr,
                            branch_else_start,
                        );
                    write!(f, [condition])?;
                    if use_compact_tree_layout {
                        write!(
                            f,
                            [
                                space(),
                                token("?"),
                                space(),
                                format_with(|f| {
                                    if let Some(comments) = question_comments.get(branch_index) {
                                        write_ternary_separator_comments(f, comments)?;
                                    }
                                    Ok(())
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
                                }),
                            ]
                        )?;
                    } else {
                        write!(
                            f,
                            [indent(&format_args![
                                soft_line_break_or_space(),
                                token("?"),
                                space(),
                                format_with(|f| {
                                    if let Some(comments) = question_comments.get(branch_index) {
                                        write_ternary_separator_comments(f, comments)?;
                                    }
                                    Ok(())
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
                                }),
                            ])]
                        )?;
                    }
                }
                write!(f, [final_else])
            }))]
        )?;
    }

    if ternary_requires_terminator(f.context(), node_id) {
        write!(f, [token(";")])?;
    }

    Ok(())
}
