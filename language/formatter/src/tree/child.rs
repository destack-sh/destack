use super::attribute::argument_transparent_value_id;
use crate::TsppFormatContext;
use crate::chain::{
    chain_nodes, has_comment_between_expressions, member_has_intervening_comment,
    transparent_inner_expression,
};
use tspp_dir::{
    Block, BlockForm, Declaration, Expression, FunctionDeclaration, FunctionForm, IfForm, Literal,
    LocalNodeId, MatchArm, Node, NodeType, Tree, TreeChild, TreeStore,
};

/// Return whether one node span contains a line comment.
pub(crate) fn node_has_line_comment<T>(
    context: &TsppFormatContext<'_>,
    node_id: LocalNodeId<T>,
) -> bool
where
    T: Node,
    Tree: TreeStore<T>,
{
    let span = context.span(node_id);

    context
        .source_comments_in_range(span.start, span.end)
        .iter()
        .copied()
        .any(|comment| comment.is_line())
}

/// Return whether one tree child has a line comment outside the value span.
pub(crate) fn tree_child_has_outer_line_comment(
    context: &TsppFormatContext<'_>,
    child_id: LocalNodeId<TreeChild>,
    value_id: LocalNodeId<Expression>,
) -> bool {
    let child_span = context.span(child_id);
    let value_span = context.span(value_id);

    context
        .source_comments_in_range(child_span.start, value_span.start)
        .iter()
        .chain(
            context
                .source_comments_in_range(value_span.end, child_span.end)
                .iter(),
        )
        .any(|comment| comment.is_line())
}

/// Return whether one tree callback body forces multiline element layout.
fn tree_callback_body_requires_break(
    context: &TsppFormatContext<'_>,
    declaration_id: LocalNodeId<Declaration>,
) -> bool {
    let Declaration::Function(FunctionDeclaration { body, .. }) = context.tree.get(declaration_id)
    else {
        return false;
    };
    let Some(body_id) = *body else {
        return false;
    };

    let body_id = transparent_inner_expression(context, body_id);

    matches!(
        context.tree.get(body_id),
        Expression::Block(_) | Expression::TreeExpression { .. }
    )
}

/// Return whether one expression contains a callback body that forces tree breaks.
pub(crate) fn tree_expression_contains_callback_break(
    context: &TsppFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let expression_id = transparent_inner_expression(context, expression_id);

    match context.tree.get(expression_id) {
        Expression::Call {
            left, arguments, ..
        } => {
            if arguments.iter().copied().any(|argument_id| {
                argument_transparent_value_id(context, argument_id).is_some_and(|value_id| {
                    matches!(context.tree.get(value_id), Expression::Declaration(declaration_id)
                        if tree_callback_body_requires_break(context, *declaration_id))
                })
            }) {
                return true;
            }

            tree_expression_contains_callback_break(context, *left)
        }
        Expression::New { arguments, .. } => arguments.iter().copied().any(|argument_id| {
            argument_transparent_value_id(context, argument_id).is_some_and(|value_id| {
                matches!(context.tree.get(value_id), Expression::Declaration(declaration_id)
                        if tree_callback_body_requires_break(context, *declaration_id))
            })
        }),
        Expression::Member { left, .. }
        | Expression::Index { left, .. }
        | Expression::Maybe { left, .. }
        | Expression::Must { left, .. }
        | Expression::Chain { expression: left } => {
            tree_expression_contains_callback_break(context, *left)
        }
        Expression::Declaration(declaration_id) => {
            tree_callback_body_requires_break(context, *declaration_id)
        }
        Expression::Block(block_id) => context
            .tree
            .get(*block_id)
            .iter_expressions()
            .any(|expr_id| tree_expression_contains_callback_break(context, expr_id)),
        _ => false,
    }
}

/// Check whether a tree child expression should stay inline inside `{ ... }`.
pub(crate) fn tree_child_should_inline_braced_expression(
    context: &TsppFormatContext<'_>,
    child_id: LocalNodeId<TreeChild>,
) -> bool {
    let TreeChild::Expression { value } = context.tree.get(child_id) else {
        return false;
    };
    let value_id = transparent_inner_expression(context, *value);
    let value_expression = context.tree.get(value_id);
    let child_span = context.span(child_id);
    let value_span = context.span(value_id);

    if child_span.file == value_span.file
        && child_span.start < value_span.start
        && !context
            .source_comments_in_range(child_span.start, value_span.start)
            .is_empty()
    {
        return false;
    }

    if tree_child_has_outer_line_comment(context, child_id, value_id) {
        return false;
    }

    match value_expression {
        Expression::Literal(Literal::String(_)) | Expression::Literal(Literal::Character(_)) => {
            true
        }
        Expression::ArrayExpression { .. }
        | Expression::ObjectExpression { .. }
        | Expression::StructExpression { .. }
        | Expression::Call { .. }
        | Expression::TemplateExpression { .. }
        | Expression::TaggedTemplateExpression { .. }
        | Expression::Await { .. }
        | Expression::AwaitMaybe { .. }
        | Expression::AwaitMust { .. }
        | Expression::Binary { .. }
        | Expression::Member { .. }
        | Expression::Index { .. }
        | Expression::Maybe { .. }
        | Expression::Must { .. } => !expression_chain_has_separator_comment(context, value_id),
        Expression::If {
            form: IfForm::Ternary,
            condition,
            then_expression,
            else_expression,
            ..
        } => {
            let has_branch_prefix_star_comment =
                expression_has_prefix_star_comment(context, *then_expression)
                    || else_expression.is_some_and(|else_id| {
                        expression_has_prefix_star_comment(context, else_id)
                    });
            if has_branch_prefix_star_comment {
                return false;
            }

            condition.as_expression().is_some()
        }
        Expression::Declaration(declaration_id) => {
            matches!(
                context.tree.get(*declaration_id),
                Declaration::Function(FunctionDeclaration { signature, .. })
                    if signature.form == FunctionForm::Lambda
            ) && context
                .source_comments_in_range(value_span.end, child_span.end)
                .is_empty()
        }
        _ => false,
    }
}

/// Return whether one expression chain has comments between its formatted hops.
pub(crate) fn expression_chain_has_separator_comment(
    context: &TsppFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let chain = chain_nodes(context, expression_id);
    if chain.len() <= 1 {
        return false;
    }

    if chain
        .windows(2)
        .any(|adjacent| has_comment_between_expressions(context, adjacent[0], adjacent[1]))
    {
        return true;
    }

    chain
        .iter()
        .copied()
        .any(|chain_node_id| member_has_intervening_comment(context, chain_node_id))
}

/// Return whether one expression has one prefix block-star comment.
fn expression_has_prefix_star_comment(
    context: &TsppFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let expression_span = context.span(expression_id);
    let Some(previous_token) = context.previous_token_before_span(expression_span) else {
        return false;
    };
    if previous_token.span.end >= expression_span.start {
        return false;
    }

    context
        .source_comments_in_range(previous_token.span.end, expression_span.start)
        .iter()
        .any(|comment| comment.is_block())
}

/// Check whether a tree child forces the element to break.
pub(crate) fn tree_child_breaks_element(
    context: &TsppFormatContext<'_>,
    child_id: LocalNodeId<TreeChild>,
) -> bool {
    if matches!(context.tree.get(child_id), TreeChild::Empty) {
        return node_has_line_comment(context, child_id);
    }

    let Some(value_id) = context.tree.get(child_id).value() else {
        return false;
    };
    let value_id = transparent_inner_expression(context, value_id);
    let value_expression = context.tree.get(value_id);
    let is_text_node = matches!(context.tree.get(child_id), TreeChild::Text { .. });
    let has_line_comment =
        node_has_line_comment(context, child_id) || node_has_line_comment(context, value_id);
    if has_line_comment {
        return true;
    }

    // ternary branch comments and wrappers
    let ternary_has_comment_or_parenthesized_branch = match value_expression {
        Expression::If {
            form: IfForm::Ternary,
            condition,
            then_expression,
            else_expression,
            ..
        } => {
            let condition_has_line_comment = condition
                .as_expression()
                .is_none_or(|condition| node_has_line_comment(context, condition));

            node_has_line_comment(context, value_id)
                || condition_has_line_comment
                || node_has_line_comment(context, *then_expression)
                || else_expression.is_some_and(|else_id| node_has_line_comment(context, else_id))
        }
        _ => false,
    };

    if (context.has_annotation(child_id) || context.has_annotation(value_id)) && !is_text_node {
        if matches!(
            value_expression,
            Expression::If {
                form: IfForm::Ternary,
                ..
            }
        ) {
            return ternary_has_comment_or_parenthesized_branch;
        }

        return true;
    }

    match value_expression {
        Expression::If {
            form: IfForm::Ternary,
            ..
        } => ternary_has_comment_or_parenthesized_branch,
        Expression::Block(_) | Expression::Match { .. } | Expression::Try { .. } => true,
        Expression::Declaration(declaration_id) => {
            tree_callback_body_requires_break(context, *declaration_id)
        }
        Expression::TreeExpression { .. } => false,
        _ => tree_expression_contains_callback_break(context, value_id),
    }
}

/// Return whether one tree child control value should expand like tree branch expressions.
pub(crate) fn tree_control_child_should_expand(
    context: &TsppFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let Some(child_id) = tree_child_control_child(context, expression_id) else {
        return false;
    };
    let Some((tree_id, NodeType::Expression)) = context.parent_by_id(child_id.id) else {
        return false;
    };
    let tree_id = LocalNodeId::<Expression>::new(tree_id);
    if !matches!(context.tree.get(tree_id), Expression::TreeExpression { .. }) {
        return false;
    }

    let expression_id = tree_child_control_expression(context, expression_id);
    let Expression::If {
        form: IfForm::If,
        then_expression,
        else_expression,
        ..
    } = context.tree.get(expression_id)
    else {
        return tree_control_child_has_tree_branch(context, expression_id);
    };

    let span = context.span(expression_id);
    if context
        .source_comments_in_range(span.start, span.end)
        .iter()
        .any(|comment| comment.is_line())
    {
        return true;
    }

    expression_branch_has_tree_value(context, *then_expression)
        || else_expression.is_some_and(|else_expression| {
            expression_branch_has_tree_value(context, else_expression)
        })
}

/// Return the tree child that owns one control child expression.
fn tree_child_control_child(
    context: &TsppFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> Option<LocalNodeId<TreeChild>> {
    let (parent_id, parent_type) = context.parent(expression_id)?;

    if parent_type == NodeType::TreeChild {
        return Some(LocalNodeId::<TreeChild>::new(parent_id));
    }

    if parent_type != NodeType::Block {
        return None;
    }

    let block_id = LocalNodeId::<Block>::new(parent_id);
    let block = context.tree.get(block_id);
    if block.form != BlockForm::Implicit || block.len() != 1 {
        return None;
    }

    let Some((block_expression_id, NodeType::Expression)) = context.parent_by_id(parent_id) else {
        return None;
    };
    let Some((child_id, NodeType::TreeChild)) = context.parent_by_id(block_expression_id) else {
        return None;
    };

    Some(LocalNodeId::<TreeChild>::new(child_id))
}

/// Return the control expression owned by one tree child value.
fn tree_child_control_expression(
    context: &TsppFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> LocalNodeId<Expression> {
    let expression_id = transparent_inner_expression(context, expression_id);

    let Expression::Block(block_id) = context.tree.get(expression_id) else {
        return expression_id;
    };

    let block = context.tree.get(*block_id);
    if block.form != BlockForm::Implicit || block.len() != 1 {
        return expression_id;
    }

    let Some(inner_expression_id) = block.first_expression() else {
        return expression_id;
    };

    inner_expression_id
}

/// Return whether one non-if control child has a tree-valued branch.
fn tree_control_child_has_tree_branch(
    context: &TsppFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    match context.tree.get(expression_id) {
        Expression::Match { arms, .. } => {
            arms.iter()
                .copied()
                .any(|arm_id| match context.tree.get(arm_id) {
                    MatchArm::Expression { body, .. } => {
                        expression_branch_has_tree_value(context, *body)
                    }
                    MatchArm::Block { body, .. } => block_has_tree_value(context, *body),
                })
        }
        Expression::Try {
            body,
            catch,
            finally,
            ..
        } => {
            expression_branch_has_tree_value(context, *body)
                || catch.is_some_and(|catch| {
                    let catch = context.tree.get(catch);
                    expression_branch_has_tree_value(context, catch.body)
                })
                || finally.is_some_and(|finally| expression_branch_has_tree_value(context, finally))
        }
        _ => false,
    }
}

/// Return whether one expression branch is transparently tree-valued.
fn expression_branch_has_tree_value(
    context: &TsppFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let expression_id = transparent_inner_expression(context, expression_id);
    match context.tree.get(expression_id) {
        Expression::TreeExpression { .. } => true,
        Expression::Block(block_id) => block_has_tree_value(context, *block_id),
        _ => false,
    }
}

/// Return whether one block branch is tree-valued.
fn block_has_tree_value(context: &TsppFormatContext<'_>, block_id: LocalNodeId<Block>) -> bool {
    let block = context.tree.get(block_id);
    let expression_id = if let Some(tail_expression) = block.tail_expression {
        Some(tail_expression)
    } else if block.len() == 1 {
        block.first_expression()
    } else {
        None
    };

    expression_id
        .is_some_and(|expression_id| expression_branch_has_tree_value(context, expression_id))
}
