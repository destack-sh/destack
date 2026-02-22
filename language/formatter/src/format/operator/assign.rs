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
use destack_fir::format::Buffer;
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

/// Collected policy facts for assignment expression layout.
#[derive(Debug, Clone, Copy)]
struct AssignmentLayoutFacts {
    /// The transparent rhs expression id.
    inner_right_id: LocalNodeId<Expression>,
    /// Whether current assignment is the left child of an assignment parent.
    has_left_assignment_parent: bool,
    /// Whether the left expression has annotations.
    left_has_annotation: bool,
    /// Whether the right expression has annotations.
    right_has_annotation: bool,
    /// Whether the assignment expression has annotations.
    node_has_annotation: bool,
    /// Whether current assignment is used as an index operand.
    is_index_operand_assignment: bool,
    /// Whether assignment source span contains newlines.
    assignment_has_newline: bool,
    /// Whether left source span contains newlines.
    left_has_newline: bool,
    /// Whether right source span contains newlines.
    right_has_newline: bool,
    /// Whether left expression has postfix annotations.
    has_left_postfix: bool,
    /// Whether rhs is a binary expression.
    right_is_binary: bool,
    /// Whether rhs is a sequence expression.
    right_is_sequence: bool,
    /// Whether rhs is another assignment.
    right_is_assign: bool,
    /// Whether rhs is chain-like.
    right_is_chain: bool,
    /// Whether rhs is a tail lambda of an assignment chain.
    right_is_chain_tail_lambda: bool,
    /// Whether rhs is lambda-like.
    right_is_lambda: bool,
    /// Whether rhs handles internal break policy.
    right_handles_its_own_breaking: bool,
    /// Whether rhs has prefix annotations.
    right_has_prefix_annotation: bool,
    /// Whether rhs has inline assignment seam prefix comments.
    right_has_assignment_seam_inline_prefix_comment: bool,
    /// Whether rhs prefix annotations force operator break.
    right_has_prefix_annotation_that_forces_operator_break: bool,
    /// Whether there is any comment between left and right spans.
    right_has_between_comment: bool,
    /// Whether rhs is an inline-safe index operand value.
    right_is_inline_index_operand_value: bool,
    /// Whether rhs is one string-like atomic literal.
    is_string_literal: bool,
    /// Whether rhs starts with keyword-prefixed expression directly.
    right_is_keyword_prefixed_expression: bool,
    /// Whether rhs chain starts with keyword-prefixed expression.
    right_chain_starts_with_keyword_prefixed_expression: bool,
    /// Whether rhs is multiline and compact in source.
    right_is_compact_multiline: bool,
    /// Whether left assignment chain is multiline.
    left_assignment_chain_is_multiline: bool,
    /// Whether right assignment chain is multiline.
    right_assignment_chain_is_multiline: bool,
    /// Whether rhs is multiline binary that should break after operator.
    right_is_multiline_binary: bool,
    /// Whether assignment appears under one call argument.
    node_is_call_argument: bool,
    /// Whether rhs is collection or call-like expression.
    right_is_collection_or_call_like: bool,
    /// Whether rhs is an anonymous class declaration expression.
    right_is_anonymous_class_declaration: bool,
    /// Whether left target is a large object target.
    left_is_expanded_object_target: bool,
    /// Whether rhs is a short object literal.
    right_is_short_object: bool,
    /// Whether rhs is an inline atomic expression.
    right_is_inline_atomic: bool,
}

/// Assignment rendering strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AssignmentRenderStrategy {
    /// Grouped inline render with one space after operator.
    GroupInlineSpace,
    /// Ungrouped inline render with one space after operator.
    UngroupedInlineSpace,
    /// Grouped inline render with indented rhs after operator.
    GroupInlineSpaceIndentedRight,
    /// Grouped render with softline seam and dedented rhs.
    GroupSoftlineDedentRight,
    /// Grouped render with softline seam and plain rhs.
    GroupSoftlinePlainRight,
    /// Grouped render with hardline seam and dedented rhs.
    GroupHardlineDedentRight,
    /// Grouped render with hardline seam and plain rhs.
    GroupHardlinePlainRight,
}

/// Collect assignment layout facts from one node.
fn collect_assignment_layout_facts(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    left: LocalNodeId<Expression>,
    right: LocalNodeId<Expression>,
) -> AssignmentLayoutFacts {
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
    let left_has_annotation = context.has_annotation(left);
    let right_has_annotation = context.has_annotation(right);
    let node_has_annotation = context.has_annotation(node_id);
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
    let right_is_chain = is_expression_chain(context.tree, inner_right_id)
        || is_chain_root(context.tree, inner_right_id);
    let right_is_chain_tail_lambda =
        is_assignment_chain_tail_lambda(context, node_id, inner_right_id);
    let right_is_lambda = is_lambda_expression(context, inner_right_id);
    let right_handles_its_own_breaking = right_is_binary
        || right_is_sequence
        || right_is_assign
        || right_is_chain
        || right_is_chain_tail_lambda
        || right_is_lambda;
    let right_has_prefix_annotation = context.has_prefix_annotation(right);
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

    // assignment chain profile
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

    // rhs break profile
    let right_is_multiline_binary = if right_is_binary {
        let binary_operand_count = match inner_right_expression {
            Expression::Binary { operator, .. } => {
                flattened_binary_operand_count(context.tree, inner_right_id, *operator)
            }
            _ => 0,
        };
        binary_operand_count > LONG_BINARY_OPERAND_COUNT_THRESHOLD
            && (right_has_between_comment || right_has_newline)
    } else {
        false
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

    // fallback profile
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

    AssignmentLayoutFacts {
        inner_right_id,
        has_left_assignment_parent,
        left_has_annotation,
        right_has_annotation,
        node_has_annotation,
        is_index_operand_assignment,
        assignment_has_newline,
        left_has_newline,
        right_has_newline,
        has_left_postfix,
        right_is_binary,
        right_is_sequence,
        right_is_assign,
        right_is_chain,
        right_is_chain_tail_lambda,
        right_is_lambda,
        right_handles_its_own_breaking,
        right_has_prefix_annotation,
        right_has_assignment_seam_inline_prefix_comment,
        right_has_prefix_annotation_that_forces_operator_break,
        right_has_between_comment,
        right_is_inline_index_operand_value,
        is_string_literal,
        right_is_keyword_prefixed_expression,
        right_chain_starts_with_keyword_prefixed_expression,
        right_is_compact_multiline,
        left_assignment_chain_is_multiline,
        right_assignment_chain_is_multiline,
        right_is_multiline_binary,
        node_is_call_argument,
        right_is_collection_or_call_like,
        right_is_anonymous_class_declaration,
        left_is_expanded_object_target,
        right_is_short_object,
        right_is_inline_atomic,
    }
}

/// Return whether assignment should stay inline in one index operand seam.
fn assignment_prefers_inline_index_operand_strategy(facts: AssignmentLayoutFacts) -> bool {
    facts.is_index_operand_assignment
        && facts.right_is_inline_index_operand_value
        && !facts.assignment_has_newline
        && !facts.left_has_newline
        && !facts.right_has_newline
        && !facts.left_has_annotation
        && !facts.right_has_annotation
        && !facts.node_has_annotation
        && !facts.right_has_prefix_annotation_that_forces_operator_break
        && !facts.right_has_between_comment
}

/// Return whether rhs self-managed layout should prefer a break after operator.
fn assignment_rhs_prefers_operator_break(
    context: &DestackFormatContext<'_>,
    facts: AssignmentLayoutFacts,
) -> bool {
    if facts.right_is_assign {
        return facts.right_has_prefix_annotation_that_forces_operator_break
            || facts.right_has_between_comment
            || facts.node_is_call_argument;
    }

    if facts.right_is_lambda {
        return facts.right_has_prefix_annotation_that_forces_operator_break
            || facts.right_has_between_comment
            || facts.right_is_compact_multiline;
    }

    if facts.right_is_chain {
        return !facts.right_is_lambda
            && (facts.right_is_chain_tail_lambda
                || expression_chain_should_break(context, facts.inner_right_id)
                || facts.right_has_prefix_annotation_that_forces_operator_break
                || facts.right_has_between_comment);
    }

    if facts.right_is_binary {
        return !facts.right_is_lambda
            && (facts.right_is_chain_tail_lambda
                || facts.right_has_prefix_annotation_that_forces_operator_break
                || facts.right_has_between_comment
                || facts.assignment_has_newline
                || facts.right_is_compact_multiline);
    }

    !facts.right_is_lambda
        && (facts.right_is_chain_tail_lambda
            || facts.right_has_prefix_annotation_that_forces_operator_break
            || facts.right_is_compact_multiline
            || facts.right_has_between_comment)
}

/// Select one assignment render strategy from collected facts.
fn select_assignment_render_strategy(
    context: &DestackFormatContext<'_>,
    facts: AssignmentLayoutFacts,
) -> AssignmentRenderStrategy {
    // short circuit: trivia free simple assignments stay inline
    if assignment_prefers_inline_index_operand_strategy(facts) {
        return AssignmentRenderStrategy::GroupInlineSpace;
    }

    // string literals are atomic: never break at `=`
    if facts.is_string_literal {
        return AssignmentRenderStrategy::GroupInlineSpace;
    }

    // keep rhs keyword expressions (`await`, `await?`, `comptime`) attached to `=`
    if (facts.right_is_keyword_prefixed_expression
        || facts.right_chain_starts_with_keyword_prefixed_expression)
        && !facts.right_has_prefix_annotation_that_forces_operator_break
        && !facts.right_has_between_comment
    {
        return AssignmentRenderStrategy::GroupInlineSpace;
    }

    // keep assignment seam inline prefix comments with the operator
    if facts.right_has_assignment_seam_inline_prefix_comment {
        return AssignmentRenderStrategy::GroupInlineSpaceIndentedRight;
    }

    // left associative assignment chains: keep inner chain steps inline
    if facts.has_left_assignment_parent
        && !facts.right_has_prefix_annotation_that_forces_operator_break
        && !facts.right_has_between_comment
        && !facts.right_has_newline
        && !facts.left_assignment_chain_is_multiline
    {
        return AssignmentRenderStrategy::GroupSoftlineDedentRight;
    }

    // expanded right-associative assignment chains should break on each seam
    if facts.right_assignment_chain_is_multiline
        && !facts.right_has_prefix_annotation_that_forces_operator_break
        && !facts.right_has_between_comment
    {
        return AssignmentRenderStrategy::GroupHardlineDedentRight;
    }

    // break long binary rhs values after the operator
    if facts.right_is_multiline_binary {
        return AssignmentRenderStrategy::GroupHardlineDedentRight;
    }

    // keep simple rhs collection/call-like values inline
    if (facts.right_is_collection_or_call_like || facts.right_is_anonymous_class_declaration)
        && !facts.right_is_chain
        && !facts.right_has_prefix_annotation
        && !facts.right_has_between_comment
    {
        return AssignmentRenderStrategy::GroupInlineSpace;
    }

    // rhs-managed layout cases
    if facts.right_handles_its_own_breaking {
        if assignment_rhs_prefers_operator_break(context, facts) {
            if facts.right_has_prefix_annotation_that_forces_operator_break
                || facts.right_has_between_comment
                || facts.right_is_sequence
            {
                return AssignmentRenderStrategy::GroupHardlinePlainRight;
            }

            return AssignmentRenderStrategy::GroupHardlineDedentRight;
        }

        if facts.right_is_chain {
            return AssignmentRenderStrategy::GroupInlineSpace;
        }

        return AssignmentRenderStrategy::GroupSoftlineDedentRight;
    }

    // fallback expression layout cases
    let should_force_break_for_multiline_left =
        facts.left_has_newline && !facts.right_is_short_object && !facts.right_is_inline_atomic;
    if should_force_break_for_multiline_left {
        return AssignmentRenderStrategy::GroupHardlineDedentRight;
    }

    if (facts.left_has_newline || facts.left_is_expanded_object_target)
        && (facts.right_is_short_object || facts.right_is_inline_atomic)
    {
        return AssignmentRenderStrategy::UngroupedInlineSpace;
    }

    AssignmentRenderStrategy::GroupSoftlinePlainRight
}

/// Render one assignment expression with one selected layout strategy.
fn render_assign_expression_with_strategy<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    left: LocalNodeId<Expression>,
    operator: &AssignOperator,
    right: LocalNodeId<Expression>,
    has_left_postfix: bool,
    strategy: AssignmentRenderStrategy,
) -> FormatResult<()> {
    // format space before operator, respecting postfix comments
    let space_before_operator = format_with(|f| {
        if !has_left_postfix {
            write!(f, [space()])?;
        }

        Ok(())
    });

    // render selected strategy
    match strategy {
        AssignmentRenderStrategy::GroupInlineSpace => {
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
        }
        AssignmentRenderStrategy::UngroupedInlineSpace => {
            write!(f, [left, space_before_operator, operator, space(), right])?;
        }
        AssignmentRenderStrategy::GroupInlineSpaceIndentedRight => {
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
        }
        AssignmentRenderStrategy::GroupSoftlineDedentRight => {
            write!(
                f,
                [group(&format_args![
                    left,
                    space_before_operator,
                    operator,
                    indent(&format_args![soft_line_break_or_space(), dedent(&right)])
                ])]
            )?;
        }
        AssignmentRenderStrategy::GroupSoftlinePlainRight => {
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
        AssignmentRenderStrategy::GroupHardlineDedentRight => {
            write!(
                f,
                [group(&format_args![
                    left,
                    space_before_operator,
                    operator,
                    indent(&format_args![hard_line_break(), dedent(&right)])
                ])]
            )?;
        }
        AssignmentRenderStrategy::GroupHardlinePlainRight => {
            write!(
                f,
                [group(&format_args![
                    left,
                    space_before_operator,
                    operator,
                    indent(&format_args![hard_line_break(), right])
                ])]
            )?;
        }
    }

    Ok(())
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

/// Format an assignment expression with shared rhs break policy.
pub(crate) fn format_assign_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    left: LocalNodeId<Expression>,
    operator: &AssignOperator,
    right: LocalNodeId<Expression>,
) -> FormatResult<()> {
    // facts
    let facts = collect_assignment_layout_facts(f.context(), node_id, left, right);

    // rules
    let strategy = select_assignment_render_strategy(f.context(), facts);

    // render
    render_assign_expression_with_strategy(
        f,
        left,
        operator,
        right,
        facts.has_left_postfix,
        strategy,
    )
}
