use super::binary::format_binary_operand_with_grouping_parentheses;
use super::r#type::{
    expression_has_type_grouping_semantics, normalize_parenthesized_type_grouping_inner_expression,
};
use super::types::{is_in_type_template_literal_interpolation, is_object_like_type_expression};
use crate::format::chain::transparent_inner_expression;
use crate::format::context::{
    expression_has_line_postfix_slash_comment, expression_has_own_line_prefix,
    expression_has_postfix_comment,
};
use crate::format::expression::{
    expression_has_leading_prefix_comment, should_drop_parenthesized_expression_wrapper,
    write_expression_without_prefix_annotations,
};
use crate::{DestackFormatContext, DestackFormatter};
use destack_ast::{
    AnnotationPosition, BinaryOperator, Comment, CommentStyle, Declaration, Expression,
    LocalNodeId, NodeType, TokenType, TypeBinaryOperator, TypeLiteral,
};
use destack_fir::format::{Buffer, FormatResult};
use destack_fir::prelude::{
    align, format_with, group, hard_line_break, if_group_breaks, indent, soft_line_break_or_space,
    space, text, token,
};
use destack_fir::{format_args, write};
use smallvec::SmallVec;

#[derive(Clone, Copy, Debug, Default)]
struct LeadingCommentsInfo {
    has_own_line_comment: bool,
    has_end_of_line_comment: bool,
    has_trailing_own_line_doc_comment: bool,
}

/// Return whether one binary-like root has type grouping semantics.
fn binary_like_has_type_semantics(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    expression_has_type_grouping_semantics(context, node_id)
        || is_in_type_template_literal_interpolation(context, node_id)
}

/// Return whether one binary-like root should use type-union ownership.
pub(crate) fn binary_like_is_type_union(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    operator: BinaryOperator,
) -> bool {
    if operator != BinaryOperator::ElementwiseOr {
        return false;
    }

    let normalized_root_id =
        transparent_type_binary_root_expression(context, node_id, BinaryOperator::ElementwiseOr);
    matches!(
        context.tree.get(normalized_root_id),
        Expression::Binary {
            operator: BinaryOperator::ElementwiseOr,
            ..
        }
    ) && binary_like_has_type_semantics(context, normalized_root_id)
}

/// Return whether one binary-like root should use type-intersection ownership.
pub(crate) fn binary_like_is_type_intersection(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    operator: BinaryOperator,
) -> bool {
    if operator != BinaryOperator::ElementwiseAnd {
        return false;
    }

    let normalized_root_id =
        transparent_type_binary_root_expression(context, node_id, BinaryOperator::ElementwiseAnd);
    matches!(
        context.tree.get(normalized_root_id),
        Expression::Binary {
            operator: BinaryOperator::ElementwiseAnd,
            ..
        }
    ) && binary_like_has_type_semantics(context, normalized_root_id)
}

/// Flatten operands for one binary-like owner according to its semantic family.
pub(crate) fn flatten_binary_like_operands(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    operator: BinaryOperator,
) -> SmallVec<[(Option<BinaryOperator>, LocalNodeId<Expression>); 8]> {
    if binary_like_is_type_union(context, node_id, operator) {
        let normalized_root_id = transparent_type_binary_root_expression(
            context,
            node_id,
            BinaryOperator::ElementwiseOr,
        );
        return flatten_type_binary_expression(context, normalized_root_id, operator);
    }

    if binary_like_is_type_intersection(context, node_id, operator) {
        return flatten_type_binary_expression(context, node_id, operator);
    }

    super::binary::flatten_binary_expression(context, node_id, operator)
}

/// Flattens associative type binary chains while unwrapping redundant parentheses.
pub(crate) fn flatten_type_binary_expression(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
    target_operator: BinaryOperator,
) -> SmallVec<[(Option<BinaryOperator>, LocalNodeId<Expression>); 8]> {
    let mut operands = SmallVec::new();
    flatten_type_binary_recursive(context, expression_id, target_operator, &mut operands, None);
    operands
}

/// Recursively flatten type binary chains and preserve operand operators.
fn flatten_type_binary_recursive(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
    target_operator: BinaryOperator,
    operands: &mut SmallVec<[(Option<BinaryOperator>, LocalNodeId<Expression>); 8]>,
    preceding_operator: Option<BinaryOperator>,
) {
    let expression_id =
        normalize_type_binary_operand_expression(context, expression_id, target_operator);

    if let Expression::Binary {
        left,
        operator,
        right,
    } = context.tree.get(expression_id)
        && *operator == target_operator
    {
        flatten_type_binary_recursive(
            context,
            *left,
            target_operator,
            operands,
            preceding_operator,
        );
        flatten_type_binary_recursive(context, *right, target_operator, operands, Some(*operator));
        return;
    }

    operands.push((preceding_operator, expression_id));
}

/// Remove redundant parenthesized wrappers around associative type operands.
fn normalize_type_binary_operand_expression(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
    target_operator: BinaryOperator,
) -> LocalNodeId<Expression> {
    let mut current_id = expression_id;

    loop {
        let Expression::Parenthesized { expression } = context.tree.get(current_id) else {
            break;
        };

        if context.has_annotation(current_id) {
            break;
        }

        let inner_id = *expression;
        let inner_is_flattenable = matches!(
            context.tree.get(inner_id),
            Expression::Binary { operator, .. } if *operator == target_operator
        );
        let inner_is_parenthesized =
            matches!(context.tree.get(inner_id), Expression::Parenthesized { .. });
        if !inner_is_flattenable && !inner_is_parenthesized {
            break;
        }

        current_id = inner_id;
    }

    current_id
}

/// Return whether one type binary operand needs grouping parentheses.
pub(crate) fn type_binary_operand_needs_grouping_parentheses(
    context: &DestackFormatContext<'_>,
    parent_operator: BinaryOperator,
    operand_id: LocalNodeId<Expression>,
) -> bool {
    if !matches!(
        parent_operator,
        BinaryOperator::ElementwiseOr | BinaryOperator::ElementwiseAnd
    ) {
        return false;
    }

    if matches!(
        context.tree.get(operand_id),
        Expression::Parenthesized { .. }
    ) {
        return false;
    }

    let inner_id = transparent_inner_expression(context, operand_id);
    let Expression::Binary {
        operator: inner_operator,
        ..
    } = context.tree.get(inner_id)
    else {
        return false;
    };

    parent_operator == BinaryOperator::ElementwiseAnd
        && *inner_operator == BinaryOperator::ElementwiseOr
        && expression_has_type_grouping_semantics(context, inner_id)
}

/// Return the transparent inner expression for one type-binary chain root.
fn transparent_type_binary_chain_inner_expression(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    operator: BinaryOperator,
) -> Option<LocalNodeId<Expression>> {
    match context.tree.get(node_id) {
        Expression::Statement(inner_expression_id) => Some(*inner_expression_id),
        Expression::Parenthesized {
            expression: inner_expression_id,
        } => Some(*inner_expression_id),
        Expression::Binary {
            operator: expression_operator,
            ..
        } if *expression_operator == operator
            && expression_has_type_grouping_semantics(context, node_id) =>
        {
            let operands = flatten_type_binary_expression(context, node_id, operator);
            if operands.len() == 1 && operands[0].1 != node_id {
                Some(operands[0].1)
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Collect transparent owners from one type-binary root down to its normalized inner node.
pub(crate) fn collect_transparent_type_binary_root_owner_ids(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    operator: BinaryOperator,
) -> SmallVec<[LocalNodeId<Expression>; 8]> {
    let mut owner_ids = SmallVec::new();
    let mut current_expression_id = node_id;
    owner_ids.push(node_id);

    while let Some(inner_expression_id) =
        transparent_type_binary_chain_inner_expression(context, current_expression_id, operator)
    {
        owner_ids.push(inner_expression_id);
        current_expression_id = inner_expression_id;
    }

    owner_ids
}

/// Return the normalized inner root for one type-binary chain.
pub(crate) fn transparent_type_binary_root_expression(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    operator: BinaryOperator,
) -> LocalNodeId<Expression> {
    collect_transparent_type_binary_root_owner_ids(context, node_id, operator)
        .last()
        .copied()
        .unwrap_or(node_id)
}

/// Return whether one operator expression owns its prefix annotations locally.
pub(crate) fn operator_expression_owns_prefix_annotations(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
    expression: &Expression,
    is_ignored: bool,
) -> bool {
    let parenthesized_delegates_union_prefix_annotations = if !is_ignored {
        match expression {
            Expression::Parenthesized {
                expression: inner_expression_id,
            } if should_drop_parenthesized_expression_wrapper(
                context,
                expression_id,
                *inner_expression_id,
            ) =>
            {
                let normalized_inner_id = normalize_parenthesized_type_grouping_inner_expression(
                    context,
                    *inner_expression_id,
                );

                matches!(
                    context.tree.get(normalized_inner_id),
                    Expression::Binary {
                        operator: BinaryOperator::ElementwiseOr,
                        ..
                    }
                ) && union_owns_prefix_annotations(context, expression_id)
            }
            _ => false,
        }
    } else {
        false
    };

    matches!(
        expression,
        Expression::Binary {
            operator: BinaryOperator::ElementwiseOr,
            ..
        } if union_owns_prefix_annotations(context, expression_id) && !is_ignored
    ) || parenthesized_delegates_union_prefix_annotations
}

impl LeadingCommentsInfo {
    fn from_comment_nodes(
        context: &DestackFormatContext<'_>,
        comment_ids: &[LocalNodeId<Comment>],
    ) -> Self {
        let mut info = Self::default();

        for comment_id in comment_ids.iter().copied() {
            let comment_span = context.span(comment_id);
            let comment_source = context.span_str(comment_span);
            let is_doc_comment = comment_source.starts_with("/**");

            info.has_own_line_comment |= context.span_starts_on_own_line(comment_span);
            info.has_end_of_line_comment |=
                context.span_has_newline_before_next_non_whitespace_token(comment_span);
            info.has_trailing_own_line_doc_comment |= is_doc_comment
                && context.span_has_newline_before_next_non_whitespace_token(comment_span);
        }

        info
    }
}

/// Write raw leading comment nodes that belong before one union shell.
fn write_union_leading_comment_nodes<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    comment_ids: &[LocalNodeId<Comment>],
) -> FormatResult<()> {
    for (index, comment_id) in comment_ids.iter().copied().enumerate() {
        write!(f, [comment_id])?;

        if let Some(next_comment_id) = comment_ids.get(index + 1).copied() {
            let comment_span = f.context().span(comment_id);
            let next_comment_span = f.context().span(next_comment_id);
            let gap_span = destack_source::Span::new(
                comment_span.file,
                comment_span.end,
                next_comment_span.start,
            );

            if f.context().has_newline(gap_span) {
                write!(f, [hard_line_break()])?;
            } else {
                write!(f, [space()])?;
            }
        }
    }

    Ok(())
}

/// Write shell-owned prefix annotations using their raw source text.
fn write_union_leading_prefix_annotations<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    annotation_ids: &[LocalNodeId<crate::Annotation>],
) -> FormatResult<()> {
    for (index, annotation_id) in annotation_ids.iter().copied().enumerate() {
        let annotation_span = f.context().annotation_span(annotation_id);
        write!(f, [text(f.context().span_str(annotation_span))])?;

        let is_last = index + 1 == annotation_ids.len();
        if !f
            .context()
            .annotation_next_token_is_on_same_line(annotation_id)
        {
            write!(f, [hard_line_break()])?;
        } else if !is_last {
            write!(f, [space()])?;
        }
    }

    Ok(())
}

/// Collect comment nodes between one transparent union owner head and the first `|`.
fn type_union_leading_comment_nodes(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    operands: &SmallVec<[(Option<BinaryOperator>, LocalNodeId<Expression>); 8]>,
) -> Vec<LocalNodeId<Comment>> {
    let Some(first_operand) = operands.first() else {
        return Vec::new();
    };

    let owner_id =
        transparent_type_binary_chain_owner(context, node_id, BinaryOperator::ElementwiseOr);
    let owner_span = context.span(owner_id);
    let Some(separator_span) = type_union_operand_separator_token_span(context, first_operand.1)
    else {
        return Vec::new();
    };
    if owner_span.file == separator_span.file && owner_span.start < separator_span.start {
        let comment_nodes = context.comment_nodes_in_range(owner_span.start, separator_span.start);
        if !comment_nodes.is_empty() {
            return comment_nodes;
        }
    }

    let Some(previous_token) = context.previous_non_whitespace_token_before_span(separator_span)
    else {
        return Vec::new();
    };
    if previous_token.span.file != separator_span.file
        || previous_token.span.end >= separator_span.start
    {
        return Vec::new();
    }

    context.comment_nodes_before_character(previous_token.span.end, b'|')
}

/// Return whether leading shell comments span multiple source lines.
fn type_union_leading_gap_comments_span_multiple_lines(
    context: &DestackFormatContext<'_>,
    comment_ids: &[LocalNodeId<Comment>],
) -> bool {
    comment_ids.windows(2).any(|comment_pair| {
        let left_span = context.span(comment_pair[0]);
        let right_span = context.span(comment_pair[1]);
        left_span.file == right_span.file
            && context.has_newline(destack_source::Span::new(
                left_span.file,
                left_span.end,
                right_span.start,
            ))
    })
}

/// Split leading gap comments into shell-head comments and first-separator comments.
fn split_type_union_leading_gap_comment_nodes(
    context: &DestackFormatContext<'_>,
    comment_ids: &[LocalNodeId<Comment>],
) -> (Vec<LocalNodeId<Comment>>, Vec<LocalNodeId<Comment>>) {
    let Some(last_comment_id) = comment_ids.last().copied() else {
        return (Vec::new(), Vec::new());
    };
    let last_comment_span = context.span(last_comment_id);
    let has_trailing_separator_comment_cluster = comment_ids.len() > 1
        && context.span_starts_on_own_line(last_comment_span)
        && !context.span_has_newline_before_next_non_whitespace_token(last_comment_span);
    if !has_trailing_separator_comment_cluster {
        return (comment_ids.to_vec(), Vec::new());
    }

    let mut separator_comment_start = comment_ids.len() - 1;
    while separator_comment_start > 0 {
        let current_span = context.span(comment_ids[separator_comment_start]);
        if context.span_starts_on_own_line(current_span) {
            break;
        }

        separator_comment_start -= 1;
    }

    (
        comment_ids[..separator_comment_start].to_vec(),
        comment_ids[separator_comment_start..].to_vec(),
    )
}

/// Return whether one type union should stay attached after `=` to keep shell comments inline.
pub(crate) fn type_union_prefers_inline_assignment_seam(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let union_root_id =
        transparent_type_binary_root_expression(context, node_id, BinaryOperator::ElementwiseOr);
    if !matches!(
        context.tree.get(union_root_id),
        Expression::Binary {
            operator: BinaryOperator::ElementwiseOr,
            ..
        }
    ) {
        return false;
    }

    let operands =
        flatten_type_binary_expression(context, union_root_id, BinaryOperator::ElementwiseOr);
    let leading_gap_comment_nodes =
        type_union_leading_comment_nodes(context, union_root_id, &operands);

    !leading_gap_comment_nodes.is_empty()
        && !type_union_leading_gap_comments_span_multiple_lines(context, &leading_gap_comment_nodes)
}

/// Collect prefix annotations on the first operand that source places before the first `|`.
fn type_union_leading_shell_prefix_annotations(
    context: &DestackFormatContext<'_>,
    union_expression_id: LocalNodeId<Expression>,
    operand_expression_id: LocalNodeId<Expression>,
) -> Vec<LocalNodeId<crate::Annotation>> {
    let mut owner_ids = collect_transparent_type_binary_root_owner_ids(
        context,
        union_expression_id,
        BinaryOperator::ElementwiseOr,
    );
    if !owner_ids.contains(&operand_expression_id) {
        owner_ids.push(operand_expression_id);
    }

    owner_ids
        .into_iter()
        .flat_map(|owner_id| context.annotation_ids(owner_id).iter().copied())
        .filter(|annotation_id| {
            let previous_token_type = context
                .annotation_previous_non_whitespace_token(*annotation_id)
                .map(|token| token.token.ty);

            matches!(
                context.annotation(*annotation_id).position(),
                AnnotationPosition::BlockPrefix | AnnotationPosition::LinePrefix
            ) && context.annotation_next_non_whitespace_token_type(*annotation_id)
                == Some(TokenType::ElementwiseOr)
                && previous_token_type != Some(TokenType::ElementwiseOr)
                && context
                    .span_str(context.annotation_span(*annotation_id))
                    .trim_start()
                    .starts_with("/**")
        })
        .collect()
}

/// Return the leftmost terminal expression along one transparent union left spine.
fn leftmost_union_terminal_expression(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> LocalNodeId<Expression> {
    let mut current_id = node_id;
    loop {
        current_id = match context.tree.get(current_id) {
            Expression::Parenthesized { expression } | Expression::Statement(expression) => {
                *expression
            }
            Expression::Binary {
                operator: BinaryOperator::ElementwiseOr,
                left,
                ..
            } if expression_has_type_grouping_semantics(context, current_id)
                || is_in_type_template_literal_interpolation(context, current_id) =>
            {
                *left
            }
            _ => return current_id,
        };
    }
}

/// Return whether one expression is object-like for union hug layout.
fn expression_is_hug_object_like_union_operand(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let expression_id = transparent_inner_expression(context, expression_id);

    is_object_like_type_expression(context, expression_id)
        || matches!(
            context.tree.get(expression_id),
            Expression::Path { .. } | Expression::TypeLiteral(TypeLiteral::Object)
        )
}

/// Return whether one expression is void-like for union hug layout.
fn expression_is_hug_void_like_union_operand(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let expression_id = transparent_inner_expression(context, expression_id);

    matches!(
        context.tree.get(expression_id),
        Expression::TypeLiteral(TypeLiteral::Void | TypeLiteral::Null | TypeLiteral::Undefined)
    )
}

/// Return whether one union is the value of a type declaration through transparent wrappers.
fn union_is_type_declaration_value(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let mut current_expression_id = node_id;

    while let Some((parent_id, parent_type)) = context.parent(current_expression_id) {
        if parent_type == NodeType::Declaration {
            let declaration_id = LocalNodeId::<Declaration>::new(parent_id);
            let Declaration::Type { value, .. } = context.tree.get(declaration_id) else {
                return false;
            };
            return *value == current_expression_id;
        }

        if parent_type != NodeType::Expression {
            return false;
        }

        let parent_expression_id = LocalNodeId::<Expression>::new(parent_id);
        match context.tree.get(parent_expression_id) {
            Expression::Statement(inner_expression_id)
                if *inner_expression_id == current_expression_id =>
            {
                current_expression_id = parent_expression_id;
            }
            Expression::Parenthesized {
                expression: inner_expression_id,
            } if *inner_expression_id == current_expression_id => {
                current_expression_id = parent_expression_id;
            }
            Expression::Binary { operator, .. }
                if expression_has_type_grouping_semantics(context, parent_expression_id)
                    && matches!(
                        operator,
                        BinaryOperator::ElementwiseOr | BinaryOperator::ElementwiseAnd
                    ) =>
            {
                let operands =
                    flatten_type_binary_expression(context, parent_expression_id, *operator);
                if operands.len() != 1 || operands[0].1 != current_expression_id {
                    return false;
                }

                current_expression_id = parent_expression_id;
            }
            _ => {
                return false;
            }
        }
    }

    false
}

/// Return the transparent top owner for one type-binary chain node.
fn transparent_type_binary_chain_owner(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    operator: BinaryOperator,
) -> LocalNodeId<Expression> {
    let mut current_expression_id = node_id;
    let mut owner_id = node_id;

    while let Some((parent_id, parent_type)) = context.parent(current_expression_id) {
        if parent_type != NodeType::Expression {
            break;
        }

        let parent_expression_id = LocalNodeId::<Expression>::new(parent_id);
        match context.tree.get(parent_expression_id) {
            Expression::Statement(inner_expression_id)
                if *inner_expression_id == current_expression_id =>
            {
                owner_id = parent_expression_id;
                current_expression_id = parent_expression_id;
            }
            Expression::Parenthesized {
                expression: inner_expression_id,
            } if *inner_expression_id == current_expression_id => {
                owner_id = parent_expression_id;
                current_expression_id = parent_expression_id;
            }
            Expression::Binary {
                operator: parent_operator,
                ..
            } if *parent_operator == operator
                && expression_has_type_grouping_semantics(context, parent_expression_id) =>
            {
                let operands =
                    flatten_type_binary_expression(context, parent_expression_id, operator);
                if operands.len() != 1 || operands[0].1 != current_expression_id {
                    break;
                }

                owner_id = parent_expression_id;
                current_expression_id = parent_expression_id;
            }
            _ => break,
        }
    }

    owner_id
}

/// Return whether one type-union expression should render prefix annotations in its own layout.
pub(crate) fn union_owns_prefix_annotations(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let owner_ids = collect_transparent_type_binary_root_owner_ids(
        context,
        node_id,
        BinaryOperator::ElementwiseOr,
    );
    let union_root_id = owner_ids.last().copied().unwrap_or(node_id);
    if !owner_ids
        .iter()
        .copied()
        .any(|owner_id| context.has_prefix_annotation(owner_id))
    {
        return false;
    }

    let owns_prefix_in_parent_context = union_is_type_declaration_value(context, node_id)
        || is_in_type_template_literal_interpolation(context, node_id);
    if !owns_prefix_in_parent_context {
        return false;
    }

    let leftmost_terminal_expression = leftmost_union_terminal_expression(context, union_root_id);
    !context.has_prefix_annotation(leftmost_terminal_expression)
}

/// Return whether one type union should use object-and-void hug layout.
fn should_hug_type_union_operands(
    context: &DestackFormatContext<'_>,
    operands: &SmallVec<[(Option<BinaryOperator>, LocalNodeId<Expression>); 8]>,
) -> bool {
    if operands.len() <= 1 {
        return true;
    }

    if operands
        .iter()
        .any(|operand| context.has_annotation(operand.1))
    {
        return false;
    }

    let object_operand_expression_id = operands.iter().find_map(|operand| {
        expression_is_hug_object_like_union_operand(context, operand.1)
            .then_some(transparent_inner_expression(context, operand.1))
    });
    let Some(object_operand_expression_id) = object_operand_expression_id else {
        return false;
    };

    operands.iter().all(|operand| {
        let expression_id = transparent_inner_expression(context, operand.1);
        expression_id == object_operand_expression_id
            || expression_is_hug_void_like_union_operand(context, expression_id)
    })
}

/// Return whether one union root ends with an own-line doc prefix annotation.
pub(crate) fn union_has_trailing_own_line_doc_prefix_annotation(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let union_root_id =
        transparent_type_binary_root_expression(context, node_id, BinaryOperator::ElementwiseOr);
    if !matches!(
        context.tree.get(union_root_id),
        Expression::Binary {
            operator: BinaryOperator::ElementwiseOr,
            ..
        }
    ) {
        return false;
    }

    let operands =
        flatten_type_binary_expression(context, union_root_id, BinaryOperator::ElementwiseOr);
    let leading_comment_nodes = type_union_leading_comment_nodes(context, union_root_id, &operands);
    let leading_comment_info =
        LeadingCommentsInfo::from_comment_nodes(context, &leading_comment_nodes);
    if leading_comment_info.has_trailing_own_line_doc_comment {
        return true;
    }

    operands.first().is_some_and(|operand| {
        !type_union_leading_shell_prefix_annotations(context, union_root_id, operand.1).is_empty()
    })
}

/// Return whether one declaration-expression ancestor has a line-postfix comment.
fn declaration_expression_ancestor_has_line_postfix_comment(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let mut current_node_id = node_id.id;
    while let Some((parent_id, parent_type)) = context.parent_by_id(current_node_id) {
        if parent_type == NodeType::Expression {
            let parent_expression_id = LocalNodeId::<Expression>::new(parent_id);
            if matches!(
                context.tree.get(parent_expression_id),
                Expression::Declaration(_)
            ) && expression_has_line_postfix_slash_comment(context, parent_expression_id)
            {
                return true;
            }
        }

        current_node_id = parent_id;
    }

    false
}

/// Return whether one type union should apply its own indentation.
pub(crate) fn type_union_should_indent(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let union_top_id =
        transparent_type_binary_chain_owner(context, node_id, BinaryOperator::ElementwiseOr);
    if is_in_type_template_literal_interpolation(context, union_top_id) {
        return true;
    }

    if !union_is_type_declaration_value(context, union_top_id) {
        return false;
    }

    !union_has_trailing_own_line_doc_prefix_annotation(context, node_id)
}

/// Return whether one union should preserve inline layout for terminal line-postfix comments.
fn should_inline_union_with_terminal_line_postfix_comment(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    operands: &SmallVec<[(Option<BinaryOperator>, LocalNodeId<Expression>); 8]>,
    union_prefers_multiline_layout: bool,
) -> bool {
    if union_prefers_multiline_layout {
        return false;
    }

    if expression_has_leading_prefix_comment(context, node_id) {
        return false;
    }

    if expression_has_postfix_comment(context, node_id) {
        return true;
    }

    if declaration_expression_ancestor_has_line_postfix_comment(context, node_id) {
        return true;
    }

    let Some(last_operand) = operands.last() else {
        return false;
    };
    if !expression_has_line_postfix_slash_comment(context, last_operand.1) {
        return false;
    }

    let has_non_last_comments = operands
        .iter()
        .take(operands.len().saturating_sub(1))
        .any(|operand| expression_has_postfix_comment(context, operand.1));
    if has_non_last_comments {
        return false;
    }

    true
}

/// Return whether one union is the parenthesized rhs of cast or satisfies.
fn union_is_parenthesized_cast_or_satisfies_rhs(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let mut current_id = node_id;
    let mut saw_parenthesized_wrapper = false;

    loop {
        let Some((parent_id, parent_type)) = context.parent(current_id) else {
            return false;
        };
        if parent_type != NodeType::Expression {
            return false;
        }

        let parent_expression_id = LocalNodeId::<Expression>::new(parent_id);
        match context.tree.get(parent_expression_id) {
            Expression::Parenthesized { expression } if *expression == current_id => {
                saw_parenthesized_wrapper = true;
                current_id = parent_expression_id;
            }
            Expression::Statement(expression) if *expression == current_id => {
                current_id = parent_expression_id;
            }
            Expression::TypeBinary {
                operator: TypeBinaryOperator::Cast | TypeBinaryOperator::Satisfies,
                right,
                ..
            } if *right == current_id => {
                return saw_parenthesized_wrapper;
            }
            _ => {
                return false;
            }
        }
    }
}

/// Write one type-union operand after its separator.
fn write_type_union_operand_after_separator<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    operand_expression_id: LocalNodeId<Expression>,
    should_indent_union: bool,
    suppress_prefix_annotations: bool,
) -> FormatResult<()> {
    let separator_comments =
        type_union_operand_separator_comments(f.context(), operand_expression_id);
    let has_separator_comments = !separator_comments.is_empty();
    let operand_has_own_line_prefix_annotation =
        expression_has_own_line_prefix(f.context(), operand_expression_id);
    let operand_is_object_like =
        expression_is_hug_object_like_union_operand(f.context(), operand_expression_id);
    let blocks_object_alignment = has_separator_comments
        || (!suppress_prefix_annotations && operand_has_own_line_prefix_annotation);
    let format_operand = format_with(move |f: &mut DestackFormatter<'ast, '_>| {
        if has_separator_comments {
            for (comment_index, comment_id) in separator_comments.iter().enumerate() {
                if comment_index > 0 {
                    write!(f, [hard_line_break()])?;
                }

                let comment = f.context().tree.get::<Comment>(*comment_id);
                let comment_span = f.context().span(*comment_id);
                let should_keep_inline = separator_comments.len() == 1
                    && comment.style == CommentStyle::Star
                    && !f.context().has_newline(comment_span)
                    && !f.context().span_starts_on_own_line(comment_span);

                if should_keep_inline {
                    write!(f, [*comment_id, space()])?;
                } else {
                    write!(f, [*comment_id, hard_line_break()])?;
                }
            }

            if suppress_prefix_annotations {
                write_expression_without_prefix_annotations(f, operand_expression_id)
            } else {
                format_binary_operand_with_grouping_parentheses(
                    f,
                    BinaryOperator::ElementwiseOr,
                    operand_expression_id,
                )
            }
        } else if operand_has_own_line_prefix_annotation && !suppress_prefix_annotations {
            write!(f, [operand_expression_id])
        } else if suppress_prefix_annotations {
            write_expression_without_prefix_annotations(f, operand_expression_id)
        } else {
            format_binary_operand_with_grouping_parentheses(
                f,
                BinaryOperator::ElementwiseOr,
                operand_expression_id,
            )
        }
    });

    // object-like arms stay visually aligned under `| `
    if !should_indent_union && !blocks_object_alignment && operand_is_object_like {
        if f.context().options.indent_style.is_space() {
            write!(f, [align(2, &format_operand)])?;
        } else {
            write!(f, [indent(&format_operand)])?;
        }
    } else {
        write!(f, [format_operand])?;
    }

    Ok(())
}

/// Render one type union in inline `A | B` form.
fn format_inline_type_union_layout<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    operands: &SmallVec<[(Option<BinaryOperator>, LocalNodeId<Expression>); 8]>,
) -> FormatResult<()> {
    write!(
        f,
        [group(&format_with(|f: &mut DestackFormatter<'ast, '_>| {
            let Some(first_operand) = operands.first() else {
                return Ok(());
            };
            format_binary_operand_with_grouping_parentheses(
                f,
                BinaryOperator::ElementwiseOr,
                first_operand.1,
            )?;

            for operand in operands.iter().skip(1) {
                write!(
                    f,
                    [
                        space(),
                        token("|"),
                        space(),
                        format_with(|f: &mut DestackFormatter<'ast, '_>| {
                            format_binary_operand_with_grouping_parentheses(
                                f,
                                BinaryOperator::ElementwiseOr,
                                operand.1,
                            )
                        }),
                    ]
                )?;
            }

            Ok(())
        }))]
    )
}

/// Return whether one intersection is nested under a type union.
fn intersection_is_nested_under_type_union(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let Some((parent_id, parent_type)) = context.parent(node_id) else {
        return false;
    };
    if parent_type != NodeType::Expression {
        return false;
    }

    matches!(
        context.tree.get(LocalNodeId::<Expression>::new(parent_id)),
        Expression::Binary {
            operator: BinaryOperator::ElementwiseOr,
            ..
        }
    )
}

/// Render destack type intersections with trailing operators and object-like guard rails.
fn format_destack_intersection_trailing_layout<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    operands: &SmallVec<[(Option<BinaryOperator>, LocalNodeId<Expression>); 8]>,
) -> FormatResult<()> {
    write!(
        f,
        [group(&format_with(|f: &mut DestackFormatter<'ast, '_>| {
            let mut previous_expression: Option<LocalNodeId<Expression>> = None;
            let mut chain_is_indented = false;
            let mut previous_is_object_like = false;

            for (index, operand) in operands.iter().enumerate() {
                if index == 0 {
                    write!(f, [operand.1])?;
                    previous_is_object_like =
                        is_object_like_type_expression(f.context(), operand.1);
                    continue;
                }

                let Some(op) = operand.0 else {
                    continue;
                };

                let has_previous_postfix_annotation = previous_expression
                    .is_some_and(|expression_id| f.context().has_postfix_annotation(expression_id));
                if !has_previous_postfix_annotation {
                    write!(f, [space()])?;
                }
                write!(f, [op])?;

                let is_object_like = is_object_like_type_expression(f.context(), operand.1);
                if !(previous_is_object_like || is_object_like) {
                    write!(
                        f,
                        [indent(&format_with(
                            |f: &mut DestackFormatter<'ast, '_>| {
                                write!(f, [soft_line_break_or_space(), operand.1])
                            }
                        ))]
                    )?;
                } else {
                    write!(f, [space()])?;

                    if !previous_is_object_like || !is_object_like {
                        chain_is_indented = index > 1;
                    }

                    if chain_is_indented {
                        write!(f, [indent(&operand.1)])?;
                    } else {
                        write!(f, [operand.1])?;
                    }
                }

                previous_is_object_like = is_object_like;
                previous_expression = Some(operand.1);
            }

            Ok(())
        }))]
    )
}

/// Render non-destack intersections with oxc-style object-like chain layout.
fn format_oxc_intersection_layout<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    operands: &SmallVec<[(Option<BinaryOperator>, LocalNodeId<Expression>); 8]>,
) -> FormatResult<()> {
    write!(
        f,
        [group(&format_with(|f: &mut DestackFormatter<'ast, '_>| {
            let last_index = operands.len().saturating_sub(1);
            let mut previous_is_object_like = false;
            let mut chain_is_indented = false;

            for (index, operand) in operands.iter().enumerate() {
                let current_is_object_like = is_object_like_type_expression(f.context(), operand.1);
                let current_has_own_line_prefix =
                    expression_has_own_line_prefix(f.context(), operand.1);

                if index == 0 {
                    format_binary_operand_with_grouping_parentheses(
                        f,
                        BinaryOperator::ElementwiseAnd,
                        operand.1,
                    )?;
                } else if current_has_own_line_prefix {
                    write!(
                        f,
                        [indent(&format_args![
                            soft_line_break_or_space(),
                            format_with(|f: &mut DestackFormatter<'ast, '_>| {
                                format_binary_operand_with_grouping_parentheses(
                                    f,
                                    BinaryOperator::ElementwiseAnd,
                                    operand.1,
                                )
                            })
                        ])]
                    )?;
                } else if !(previous_is_object_like || current_is_object_like) {
                    write!(
                        f,
                        [indent(&format_args![
                            soft_line_break_or_space(),
                            format_with(|f: &mut DestackFormatter<'ast, '_>| {
                                format_binary_operand_with_grouping_parentheses(
                                    f,
                                    BinaryOperator::ElementwiseAnd,
                                    operand.1,
                                )
                            })
                        ])]
                    )?;
                } else {
                    write!(f, [space()])?;

                    if !previous_is_object_like || !current_is_object_like {
                        chain_is_indented = index > 1;
                    }

                    if chain_is_indented {
                        write!(
                            f,
                            [indent(&format_with(
                                |f: &mut DestackFormatter<'ast, '_>| {
                                    format_binary_operand_with_grouping_parentheses(
                                        f,
                                        BinaryOperator::ElementwiseAnd,
                                        operand.1,
                                    )
                                }
                            ))]
                        )?;
                    } else {
                        format_binary_operand_with_grouping_parentheses(
                            f,
                            BinaryOperator::ElementwiseAnd,
                            operand.1,
                        )?;
                    }
                }

                if index < last_index {
                    write!(f, [space(), token("&")])?;
                }

                previous_is_object_like = current_is_object_like;
            }

            Ok(())
        }))]
    )
}

/// Format one type-intersection binary in Destack or oxc style.
pub(crate) fn format_type_intersection_binary_layout<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    operands: &SmallVec<[(Option<BinaryOperator>, LocalNodeId<Expression>); 8]>,
    is_destack: bool,
) -> FormatResult<()> {
    if is_destack {
        if intersection_is_nested_under_type_union(f.context(), node_id) {
            return format_oxc_intersection_layout(f, operands);
        }

        return format_destack_intersection_trailing_layout(f, operands);
    }

    format_oxc_intersection_layout(f, operands)
}

/// Build the type-union layout flags derived from operands and annotations.
fn type_union_layout_flags(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    operands: &SmallVec<[(Option<BinaryOperator>, LocalNodeId<Expression>); 8]>,
    in_type_template_literal_interpolation: bool,
    has_node_annotation: bool,
    has_operand_annotations: bool,
    has_operand_prefix_comments: bool,
) -> (bool, bool, bool) {
    let root_owns_prefix_annotation = union_owns_prefix_annotations(context, node_id);
    let has_separator_comments = operands
        .iter()
        .any(|operand| type_union_operand_has_separator_comments(context, operand.1));
    let has_breaking_postfix_comments = operands.iter().enumerate().any(|(index, operand)| {
        let is_last_operand = index + 1 == operands.len();
        let has_postfix_comment = expression_has_postfix_comment(context, operand.1)
            || expression_has_line_postfix_slash_comment(context, operand.1);
        has_postfix_comment && !is_last_operand
    });
    let has_breaking_operand_prefix_comments = has_operand_prefix_comments
        && operands.iter().enumerate().skip(1).any(|(index, operand)| {
            let is_last_operand = index + 1 == operands.len();
            expression_has_leading_prefix_comment(context, operand.1) && !is_last_operand
        });
    let has_own_line_doc_prefix_annotation =
        union_has_trailing_own_line_doc_prefix_annotation(context, node_id);
    let is_template_interpolation_multiline =
        in_type_template_literal_interpolation && context.node_has_newline(node_id);
    let should_hug_layout = should_hug_type_union_operands(context, operands);
    let prefers_multiline_layout = has_breaking_operand_prefix_comments
        || has_breaking_postfix_comments
        || has_separator_comments
        || has_own_line_doc_prefix_annotation
        || is_template_interpolation_multiline;
    let is_parenthesized_cast_or_satisfies_rhs =
        union_is_parenthesized_cast_or_satisfies_rhs(context, node_id);
    let should_keep_parenthesized_cast_rhs_inline = is_parenthesized_cast_or_satisfies_rhs
        && !prefers_multiline_layout
        && !has_separator_comments
        && !has_node_annotation
        && !has_operand_annotations
        && !has_operand_prefix_comments;
    let should_inline_union = !has_separator_comments
        && (should_hug_layout
            || should_keep_parenthesized_cast_rhs_inline
            || should_inline_union_with_terminal_line_postfix_comment(
                context,
                node_id,
                operands,
                prefers_multiline_layout,
            ));

    (
        root_owns_prefix_annotation,
        prefers_multiline_layout,
        should_inline_union,
    )
}

/// Format one type union with inline-or-leading-pipe behavior.
fn format_leading_pipe_union<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    operands: &SmallVec<[(Option<BinaryOperator>, LocalNodeId<Expression>); 8]>,
    should_force_expand: bool,
) -> FormatResult<()> {
    let leading_gap_comment_nodes =
        type_union_leading_comment_nodes(f.context(), node_id, operands);
    let (leading_shell_comment_nodes, first_separator_comment_nodes) =
        split_type_union_leading_gap_comment_nodes(f.context(), &leading_gap_comment_nodes);
    let leading_prefix_annotation_ids = operands
        .first()
        .map(|operand| type_union_leading_shell_prefix_annotations(f.context(), node_id, operand.1))
        .unwrap_or_default();
    let leading_comment_info =
        LeadingCommentsInfo::from_comment_nodes(f.context(), &leading_gap_comment_nodes);
    let root_owns_prefix_annotation = union_owns_prefix_annotations(f.context(), node_id);
    let has_shell_owned_leading_comments = !leading_gap_comment_nodes.is_empty();
    let union_group_id = f.group_id("type_union");
    let should_indent_union = type_union_should_indent(f.context(), node_id)
        && !leading_comment_info.has_trailing_own_line_doc_comment
        && leading_prefix_annotation_ids.is_empty();
    let has_shell_owned_leading_seam =
        has_shell_owned_leading_comments || !leading_prefix_annotation_ids.is_empty();
    let should_break_before_leading_shell_comments = !leading_prefix_annotation_ids.is_empty()
        || (!leading_shell_comment_nodes.is_empty() && !first_separator_comment_nodes.is_empty());
    let union_body = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        for (index, operand) in operands.iter().enumerate() {
            let is_first_operand = index == 0;

            // first operand
            if is_first_operand {
                if !first_separator_comment_nodes.is_empty() {
                    write_union_leading_comment_nodes(f, &first_separator_comment_nodes)?;
                    write!(f, [space(), token("|"), space()])?;

                    write_type_union_operand_after_separator(
                        f,
                        operand.1,
                        should_indent_union,
                        has_shell_owned_leading_seam,
                    )?;
                    continue;
                }

                write!(
                    f,
                    [if_group_breaks(&format_args![
                        soft_line_break_or_space(),
                        token("|"),
                        space()
                    ])
                    .with_group_id(Some(union_group_id))]
                )?;

                write_type_union_operand_after_separator(
                    f,
                    operand.1,
                    should_indent_union,
                    has_shell_owned_leading_seam,
                )?;
                continue;
            }

            // later operands
            let previous_expression = operands[index - 1].1;
            let previous_has_postfix =
                expression_has_postfix_comment(f.context(), previous_expression);
            let previous_has_line_postfix_slash_comment =
                expression_has_line_postfix_slash_comment(f.context(), previous_expression);
            let between_separator_comment_nodes = type_union_comments_before_next_separator(
                f.context(),
                previous_expression,
                operand.1,
            );
            if !between_separator_comment_nodes.is_empty() {
                let first_comment_span = f.context().span(between_separator_comment_nodes[0]);
                if f.context().span_starts_on_own_line(first_comment_span) {
                    write!(f, [hard_line_break()])?;
                } else {
                    write!(f, [space()])?;
                }

                write_union_leading_comment_nodes(f, &between_separator_comment_nodes)?;

                if let Some(separator_span) =
                    type_union_operand_separator_token_span(f.context(), operand.1)
                {
                    let last_comment_span = f
                        .context()
                        .span(*between_separator_comment_nodes.last().unwrap());
                    let gap_span = destack_source::Span::new(
                        last_comment_span.file,
                        last_comment_span.end,
                        separator_span.start,
                    );
                    if f.context().has_newline(gap_span) {
                        write!(f, [hard_line_break()])?;
                    } else {
                        write!(f, [space()])?;
                    }
                }
            } else if previous_has_postfix || previous_has_line_postfix_slash_comment {
                write!(f, [hard_line_break()])?;
            } else {
                write!(f, [soft_line_break_or_space()])?;
            }

            write!(f, [token("|"), space()])?;
            write_type_union_operand_after_separator(f, operand.1, should_indent_union, false)?;
        }

        Ok(())
    });
    let format_union_content = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        // break after the assignment operator when the shell spans multiple comment lines
        if should_break_before_leading_shell_comments {
            write!(f, [hard_line_break()])?;
        }

        // shell comments
        if !leading_shell_comment_nodes.is_empty() {
            write_union_leading_comment_nodes(f, &leading_shell_comment_nodes)?;

            if !first_separator_comment_nodes.is_empty() {
                write!(f, [hard_line_break()])?;
            }
        }

        // shell-owned prefix annotations
        if !leading_prefix_annotation_ids.is_empty() {
            write_union_leading_prefix_annotations(f, &leading_prefix_annotation_ids)?;
        }

        // root-owned prefix comments
        if root_owns_prefix_annotation
            && leading_shell_comment_nodes.is_empty()
            && first_separator_comment_nodes.is_empty()
            && leading_prefix_annotation_ids.is_empty()
        {
            write!(
                f,
                [crate::format::annotation::prefix_annotations(
                    f.context(),
                    node_id
                )]
            )?;
        }

        write!(f, [union_body])
    });
    let format_union_group = group(&format_union_content)
        .with_id(Some(union_group_id))
        .should_expand(should_force_expand);

    if should_indent_union {
        write!(f, [indent(&format_union_group)])
    } else {
        write!(f, [format_union_group])
    }
}

/// Return the `|` token that structurally owns one type-union operand.
fn type_union_operand_separator_token_span(
    context: &DestackFormatContext<'_>,
    operand_expression_id: LocalNodeId<Expression>,
) -> Option<destack_source::Span> {
    let operand_span = context.span(operand_expression_id);
    let mut token_index = context
        .tokens
        .partition_point(|token| token.span.end <= operand_span.start);

    while token_index > 0 {
        token_index -= 1;
        let token = context.tokens[token_index];

        match token.token.ty {
            TokenType::Whitespace
            | TokenType::Newline
            | TokenType::LineComment
            | TokenType::BlockComment
            | TokenType::DocLineComment
            | TokenType::DocBlockComment => continue,
            TokenType::ElementwiseOr => return Some(token.span),
            _ => return None,
        }
    }

    None
}

/// Collect comments between `|` and one type-union operand.
fn type_union_operand_separator_comments(
    context: &DestackFormatContext<'_>,
    operand_expression_id: LocalNodeId<Expression>,
) -> Vec<LocalNodeId<Comment>> {
    let Some(separator_span) =
        type_union_operand_separator_token_span(context, operand_expression_id)
    else {
        return Vec::new();
    };

    let operand_span = context.span(operand_expression_id);
    if separator_span.file != operand_span.file || separator_span.end >= operand_span.start {
        return Vec::new();
    }

    let gap_span =
        destack_source::Span::new(separator_span.file, separator_span.end, operand_span.start);
    if context
        .comments_in_range(gap_span.start, gap_span.end)
        .is_empty()
    {
        return Vec::new();
    }

    let comment_trivia = context.tree.comment_trivia();
    let first_relevant_index =
        comment_trivia.partition_point(|comment_trivia| comment_trivia.span.end < gap_span.start);

    let mut comments: Vec<(u32, LocalNodeId<Comment>)> = Vec::new();
    for comment_trivia in comment_trivia[first_relevant_index..].iter().copied() {
        if comment_trivia.span.file != gap_span.file || comment_trivia.span.start >= gap_span.end {
            break;
        }
        if comment_trivia.span.start < gap_span.start || comment_trivia.span.end > gap_span.end {
            continue;
        }

        comments.push((comment_trivia.span.start, comment_trivia.comment));
    }

    comments.sort_by_key(|(start, _)| *start);
    comments
        .into_iter()
        .map(|(_, comment_id)| comment_id)
        .collect()
}

/// Collect raw comments after one operand and before the next `|`.
fn type_union_comments_before_next_separator(
    context: &DestackFormatContext<'_>,
    previous_operand_expression_id: LocalNodeId<Expression>,
    next_operand_expression_id: LocalNodeId<Expression>,
) -> Vec<LocalNodeId<Comment>> {
    let Some(separator_span) =
        type_union_operand_separator_token_span(context, next_operand_expression_id)
    else {
        return Vec::new();
    };

    let previous_operand_span = context.span(previous_operand_expression_id);
    if previous_operand_span.file != separator_span.file
        || previous_operand_span.end >= separator_span.start
    {
        return Vec::new();
    }

    context.comment_nodes_in_range(previous_operand_span.end, separator_span.start)
}

/// Return whether `|` owns a source comment cluster before one operand.
fn type_union_operand_has_separator_comments(
    context: &DestackFormatContext<'_>,
    operand_expression_id: LocalNodeId<Expression>,
) -> bool {
    !type_union_operand_separator_comments(context, operand_expression_id).is_empty()
}

/// Format one type-union binary with inline or leading-pipe layout.
pub(crate) fn format_type_union_binary_layout<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    operands: &SmallVec<[(Option<BinaryOperator>, LocalNodeId<Expression>); 8]>,
    in_type_template_literal_interpolation: bool,
    has_node_annotation: bool,
    has_operand_annotations: bool,
    has_operand_prefix_comments: bool,
) -> FormatResult<()> {
    let (root_owns_prefix_annotation, prefers_multiline_layout, should_inline_union) =
        type_union_layout_flags(
            f.context(),
            node_id,
            operands,
            in_type_template_literal_interpolation,
            has_node_annotation,
            has_operand_annotations,
            has_operand_prefix_comments,
        );

    // inline vs leading-pipe shell
    if should_inline_union {
        if root_owns_prefix_annotation {
            write!(
                f,
                [crate::format::annotation::prefix_annotations(
                    f.context(),
                    node_id
                )]
            )?;
        }
        format_inline_type_union_layout(f, operands)?;
    } else {
        format_leading_pipe_union(f, node_id, operands, prefers_multiline_layout)?;
    }

    Ok(())
}
