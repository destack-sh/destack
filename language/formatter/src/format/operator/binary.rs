use crate::Annotation;
use crate::format::analysis::{
    first_non_trivia_token_in_span, last_non_trivia_token_in_span, timing,
};
use crate::format::call::argument_satisfies_static_seam_comment_annotation_id;
use crate::format::chain::{
    BinaryOperands, flatten_binary_expression, flatten_type_binary_expression, is_chain_root,
    is_expression_chain, should_use_trailing_coalesce,
};
use crate::format::expression::{
    Argument, BinaryOperator, DestackFormatContext, DestackFormatter, Expression, FormatResult,
    LocalNodeId, ParenthesizedDropMode, TokenType, TypeBinaryOperator,
    expression_has_leading_prefix_comment, format_with, group, hard_line_break, if_group_breaks,
    indent, should_drop_parenthesized, soft_line_break_or_space, space, token,
    transparent_inner_expression, type_binary_is_parenthesized_new_callee,
    type_binary_is_parenthesized_statement_expression, type_binary_is_statement_expression,
};
use crate::format::operator::{
    AnnotationPosition, NodeType, expression_is_trivial_inline_without_annotations,
    format_binary_operand_with_grouping_parentheses, has_comment_between_expressions,
    is_object_like_type_expression, is_type_context,
    type_binary_operand_needs_grouping_parentheses,
};
use destack_ast::{Comment, CommentStyle, Declaration, TypeLiteral};
use destack_fir::format::Buffer;
use destack_fir::{format_args, write};

/// Return whether type-binary operands are structurally complex enough to prefer multiline layout.
pub(crate) fn type_binary_operands_are_structurally_complex(
    context: &DestackFormatContext<'_>,
    operands: &BinaryOperands,
) -> bool {
    if operands.len() > 3 {
        return true;
    }

    operands.iter().any(|operand| {
        if expression_is_bodyless_function_signature_declaration(context, operand.expression) {
            return false;
        }

        !expression_is_trivial_inline_without_annotations(context, operand.expression)
    })
}

/// Return whether one union operand is object-like for hug layout.
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

/// Return whether one union operand is void-like for hug layout.
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

/// Return whether one type union should use object-and-void hug layout.
///
/// This mirrors Prettier and oxc behavior for unions such as
/// `Map<...> | undefined` by keeping them in the inline `A | B` form.
fn should_hug_type_union_operands(
    context: &DestackFormatContext<'_>,
    operands: &BinaryOperands,
) -> bool {
    if operands.len() <= 1 {
        return true;
    }

    if operands
        .iter()
        .any(|operand| context.has_non_blank_annotation(operand.expression))
    {
        return false;
    }

    let object_operand_expression_id = operands.iter().find_map(|operand| {
        expression_is_hug_object_like_union_operand(context, operand.expression)
            .then_some(transparent_inner_expression(context, operand.expression))
    });
    let Some(object_operand_expression_id) = object_operand_expression_id else {
        return false;
    };

    operands.iter().all(|operand| {
        let expression_id = transparent_inner_expression(context, operand.expression);
        expression_id == object_operand_expression_id
            || expression_is_hug_void_like_union_operand(context, expression_id)
    })
}

/// Return whether one expression is one body-less function signature declaration.
fn expression_is_bodyless_function_signature_declaration(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let expression_id = transparent_inner_expression(context, expression_id);
    let Expression::Declaration(declaration_id) = context.tree.get(expression_id) else {
        return false;
    };
    let Declaration::Function { body, .. } = context.tree.get(*declaration_id) else {
        return false;
    };

    body.is_none()
}

/// Return whether one expression appears inside a type template literal interpolation.
fn is_in_type_template_literal_interpolation(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let mut current_id = node_id.id;

    while let Some((parent_id, parent_type)) = context.parent_by_id(current_id) {
        if parent_type == NodeType::Expression
            && matches!(
                context.tree.get(LocalNodeId::<Expression>::new(parent_id)),
                Expression::TypeTemplateLiteral { .. }
            )
        {
            return true;
        }

        current_id = parent_id;
    }

    false
}

/// Return whether one union should preserve inline layout for terminal line-postfix comments.
fn should_inline_union_with_terminal_line_postfix_comment(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    operands: &BinaryOperands,
    union_prefers_multiline_layout: bool,
) -> bool {
    if union_prefers_multiline_layout {
        return false;
    }

    if expression_has_leading_prefix_comment(context, node_id) {
        return false;
    }

    if expression_has_line_postfix_comment_annotation(context, node_id) {
        return true;
    }

    if declaration_expression_ancestor_has_line_postfix_comment_annotation(context, node_id) {
        return true;
    }

    let Some(last_operand) = operands.last() else {
        return false;
    };
    if !expression_has_line_postfix_slash_comment(context, last_operand.expression) {
        return false;
    }

    let has_non_last_comments = operands
        .iter()
        .take(operands.len().saturating_sub(1))
        .any(|operand| expression_has_postfix_comment_annotation(context, operand.expression));
    if has_non_last_comments {
        return false;
    }

    true
}

/// Return whether one declaration-expression ancestor has a line-postfix comment annotation.
fn declaration_expression_ancestor_has_line_postfix_comment_annotation(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let mut current_node_id = node_id.id;
    while let Some((parent_id, parent_type)) = context.parent_by_id(current_node_id) {
        if parent_type == NodeType::Expression {
            let parent_expression_id = LocalNodeId::<Expression>::new(parent_id);
            let parent_is_declaration = matches!(
                context.tree.get(parent_expression_id),
                Expression::Declaration(_)
            );
            if parent_is_declaration
                && expression_has_line_postfix_comment_annotation(context, parent_expression_id)
            {
                return true;
            }
        }

        current_node_id = parent_id;
    }

    false
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
            } if is_type_context(context, current_id)
                || is_in_type_template_literal_interpolation(context, current_id) =>
            {
                *left
            }
            _ => return current_id,
        };
    }
}

/// Return whether one type-union expression should render prefix annotations in its own layout.
pub(crate) fn union_owns_prefix_annotations(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    if !context.has_prefix_annotation(node_id) {
        return false;
    }

    let has_prefix_comment_or_doc_annotation = context
        .visit_annotations(node_id, |annotations| {
            annotations.iter().copied().any(|annotation_id| {
                let annotation = context.annotation(annotation_id);
                matches!(
                    annotation,
                    Annotation::Comment { .. } | Annotation::Doc { .. }
                ) && matches!(
                    annotation.position(),
                    AnnotationPosition::LinePrefix | AnnotationPosition::BlockPrefix
                )
            })
        })
        .unwrap_or(false);
    if !has_prefix_comment_or_doc_annotation {
        return false;
    }

    let leftmost_terminal_expression = leftmost_union_terminal_expression(context, node_id);
    !context.has_prefix_annotation(leftmost_terminal_expression)
}

/// Return whether one binary operator is logical.
#[inline]
pub(crate) fn is_logical_binary_operator(operator: BinaryOperator) -> bool {
    matches!(
        operator,
        BinaryOperator::And | BinaryOperator::Or | BinaryOperator::Coalesce
    )
}

/// Return whether an expression ends with a `//` postfix annotation.
pub(crate) fn expression_has_line_postfix_slash_comment(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let Some(annotations) = context.annotations(expression_id) else {
        return false;
    };

    annotations.into_iter().any(|annotation_id| {
        let annotation = context.annotation(annotation_id);
        let Annotation::Comment { node, position } = annotation else {
            return false;
        };

        if !matches!(
            position,
            AnnotationPosition::LinePostfix | AnnotationPosition::LinePostfixBoundary
        ) {
            return false;
        }

        let comment = context.tree.get::<Comment>(node);
        comment.style == CommentStyle::Slash
    })
}

/// Return whether an expression has one line-postfix comment annotation.
fn expression_has_line_postfix_comment_annotation(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let Some(annotations) = context.annotations(expression_id) else {
        return false;
    };

    annotations.into_iter().any(|annotation_id| {
        let annotation = context.annotation(annotation_id);
        matches!(
            annotation,
            Annotation::Comment { .. } | Annotation::Doc { .. }
        ) && matches!(
            annotation.position(),
            AnnotationPosition::LinePostfix | AnnotationPosition::LinePostfixBoundary
        )
    })
}

/// Return whether an expression has one postfix comment annotation.
fn expression_has_postfix_comment_annotation(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let Some(annotations) = context.annotations(expression_id) else {
        return false;
    };

    annotations.into_iter().any(|annotation_id| {
        let annotation = context.annotation(annotation_id);
        matches!(
            annotation,
            Annotation::Comment { .. } | Annotation::Doc { .. }
        ) && matches!(
            annotation.position(),
            AnnotationPosition::BlockPostfix
                | AnnotationPosition::LinePostfix
                | AnnotationPosition::LinePostfixBoundary
        )
    })
}

/// Return whether an expression starts with a `//` line-prefix annotation.
pub(crate) fn expression_has_line_prefix_slash_comment(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let Some(annotations) = context.annotations(expression_id) else {
        return false;
    };

    annotations.into_iter().any(|annotation_id| {
        let annotation = context.annotation(annotation_id);
        let Annotation::Comment { node, position } = annotation else {
            return false;
        };
        if position != AnnotationPosition::LinePrefix {
            return false;
        }

        let comment = context.tree.get::<Comment>(node);
        comment.style == CommentStyle::Slash
    })
}

/// Return whether an expression starts with one own-line prefix annotation.
fn expression_has_own_line_prefix_annotation(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let Some(annotations) = context.annotations(expression_id) else {
        return false;
    };

    annotations.into_iter().any(|annotation_id| {
        let annotation = context.annotation(annotation_id);
        if !matches!(
            annotation.position(),
            AnnotationPosition::LinePrefix | AnnotationPosition::BlockPrefix
        ) {
            return false;
        }

        context.annotation_starts_on_own_line(annotation_id)
    })
}

/// Return whether an expression starts with an inline `/* ... */` prefix annotation.
pub(crate) fn expression_has_inline_block_prefix_star_comment(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let Some(annotations) = context.annotations(expression_id) else {
        return false;
    };

    annotations.into_iter().any(|annotation_id| {
        let annotation = context.annotation(annotation_id);
        let Annotation::Comment { node, position } = annotation else {
            return false;
        };
        if !matches!(
            position,
            AnnotationPosition::BlockPrefix | AnnotationPosition::LinePrefix
        ) {
            return false;
        }

        let comment = context.tree.get::<Comment>(node);
        if comment.style != CommentStyle::Star {
            return false;
        }

        let annotation_span = context.annotation_span(annotation_id);
        !context.has_newline(annotation_span)
    })
}

/// Return whether an expression ends with one inline postfix-boundary `/* ... */` annotation that
/// needs binary-formatter owned spacing before the operator token.
pub(crate) fn expression_has_inline_block_postfix_boundary_star_comment(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let Some(annotations) = context.annotations(expression_id) else {
        return false;
    };

    annotations.into_iter().any(|annotation_id| {
        let annotation = context.annotation(annotation_id);
        let Annotation::Comment { node, position } = annotation else {
            return false;
        };
        if !matches!(
            position,
            AnnotationPosition::LinePostfixBoundary | AnnotationPosition::BlockPostfix
        ) {
            return false;
        }

        let comment = context.tree.get::<Comment>(node);
        if comment.style != CommentStyle::Star {
            return false;
        }

        let annotation_span = context.annotation_span(annotation_id);
        !context.has_newline(annotation_span)
    })
}

/// Return whether one type-binary node is a static type argument under a remap path seam.
fn type_binary_is_static_argument_under_remap_path(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let Some((argument_id, argument_parent_type)) = context.parent(expression_id) else {
        return false;
    };
    if argument_parent_type != NodeType::Argument {
        return false;
    }

    let argument_id = LocalNodeId::<Argument>::new(argument_id);
    let Some((path_owner_id, path_owner_type)) = context.parent(argument_id) else {
        return false;
    };
    if path_owner_type != NodeType::Expression {
        return false;
    }

    let path_owner_id = LocalNodeId::<Expression>::new(path_owner_id);
    if !matches!(context.tree.get(path_owner_id), Expression::Path { .. }) {
        return false;
    }

    let Some(annotation_ids) = context.annotations(path_owner_id) else {
        return false;
    };
    annotation_ids.iter().copied().any(|annotation_id| {
        let annotation = context.annotation(annotation_id);
        let Annotation::Comment { node, position } = annotation else {
            return false;
        };
        if !matches!(
            position,
            AnnotationPosition::LinePostfix | AnnotationPosition::LinePostfixBoundary
        ) {
            return false;
        }

        let comment = context.tree.get::<Comment>(node);
        comment.style == CommentStyle::Slash
    })
}

/// Return whether mixed logical precedence should parenthesize the right expression.
#[inline]
pub(crate) fn is_mixed_logical_precedence_pair(
    left_operator: BinaryOperator,
    right_operator: BinaryOperator,
) -> bool {
    matches!(left_operator, BinaryOperator::Or | BinaryOperator::Coalesce)
        && left_operator != right_operator
        && matches!(
            right_operator,
            BinaryOperator::And | BinaryOperator::Coalesce
        )
}

/// Write one separating space after the left operand when no postfix trivia exists.
pub(crate) fn write_space_after_binary_left_if_needed<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    left: LocalNodeId<Expression>,
    operator: BinaryOperator,
) -> FormatResult<()> {
    let allow_inline_block_postfix_star_space =
        expression_has_inline_block_postfix_boundary_star_comment(f.context(), left);
    let allow_logical_space_after_line_comment = is_logical_binary_operator(operator)
        && expression_has_line_postfix_slash_comment(f.context(), left);
    if f.context().has_postfix_annotation(left)
        && !allow_logical_space_after_line_comment
        && !allow_inline_block_postfix_star_space
    {
        return Ok(());
    }

    write!(f, [space()])
}

/// Try formatting `??` using trailing-operator layout.
pub(crate) fn try_format_trailing_coalesce<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    left: LocalNodeId<Expression>,
    operator: BinaryOperator,
    right: LocalNodeId<Expression>,
) -> FormatResult<bool> {
    if operator != BinaryOperator::Coalesce
        || !should_use_trailing_coalesce(f.context(), node_id, left)
    {
        return Ok(false);
    }

    write!(
        f,
        [group(&format_args![
            left,
            indent(&format_with(|f| {
                write_space_after_binary_left_if_needed(f, left, operator)?;
                write!(
                    f,
                    [
                        operator,
                        indent(&format_args![soft_line_break_or_space(), right])
                    ]
                )
            }))
        ])]
    )?;

    Ok(true)
}

/// Try formatting logical operators with a right-side line-prefix comment seam.
pub(crate) fn try_format_logical_right_prefix_line_comment<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    left: LocalNodeId<Expression>,
    operator: BinaryOperator,
    right: LocalNodeId<Expression>,
) -> FormatResult<bool> {
    if !is_logical_binary_operator(operator) {
        return Ok(false);
    }
    if !expression_has_line_prefix_slash_comment(f.context(), right) {
        return Ok(false);
    }

    write!(
        f,
        [group(&format_args![
            left,
            format_with(|f| write_space_after_binary_left_if_needed(f, left, operator)),
            operator,
            space(),
            indent(&format_args![right])
        ])]
    )?;

    Ok(true)
}

/// Try formatting logical operators with an inline right-side block-prefix comment seam.
pub(crate) fn try_format_logical_right_prefix_block_comment<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    left: LocalNodeId<Expression>,
    operator: BinaryOperator,
    right: LocalNodeId<Expression>,
) -> FormatResult<bool> {
    if !is_logical_binary_operator(operator) {
        return Ok(false);
    }
    if !expression_has_inline_block_prefix_star_comment(f.context(), right) {
        return Ok(false);
    }
    if flatten_binary_expression(f.context(), node_id, operator).len() > 2 {
        return Ok(false);
    }

    write!(
        f,
        [group(&format_args![
            left,
            format_with(|f| write_space_after_binary_left_if_needed(f, left, operator)),
            operator,
            indent(&format_args![soft_line_break_or_space(), right])
        ])]
    )?;

    Ok(true)
}

/// Try formatting mixed logical precedence pairs with explicit right parentheses.
pub(crate) fn try_format_mixed_logical_precedence<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    left: LocalNodeId<Expression>,
    operator: BinaryOperator,
    right: LocalNodeId<Expression>,
) -> FormatResult<bool> {
    let Expression::Binary {
        operator: right_operator,
        ..
    } = f.context().tree.get(right)
    else {
        return Ok(false);
    };
    if !is_mixed_logical_precedence_pair(operator, *right_operator) {
        return Ok(false);
    }

    let right_span = f.context().span(right);
    let should_preserve_grouping_for_comments = f.context().has_comment(right_span)
        || has_comment_between_expressions(f.context(), left, right);
    if !should_preserve_grouping_for_comments {
        return Ok(false);
    }

    write!(
        f,
        [group(&format_args![
            left,
            format_with(|f| write_space_after_binary_left_if_needed(f, left, operator)),
            operator,
            space(),
            token("("),
            right,
            token(")")
        ])]
    )?;

    Ok(true)
}

/// Try formatting logical expressions with parenthesized-tail policies.
pub(crate) fn try_format_logical_parenthesized_cases<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    _node_id: LocalNodeId<Expression>,
    left: LocalNodeId<Expression>,
    operator: BinaryOperator,
    right: LocalNodeId<Expression>,
) -> FormatResult<bool> {
    if !is_logical_binary_operator(operator) {
        return Ok(false);
    }

    let left_span = f.context().span(left);
    let left_has_multiline_parenthesized_tail = f.context().has_newline(left_span)
        && last_non_trivia_token_in_span(f.context(), left_span)
            .is_some_and(|token| token.token.ty == TokenType::CloseParenthesis);
    let right_is_inline_trivial =
        expression_is_trivial_inline_without_annotations(f.context(), right);
    let right_has_prefix = f.context().has_prefix_annotation(right);
    let right_inner_expression = transparent_inner_expression(f.context(), right);
    let right_is_tree_expression = matches!(
        f.context().tree.get(right_inner_expression),
        Expression::TreeExpression { .. }
    );

    // parenthesized multiline left tail with short `&&` right side
    if left_has_multiline_parenthesized_tail
        && right_is_inline_trivial
        && !right_has_prefix
        && operator == BinaryOperator::And
    {
        write!(
            f,
            [group(&format_args![
                left,
                format_with(|f| write_space_after_binary_left_if_needed(f, left, operator)),
                operator,
                indent(&format_args![hard_line_break(), right])
            ])]
        )?;
        return Ok(true);
    }

    // prefix-commented left parentheses keep trailing logical operators
    let left_prefers_trailing_operator = matches!(
        f.context().tree.get(left),
        Expression::Parenthesized { expression }
            if f.context().has_prefix_annotation(left)
                || f.context().has_prefix_annotation(*expression)
    );
    if left_prefers_trailing_operator && right_is_inline_trivial {
        write!(
            f,
            [group(&format_args![
                left,
                format_with(|f| write_space_after_binary_left_if_needed(f, left, operator)),
                operator,
                space(),
                right
            ])]
        )?;
        return Ok(true);
    }

    // keep `&& (` and `|| (` attached for grouped and jsx-like right branches
    if right_is_tree_expression && !right_has_prefix {
        write!(
            f,
            [group(&format_args![
                left,
                format_with(|f| write_space_after_binary_left_if_needed(f, left, operator)),
                operator,
                space(),
                right
            ])]
        )?;
        return Ok(true);
    }

    Ok(false)
}

/// Return whether one union root ends with an own-line doc prefix annotation.
fn union_has_trailing_own_line_doc_prefix_annotation(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let Some(annotation_ids) = context.annotations(node_id) else {
        return false;
    };

    annotation_ids
        .into_iter()
        .rev()
        .find(|annotation_id| {
            matches!(
                context.annotation(*annotation_id).position(),
                AnnotationPosition::LinePrefix | AnnotationPosition::BlockPrefix
            )
        })
        .is_some_and(|annotation_id| {
            matches!(context.annotation(annotation_id), Annotation::Doc { .. })
                && context.annotation_starts_on_own_line(annotation_id)
                && !context.annotation_next_token_is_on_same_line(annotation_id)
        })
}

/// Return whether one expression has an own-line leading prefix non-doc comment annotation.
fn expression_has_own_line_leading_prefix_comment(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    context
        .visit_annotations(expression_id, |annotation_ids| {
            annotation_ids.iter().copied().any(|annotation_id| {
                matches!(
                    context.annotation(annotation_id).position(),
                    AnnotationPosition::LinePrefix | AnnotationPosition::BlockPrefix
                ) && context.annotation_starts_on_own_line(annotation_id)
                    && matches!(
                        context.annotation(annotation_id),
                        Annotation::Comment { .. }
                    )
            })
        })
        .unwrap_or(false)
}

/// Return whether one union node has a block-prefix non-doc comment annotation.
fn union_has_block_prefix_comment(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    context
        .visit_annotations(node_id, |annotation_ids| {
            annotation_ids.iter().copied().any(|annotation_id| {
                context.annotation(annotation_id).position() == AnnotationPosition::BlockPrefix
                    && matches!(
                        context.annotation(annotation_id),
                        Annotation::Comment { .. }
                    )
            })
        })
        .unwrap_or(false)
}

/// Return whether one union expression is wrapped by transparent parenthesized ancestors.
fn union_has_parenthesized_wrapper(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let mut current_expression_id = node_id;
    while let Some((parent_id, parent_type)) = context.parent(current_expression_id) {
        if parent_type != NodeType::Expression {
            return false;
        }

        let parent_expression_id = LocalNodeId::<Expression>::new(parent_id);
        match context.tree.get(parent_expression_id) {
            Expression::Parenthesized { expression } if *expression == current_expression_id => {
                return true;
            }
            Expression::Statement(inner_expression_id)
                if *inner_expression_id == current_expression_id =>
            {
                current_expression_id = parent_expression_id;
            }
            _ => return false,
        }
    }

    false
}

/// Return whether one type union should apply its own indentation.
fn type_union_should_indent(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    root_owns_prefix_annotation: bool,
) -> bool {
    if is_in_type_template_literal_interpolation(context, node_id) {
        return true;
    }

    let is_type_declaration_value = union_is_type_declaration_value(context, node_id);
    if !is_type_declaration_value {
        return false;
    }

    if union_has_parenthesized_wrapper(context, node_id) && !context.has_annotation(node_id) {
        return true;
    }

    if root_owns_prefix_annotation
        && union_has_trailing_own_line_doc_prefix_annotation(context, node_id)
    {
        return false;
    }

    true
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
            Expression::Parenthesized { expression } if *expression == current_expression_id => {
                current_expression_id = parent_expression_id;
            }
            Expression::Statement(expression) if *expression == current_expression_id => {
                current_expression_id = parent_expression_id;
            }
            _ => return false,
        }
    }

    false
}

/// Return whether an intersection expression is directly nested under one type-union owner.
fn intersection_is_nested_under_type_union(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let mut current_expression_id = node_id;
    while let Some((parent_id, parent_type)) = context.parent(current_expression_id) {
        if parent_type != NodeType::Expression {
            return false;
        }

        let parent_expression_id = LocalNodeId::<Expression>::new(parent_id);
        match context.tree.get(parent_expression_id) {
            Expression::Parenthesized { expression } if *expression == current_expression_id => {
                current_expression_id = parent_expression_id;
            }
            Expression::Statement(inner_expression_id)
                if *inner_expression_id == current_expression_id =>
            {
                current_expression_id = parent_expression_id;
            }
            Expression::Binary {
                operator: BinaryOperator::ElementwiseOr,
                ..
            } => return is_type_context(context, parent_expression_id),
            _ => return false,
        }
    }

    false
}

/// Format one type union with inline-or-leading-pipe behavior.
pub(crate) fn format_leading_pipe_union<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    operands: &BinaryOperands,
    should_force_expand: bool,
) -> FormatResult<()> {
    let root_owns_prefix_annotation = union_owns_prefix_annotations(f.context(), node_id);
    let union_group_id = f.group_id("type_union");
    let should_indent_union =
        type_union_should_indent(f.context(), node_id, root_owns_prefix_annotation);
    let root_has_block_prefix_comment =
        root_owns_prefix_annotation && union_has_block_prefix_comment(f.context(), node_id);
    let first_operand_has_own_line_prefix_comment = operands.first().is_some_and(|operand| {
        expression_has_own_line_leading_prefix_comment(f.context(), operand.expression)
    });
    let union_body = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        for (index, operand) in operands.iter().enumerate() {
            let is_first_operand = index == 0;

            // first operand: print a leading `|` only in broken groups
            if is_first_operand {
                if first_operand_has_own_line_prefix_comment || root_has_block_prefix_comment {
                    write!(
                        f,
                        [if_group_breaks(&format_args![token("|"), space()])
                            .with_group_id(Some(union_group_id))]
                    )?;
                } else {
                    write!(
                        f,
                        [if_group_breaks(&format_args![
                            soft_line_break_or_space(),
                            token("|"),
                            space()
                        ])
                        .with_group_id(Some(union_group_id))]
                    )?;
                }

                format_binary_operand_with_grouping_parentheses(
                    f,
                    BinaryOperator::ElementwiseOr,
                    operand.expression,
                )?;

                continue;
            }
            // later operands: preserve hard breaks after postfix comments
            else {
                let previous_expression = operands[index - 1].expression;
                let previous_has_postfix =
                    expression_has_postfix_comment_annotation(f.context(), previous_expression);
                let previous_has_line_postfix_slash_comment =
                    expression_has_line_postfix_slash_comment(f.context(), previous_expression);
                if previous_has_postfix || previous_has_line_postfix_slash_comment {
                    write!(f, [hard_line_break()])?;
                } else {
                    write!(f, [soft_line_break_or_space()])?;
                }

                write!(f, [token("|"), space()])?;
            }

            // object-like arms keep one nested indent for readability
            // operand rendering is handled by expression format rules
            format_binary_operand_with_grouping_parentheses(
                f,
                BinaryOperator::ElementwiseOr,
                operand.expression,
            )?;
        }

        Ok(())
    });
    let format_union_body = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        if should_indent_union {
            write!(
                f,
                [group(&indent(&union_body))
                    .with_id(Some(union_group_id))
                    .should_expand(should_force_expand)]
            )
        } else {
            write!(
                f,
                [group(&union_body)
                    .with_id(Some(union_group_id))
                    .should_expand(should_force_expand)]
            )
        }
    });

    if root_owns_prefix_annotation {
        if should_indent_union {
            write!(f, [indent(&f.context().any_prefix_annotations(node_id))])?;
        } else {
            write!(f, [f.context().any_prefix_annotations(node_id)])?;
        }

        write!(f, [format_union_body])
    } else {
        write!(f, [format_union_body])
    }
}

/// Write a cast or satisfies operator and right operand.
fn write_type_binary_operator_and_right<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    operator: &TypeBinaryOperator,
    right: LocalNodeId<Expression>,
) -> FormatResult<()> {
    write!(f, [operator, space()])?;
    write!(f, [right])
}

/// Return whether this cast expression should keep TypeScript angle assertion syntax.
fn cast_prefers_angle_assertion_syntax(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    if context.options.language_type.supports_jsx() {
        return false;
    }

    let Some(main_span) = context.tree.get_main_span(node_id) else {
        return false;
    };

    first_non_trivia_token_in_span(context, main_span)
        .is_some_and(|token| token.token.ty == TokenType::LessThan)
}

/// Return one satisfies seam line comment node from rhs ownership variants.
fn satisfies_seam_comment_node_id(
    context: &DestackFormatContext<'_>,
    right_expression_id: LocalNodeId<Expression>,
    static_arguments: &[LocalNodeId<Argument>],
) -> Option<LocalNodeId<Comment>> {
    if let Some(annotation_ids) = context.annotations(right_expression_id) {
        for annotation_id in annotation_ids {
            let Annotation::Comment {
                node,
                position: AnnotationPosition::LinePrefix | AnnotationPosition::BlockPrefix,
            } = context.annotation(annotation_id)
            else {
                continue;
            };

            let comment = context.tree.get::<Comment>(node);
            if comment.style != CommentStyle::Slash {
                continue;
            }

            return Some(node);
        }
    }

    static_arguments.first().and_then(|argument_id| {
        let annotation_id =
            argument_satisfies_static_seam_comment_annotation_id(context, *argument_id)?;
        let Annotation::Comment { node, .. } = context.annotation(annotation_id) else {
            return None;
        };
        let comment = context.tree.get::<Comment>(node);
        if comment.style != CommentStyle::Slash {
            return None;
        }
        Some(node)
    })
}

/// Try to write one trivial object literal inline for satisfies seam comment layout.
fn try_write_inline_object_left_for_satisfies_seam_comment<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    expression_id: LocalNodeId<Expression>,
) -> FormatResult<bool> {
    let Expression::ObjectExpression { properties, .. } = f.context().tree.get(expression_id)
    else {
        return Ok(false);
    };
    if properties.len() != 1 {
        return Ok(false);
    }
    if f.context().has_annotation(expression_id) || f.context().node_has_newline(expression_id) {
        return Ok(false);
    }

    let property_id = properties[0];
    if f.context().has_annotation(property_id)
        || f.context().node_has_newline(property_id)
        || f.context().has_comment(f.context().span(property_id))
    {
        return Ok(false);
    }

    write!(f, [token("{"), space(), property_id, space(), token("}")])?;
    Ok(true)
}

/// Format a type-binary expression with chain-aware left-hand expansion.
pub(crate) fn format_type_binary_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    left: LocalNodeId<Expression>,
    operator: &TypeBinaryOperator,
    right: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let cast_uses_angle_assertion = *operator == TypeBinaryOperator::Cast
        && cast_prefers_angle_assertion_syntax(f.context(), node_id);

    let mut formatted_left = left;
    if !cast_uses_angle_assertion
        && matches!(
            operator,
            TypeBinaryOperator::Cast | TypeBinaryOperator::Satisfies
        )
        && let Expression::Parenthesized { expression } = f.context().tree.get(left)
        && should_drop_parenthesized(
            f.context(),
            left,
            *expression,
            ParenthesizedDropMode::TypeBinaryLeft { node_id },
        )
    {
        formatted_left = *expression;
    }

    // statement-level satisfies/cast over object literals should keep `({ ... })` lhs wrapping
    let left_needs_statement_object_parentheses =
        matches!(
            operator,
            TypeBinaryOperator::Cast | TypeBinaryOperator::Satisfies
        ) && matches!(
            f.context().tree.get(formatted_left),
            Expression::ObjectExpression { .. }
        ) && (type_binary_is_statement_expression(f.context(), node_id)
            || type_binary_is_parenthesized_statement_expression(f.context(), node_id));

    let format_left = |f: &mut DestackFormatter<'ast, '_>| -> FormatResult<()> {
        if left_needs_statement_object_parentheses {
            write!(f, [token("("), formatted_left, token(")")])
        } else {
            write!(f, [formatted_left])
        }
    };

    if cast_uses_angle_assertion {
        write!(f, [token("<"), right, token(">"), format_with(format_left)])?;
        return Ok(());
    }

    let has_postfix = f.context().has_postfix_annotation(formatted_left);
    let left_has_leading_prefix_comment =
        expression_has_leading_prefix_comment(f.context(), formatted_left);
    let left_is_chain_expression = is_expression_chain(f.context().tree, formatted_left)
        || is_chain_root(f.context().tree, formatted_left);
    let is_parenthesized_new_callee = type_binary_is_parenthesized_new_callee(f.context(), node_id);

    // satisfies separator seam comments before multi-argument static type lists:
    // `... satisfies // note\nRecord<A, B>` -> `... satisfies Record< // note\n    A,\n    B\n>`
    if *operator == TypeBinaryOperator::Satisfies
        && let Expression::Path {
            path,
            static_arguments: Some(static_arguments),
        } = f.context().tree.get(right)
        && static_arguments.len() > 1
        && path.segments.len() == 1
        && let Some(seam_comment_id) =
            satisfies_seam_comment_node_id(f.context(), right, static_arguments)
    {
        let wrote_inline_object_left =
            try_write_inline_object_left_for_satisfies_seam_comment(f, formatted_left)?;
        if !wrote_inline_object_left {
            write!(f, [group(&format_with(format_left))])?;
        }
        if !has_postfix {
            write!(f, [space()])?;
        }
        write!(
            f,
            [
                operator,
                space(),
                path.segments[0],
                token("<"),
                space(),
                seam_comment_id
            ]
        )?;
        write!(
            f,
            [indent(&format_with(|f| {
                write!(f, [hard_line_break()])?;
                for (index, argument_id) in static_arguments.iter().enumerate() {
                    write!(f, [*argument_id])?;
                    if index + 1 < static_arguments.len() {
                        write!(f, [token(","), hard_line_break()])?;
                    }
                }
                Ok(())
            }))]
        )?;
        write!(f, [hard_line_break(), token(">")])?;
        return Ok(());
    }

    let should_expand_chain_left = matches!(
        operator,
        TypeBinaryOperator::Cast | TypeBinaryOperator::Satisfies
    ) && left_is_chain_expression
        && is_parenthesized_new_callee;

    if should_expand_chain_left {
        write!(
            f,
            [group(&format_with(|f: &mut DestackFormatter<'ast, '_>| {
                write!(
                    f,
                    [group(&format_with(|f: &mut DestackFormatter<'ast, '_>| {
                        format_left(f)
                    }))
                    .should_expand(true)]
                )?;
                if !has_postfix {
                    write!(f, [space()])?;
                }
                write_type_binary_operator_and_right(f, operator, right)
            }))]
        )?;
    } else {
        let keep_left_and_operator_on_same_line = matches!(
            operator,
            TypeBinaryOperator::Cast | TypeBinaryOperator::Satisfies
        );

        write!(
            f,
            [group(&format_args![
                format_with(format_left),
                indent(&format_with(|f| {
                    if !has_postfix {
                        if left_has_leading_prefix_comment || keep_left_and_operator_on_same_line {
                            write!(f, [space()])?;
                        } else {
                            write!(f, [soft_line_break_or_space()])?;
                        }
                    }
                    write_type_binary_operator_and_right(f, operator, right)
                }))
            ])]
        )?;
    }

    Ok(())
}

/// Render remap static-argument type binary seams with forced inline operator spacing.
pub(crate) fn format_remap_static_argument_binary_layout<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    operator: BinaryOperator,
    operands: &BinaryOperands,
) -> FormatResult<()> {
    write!(
        f,
        [group(&format_with(|f: &mut DestackFormatter<'ast, '_>| {
            let Some(first_operand) = operands.first() else {
                return Ok(());
            };
            format_binary_operand_with_grouping_parentheses(f, operator, first_operand.expression)?;

            for operand in operands.iter().skip(1) {
                let Some(op) = operand.operator else {
                    continue;
                };
                write!(
                    f,
                    [
                        space(),
                        op,
                        space(),
                        format_with(|f: &mut DestackFormatter<'ast, '_>| {
                            format_binary_operand_with_grouping_parentheses(
                                f,
                                operator,
                                operand.expression,
                            )
                        }),
                    ]
                )?;
            }

            Ok(())
        }))]
    )
}

/// Render clean non-type binary short-circuit layout.
pub(crate) fn format_clean_binary_short_circuit_layout<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    operator: BinaryOperator,
    operands: &BinaryOperands,
) -> FormatResult<()> {
    write!(
        f,
        [group(&format_with(|f: &mut DestackFormatter<'ast, '_>| {
            let Some(first_operand) = operands.first() else {
                return Ok(());
            };
            format_binary_operand_with_grouping_parentheses(f, operator, first_operand.expression)?;

            for operand in operands.iter().skip(1) {
                let Some(op) = operand.operator else {
                    continue;
                };
                write!(
                    f,
                    [
                        space(),
                        op,
                        indent(&format_with(|f: &mut DestackFormatter<'ast, '_>| {
                            write!(f, [soft_line_break_or_space()])?;
                            format_binary_operand_with_grouping_parentheses(
                                f,
                                operator,
                                operand.expression,
                            )
                        }))
                    ]
                )?;
            }

            Ok(())
        }))]
    )
}

/// Render one type union in inline `A | B` form.
fn format_inline_type_union_layout<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    operands: &BinaryOperands,
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
                first_operand.expression,
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
                                operand.expression,
                            )
                        }),
                    ]
                )?;
            }

            Ok(())
        }))]
    )
}

/// Render destack type intersections with trailing operators and object-like guard rails.
pub(crate) fn format_destack_intersection_trailing_layout<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    operands: &BinaryOperands,
) -> FormatResult<()> {
    write!(
        f,
        [group(&format_with(|f: &mut DestackFormatter<'ast, '_>| {
            let mut previous_expression: Option<LocalNodeId<Expression>> = None;
            let mut chain_is_indented = false;
            let mut previous_is_object_like = false;

            for (index, operand) in operands.iter().enumerate() {
                if index == 0 {
                    write!(f, [operand.expression])?;
                    previous_is_object_like =
                        is_object_like_type_expression(f.context(), operand.expression);
                    continue;
                }

                let Some(op) = operand.operator else {
                    continue;
                };

                let has_previous_postfix_annotation = previous_expression
                    .is_some_and(|expression_id| f.context().has_postfix_annotation(expression_id));
                if !has_previous_postfix_annotation {
                    write!(f, [space()])?;
                }
                write!(f, [op])?;

                let is_object_like =
                    is_object_like_type_expression(f.context(), operand.expression);
                if !(previous_is_object_like || is_object_like) {
                    write!(
                        f,
                        [indent(&format_with(
                            |f: &mut DestackFormatter<'ast, '_>| {
                                write!(f, [soft_line_break_or_space(), operand.expression])
                            }
                        ))]
                    )?;
                } else {
                    write!(f, [space()])?;

                    if !previous_is_object_like || !is_object_like {
                        chain_is_indented = index > 1;
                    }

                    if chain_is_indented {
                        write!(f, [indent(&operand.expression)])?;
                    } else {
                        write!(f, [operand.expression])?;
                    }
                }

                previous_is_object_like = is_object_like;
                previous_expression = Some(operand.expression);
            }

            Ok(())
        }))]
    )
}

/// Render non-destack intersections with oxc-style object-like chain layout.
fn format_oxc_intersection_layout<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    operands: &BinaryOperands,
) -> FormatResult<()> {
    write!(
        f,
        [group(&format_with(|f: &mut DestackFormatter<'ast, '_>| {
            let last_index = operands.len().saturating_sub(1);
            let mut previous_is_object_like = false;
            let mut chain_is_indented = false;

            for (index, operand) in operands.iter().enumerate() {
                let current_is_object_like =
                    is_object_like_type_expression(f.context(), operand.expression);
                let current_has_own_line_prefix =
                    expression_has_own_line_prefix_annotation(f.context(), operand.expression);

                // first intersection element: always inline
                if index == 0 {
                    format_binary_operand_with_grouping_parentheses(
                        f,
                        BinaryOperator::ElementwiseAnd,
                        operand.expression,
                    )?;
                }
                // own-line prefix comments: break before current element
                else if current_has_own_line_prefix {
                    write!(
                        f,
                        [indent(&format_args![
                            soft_line_break_or_space(),
                            format_with(|f: &mut DestackFormatter<'ast, '_>| {
                                format_binary_operand_with_grouping_parentheses(
                                    f,
                                    BinaryOperator::ElementwiseAnd,
                                    operand.expression,
                                )
                            })
                        ])]
                    )?;
                }
                // non object-like seams: break when groups overflow
                else if !(previous_is_object_like || current_is_object_like) {
                    write!(
                        f,
                        [indent(&format_args![
                            soft_line_break_or_space(),
                            format_with(|f: &mut DestackFormatter<'ast, '_>| {
                                format_binary_operand_with_grouping_parentheses(
                                    f,
                                    BinaryOperator::ElementwiseAnd,
                                    operand.expression,
                                )
                            })
                        ])]
                    )?;
                }
                // object-like seams: keep compact chain unless object/non-object transition recurs
                else {
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
                                        operand.expression,
                                    )
                                }
                            ))]
                        )?;
                    } else {
                        format_binary_operand_with_grouping_parentheses(
                            f,
                            BinaryOperator::ElementwiseAnd,
                            operand.expression,
                        )?;
                    }
                }

                // separator between intersection elements
                if index < last_index {
                    write!(f, [space(), token("&")])?;
                }

                previous_is_object_like = current_is_object_like;
            }

            Ok(())
        }))]
    )
}

/// Write one non-head default flattened operand with selected mode.
pub(crate) fn write_default_flattened_non_head_operand<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    root_operator: BinaryOperator,
    operand_operator: BinaryOperator,
    operand_expression: LocalNodeId<Expression>,
    previous_expression: Option<LocalNodeId<Expression>>,
    is_type_union: bool,
    is_type_intersection: bool,
) -> FormatResult<()> {
    let is_type_binary = is_type_union || is_type_intersection;
    let has_postfix = previous_expression
        .is_some_and(|expression_id| f.context().has_postfix_annotation(expression_id));
    let previous_has_prefix_annotation = previous_expression.is_some_and(|expression_id| {
        expression_has_leading_prefix_comment(f.context(), expression_id)
    });
    let previous_has_line_prefix_slash_comment = previous_expression.is_some_and(|expression_id| {
        expression_has_line_prefix_slash_comment(f.context(), expression_id)
    });
    let previous_is_parenthesized_multiline = previous_expression.is_some_and(|expression_id| {
        matches!(
            f.context().tree.get(expression_id),
            Expression::Parenthesized { .. }
        ) && f.context().node_has_newline(expression_id)
    });
    let previous_needs_grouping_parentheses_multiline =
        previous_expression.is_some_and(|expression_id| {
            type_binary_operand_needs_grouping_parentheses(
                f.context(),
                root_operator,
                expression_id,
            ) && f.context().node_has_newline(expression_id)
        });
    let previous_is_parenthesized_or_grouped_multiline =
        previous_is_parenthesized_multiline || previous_needs_grouping_parentheses_multiline;
    let previous_has_line_postfix_slash_comment =
        previous_expression.is_some_and(|expression_id| {
            expression_has_line_postfix_slash_comment(f.context(), expression_id)
        });
    let previous_requires_type_grouping_break =
        is_type_binary && previous_has_line_postfix_slash_comment;
    let current_has_line_prefix_slash_comment =
        expression_has_line_prefix_slash_comment(f.context(), operand_expression);
    let current_prefers_trailing_operator = is_logical_binary_operator(operand_operator)
        && expression_has_leading_prefix_comment(f.context(), operand_expression);
    let left_inline_block_postfix_comment_allows_space =
        previous_expression.is_some_and(|expression_id| {
            expression_has_inline_block_postfix_boundary_star_comment(f.context(), expression_id)
        });
    let left_postfix_comment_allows_space =
        left_inline_block_postfix_comment_allows_space && !previous_has_line_postfix_slash_comment;

    // elementwise intersections with grouped multiline left operand
    if operand_operator == BinaryOperator::ElementwiseAnd
        && previous_is_parenthesized_or_grouped_multiline
    {
        write!(
            f,
            [
                space(),
                operand_operator,
                indent(&format_args![
                    hard_line_break(),
                    format_with(|f| {
                        format_binary_operand_with_grouping_parentheses(
                            f,
                            root_operator,
                            operand_expression,
                        )
                    })
                ])
            ]
        )?;

        return Ok(());
    }

    // type-binary seams with line-prefix current operand comments
    if is_type_binary && current_has_line_prefix_slash_comment {
        if !has_postfix {
            write!(f, [space()])?;
        } else if previous_requires_type_grouping_break || previous_has_line_postfix_slash_comment {
            write!(f, [hard_line_break()])?;
        }
        write!(
            f,
            [
                operand_operator,
                space(),
                indent(&format_with(|f: &mut DestackFormatter<'ast, '_>| {
                    format_binary_operand_with_grouping_parentheses(
                        f,
                        root_operator,
                        operand_expression,
                    )
                }))
            ]
        )?;

        return Ok(());
    }

    // logical operators that should trail current seams
    if current_prefers_trailing_operator {
        if !has_postfix || left_postfix_comment_allows_space {
            write!(f, [space()])?;
        } else if previous_requires_type_grouping_break || previous_has_line_postfix_slash_comment {
            write!(f, [hard_line_break()])?;
        }
        write!(
            f,
            [
                operand_operator,
                indent(&format_args![
                    hard_line_break(),
                    format_with(|f| {
                        format_binary_operand_with_grouping_parentheses(
                            f,
                            root_operator,
                            operand_expression,
                        )
                    })
                ])
            ]
        )?;

        return Ok(());
    }

    // type-binary seams after previous line-prefix comments
    if is_type_binary && previous_has_line_prefix_slash_comment {
        if !has_postfix {
            write!(f, [space()])?;
        } else if previous_requires_type_grouping_break || previous_has_line_postfix_slash_comment {
            write!(f, [hard_line_break()])?;
        }

        if is_type_intersection {
            write!(
                f,
                [
                    operand_operator,
                    indent(&format_args![
                        hard_line_break(),
                        format_with(|f| {
                            format_binary_operand_with_grouping_parentheses(
                                f,
                                root_operator,
                                operand_expression,
                            )
                        })
                    ])
                ]
            )?;
        } else {
            write!(
                f,
                [
                    operand_operator,
                    space(),
                    indent(&format_with(|f: &mut DestackFormatter<'ast, '_>| {
                        format_binary_operand_with_grouping_parentheses(
                            f,
                            root_operator,
                            operand_expression,
                        )
                    }))
                ]
            )?;
        }

        return Ok(());
    }

    // seams after previous prefix annotations
    if previous_has_prefix_annotation {
        if !has_postfix || (!is_type_binary && left_postfix_comment_allows_space) {
            write!(f, [space()])?;
        } else if previous_requires_type_grouping_break || previous_has_line_postfix_slash_comment {
            write!(f, [hard_line_break()])?;
        }

        if is_type_binary {
            write!(
                f,
                [
                    operand_operator,
                    indent(&format_args![
                        hard_line_break(),
                        format_with(|f| {
                            format_binary_operand_with_grouping_parentheses(
                                f,
                                root_operator,
                                operand_expression,
                            )
                        })
                    ])
                ]
            )?;
        } else {
            write!(f, [operand_operator, space()])?;
            format_binary_operand_with_grouping_parentheses(f, root_operator, operand_expression)?;
        }

        return Ok(());
    }

    // generic seam rendering
    let seam_document = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        if !has_postfix {
            if previous_is_parenthesized_multiline && is_logical_binary_operator(operand_operator) {
                write!(f, [space()])?;
            } else {
                write!(f, [soft_line_break_or_space()])?;
            }
        } else if left_postfix_comment_allows_space {
            write!(f, [space()])?;
        } else if previous_requires_type_grouping_break || previous_has_line_postfix_slash_comment {
            write!(f, [hard_line_break()])?;
        }

        write!(f, [operand_operator, space()])?;
        format_binary_operand_with_grouping_parentheses(f, root_operator, operand_expression)
    });

    write!(f, [indent(&seam_document)])?;

    Ok(())
}

/// Render default flattened binary layout after specialized layouts are excluded.
pub(crate) fn format_default_flattened_binary_layout<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    root_operator: BinaryOperator,
    operands: &BinaryOperands,
    is_type_union: bool,
    is_type_intersection: bool,
    should_force_type_binary_expansion: bool,
) -> FormatResult<()> {
    write!(
        f,
        [group(&format_with(|f: &mut DestackFormatter<'ast, '_>| {
            let mut previous_expression = None;

            for operand in operands {
                if let Some(operand_operator) = operand.operator {
                    write_default_flattened_non_head_operand(
                        f,
                        root_operator,
                        operand_operator,
                        operand.expression,
                        previous_expression,
                        is_type_union,
                        is_type_intersection,
                    )?;
                } else {
                    let first_operand_has_prefix_annotation =
                        f.context().has_prefix_annotation(operand.expression);
                    let should_indent_first_operand = first_operand_has_prefix_annotation
                        && (is_type_union || is_type_intersection);
                    if should_indent_first_operand {
                        write!(
                            f,
                            [indent(&format_args![format_with(|f| {
                                format_binary_operand_with_grouping_parentheses(
                                    f,
                                    root_operator,
                                    operand.expression,
                                )
                            })])]
                        )?;
                    } else {
                        format_binary_operand_with_grouping_parentheses(
                            f,
                            root_operator,
                            operand.expression,
                        )?;
                    }
                }

                previous_expression = Some(operand.expression);
            }

            Ok(())
        }))
        .should_expand(should_force_type_binary_expansion)]
    )?;

    Ok(())
}

/// Format a binary expression with all operator-specific layout policies.
pub(crate) fn format_binary_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    left: LocalNodeId<Expression>,
    operator: &BinaryOperator,
    right: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let _timing = f
        .context()
        .timing_scope(timing::FORMAT_EXPRESSION_OPERATOR_BINARY);

    // specialized logical and coalesce layout paths
    if try_format_trailing_coalesce(f, node_id, left, *operator, right)? {
        return Ok(());
    }

    // logical right-prefix block comment seams
    if try_format_logical_right_prefix_block_comment(f, node_id, left, *operator, right)? {
        return Ok(());
    }

    // logical right-prefix line comment seams
    if try_format_logical_right_prefix_line_comment(f, left, *operator, right)? {
        return Ok(());
    }

    // mixed-precedence logical grouping seams
    if try_format_mixed_logical_precedence(f, left, *operator, right)? {
        return Ok(());
    }

    // parenthesized logical seam layouts
    if try_format_logical_parenthesized_cases(f, node_id, left, *operator, right)? {
        return Ok(());
    }

    let in_type_context = is_type_context(f.context(), node_id);
    let in_type_template_literal_interpolation =
        is_in_type_template_literal_interpolation(f.context(), node_id);
    let has_type_semantics = in_type_context || in_type_template_literal_interpolation;
    let is_destack = f.context().options.language_type.is_destack();
    let is_type_intersection = has_type_semantics && *operator == BinaryOperator::ElementwiseAnd;
    let is_type_union = has_type_semantics && *operator == BinaryOperator::ElementwiseOr;
    let operands = if is_type_union || is_type_intersection {
        flatten_type_binary_expression(f.context(), node_id, *operator)
    } else {
        flatten_binary_expression(f.context(), node_id, *operator)
    };
    let should_force_type_binary_expansion = is_type_intersection
        && type_binary_operands_are_structurally_complex(f.context(), &operands);
    let has_node_annotation = f.context().has_annotation(node_id);
    let has_operand_annotations = operands
        .iter()
        .any(|operand| f.context().has_annotation(operand.expression));
    let has_operand_prefix_comments = operands
        .iter()
        .any(|operand| expression_has_leading_prefix_comment(f.context(), operand.expression));
    // remap template seams with trailing line comments should keep static type args inline
    if type_binary_is_static_argument_under_remap_path(f.context(), node_id) {
        format_remap_static_argument_binary_layout(f, *operator, &operands)?;
        return Ok(());
    }

    // clean non-type binaries without annotation or prefix trivia
    if !is_type_union
        && !is_type_intersection
        && !has_node_annotation
        && !has_operand_annotations
        && !has_operand_prefix_comments
    {
        f.context()
            .increment_counter("stats.binary.clean.short_circuit", 1);
        format_clean_binary_short_circuit_layout(f, *operator, &operands)?;
        return Ok(());
    }

    // union types use one deterministic layout path: leading-pipe group
    if is_type_union {
        let root_owns_prefix_annotation = union_owns_prefix_annotations(f.context(), node_id);
        let union_has_breaking_postfix_comments =
            operands.iter().enumerate().any(|(index, operand)| {
                let is_last_operand = index + 1 == operands.len();
                let has_postfix_comment =
                    expression_has_postfix_comment_annotation(f.context(), operand.expression)
                        || expression_has_line_postfix_slash_comment(
                            f.context(),
                            operand.expression,
                        );
                has_postfix_comment && !is_last_operand
            });
        let union_has_breaking_operand_prefix_comments = has_operand_prefix_comments
            && operands.iter().enumerate().skip(1).any(|(index, operand)| {
                let is_last_operand = index + 1 == operands.len();
                expression_has_leading_prefix_comment(f.context(), operand.expression)
                    && !is_last_operand
            });
        let union_has_own_line_doc_prefix_annotation =
            union_has_trailing_own_line_doc_prefix_annotation(f.context(), node_id);
        let union_is_template_interpolation_multiline =
            in_type_template_literal_interpolation && f.context().node_has_newline(node_id);
        let union_should_hug_layout = should_hug_type_union_operands(f.context(), &operands);
        let union_prefers_multiline_layout = union_has_breaking_operand_prefix_comments
            || union_has_breaking_postfix_comments
            || union_has_own_line_doc_prefix_annotation
            || union_is_template_interpolation_multiline;
        let should_inline_union = union_should_hug_layout
            || should_inline_union_with_terminal_line_postfix_comment(
                f.context(),
                node_id,
                &operands,
                union_prefers_multiline_layout,
            );
        if should_inline_union {
            if root_owns_prefix_annotation {
                write!(f, [f.context().any_prefix_annotations(node_id)])?;
            }
            format_inline_type_union_layout(f, &operands)?;
        } else {
            format_leading_pipe_union(f, node_id, &operands, union_prefers_multiline_layout)?;
        }

        return Ok(());
    }

    // destack intersections use trailing operator layout
    if is_type_intersection && is_destack {
        if intersection_is_nested_under_type_union(f.context(), node_id) {
            format_oxc_intersection_layout(f, &operands)?;
        } else {
            format_destack_intersection_trailing_layout(f, &operands)?;
        }
        return Ok(());
    }

    // non-destack intersection types use oxc-style object-like chain layout
    if is_type_intersection && !is_destack {
        f.context()
            .increment_counter("stats.binary.type_clean.short_circuit", 1);
        format_oxc_intersection_layout(f, &operands)?;
        return Ok(());
    }

    // default flattened binary formatting
    format_default_flattened_binary_layout(
        f,
        *operator,
        &operands,
        is_type_union,
        is_type_intersection,
        should_force_type_binary_expansion,
    )?;

    Ok(())
}
