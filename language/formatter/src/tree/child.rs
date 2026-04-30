use super::attribute::argument_transparent_value_id;
use crate::annotation::format_comment;
use crate::chain::{
    chain_nodes, has_comment_between_expressions, member_has_intervening_comment,
    transparent_inner_expression,
};
use crate::{DestackFormatContext, DestackFormatter};
use destack_ast::{
    Argument, Block, BlockFormat, Comment, Declaration, Expression, FunctionDeclaration,
    FunctionKind, IfCondition, IfKind, LocalNodeId, MatchCase, Node, NodeType, ScalarLiteral,
    TokenType, Tree, TreeImpl,
};
use destack_fir::format::{Buffer, FormatResult};
use destack_fir::prelude::{hard_line_break, space};
use destack_fir::write;
use destack_source::Span;

/// Return whether one node span contains a line comment.
pub(crate) fn node_has_line_comment<T>(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<T>,
) -> bool
where
    T: Node,
    Tree: TreeImpl<T>,
{
    let span = context.span(node_id);

    context
        .comment_tokens_in_range(span.start, span.end)
        .iter()
        .copied()
        .any(|comment| context.comment_is_line(comment))
}

/// Return whether one tree argument has a line comment outside the value span.
pub(crate) fn tree_argument_has_outer_line_comment(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
    value_id: LocalNodeId<Expression>,
) -> bool {
    let argument_span = context.span(argument_id);
    let value_span = context.span(value_id);

    context
        .comment_tokens_in_range(argument_span.start, value_span.start)
        .iter()
        .chain(
            context
                .comment_tokens_in_range(value_span.end, argument_span.end)
                .iter(),
        )
        .any(|comment| context.comment_is_line(*comment))
}

/// Return whether one tree callback body forces multiline element layout.
fn tree_callback_body_requires_break(
    context: &DestackFormatContext<'_>,
    declaration_id: LocalNodeId<Declaration>,
) -> bool {
    let Declaration::Function(FunctionDeclaration { body, .. }) = context.tree.get(declaration_id)
    else {
        return false;
    };
    let Some(body_id) = *body else {
        return false;
    };

    matches!(
        context.tree.get(body_id),
        Expression::Block(_) | Expression::TreeExpression { .. }
    )
}

/// Return whether one expression contains a callback body that forces tree breaks.
pub(crate) fn tree_expression_contains_callback_break(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let expression_id = transparent_inner_expression(context, expression_id);

    match context.tree.get(expression_id) {
        Expression::Call {
            left, arguments, ..
        }
        | Expression::New {
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
        Expression::Member { left, .. }
        | Expression::PrivateMember { left, .. }
        | Expression::Index { left, .. }
        | Expression::Maybe { left, .. }
        | Expression::Must { left, .. } => tree_expression_contains_callback_break(context, *left),
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
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    let Some(value_id) = argument_transparent_value_id(context, argument_id) else {
        return false;
    };
    let value_expression = context.tree.get(value_id);
    let argument_span = context.span(argument_id);
    let value_span = context.span(value_id);

    if argument_span.file == value_span.file {
        if argument_span.start < value_span.start
            && !context
                .comment_tokens_in_range(argument_span.start, value_span.start)
                .is_empty()
        {
            return false;
        }

        if value_span.end < argument_span.end
            && !context
                .comment_tokens_in_range(value_span.end, argument_span.end)
                .is_empty()
        {
            return false;
        }
    }

    if tree_argument_has_outer_line_comment(context, argument_id, value_id) {
        return false;
    }

    match value_expression {
        Expression::ScalarLiteral(ScalarLiteral::String(_))
        | Expression::ScalarLiteral(ScalarLiteral::Character(_)) => true,
        Expression::ArrayExpression { .. }
        | Expression::ObjectExpression { .. }
        | Expression::Call { .. }
        | Expression::TemplateExpression { .. }
        | Expression::TaggedTemplateExpression { .. }
        | Expression::Await { .. }
        | Expression::AwaitMaybe { .. }
        | Expression::Binary { .. }
        | Expression::Member { .. }
        | Expression::PrivateMember { .. }
        | Expression::Index { .. }
        | Expression::Maybe { .. }
        | Expression::Must { .. } => !expression_chain_has_separator_comment(context, value_id),
        Expression::If {
            kind: IfKind::Ternary,
            condition,
            then_expression,
            else_expression,
            ..
        } => {
            let has_branch_prefix_star_comment =
                expression_chain_has_prefix_star_comment(context, *then_expression)
                    || else_expression.is_some_and(|else_id| {
                        expression_chain_has_prefix_star_comment(context, else_id)
                    });
            if has_branch_prefix_star_comment {
                return false;
            }

            matches!(condition, IfCondition::Expression { .. })
        }
        Expression::Declaration(declaration_id) => matches!(
            context.tree.get(*declaration_id),
            Declaration::Function(FunctionDeclaration { signature, .. })
                if signature.kind == FunctionKind::Lambda
        ),
        _ => false,
    }
}

/// Return whether one expression chain has comments between its formatted hops.
pub(crate) fn expression_chain_has_separator_comment(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let chain = chain_nodes(context.tree, expression_id);
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
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let expression_span = context.span(expression_id);
    let Some(previous_token) = context.previous_non_whitespace_token_before_span(expression_span)
    else {
        return false;
    };
    if previous_token.span.end >= expression_span.start {
        return false;
    }

    context
        .comment_tokens_in_range(previous_token.span.end, expression_span.start)
        .iter()
        .any(|token| {
            matches!(
                token.token.ty,
                TokenType::BlockComment | TokenType::DocBlockComment
            )
        })
}

/// Return whether one expression or its parenthesized inner chain has one prefix block-star comment.
fn expression_chain_has_prefix_star_comment(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    expression_has_prefix_star_comment(context, expression_id)
}

/// Check whether a tree child forces the element to break.
pub(crate) fn tree_child_breaks_element(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    let Some(value_id) = argument_transparent_value_id(context, argument_id) else {
        return false;
    };
    let value_expression = context.tree.get(value_id);
    let is_text_node = matches!(
        value_expression,
        Expression::ScalarLiteral(ScalarLiteral::String(_))
    );
    let has_line_comment =
        node_has_line_comment(context, argument_id) || node_has_line_comment(context, value_id);
    if has_line_comment {
        return true;
    }

    // ternary branch comments and wrappers
    let ternary_has_comment_or_parenthesized_branch = match value_expression {
        Expression::If {
            kind: IfKind::Ternary,
            condition,
            then_expression,
            else_expression,
            ..
        } => {
            let condition_has_line_comment = match condition {
                IfCondition::Expression { condition } => node_has_line_comment(context, *condition),
                IfCondition::Let { .. } => true,
            };

            node_has_line_comment(context, value_id)
                || condition_has_line_comment
                || node_has_line_comment(context, *then_expression)
                || else_expression.is_some_and(|else_id| node_has_line_comment(context, else_id))
        }
        _ => false,
    };

    if (context.has_annotation(argument_id) || context.has_annotation(value_id))
        && !is_text_node
        && !matches!(value_expression, Expression::Stub)
    {
        if matches!(
            value_expression,
            Expression::If {
                kind: IfKind::Ternary,
                ..
            }
        ) {
            return ternary_has_comment_or_parenthesized_branch;
        }

        return true;
    }

    match value_expression {
        Expression::Stub => context.options.language_type.is_destack(),
        Expression::If {
            kind: IfKind::Ternary,
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

/// Return whether one tree child control value should expand like JSX branch expressions.
pub(crate) fn tree_control_child_should_expand(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let Some(argument_id) = tree_child_control_argument(context, expression_id) else {
        return false;
    };
    let Some((tree_id, NodeType::Expression)) = context.parent_by_id(argument_id.id) else {
        return false;
    };
    let tree_id = LocalNodeId::<Expression>::new(tree_id);
    if !matches!(context.tree.get(tree_id), Expression::TreeExpression { .. }) {
        return false;
    }

    let expression_id = tree_child_control_expression(context, expression_id);
    let Expression::If {
        kind: IfKind::If,
        then_expression,
        else_expression,
        ..
    } = context.tree.get(expression_id)
    else {
        return tree_control_child_has_tree_branch(context, expression_id);
    };

    let span = context.span(expression_id);
    if context
        .comment_tokens_in_range(span.start, span.end)
        .iter()
        .any(|comment| context.comment_is_line(*comment))
    {
        return true;
    }

    expression_branch_has_tree_value(context, *then_expression)
        || else_expression.is_some_and(|else_expression| {
            expression_branch_has_tree_value(context, else_expression)
        })
}

/// Return the tree argument that owns one control child expression.
fn tree_child_control_argument(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> Option<LocalNodeId<Argument>> {
    let (parent_id, parent_type) = context.parent(expression_id)?;

    if parent_type == NodeType::Argument {
        return Some(LocalNodeId::<Argument>::new(parent_id));
    }

    if parent_type != NodeType::Block {
        return None;
    }

    let block_id = LocalNodeId::<Block>::new(parent_id);
    let block = context.tree.get(block_id);
    if block.format != BlockFormat::Implicit || block.len() != 1 {
        return None;
    }

    let Some((block_expression_id, NodeType::Expression)) = context.parent_by_id(parent_id) else {
        return None;
    };
    let Some((argument_id, NodeType::Argument)) = context.parent_by_id(block_expression_id) else {
        return None;
    };

    Some(LocalNodeId::<Argument>::new(argument_id))
}

/// Return the control expression owned by one tree child value.
fn tree_child_control_expression(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> LocalNodeId<Expression> {
    let expression_id = transparent_inner_expression(context, expression_id);

    let Expression::Block(block_id) = context.tree.get(expression_id) else {
        return expression_id;
    };

    let block = context.tree.get(*block_id);
    if block.format != BlockFormat::Implicit || block.len() != 1 {
        return expression_id;
    }

    let Some(inner_expression_id) = block.first_expression() else {
        return expression_id;
    };

    inner_expression_id
}

/// Return whether one non-if control child has a tree-valued branch.
fn tree_control_child_has_tree_branch(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    match context.tree.get(expression_id) {
        Expression::Match { cases, .. } => {
            cases
                .iter()
                .copied()
                .any(|case_id| match context.tree.get(case_id) {
                    MatchCase::Expression { body, .. } => {
                        expression_branch_has_tree_value(context, *body)
                    }
                    MatchCase::Block { body, .. } => block_has_tree_value(context, *body),
                })
        }
        Expression::Try {
            try_expression,
            catch_expression,
            finally_expression,
            ..
        } => {
            expression_branch_has_tree_value(context, *try_expression)
                || catch_expression.is_some_and(|catch_expression| {
                    expression_branch_has_tree_value(context, catch_expression)
                })
                || finally_expression.is_some_and(|finally_expression| {
                    expression_branch_has_tree_value(context, finally_expression)
                })
        }
        _ => false,
    }
}

/// Return whether one expression branch is transparently tree-valued.
fn expression_branch_has_tree_value(
    context: &DestackFormatContext<'_>,
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
fn block_has_tree_value(context: &DestackFormatContext<'_>, block_id: LocalNodeId<Block>) -> bool {
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

/// Format one multiline stub comment list.
pub(crate) fn format_multiline_stub_comment_nodes<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    comment_nodes: &[Comment],
) -> FormatResult<()> {
    for (index, comment) in comment_nodes.iter().copied().enumerate() {
        if index > 0 {
            write!(f, [hard_line_break()])?;
        }
        format_comment(f, comment)?;
    }

    Ok(())
}

/// Format inline stub comments from one source span.
pub(crate) fn format_inline_stub_comments<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    span: Span,
) -> FormatResult<bool> {
    let comment_nodes = {
        let comments = f.context().comments();
        comments.comments_in_range(span.start, span.end).to_vec()
    };

    for (index, comment) in comment_nodes.iter().copied().enumerate() {
        if index > 0 {
            write!(f, [space()])?;
        }

        format_comment(f, comment)?;
    }

    Ok(!comment_nodes.is_empty())
}
