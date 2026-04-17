use crate::format::annotation::{format_trailing_comments_before_boundary, prefix_annotations};
use crate::format::context::ParenthesizedExpressionView;
use crate::format::expression::{
    expression_has_only_prefix_comment_or_doc_annotations,
    write_expression_without_prefix_annotations,
};
use crate::format::operator::expression_is_type_position;
use crate::{DestackFormatContext, DestackFormatter};
use destack_ast::{
    BinaryOperator, Expression, IfKind, LocalNodeId, Member, NodeType, OperatorPrecedence,
    Property, TokenType, UnaryOperator,
};
use destack_fir::format::{Buffer, FormatResult};
use destack_fir::prelude::{
    block_indent, format_with, group, hard_line_break, indent, soft_line_break_or_space, space,
    token,
};
use destack_fir::write;
use destack_source::Span;
use smallvec::SmallVec;

/// Return the trailing trivia gap after one expression.
fn binary_expression_postfix_gap(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> Option<Span> {
    let expression_span = context.span(expression_id);
    let gap_end =
        if let Some(next_token) = context.next_non_trivia_token_after_span(expression_span) {
            if next_token.span.file != expression_span.file
                || next_token.span.start <= expression_span.end
            {
                return None;
            }

            next_token.span.start
        } else {
            let (line_index, _) = context.source_position(expression_span.end)?;
            let line_span = context.source_line_span(line_index)?;
            line_span.end
        };

    (gap_end > expression_span.end)
        .then(|| Span::new(expression_span.file, expression_span.end, gap_end))
}

/// Return whether one expression ends with a line postfix comment.
fn binary_expression_has_line_suffix_comment(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let Some(gap_span) = binary_expression_postfix_gap(context, expression_id) else {
        return false;
    };

    context
        .comments_in_range(gap_span.start, gap_span.end)
        .iter()
        .copied()
        .any(|comment| context.comment_is_line(comment))
}

/// Return whether one expression ends with an inline block postfix comment.
fn binary_expression_has_inline_block_postfix_comment(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let Some(gap_span) = binary_expression_postfix_gap(context, expression_id) else {
        return false;
    };
    if context.has_newline(gap_span) {
        return false;
    }

    context
        .comment_tokens_in_range(gap_span.start, gap_span.end)
        .iter()
        .any(|token| {
            matches!(
                token.token.ty,
                TokenType::BlockComment | TokenType::DocBlockComment
            )
        })
}

/// Return whether one operator belongs to the equality family.
#[inline]
fn binary_operator_is_equality(operator: BinaryOperator) -> bool {
    matches!(
        operator,
        BinaryOperator::Equal
            | BinaryOperator::NotEqual
            | BinaryOperator::EqualStrict
            | BinaryOperator::NotEqualStrict
    )
}

/// Return whether one operator belongs to the multiplicative family.
#[inline]
fn binary_operator_is_multiplicative(operator: BinaryOperator) -> bool {
    matches!(
        operator,
        BinaryOperator::Multiply
            | BinaryOperator::WrappingMultiply
            | BinaryOperator::SaturatingMultiply
            | BinaryOperator::Divide
            | BinaryOperator::Remainder
    )
}

/// Return whether one operator belongs to the shift family.
#[inline]
fn binary_operator_is_shift(operator: BinaryOperator) -> bool {
    matches!(
        operator,
        BinaryOperator::ShiftLeft
            | BinaryOperator::SaturatingShiftLeft
            | BinaryOperator::ShiftRight
            | BinaryOperator::UnsignedShiftRight
    )
}

/// Return whether one operator is a remainder operator.
#[inline]
fn binary_operator_is_remainder(operator: BinaryOperator) -> bool {
    operator == BinaryOperator::Remainder
}

/// Return whether nested binaries should flatten into one group.
#[inline]
fn should_flatten_binary(parent_operator: BinaryOperator, operator: BinaryOperator) -> bool {
    let parent_precedence_group = parent_operator.precedence_group();
    let operator_precedence_group = operator.precedence_group();

    if parent_precedence_group != operator_precedence_group {
        return false;
    }

    if matches!(
        parent_operator,
        BinaryOperator::Exponent
            | BinaryOperator::WrappingExponent
            | BinaryOperator::SaturatingExponent
    ) {
        return false;
    }

    if binary_operator_is_equality(parent_operator) && binary_operator_is_equality(operator) {
        return false;
    }

    if binary_operator_is_multiplicative(parent_operator)
        && binary_operator_is_multiplicative(operator)
    {
        if binary_operator_is_remainder(parent_operator) || binary_operator_is_remainder(operator) {
            return false;
        }

        return parent_operator == operator;
    }

    if binary_operator_is_shift(parent_operator) && binary_operator_is_shift(operator) {
        return false;
    }

    true
}

/// Return whether one direct child expression is the left operand of its binary parent.
fn binary_operand_is_left(
    context: &DestackFormatContext<'_>,
    operand_id: LocalNodeId<Expression>,
) -> Option<bool> {
    let (parent_id, parent_type) = context.parent(operand_id)?;
    if parent_type != NodeType::Expression {
        return None;
    }

    let parent_expression_id = LocalNodeId::<Expression>::new(parent_id);
    let Expression::Binary { left, right, .. } = context.tree.get(parent_expression_id) else {
        return None;
    };

    if *left == operand_id {
        return Some(true);
    }

    if *right == operand_id {
        return Some(false);
    }

    None
}

/// Return whether the parent inlines this binary root in the flattened layout.
fn binary_parent_inlines_flattened_layout(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let Some((parent_id, parent_type)) = context.parent(expression_id) else {
        return false;
    };

    match parent_type {
        // variable declarators inline the binary element and own the outer indent
        NodeType::Declarator => true,

        // assignment expressions inline the rhs element and own the outer indent
        NodeType::Expression => matches!(
            context.tree.get(LocalNodeId::<Expression>::new(parent_id)),
            Expression::Assign { right, .. } if right.id == expression_id.id
        ),

        // object property values inline the binary element and own the outer indent
        NodeType::Property => matches!(
            context.tree.get(LocalNodeId::<Property>::new(parent_id)),
            Property::Field { value, .. }
                if *value == expression_id && !expression_is_type_position(context, expression_id)
        ),

        // class field initializers inline the binary element and own the outer indent
        NodeType::Member => matches!(
            context.tree.get(LocalNodeId::<Member>::new(parent_id)),
            Member::Field { default, .. } if default.is_some_and(|default| default == expression_id)
        ),

        _ => false,
    }
}

/// Return the earliest boundary that structurally belongs to one binary operand.
fn binary_operand_comment_boundary_start(
    context: &DestackFormatContext<'_>,
    operand_id: LocalNodeId<Expression>,
) -> u32 {
    if let Some(leading_span) = context
        .tree
        .get_side_span(operand_id, destack_source::NodeSpanType::Leading)
    {
        return leading_span.start;
    }

    let operand_span = context.span(operand_id);
    context
        .previous_non_trivia_token_before_span(operand_span)
        .map_or(operand_span.start, |token| token.span.start)
}

/// Return whether one expression is a prefix-like left operand of `in`.
fn expression_is_relational_prefix_left_operand(expression: &Expression) -> bool {
    match expression {
        Expression::Unary { operator, .. } => matches!(
            operator,
            UnaryOperator::Not
                | UnaryOperator::Plus
                | UnaryOperator::Negate
                | UnaryOperator::WrappingNegate
                | UnaryOperator::ElementwiseNot
                | UnaryOperator::Typeof
                | UnaryOperator::Void
                | UnaryOperator::Dereference
                | UnaryOperator::Spread
        ),
        Expression::Await { .. }
        | Expression::AwaitMaybe { .. }
        | Expression::Yield { .. }
        | Expression::Delete { .. } => true,
        _ => false,
    }
}

/// Return whether a binary operand requires explicit grouping parentheses.
fn binary_operand_requires_grouping_parentheses(
    context: &DestackFormatContext<'_>,
    parent_operator: BinaryOperator,
    operand_id: LocalNodeId<Expression>,
) -> bool {
    let Some(operand_is_left) = binary_operand_is_left(context, operand_id) else {
        return false;
    };

    let expression_id = match context.tree.get(operand_id) {
        Expression::Parenthesized { expression } => *expression,
        _ => operand_id,
    };
    let expression = context.tree.get(expression_id);
    if operand_is_left
        && matches!(parent_operator, BinaryOperator::In)
        && expression_is_relational_prefix_left_operand(expression)
    {
        return true;
    }

    let Expression::Binary {
        operator: operand_operator,
        ..
    } = expression
    else {
        return false;
    };

    let parent_precedence = parent_operator.precedence_group() as u16;
    let operand_precedence = operand_operator.precedence_group() as u16;
    if parent_precedence > operand_precedence {
        return true;
    }

    !operand_is_left
        && parent_precedence == operand_precedence
        && !should_flatten_binary(parent_operator, *operand_operator)
}

/// Return whether one expression is the same binary kind as the current owner.
#[inline]
fn expression_is_same_binary_kind(expression: &Expression, operator: BinaryOperator) -> bool {
    matches!(
        expression,
        Expression::Binary {
            operator: other_operator,
            ..
        } if is_logical_binary_operator(*other_operator) == is_logical_binary_operator(operator)
    )
}

/// Return whether one flattened rhs owner should group its operator and rhs shell.
fn binary_operand_owner_should_group(
    context: &DestackFormatContext<'_>,
    owner_id: LocalNodeId<Expression>,
) -> bool {
    let Expression::Binary {
        left,
        operator,
        right,
    } = context.tree.get(owner_id)
    else {
        return true;
    };

    let parent_is_same_kind = context
        .parent(owner_id)
        .is_some_and(|(parent_id, parent_type)| {
            parent_type == NodeType::Expression
                && expression_is_same_binary_kind(
                    context.tree.get(LocalNodeId::<Expression>::new(parent_id)),
                    *operator,
                )
        });
    let left_is_same_kind = expression_is_same_binary_kind(context.tree.get(*left), *operator);
    let right_is_same_kind = expression_is_same_binary_kind(context.tree.get(*right), *operator);

    !(parent_is_same_kind || left_is_same_kind || right_is_same_kind)
}

/// Flattens a binary expression chain while preserving the source owner of each rhs operand.
fn flatten_binary_expression_with_owners(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
    target_operator: BinaryOperator,
) -> (
    SmallVec<[(Option<BinaryOperator>, LocalNodeId<Expression>); 8]>,
    SmallVec<[Option<LocalNodeId<Expression>>; 8]>,
) {
    let mut operands = SmallVec::new();
    let mut owner_ids = SmallVec::new();
    flatten_binary_recursive_with_owners(
        context,
        expression_id,
        target_operator,
        &mut operands,
        &mut owner_ids,
        None,
        true,
    );
    (operands, owner_ids)
}

/// Recursively collect binary expression operands together with their source owners.
fn flatten_binary_recursive_with_owners(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
    target_operator: BinaryOperator,
    operands: &mut SmallVec<[(Option<BinaryOperator>, LocalNodeId<Expression>); 8]>,
    owner_ids: &mut SmallVec<[Option<LocalNodeId<Expression>>; 8]>,
    preceding_operator: Option<BinaryOperator>,
    is_root: bool,
) {
    if let Expression::Binary {
        left,
        operator,
        right,
    } = context.tree.get(expression_id)
        && *operator == target_operator
        && (is_root || should_flatten_binary(*operator, target_operator))
        && (!context.has_annotation(expression_id) || is_root)
    {
        // recursively flatten the left side
        flatten_binary_recursive_with_owners(
            context,
            *left,
            target_operator,
            operands,
            owner_ids,
            None,
            false,
        );

        // add the right operand with its source owner
        operands.push((Some(*operator), *right));
        owner_ids.push(Some(expression_id));
        return;
    }

    // not a binary expression or different precedence: add as-is
    operands.push((preceding_operator, expression_id));
    owner_ids.push(None);
}

/// Return precedence value for an expression.
#[inline]
pub(crate) fn expression_precedence(expr: &Expression) -> u16 {
    match expr {
        // postfix operators
        Expression::Call { .. }
        | Expression::Member { .. }
        | Expression::PrivateMember { .. }
        | Expression::Index { .. }
        | Expression::Instantiation { .. }
        | Expression::Maybe { .. }
        | Expression::Must { .. } => OperatorPrecedence::Postfix as u16,

        // postfix unary
        Expression::Unary { operator, .. } if operator.is_postfix() => {
            OperatorPrecedence::Postfix as u16
        }

        // prefix unary
        Expression::Unary { .. } => OperatorPrecedence::Prefix as u16,

        // prefix expressions
        Expression::Await { .. }
        | Expression::AwaitMaybe { .. }
        | Expression::Yield { .. }
        | Expression::Delete { .. }
        | Expression::Throw { .. }
        | Expression::Return { .. } => OperatorPrecedence::Prefix as u16,

        // binary
        Expression::Binary { operator, .. } => operator.precedence_group() as u16,
        Expression::As { .. } | Expression::Satisfies { .. } => 500,
        Expression::Is { .. } | Expression::InstanceOf { .. } => {
            OperatorPrecedence::Comparison as u16
        }

        // assignment
        Expression::Assign { operator, .. } => operator.precedence(),

        // ternary
        Expression::If {
            kind: IfKind::Ternary,
            ..
        } => OperatorPrecedence::AssignmentBoolean as u16 - 1,

        // atomic/primary expressions
        _ => u16::MAX,
    }
}

/// Format a binary operand with grouping parentheses when needed in type-slot grouping.
pub(crate) fn format_binary_operand_with_grouping_parentheses<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    parent_operator: BinaryOperator,
    operand_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let operand_id = match f.context().tree.get(operand_id) {
        Expression::Parenthesized { expression } => {
            let parent_expression = Expression::Binary {
                left: operand_id,
                operator: parent_operator,
                right: operand_id,
            };

            if binary_drops_parenthesized_operand_wrapper(
                f.context(),
                operand_id,
                *expression,
                &parent_expression,
            ) {
                *expression
            } else {
                operand_id
            }
        }
        _ => operand_id,
    };

    let operand_has_annotation = f.context().has_annotation(operand_id);
    let expression = f.context().tree.get(operand_id);
    let suppress_precedence_parentheses_for_type_relation = matches!(
        (expression, parent_operator),
        (
            Expression::Is { .. } | Expression::InstanceOf { .. },
            BinaryOperator::And | BinaryOperator::Or | BinaryOperator::Coalesce,
        )
    );
    let needs_mixed_logical_grouping_parentheses = matches!(
        (parent_operator, expression),
        (
            BinaryOperator::Or | BinaryOperator::Coalesce,
            Expression::Binary {
                operator: BinaryOperator::And | BinaryOperator::Coalesce,
                ..
            },
        )
    );
    let needs_precedence_parentheses = !matches!(expression, Expression::Parenthesized { .. })
        && (expression_precedence(expression) < parent_operator.precedence_group() as u16
            || binary_operand_requires_grouping_parentheses(
                f.context(),
                parent_operator,
                operand_id,
            ))
        && !suppress_precedence_parentheses_for_type_relation;
    let needs_grouping_parentheses =
        needs_precedence_parentheses || needs_mixed_logical_grouping_parentheses;
    let operand_has_prefix_annotation = f.context().has_prefix_annotation(operand_id);

    if needs_grouping_parentheses {
        if operand_has_prefix_annotation {
            write!(f, [prefix_annotations(f.context(), operand_id), token("(")])?;
            write_expression_without_prefix_annotations(f, operand_id)?;
            write!(f, [token(")")])?;
        } else if operand_has_annotation {
            write!(
                f,
                [
                    token("("),
                    block_indent(&operand_id),
                    hard_line_break(),
                    token(")")
                ]
            )?;
        } else {
            write!(f, [token("("), operand_id, token(")")])?;
        }
    } else {
        write!(f, [operand_id])?;
    }

    Ok(())
}

/// Return whether one relational binary keeps a unary left wrapper explicit.
pub(crate) fn binary_keeps_unary_left_parenthesized_wrapper(
    node_id: LocalNodeId<Expression>,
    inner_expression: &Expression,
    parent_expression: &Expression,
) -> bool {
    matches!(
        parent_expression,
        Expression::Binary {
            left,
            operator: BinaryOperator::In,
            ..
        } if *left == node_id
    ) && matches!(inner_expression, Expression::Unary { .. })
}

/// Decide whether a binary operand can drop one parenthesized wrapper.
pub(crate) fn binary_drops_parenthesized_operand_wrapper(
    context: &DestackFormatContext<'_>,
    parenthesized_id: LocalNodeId<Expression>,
    inner_expression_id: LocalNodeId<Expression>,
    parent_expression: &Expression,
) -> bool {
    let Expression::Binary { operator, .. } = parent_expression else {
        return false;
    };

    if context.has_annotation(parenthesized_id)
        || context.has_annotation(inner_expression_id)
        || expression_has_only_prefix_comment_or_doc_annotations(context, parenthesized_id)
        || expression_has_only_prefix_comment_or_doc_annotations(context, inner_expression_id)
        || ParenthesizedExpressionView::from_node(context, parenthesized_id)
            .is_some_and(ParenthesizedExpressionView::has_leading_inner_trivia)
    {
        return false;
    }

    let inner_expression = context.tree.get(inner_expression_id);

    if binary_keeps_unary_left_parenthesized_wrapper(
        parenthesized_id,
        inner_expression,
        parent_expression,
    ) {
        return false;
    }

    let suppress_precedence_parentheses_for_type_relation = matches!(
        (inner_expression, operator),
        (
            Expression::Is { .. } | Expression::InstanceOf { .. },
            BinaryOperator::And | BinaryOperator::Or | BinaryOperator::Coalesce,
        )
    );
    let needs_mixed_logical_grouping_parentheses = matches!(
        (operator, inner_expression),
        (
            BinaryOperator::Or | BinaryOperator::Coalesce,
            Expression::Binary {
                operator: BinaryOperator::And | BinaryOperator::Coalesce,
                ..
            },
        )
    );
    let needs_precedence_parentheses = (expression_precedence(inner_expression)
        < operator.precedence_group() as u16
        || binary_operand_requires_grouping_parentheses(context, *operator, parenthesized_id))
        && !suppress_precedence_parentheses_for_type_relation;
    !(needs_precedence_parentheses || needs_mixed_logical_grouping_parentheses)
}

/// Return whether one binary operator is logical.
#[inline]
pub(crate) fn is_logical_binary_operator(operator: BinaryOperator) -> bool {
    matches!(
        operator,
        BinaryOperator::And | BinaryOperator::Or | BinaryOperator::Coalesce
    )
}

/// Write one separating space after the left operand when no postfix trivia exists.
pub(crate) fn write_space_after_binary_left_if_needed<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    left: LocalNodeId<Expression>,
    operator: BinaryOperator,
) -> FormatResult<()> {
    let allow_inline_block_postfix_star_space =
        binary_expression_has_inline_block_postfix_comment(f.context(), left);
    let allow_logical_space_after_line_comment = is_logical_binary_operator(operator)
        && binary_expression_has_line_suffix_comment(f.context(), left);
    if f.context().has_postfix_annotation(left)
        && !allow_logical_space_after_line_comment
        && !allow_inline_block_postfix_star_space
    {
        return Ok(());
    }

    write!(f, [space()])
}

// union-specific prefix splitting now lives in operator/union.rs

/// One local binary-like formatting owner.
struct BinaryLikeExpression {
    node_id: LocalNodeId<Expression>,
    operator: BinaryOperator,
    operands: SmallVec<[(Option<BinaryOperator>, LocalNodeId<Expression>); 8]>,
    operand_owner_ids: SmallVec<[Option<LocalNodeId<Expression>>; 8]>,
}

impl BinaryLikeExpression {
    /// Build one binary-like formatting owner.
    fn new(
        context: &DestackFormatContext<'_>,
        node_id: LocalNodeId<Expression>,
        operator: BinaryOperator,
    ) -> Self {
        let (operands, operand_owner_ids) =
            flatten_binary_expression_with_owners(context, node_id, operator);

        Self {
            node_id,
            operator,
            operands,
            operand_owner_ids,
        }
    }

    /// Format this owner using the default flattened layout.
    fn format_default_flattened_layout<'ast>(
        &self,
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        let Some((_, head_expression)) = self.operands.first().copied() else {
            return Ok(());
        };

        let format_tail = format_with(|f: &mut DestackFormatter<'ast, '_>| {
            let mut previous_expression = Some(head_expression);

            for (index, operand) in self.operands.iter().enumerate().skip(1) {
                let operand_operator = operand
                    .0
                    .expect("flattened non-head binary operand has an operator");
                self.write_flattened_operand(
                    f,
                    index,
                    operand_operator,
                    operand.1,
                    previous_expression,
                )?;
                previous_expression = Some(operand.1);
            }

            Ok(())
        });

        write!(
            f,
            [group(&format_with(|f: &mut DestackFormatter<'ast, '_>| {
                format_binary_operand_with_grouping_parentheses(f, self.operator, head_expression)?;

                if self.operands.len() > 1 {
                    if binary_parent_inlines_flattened_layout(f.context(), self.node_id) {
                        write!(f, [format_tail])?;
                    } else {
                        write!(f, [indent(&format_tail)])?;
                    }
                }

                Ok(())
            }))
            .should_expand(false)]
        )?;

        Ok(())
    }

    /// Write one non-head operand in the flattened binary chain.
    fn write_flattened_operand<'ast>(
        &self,
        f: &mut DestackFormatter<'ast, '_>,
        operand_index: usize,
        operand_operator: BinaryOperator,
        operand_expression: LocalNodeId<Expression>,
        previous_expression: Option<LocalNodeId<Expression>>,
    ) -> FormatResult<()> {
        let enclosing_span = f.context().span(self.node_id);

        if let Some(previous_expression) = previous_expression {
            let previous_span = f.context().span(previous_expression);
            let operand_span = f.context().span(operand_expression);
            let trailing_boundary_start =
                binary_operand_comment_boundary_start(f.context(), operand_expression);

            write!(
                f,
                [format_trailing_comments_before_boundary(
                    enclosing_span,
                    previous_span,
                    trailing_boundary_start,
                    operand_span.start
                )]
            )?;

            write_space_after_binary_left_if_needed(f, previous_expression, operand_operator)?;
        }

        let operator_and_operand = format_with(|f: &mut DestackFormatter<'ast, '_>| {
            write!(f, [operand_operator, soft_line_break_or_space()])?;
            format_binary_operand_with_grouping_parentheses(f, self.operator, operand_expression)
        });

        let should_break = previous_expression.is_some_and(|previous_expression| {
            binary_expression_has_line_suffix_comment(f.context(), previous_expression)
        });
        let owner_id = self.operand_owner_ids.get(operand_index).copied().flatten();

        if owner_id.is_some_and(|owner_id| binary_operand_owner_should_group(f.context(), owner_id))
        {
            return write!(
                f,
                [group(&operator_and_operand).should_expand(should_break)]
            );
        }

        if should_break {
            return write!(f, [group(&operator_and_operand).should_expand(true)]);
        }

        write!(f, [operator_and_operand])
    }
}

/// Format a binary expression with all operator-specific layout policies.
pub(crate) fn format_binary_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    _left: LocalNodeId<Expression>,
    operator: &BinaryOperator,
    _right: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let binary_like = BinaryLikeExpression::new(f.context(), node_id, *operator);

    binary_like.format_default_flattened_layout(f)
}
