use crate::format::analysis::{
    previous_non_whitespace_token_before_annotation, previous_non_whitespace_token_before_span,
};
use crate::format::chain::{
    flattened_binary_operand_count, has_comment_between_expressions,
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
use destack_fir::format::Buffer;
use destack_fir::prelude::dedent;
use destack_fir::{format_args, write};

/// Return whether one token is an assignment operator token.
#[inline]
fn is_assignment_operator_token(token_type: TokenType) -> bool {
    AssignOperator::from_token(token_type).is_some()
}

/// Return whether one expression has any own-line prefix annotation.
fn expression_has_own_line_prefix_annotation(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    context
        .visit_annotations(expression_id, |annotation_ids| {
            annotation_ids.iter().any(|annotation_id| {
                let annotation = context.annotation(*annotation_id);
                let is_prefix = matches!(
                    annotation,
                    Annotation::Comment {
                        position: AnnotationPosition::LinePrefix | AnnotationPosition::BlockPrefix,
                        ..
                    } | Annotation::Doc {
                        position: AnnotationPosition::LinePrefix | AnnotationPosition::BlockPrefix,
                        ..
                    } | Annotation::Decorator {
                        position: AnnotationPosition::LinePrefix | AnnotationPosition::BlockPrefix,
                        ..
                    }
                );
                is_prefix && context.annotation_starts_on_own_line(*annotation_id)
            })
        })
        .unwrap_or(false)
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
                let Some(previous_token) =
                    previous_non_whitespace_token_before_annotation(context, *annotation_id)
                else {
                    return false;
                };
                if !is_assignment_operator_token(previous_token.token.ty) {
                    return false;
                }

                let comment_span = context.span(node);
                let assignment_and_comment_share_line = context.file.is_same_line(
                    previous_token.span.end.saturating_sub(1),
                    comment_span.start,
                );
                if !assignment_and_comment_share_line {
                    return false;
                }

                if context.annotation_starts_on_own_line(*annotation_id) {
                    return false;
                }

                match comment.style {
                    CommentStyle::Slash => true,
                    CommentStyle::Star => {
                        !context.has_newline(comment_span)
                            && context.annotation_next_token_is_on_same_line(*annotation_id)
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

/// Write one grouped inline assignment with one space around the operator.
fn write_grouped_inline_assignment<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    left: LocalNodeId<Expression>,
    operator: &AssignOperator,
    right: LocalNodeId<Expression>,
    has_left_postfix: bool,
) -> FormatResult<()> {
    let space_before_operator = format_with(|f| {
        if !has_left_postfix {
            write!(f, [space()])?;
        }

        Ok(())
    });

    write!(
        f,
        [group(&format_args![
            left,
            space_before_operator,
            operator,
            space(),
            right
        ])]
    )
}

/// Write one inline assignment without grouping.
fn write_inline_assignment<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    left: LocalNodeId<Expression>,
    operator: &AssignOperator,
    right: LocalNodeId<Expression>,
    has_left_postfix: bool,
) -> FormatResult<()> {
    let space_before_operator = format_with(|f| {
        if !has_left_postfix {
            write!(f, [space()])?;
        }

        Ok(())
    });

    write!(f, [left, space_before_operator, operator, space(), right])
}

/// Write one grouped assignment with indented inline rhs.
fn write_grouped_inline_indented_assignment<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    left: LocalNodeId<Expression>,
    operator: &AssignOperator,
    right: LocalNodeId<Expression>,
    has_left_postfix: bool,
) -> FormatResult<()> {
    let space_before_operator = format_with(|f| {
        if !has_left_postfix {
            write!(f, [space()])?;
        }

        Ok(())
    });

    write!(
        f,
        [group(&format_args![
            left,
            space_before_operator,
            operator,
            space(),
            indent(&right)
        ])]
    )
}

/// Write one grouped assignment with a soft line break rhs seam.
fn write_grouped_softline_assignment<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    left: LocalNodeId<Expression>,
    operator: &AssignOperator,
    right: LocalNodeId<Expression>,
    has_left_postfix: bool,
    should_dedent_right: bool,
) -> FormatResult<()> {
    let space_before_operator = format_with(|f| {
        if !has_left_postfix {
            write!(f, [space()])?;
        }

        Ok(())
    });

    let right_is_function_with_block = matches!(
        f.context().tree.get(right),
        Expression::Declaration(declaration_id)
            if matches!(
                f.context().tree.get(*declaration_id),
                Declaration::Function { body: Some(body_id), .. }
                    if matches!(f.context().tree.get(*body_id), Expression::Block(_))
            )
    );
    let should_dedent_right = should_dedent_right
        && !matches!(f.context().tree.get(right), Expression::Assign { .. })
        && !right_is_function_with_block;
    if should_dedent_right {
        write!(
            f,
            [group(&format_args![
                left,
                space_before_operator,
                operator,
                indent(&format_args![soft_line_break_or_space(), dedent(&right)])
            ])]
        )
    } else {
        write!(
            f,
            [group(&format_args![
                left,
                space_before_operator,
                operator,
                indent(&format_args![soft_line_break_or_space(), right])
            ])]
        )
    }
}

/// Write one grouped assignment with a hard line break rhs seam.
fn write_grouped_hardline_assignment<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    left: LocalNodeId<Expression>,
    operator: &AssignOperator,
    right: LocalNodeId<Expression>,
    has_left_postfix: bool,
    should_dedent_right: bool,
) -> FormatResult<()> {
    let space_before_operator = format_with(|f| {
        if !has_left_postfix {
            write!(f, [space()])?;
        }

        Ok(())
    });

    let right_is_function_with_block = matches!(
        f.context().tree.get(right),
        Expression::Declaration(declaration_id)
            if matches!(
                f.context().tree.get(*declaration_id),
                Declaration::Function { body: Some(body_id), .. }
                    if matches!(f.context().tree.get(*body_id), Expression::Block(_))
            )
    );
    let should_dedent_right = should_dedent_right
        && !matches!(f.context().tree.get(right), Expression::Assign { .. })
        && !right_is_function_with_block;
    if should_dedent_right {
        write!(
            f,
            [group(&format_args![
                left,
                space_before_operator,
                operator,
                indent(&format_args![hard_line_break(), dedent(&right)])
            ])]
        )
    } else {
        write!(
            f,
            [group(&format_args![
                left,
                space_before_operator,
                operator,
                indent(&format_args![hard_line_break(), right])
            ])]
        )
    }
}

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

/// Format an assignment expression with shared rhs break layout.
pub(crate) fn format_assign_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    left: LocalNodeId<Expression>,
    operator: &AssignOperator,
    right: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let context = f.context();

    // topology and trivia
    let inner_right_id = transparent_inner_expression(context, right);
    let inner_right_expression = context.tree.get(inner_right_id);
    let has_assignment_parent = context
        .parent(node_id)
        .is_some_and(|(parent_id, parent_type)| {
            if parent_type != NodeType::Expression {
                return false;
            }
            matches!(
                context.tree.get(LocalNodeId::<Expression>::new(parent_id)),
                Expression::Assign { .. }
            )
        });
    let has_left_assignment_parent =
        context
            .parent(node_id)
            .is_some_and(|(parent_id, parent_type)| {
                if parent_type != NodeType::Expression {
                    return false;
                }
                matches!(
                    context.tree.get(LocalNodeId::<Expression>::new(parent_id)),
                    Expression::Assign { left, .. } if left.id == node_id.id
                )
            });
    let right_has_annotation = context.has_annotation(right);
    let is_index_operand_assignment = assignment_is_index_operand(context, node_id);
    let assignment_has_newline = context.node_has_newline(node_id);
    let left_has_newline = context.node_has_newline(left);
    let right_has_newline = context.node_has_newline(right);
    let has_left_postfix = context.has_postfix_annotation(left);

    // rhs shape
    let right_is_binary = matches!(inner_right_expression, Expression::Binary { .. });
    let right_is_sequence = matches!(
        inner_right_expression,
        Expression::SequenceExpression { .. }
    );
    let right_is_assign = matches!(inner_right_expression, Expression::Assign { .. });
    let right_is_path_chain = matches!(
        inner_right_expression,
        Expression::Path { path, .. } if path.segments.len() > 1
    );
    let right_is_chain = right_is_path_chain
        || is_expression_chain(context.tree, inner_right_id)
        || is_chain_root(context.tree, inner_right_id);
    let right_is_chain_tail_lambda =
        is_assignment_chain_tail_lambda(context, node_id, inner_right_id);
    let right_is_lambda = is_lambda_expression(context, inner_right_id);
    let right_is_self_breaking = right_is_binary
        || right_is_sequence
        || right_is_assign
        || right_is_chain
        || right_is_chain_tail_lambda
        || right_is_lambda;
    let right_has_prefix_annotation = context.has_prefix_annotation(right);
    let right_has_own_line_prefix_annotation =
        expression_has_own_line_prefix_annotation(context, right);
    let right_has_assignment_seam_inline_prefix_comment =
        expression_has_assignment_seam_inline_prefix_comment(context, right)
            || assignment_seam_has_line_comment_between(context, left, right);
    let right_has_prefix_annotation_that_forces_operator_break =
        right_has_prefix_annotation && !right_has_assignment_seam_inline_prefix_comment;
    let right_has_between_comment = has_comment_between_expressions(context, left, right);
    let right_is_inline_index_operand_value = matches!(
        inner_right_expression,
        Expression::Call { .. }
            | Expression::Path { .. }
            | Expression::Member { .. }
            | Expression::PrivateMember { .. }
            | Expression::Index { .. }
            | Expression::ScalarLiteral(_)
    );
    let is_string_literal = matches!(
        inner_right_expression,
        Expression::ScalarLiteral(ScalarLiteral::String(_)) | Expression::TemplateExpression { .. }
    );
    let right_is_keyword_prefixed_expression = matches!(
        inner_right_expression,
        Expression::Await { .. } | Expression::AwaitMaybe { .. } | Expression::Comptime { .. }
    );
    let right_chain_starts_with_keyword_prefixed_expression =
        expression_chain_starts_with_keyword_prefix_expression(context.tree, inner_right_id);
    let right_is_compact_multiline = right_has_newline;

    // assignment chain path
    let chain_root_id = left_assignment_chain_root(context, node_id);
    let chain_root_is_current = chain_root_id.id == node_id.id;
    let left_assignment_chain_is_multiline = has_left_assignment_parent
        && !chain_root_is_current
        && context.node_has_newline(chain_root_id);
    let right_chain_root_id = right_assignment_chain_root(context, node_id);
    let right_assignment_chain_is_multiline = has_assignment_parent && {
        let right_parent_is_long = right_assignment_parent(context, node_id)
            .is_some_and(|parent_id| context.node_has_newline(parent_id));
        right_parent_is_long || context.node_has_newline(right_chain_root_id)
    };

    // rhs break path
    let right_is_multiline_binary = right_is_binary && {
        let binary_operand_count = match inner_right_expression {
            Expression::Binary { operator, .. } => {
                flattened_binary_operand_count(context, inner_right_id, *operator)
            }
            _ => 0,
        };
        binary_operand_count > LONG_BINARY_OPERAND_COUNT_THRESHOLD
            && (right_has_between_comment || right_has_newline)
    };
    let node_is_call_argument =
        context.any_ancestor(node_id, |_, parent_type| parent_type == NodeType::Argument);
    let right_is_collection_or_call_like = matches!(
        inner_right_expression,
        Expression::ObjectExpression { .. }
            | Expression::ArrayExpression { .. }
            | Expression::TupleExpression { .. }
            | Expression::Call { .. }
            | Expression::New { .. }
            | Expression::Instantiation { .. }
    );
    let right_is_anonymous_class_declaration = matches!(
        inner_right_expression,
        Expression::Declaration(declaration_id)
            if matches!(
                context.tree.get(*declaration_id),
                Declaration::Class { descriptor, .. } if descriptor.name.is_none()
            )
    );

    // default path
    let left_inner_id = transparent_inner_expression(context, left);
    let left_is_expanded_object_target = matches!(
        context.tree.get(left_inner_id),
        Expression::ObjectExpression { properties, .. }
            if properties.len() > EXPANDED_OBJECT_TARGET_PROPERTY_THRESHOLD
                && is_assignment_left_target(context, left_inner_id)
    );
    let right_is_short_object = matches!(
        inner_right_expression,
        Expression::ObjectExpression { properties, .. } if properties.len() <= SHORT_OBJECT_PROPERTY_MAX
    );
    let right_is_inline_atomic =
        expression_is_trivial_inline_without_annotations(context, inner_right_id);

    // short circuit: trivia free simple assignments stay inline
    let should_use_inline_index_operand = is_index_operand_assignment
        && right_is_inline_index_operand_value
        && !assignment_has_newline
        && !left_has_newline
        && !right_has_newline
        && !right_has_annotation
        && !right_has_prefix_annotation_that_forces_operator_break
        && !right_has_between_comment;
    if should_use_inline_index_operand {
        return write_grouped_inline_assignment(f, left, operator, right, has_left_postfix);
    }

    // string literals are atomic: never break at `=`
    if is_string_literal {
        return write_grouped_inline_assignment(f, left, operator, right, has_left_postfix);
    }

    // keep rhs keyword expressions (`await`, `await?`, `comptime`) attached to `=`
    if (right_is_keyword_prefixed_expression || right_chain_starts_with_keyword_prefixed_expression)
        && !right_has_prefix_annotation_that_forces_operator_break
        && !right_has_between_comment
    {
        return write_grouped_inline_assignment(f, left, operator, right, has_left_postfix);
    }

    // keep assignment seam inline prefix comments with the operator
    if right_has_assignment_seam_inline_prefix_comment {
        return write_grouped_inline_indented_assignment(
            f,
            left,
            operator,
            right,
            has_left_postfix,
        );
    }

    // non-inline seam comments between `=` and rhs always break at the operator
    if right_has_prefix_annotation_that_forces_operator_break
        || right_has_between_comment
        || right_has_own_line_prefix_annotation
    {
        return write_grouped_hardline_assignment(
            f,
            left,
            operator,
            right,
            has_left_postfix,
            false,
        );
    }

    // left associative assignment chains: keep inner chain steps inline
    if has_left_assignment_parent && !right_has_newline && !left_assignment_chain_is_multiline {
        return write_grouped_inline_assignment(f, left, operator, right, has_left_postfix);
    }

    // expanded right-associative assignment chains should break on each seam
    if right_assignment_chain_is_multiline {
        return write_grouped_hardline_assignment(
            f,
            left,
            operator,
            right,
            has_left_postfix,
            false,
        );
    }

    // break long binary rhs values after the operator
    if right_is_multiline_binary {
        return write_grouped_hardline_assignment(f, left, operator, right, has_left_postfix, true);
    }

    // keep simple rhs collection/call-like values inline
    if (right_is_collection_or_call_like || right_is_anonymous_class_declaration)
        && !right_is_chain
        && !right_has_prefix_annotation
        && !right_has_between_comment
    {
        return write_grouped_inline_assignment(f, left, operator, right, has_left_postfix);
    }

    // rhs-managed layout cases
    if right_is_self_breaking {
        let right_has_forced_break_trivia =
            right_has_prefix_annotation_that_forces_operator_break || right_has_between_comment;
        let should_break_after_operator =
            // rhs assignment
            (right_is_assign && (right_has_forced_break_trivia || node_is_call_argument))
                // rhs lambda
                || (right_is_lambda && right_has_forced_break_trivia)
                // rhs chain
                || (right_is_chain
                    && !right_is_lambda
                    && (right_is_chain_tail_lambda || right_has_forced_break_trivia))
                // rhs binary
                || (right_is_binary
                    && !right_is_lambda
                    && (right_is_chain_tail_lambda
                        || right_has_forced_break_trivia
                        || assignment_has_newline
                        || right_is_compact_multiline))
                // other rhs-managed forms
                || (!right_is_assign
                    && !right_is_lambda
                    && !right_is_chain
                    && !right_is_binary
                    && (right_is_chain_tail_lambda
                        || right_has_prefix_annotation_that_forces_operator_break
                        || right_is_compact_multiline
                        || right_has_between_comment));

        if should_break_after_operator {
            if right_has_prefix_annotation_that_forces_operator_break
                || right_has_between_comment
                || right_is_sequence
            {
                return write_grouped_hardline_assignment(
                    f,
                    left,
                    operator,
                    right,
                    has_left_postfix,
                    false,
                );
            }

            return write_grouped_hardline_assignment(
                f,
                left,
                operator,
                right,
                has_left_postfix,
                !right_is_lambda,
            );
        }

        if right_is_chain {
            if right_has_newline {
                return write_grouped_inline_assignment(f, left, operator, right, has_left_postfix);
            }

            return write_grouped_softline_assignment(
                f,
                left,
                operator,
                right,
                has_left_postfix,
                !right_is_lambda,
            );
        }

        if right_is_assign {
            return write_grouped_inline_assignment(f, left, operator, right, has_left_postfix);
        }

        return write_grouped_softline_assignment(f, left, operator, right, has_left_postfix, true);
    }

    // default expression layout cases
    let should_force_break_for_multiline_left =
        left_has_newline && !right_is_short_object && !right_is_inline_atomic;
    if should_force_break_for_multiline_left {
        return write_grouped_hardline_assignment(f, left, operator, right, has_left_postfix, true);
    }

    if (left_has_newline || left_is_expanded_object_target)
        && (right_is_short_object || right_is_inline_atomic)
    {
        return write_inline_assignment(f, left, operator, right, has_left_postfix);
    }

    // keep simple atomic rhs values inline when there are no break signals
    if right_is_inline_atomic
        && !right_is_chain
        && !left_has_newline
        && !right_has_newline
        && !right_has_annotation
        && !right_has_prefix_annotation_that_forces_operator_break
        && !right_has_between_comment
    {
        return write_grouped_inline_assignment(f, left, operator, right, has_left_postfix);
    }

    write_grouped_softline_assignment(f, left, operator, right, has_left_postfix, false)
}
