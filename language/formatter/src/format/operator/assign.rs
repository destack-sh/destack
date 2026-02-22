use crate::format::analysis::scan::{
    previous_non_whitespace_token_before_annotation, previous_non_whitespace_token_before_span,
};
use crate::format::chain::{
    expression_chain_should_break, flattened_binary_operand_count, has_comment_between_expressions,
    is_assignment_chain_tail_lambda, is_chain_root, is_expression_chain,
};
use crate::format::expression::{
    AssignOperator, DestackFormatContext, DestackFormatter, Expression, FormatResult, LocalNodeId,
    NodeTree, NodeType, format_with, group, hard_line_break, indent, is_assignment_left_target,
    is_lambda_expression, soft_line_break_or_space, space, transparent_inner_expression,
};
use crate::format::operator::{
    Annotation, AnnotationPosition, Span, TokenType,
    expression_is_trivial_inline_without_annotations,
};
use destack_ast::{Comment, CommentStyle, Declaration, ScalarLiteral};
use destack_fir::format::{Buffer, Format};
use destack_fir::prelude::dedent;
use destack_fir::{format_args, write};

/// Return whether one token is an assignment operator token.
#[inline]
fn is_assignment_operator_token(token_type: TokenType) -> bool {
    AssignOperator::from_token(token_type).is_some()
}

/// Return whether the next non-whitespace token after one annotation starts on the same line.
fn annotation_next_token_is_on_same_line(
    context: &DestackFormatContext<'_>,
    annotation_id: LocalNodeId<Annotation>,
) -> bool {
    let span = context.annotation_span(annotation_id);
    let tokens = context.tokens;
    let mut index = tokens.partition_point(|token| token.span.start < span.end);

    while let Some(token) = tokens.get(index).copied() {
        match token.token.ty {
            TokenType::Whitespace => {
                index += 1;
                continue;
            }
            TokenType::Newline => return false,
            _ => return true,
        }
    }

    false
}

/// Return whether one expression has an inline prefix comment on an assignment seam.
pub(crate) fn expression_has_assignment_seam_inline_prefix_comment(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    context
        .visit_annotations(expression_id, |annotation_ids| {
            annotation_ids.iter().any(|annotation_id| {
                let Annotation::Comment { node, position } = context.annotation(*annotation_id)
                else {
                    return false;
                };
                if position != AnnotationPosition::LinePrefix {
                    return false;
                }

                let comment = context.tree.get::<Comment>(node);
                if !previous_non_whitespace_token_before_annotation(context, *annotation_id)
                    .is_some_and(|token| is_assignment_operator_token(token.token.ty))
                {
                    return false;
                }

                match comment.style {
                    CommentStyle::Slash => true,
                    CommentStyle::Star => {
                        let annotation_span = context.annotation_span(*annotation_id);
                        !context.has_newline(annotation_span)
                            && annotation_next_token_is_on_same_line(context, *annotation_id)
                    }
                }
            })
        })
        .unwrap_or(false)
}

/// Return whether one assignment seam has a slash line comment between left and right.
pub(crate) fn assignment_seam_has_line_comment_between(
    context: &DestackFormatContext<'_>,
    left: LocalNodeId<Expression>,
    right: LocalNodeId<Expression>,
) -> bool {
    let left_span = context.span(left);
    let right_span = context.span(right);
    if left_span.file != right_span.file || left_span.end >= right_span.start {
        return false;
    }

    let between_span = Span::new(left_span.file, left_span.end, right_span.start);
    context
        .comment_tokens()
        .iter()
        .copied()
        .any(|comment_token| {
            if !between_span.intersects(comment_token.span) {
                return false;
            }

            if !matches!(
                comment_token.token.ty,
                TokenType::LineComment | TokenType::DocLineComment
            ) {
                return false;
            }

            previous_non_whitespace_token_before_span(context, comment_token.span)
                .is_some_and(|token| is_assignment_operator_token(token.token.ty))
        })
}

/// Walk left-linked assignment parents and return the outermost chain node.
pub(crate) fn left_assignment_chain_root(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> LocalNodeId<Expression> {
    let mut current_id = node_id;

    loop {
        let Some((parent_id, parent_type)) = context.parent(current_id) else {
            break;
        };
        if parent_type != NodeType::Expression {
            break;
        }

        let parent_expression = context.tree.get(LocalNodeId::<Expression>::new(parent_id));
        let Expression::Assign { left, .. } = parent_expression else {
            break;
        };
        if left.id != current_id.id {
            break;
        }

        current_id = LocalNodeId::<Expression>::new(parent_id);
    }

    current_id
}

/// Walk right-linked assignment parents and return the outermost chain node.
pub(crate) fn right_assignment_chain_root(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> LocalNodeId<Expression> {
    let mut current_id = node_id;

    loop {
        let Some((parent_id, parent_type)) = context.parent(current_id) else {
            break;
        };
        if parent_type != NodeType::Expression {
            break;
        }

        let parent_expression = context.tree.get(LocalNodeId::<Expression>::new(parent_id));
        let Expression::Assign { right, .. } = parent_expression else {
            break;
        };
        if right.id != current_id.id {
            break;
        }

        current_id = LocalNodeId::<Expression>::new(parent_id);
    }

    current_id
}

/// Return the direct assignment parent when current is the rhs.
pub(crate) fn right_assignment_parent(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> Option<LocalNodeId<Expression>> {
    let (parent_id, parent_type) = context.parent(node_id)?;
    if parent_type != NodeType::Expression {
        return None;
    }

    let parent_id = LocalNodeId::<Expression>::new(parent_id);
    let parent_expression = context.tree.get(parent_id);
    let Expression::Assign { right, .. } = parent_expression else {
        return None;
    };
    if right.id != node_id.id {
        return None;
    }

    Some(parent_id)
}

// assignment shape thresholds
const LONG_BINARY_OPERAND_COUNT_THRESHOLD: usize = 2;
const EXPANDED_OBJECT_TARGET_PROPERTY_THRESHOLD: usize = 2;
const SHORT_OBJECT_PROPERTY_MAX: usize = 3;

/// Return whether one assignment expression is used as an index operand.
fn assignment_is_index_operand(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let mut current_id = node_id.id;

    loop {
        let Some((parent_id, parent_type)) = context.parent_by_id(current_id) else {
            return false;
        };
        if parent_type != NodeType::Expression {
            return false;
        }

        let parent_expression_id = LocalNodeId::<Expression>::new(parent_id);
        match context.tree.get(parent_expression_id) {
            Expression::Parenthesized { expression } | Expression::Statement(expression)
                if expression.id == current_id =>
            {
                current_id = parent_id;
            }
            Expression::Index { index, .. } => {
                return index.is_some_and(|index_id| index_id.id == current_id);
            }
            _ => return false,
        }
    }
}

/// Return whether one expression chain starts with a keyword-prefixed expression.
fn expression_chain_starts_with_keyword_prefix_expression(
    tree: &NodeTree,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    match tree.get(expression_id) {
        Expression::Await { .. } | Expression::AwaitMaybe { .. } | Expression::Comptime { .. } => {
            true
        }
        Expression::Member { left, .. }
        | Expression::PrivateMember { left, .. }
        | Expression::Index { left, .. }
        | Expression::Call { left, .. }
        | Expression::New { left, .. }
        | Expression::Instantiation { left, .. }
        | Expression::Maybe { left, .. }
        | Expression::Must { left, .. } => {
            expression_chain_starts_with_keyword_prefix_expression(tree, *left)
        }
        Expression::Parenthesized { expression } | Expression::Statement(expression) => {
            expression_chain_starts_with_keyword_prefix_expression(tree, *expression)
        }
        _ => false,
    }
}

/// Format an assignment expression with shared rhs break policy.
pub(crate) fn format_assign_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    left: LocalNodeId<Expression>,
    operator: &AssignOperator,
    right: LocalNodeId<Expression>,
) -> FormatResult<()> {
    // short circuit: trivia free simple assignments stay inline
    let inner_right_id = transparent_inner_expression(f.context(), right);
    let inner_right_expr = f.context().tree.get(inner_right_id);
    let has_assignment_parent =
        f.context()
            .parent(node_id)
            .is_some_and(|(parent_id, parent_type)| {
                if parent_type != NodeType::Expression {
                    return false;
                }
                matches!(
                    f.context()
                        .tree
                        .get(LocalNodeId::<Expression>::new(parent_id)),
                    Expression::Assign { .. }
                )
            });
    let has_left_assignment_parent =
        f.context()
            .parent(node_id)
            .is_some_and(|(parent_id, parent_type)| {
                if parent_type != NodeType::Expression {
                    return false;
                }
                matches!(
                    f.context()
                        .tree
                        .get(LocalNodeId::<Expression>::new(parent_id)),
                    Expression::Assign { left, .. } if left.id == node_id.id
                )
            });
    let left_has_annotation = f.context().has_annotation(left);
    let right_has_annotation = f.context().has_annotation(right);
    let node_has_annotation = f.context().has_annotation(node_id);
    let is_index_operand_assignment = assignment_is_index_operand(f.context(), node_id);
    let assignment_has_newline = f.context().node_has_newline(node_id);
    let left_has_newline = f.context().node_has_newline(left);
    let right_has_newline = f.context().node_has_newline(right);

    let has_postfix = f.context().has_postfix_annotation(left);

    // binaries, chains, and nested lambda tails handle their own breaking
    let right_is_binary = matches!(inner_right_expr, Expression::Binary { .. });
    let right_is_sequence = matches!(inner_right_expr, Expression::SequenceExpression { .. });
    let right_is_assign = matches!(inner_right_expr, Expression::Assign { .. });
    let right_is_chain_root = is_chain_root(f.context().tree, inner_right_id);
    let right_is_chain =
        is_expression_chain(f.context().tree, inner_right_id) || right_is_chain_root;
    let right_is_chain_tail_lambda =
        is_assignment_chain_tail_lambda(f.context(), node_id, inner_right_id);
    let right_is_lambda = is_lambda_expression(f.context(), inner_right_id);
    let right_handles_its_own_breaking = right_is_binary
        || right_is_sequence
        || right_is_assign
        || right_is_chain
        || right_is_chain_tail_lambda
        || right_is_lambda;
    let right_has_prefix_annotation = f.context().has_prefix_annotation(right);
    let right_has_assignment_seam_inline_prefix_comment =
        expression_has_assignment_seam_inline_prefix_comment(f.context(), right)
            || assignment_seam_has_line_comment_between(f.context(), left, right);
    let right_has_prefix_annotation_that_forces_operator_break =
        right_has_prefix_annotation && !right_has_assignment_seam_inline_prefix_comment;
    let right_has_between_comment = has_comment_between_expressions(f.context(), left, right);

    // index operands should keep compact assignment seams inside brackets
    let right_is_inline_index_operand_value = matches!(
        inner_right_expr,
        Expression::Call { .. }
            | Expression::Path { .. }
            | Expression::Member { .. }
            | Expression::PrivateMember { .. }
            | Expression::Index { .. }
            | Expression::ScalarLiteral(_)
    );
    if is_index_operand_assignment
        && right_is_inline_index_operand_value
        && !assignment_has_newline
        && !left_has_newline
        && !right_has_newline
        && !left_has_annotation
        && !right_has_annotation
        && !node_has_annotation
        && !right_has_prefix_annotation_that_forces_operator_break
        && !right_has_between_comment
    {
        write!(
            f,
            [group(&format_args![
                left,
                space(),
                operator,
                space(),
                right
            ])]
        )?;
        return Ok(());
    }

    // string literals are atomic: never break at `=`
    let is_string_literal = matches!(
        inner_right_expr,
        Expression::ScalarLiteral(ScalarLiteral::String(_)) | Expression::TemplateExpression { .. }
    );
    let right_is_keyword_prefixed_expression = matches!(
        inner_right_expr,
        Expression::Await { .. } | Expression::AwaitMaybe { .. } | Expression::Comptime { .. }
    );
    let right_chain_starts_with_keyword_prefixed_expression =
        expression_chain_starts_with_keyword_prefix_expression(f.context().tree, inner_right_id);

    // structural rhs break profile
    let right_is_multiline = right_has_newline;
    let right_is_compact_multiline = right_is_multiline;
    let assignment_is_multiline = assignment_has_newline;
    let chain_root_id = left_assignment_chain_root(f.context(), node_id);
    let chain_root_is_current = chain_root_id.id == node_id.id;
    let left_assignment_chain_is_multiline = has_left_assignment_parent
        && !chain_root_is_current
        && f.context().node_has_newline(chain_root_id);
    let right_chain_root_id = right_assignment_chain_root(f.context(), node_id);
    let right_assignment_chain_is_multiline = has_assignment_parent && {
        let right_parent_is_long = right_assignment_parent(f.context(), node_id)
            .is_some_and(|parent_id| f.context().node_has_newline(parent_id));
        right_parent_is_long || f.context().node_has_newline(right_chain_root_id)
    };

    // prefer breaking after `=` for multiline binary rhs values
    let right_is_multiline_binary = if right_is_binary {
        let binary_operand_count = match inner_right_expr {
            Expression::Binary { operator, .. } => {
                flattened_binary_operand_count(f.context().tree, inner_right_id, *operator)
            }
            _ => 0,
        };
        binary_operand_count > LONG_BINARY_OPERAND_COUNT_THRESHOLD
            && (right_has_between_comment || right_has_newline)
    } else {
        false
    };
    let node_is_call_argument = f
        .context()
        .any_ancestor(node_id, |_, parent_type| parent_type == NodeType::Argument);

    if is_string_literal {
        write!(
            f,
            [group(&format_args![
                left,
                format_with(|f| {
                    if !has_postfix {
                        write!(f, [space()])?;
                    }
                    Ok(())
                }),
                operator,
                space(),
                right
            ])]
        )?;
        return Ok(());
    }

    // format the space before the operator, respecting postfix comments
    let space_before_operator = format_with(|f| {
        if !has_postfix {
            write!(f, [space()])?;
        }
        Ok(())
    });

    // keep rhs keyword expressions (`await`, `await?`, `comptime`) attached to `=`
    if (right_is_keyword_prefixed_expression || right_chain_starts_with_keyword_prefixed_expression)
        && !right_has_prefix_annotation_that_forces_operator_break
        && !right_has_between_comment
    {
        write!(
            f,
            [group(&format_args![
                left,
                space_before_operator,
                operator,
                space(),
                right
            ])]
        )?;
        return Ok(());
    }

    // keep assignment seam inline prefix comments with the operator
    if right_has_assignment_seam_inline_prefix_comment {
        write!(
            f,
            [group(&format_args![
                left,
                space_before_operator,
                operator,
                space(),
                indent(&right)
            ])]
        )?;
        return Ok(());
    }

    // left associative assignment chains: keep inner chain steps inline
    if has_left_assignment_parent
        && !right_has_prefix_annotation_that_forces_operator_break
        && !right_has_between_comment
        && !right_is_multiline
        && !left_assignment_chain_is_multiline
    {
        write!(
            f,
            [group(&format_args![
                left,
                space_before_operator,
                operator,
                indent(&format_args![soft_line_break_or_space(), dedent(&right)])
            ])]
        )?;
        return Ok(());
    }

    // expanded right-associative assignment chains should break on each seam
    if right_assignment_chain_is_multiline
        && !right_has_prefix_annotation_that_forces_operator_break
        && !right_has_between_comment
    {
        write!(
            f,
            [group(&format_args![
                left,
                space_before_operator,
                operator,
                indent(&format_args![hard_line_break(), dedent(&right)])
            ])]
        )?;
        return Ok(());
    }

    // break long binary rhs values after the operator
    if right_is_multiline_binary {
        let format_break_after_operator = format_with(|f: &mut DestackFormatter<'ast, '_>| {
            // avoid double indentation when the rhs breaks internally
            let dedented_right = dedent(&right);
            write!(
                f,
                [
                    left,
                    space_before_operator,
                    operator,
                    indent(&format_args![hard_line_break(), dedented_right])
                ]
            )
        });

        format_break_after_operator.format(f)?;
        return Ok(());
    }

    let right_is_collection_or_call_like = matches!(
        inner_right_expr,
        Expression::ObjectExpression { .. }
            | Expression::ArrayExpression { .. }
            | Expression::TupleExpression { .. }
            | Expression::Call { .. }
            | Expression::New { .. }
            | Expression::Instantiation { .. }
    );
    let right_is_anonymous_class_declaration = matches!(
        inner_right_expr,
        Expression::Declaration(declaration_id)
            if matches!(
                f.context().tree.get(*declaration_id),
                Declaration::Class { descriptor, .. } if descriptor.name.is_none()
            )
    );
    if (right_is_collection_or_call_like || right_is_anonymous_class_declaration)
        && !right_is_chain
        && !right_has_prefix_annotation
        && !right_has_between_comment
    {
        write!(
            f,
            [group(&format_args![
                left,
                space_before_operator,
                operator,
                space(),
                right
            ])]
        )?;
        return Ok(());
    }

    if right_handles_its_own_breaking {
        let right_prefers_operator_break = if right_is_assign {
            // nested assignment chains: let the rhs chain decide width breaks internally
            right_has_prefix_annotation_that_forces_operator_break
                || right_has_between_comment
                || node_is_call_argument
        } else if right_is_lambda {
            right_has_prefix_annotation_that_forces_operator_break
                || right_has_between_comment
                || right_is_compact_multiline
        } else if right_is_chain {
            !right_is_lambda
                && (right_is_chain_tail_lambda
                    || expression_chain_should_break(f.context(), inner_right_id)
                    || right_has_prefix_annotation_that_forces_operator_break
                    || right_has_between_comment)
        }
        // binary rhs values should not depend on source-only break signals, to keep idempotence stable
        else if right_is_binary {
            !right_is_lambda
                && (right_is_chain_tail_lambda
                    || right_has_prefix_annotation_that_forces_operator_break
                    || right_has_between_comment
                    || assignment_is_multiline
                    || right_is_compact_multiline
                    || right_has_prefix_annotation_that_forces_operator_break
                    || right_has_between_comment)
        } else {
            !right_is_lambda
                && (right_is_chain_tail_lambda
                    || right_has_prefix_annotation_that_forces_operator_break
                    || right_is_compact_multiline
                    || right_has_between_comment)
        };

        let format_break_after_operator = format_with(|f| {
            if right_has_prefix_annotation_that_forces_operator_break
                || right_has_between_comment
                || right_is_sequence
            {
                write!(
                    f,
                    [
                        left,
                        space_before_operator,
                        operator,
                        indent(&format_args![hard_line_break(), right])
                    ]
                )
            } else {
                let dedented_right = dedent(&right);
                write!(
                    f,
                    [
                        left,
                        space_before_operator,
                        operator,
                        indent(&format_args![hard_line_break(), dedented_right])
                    ]
                )
            }
        });

        let format_inline = format_with(|f| {
            group(&format_args![
                left,
                space_before_operator,
                operator,
                indent(&format_args![soft_line_break_or_space(), dedent(&right)])
            ])
            .format(f)
        });
        let format_inline_chain_seam = format_with(|f| {
            group(&format_args![
                left,
                space_before_operator,
                operator,
                space(),
                right
            ])
            .format(f)
        });
        if right_prefers_operator_break {
            format_break_after_operator.format(f)?;
        } else if right_is_chain {
            format_inline_chain_seam.format(f)?;
        } else {
            format_inline.format(f)?;
        }
    } else {
        let left_inner_id = transparent_inner_expression(f.context(), left);
        let left_is_expanded_object_target = matches!(
            f.context().tree.get(left_inner_id),
            Expression::ObjectExpression { properties, .. }
                if properties.len() > EXPANDED_OBJECT_TARGET_PROPERTY_THRESHOLD
                    && is_assignment_left_target(f.context(), left_inner_id)
        );
        let right_is_short_object = matches!(
            inner_right_expr,
            Expression::ObjectExpression { properties, .. } if properties.len() <= SHORT_OBJECT_PROPERTY_MAX
        );
        let right_is_inline_atomic =
            expression_is_trivial_inline_without_annotations(f.context(), inner_right_id);
        let should_force_break_for_multiline_left =
            left_has_newline && !right_is_short_object && !right_is_inline_atomic;

        if should_force_break_for_multiline_left {
            write!(
                f,
                [group(&format_args![
                    left,
                    space_before_operator,
                    operator,
                    indent(&format_args![hard_line_break(), dedent(&right)])
                ])]
            )?;
        } else if (left_has_newline || left_is_expanded_object_target)
            && (right_is_short_object || right_is_inline_atomic)
        {
            write!(f, [left, space_before_operator, operator, space(), right])?;
        } else {
            // other expressions get indented on break
            write!(
                f,
                [group(&format_args![
                    left,
                    space_before_operator,
                    operator,
                    indent(&format_args![soft_line_break_or_space(), right])
                ])]
            )?;
        }
    }

    Ok(())
}
