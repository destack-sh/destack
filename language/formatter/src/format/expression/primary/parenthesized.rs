use crate::Annotation;
use crate::analysis::timing::tags;
use crate::directive::{
    FormatterDirective, FormatterDirectiveKind, FormatterDirectivePosition, directive_for_node,
};
use crate::expression::{
    BinaryOperator, DestackFormatContext, DestackFormatter, Expression, FormatResult, LocalNodeId,
    NodeType, ParenthesizedDropPolicy, block_indent, collect_parenthesized_boundary_comments,
    format_expression, format_with, group, hard_line_break, is_assignment_left_target,
    is_call_like_argument, is_chain_root, is_expression_chain,
    parenthesized_has_leading_inner_comments, parenthesized_has_leading_inner_newline,
    parenthesized_has_leading_inner_trivia, parenthesized_should_drop,
    should_hoist_parenthesized_inner_cast_prefix_comments, soft_block_indent, space, token,
    tree_literal_should_break,
};
use destack_ast::AnnotationPosition;
use destack_fir::format::Buffer;
use destack_fir::{format_args, write};

// parenthesized assignment target thresholds
const PAREN_ASSIGNMENT_OBJECT_EXPAND_MIN_PROPERTIES: usize = 3;
const PAREN_ASSIGNMENT_ARRAY_EXPAND_MIN_ELEMENTS: usize = 4;

/// Format a parenthesized primary expression.
pub(super) fn format_primary_parenthesized_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    expression_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let _timing = f
        .context()
        .timing_scope(tags::FORMAT_EXPRESSION_PRIMARY_PARENTHESES);

    let tree = f.context().tree;
    let expression = &expression_id;
    let inner_expression = tree.get(expression_id);
    let should_drop_parentheses = parenthesized_should_drop(
        f.context(),
        node_id,
        expression_id,
        ParenthesizedDropPolicy::ExpressionWrapper,
    );

    if should_drop_parentheses {
        write!(f, [expression_id])?;
    } else {
        let has_parenthesized_leading_inner_trivia =
            parenthesized_has_leading_inner_trivia(f.context(), node_id, expression_id);
        let has_parenthesized_leading_inner_newline =
            parenthesized_has_leading_inner_newline(f.context(), node_id, expression_id);
        let has_parenthesized_leading_inner_comments =
            parenthesized_has_leading_inner_comments(f.context(), node_id, expression_id);
        let inner_has_effective_prefix_annotation =
            expression_has_effective_prefix_annotation(f.context(), expression_id);
        let has_parenthesized_prefix_annotation =
            expression_has_effective_prefix_annotation(f.context(), node_id)
                || inner_has_effective_prefix_annotation;
        let has_parenthesized_leading_inner_comments =
            has_parenthesized_leading_inner_comments && has_parenthesized_prefix_annotation;
        let node_has_only_slash_prefix_comment_annotations =
            expression_has_only_slash_prefix_comment_annotations(f.context(), node_id);
        let should_preserve_leading_inner_newline = has_parenthesized_leading_inner_newline
            && (!node_has_only_slash_prefix_comment_annotations
                || inner_has_effective_prefix_annotation);
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
        let parent_is_postfix_continuation =
            expression_parent_is_postfix_continuation(f.context(), node_id);
        let parent_is_member_like_postfix_continuation =
            expression_parent_is_member_like_postfix_continuation(f.context(), node_id);
        let is_await_wrapped_chain = matches!(
            tree.get(expression_id),
            Expression::Await { expression } | Expression::AwaitMaybe { expression }
                if is_expression_chain(tree, *expression) || is_chain_root(tree, *expression)
        );

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
        } else if should_hoist_parenthesized_inner_cast_prefix_comments(
            f.context(),
            node_id,
            expression_id,
        ) {
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
        } else if has_parenthesized_prefix_annotation {
            let inner_has_decorator_prefix_annotation =
                expression_has_effective_decorator_prefix_annotation(f.context(), expression_id);
            if f.context().node_has_newline(expression_id)
                || should_preserve_leading_inner_newline
                || inner_has_decorator_prefix_annotation
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
                write!(f, [token("("), expression, token(")")])?;
            }
        } else if matches!(inner_expression, Expression::TypeConditional { .. }) {
            write!(f, [token("("), soft_block_indent(&expression), token(")")])?;
        } else if matches!(
            inner_expression,
            Expression::Binary {
                operator: BinaryOperator::ElementwiseOr | BinaryOperator::ElementwiseAnd,
                ..
            }
        ) {
            let should_keep_multiline = f.context().node_has_newline(expression_id);
            if should_keep_multiline {
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
                write!(f, [token("("), expression, token(")")])?;
            }
        } else if has_parenthesized_leading_inner_trivia {
            if has_parenthesized_leading_inner_newline {
                let should_keep_multiline = has_parenthesized_leading_inner_comments
                    || (is_await_wrapped_chain && parent_is_member_like_postfix_continuation);
                if !should_keep_multiline {
                    write!(f, [token("("), expression, token(")")])?;
                    let boundary_comments = collect_parenthesized_boundary_comments(
                        f.context(),
                        node_id,
                        expression_id,
                    );
                    for comment_id in boundary_comments {
                        write!(f, [space(), comment_id])?;
                    }
                    return Ok(());
                }

                write!(
                    f,
                    [
                        token("("),
                        block_indent(&expression),
                        hard_line_break(),
                        token(")")
                    ]
                )?;
            } else {
                write!(f, [token("("), expression, token(")")])?;
            }
        } else {
            let should_expand_parenthesized_chain = parent_is_postfix_continuation
                && ((is_await_wrapped_chain && parent_is_member_like_postfix_continuation)
                    || (is_expression_chain(tree, expression_id)
                        && (f.context().node_has_newline(node_id)
                            || f.context().node_has_newline(expression_id))));

            if should_expand_parenthesized_chain {
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
                write!(f, [token("("), expression, token(")")])?;
            }
        }

        let boundary_comments =
            collect_parenthesized_boundary_comments(f.context(), node_id, expression_id);
        for comment_id in boundary_comments {
            write!(f, [space(), comment_id])?;
        }
    }

    Ok(())
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

/// Return whether one parenthesized expression continues into a member-like postfix parent.
fn expression_parent_is_member_like_postfix_continuation(
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
            )
        })
}

/// Return whether an expression or its wrapped declaration has prefix annotations.
fn expression_has_effective_prefix_annotation(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    if context.has_prefix_annotation(expression_id) {
        return true;
    }

    match context.tree.get(expression_id) {
        Expression::Declaration(declaration_id) => {
            context.has_prefix_annotation(declaration_id.clone())
        }
        _ => false,
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

/// Return whether an expression has only slash style prefix comment annotations on the node itself.
fn expression_has_only_slash_prefix_comment_annotations(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let Some(annotation_ids) = context.annotations(expression_id) else {
        return false;
    };

    let mut has_prefix_comment = false;
    for annotation_id in annotation_ids {
        let Annotation::Comment { node, position } = context.annotation(annotation_id) else {
            continue;
        };
        if !matches!(
            position,
            AnnotationPosition::LinePrefix | AnnotationPosition::BlockPrefix
        ) {
            continue;
        }

        has_prefix_comment = true;
        let comment = context.tree.get::<destack_ast::Comment>(node);
        if comment.style != destack_ast::CommentStyle::Slash {
            return false;
        }
    }

    has_prefix_comment
}
