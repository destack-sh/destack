use crate::format::chain::{
    has_comment_between_expressions, should_use_trailing_coalesce, transparent_inner_expression,
};
use crate::format::context::{
    expression_has_inline_block_postfix_boundary_star_comment,
    expression_has_inline_block_prefix_star_comment, expression_has_line_postfix_slash_comment,
    expression_has_line_prefix_slash_comment,
};
use crate::format::expression::{
    expression_has_leading_prefix_comment, expression_is_trivial_inline_without_annotations,
    parenthesized_boundary_comments, parenthesized_has_leading_inner_trivia,
    write_expression_without_prefix_annotations,
};
use crate::format::operator::types::is_in_type_template_literal_interpolation;
use crate::format::operator::{
    binary_like_is_type_intersection, binary_like_is_type_union,
    expression_has_type_grouping_semantics, flatten_binary_like_operands,
    format_type_intersection_binary_layout, type_binary_operand_needs_grouping_parentheses,
};
use crate::{Annotation, DestackFormatContext, DestackFormatter};
use destack_ast::{
    AnnotationPosition, BinaryOperator, Expression, IfKind, LocalNodeId, NodeType,
    OperatorPrecedence, TokenType, TypeBinaryOperator, UnaryOperator,
};
use destack_fir::format::{Buffer, FormatResult};
use destack_fir::prelude::{
    block_indent, format_with, group, hard_line_break, indent, soft_line_break_or_space, space,
    token,
};
use destack_fir::{format_args, write};
use smallvec::SmallVec;

/// Return formatter precedence for one binary operator.
#[inline]
pub(crate) fn binary_operator_expression_precedence(operator: BinaryOperator) -> u16 {
    match operator {
        BinaryOperator::Exponent
        | BinaryOperator::WrappingExponent
        | BinaryOperator::SaturatingExponent => 1700,
        BinaryOperator::Multiply
        | BinaryOperator::WrappingMultiply
        | BinaryOperator::SaturatingMultiply
        | BinaryOperator::Divide
        | BinaryOperator::Remainder => 1600,
        BinaryOperator::Add
        | BinaryOperator::WrappingAdd
        | BinaryOperator::SaturatingAdd
        | BinaryOperator::Subtract
        | BinaryOperator::WrappingSubtract
        | BinaryOperator::SaturatingSubtract => 1500,
        BinaryOperator::ShiftLeft
        | BinaryOperator::SaturatingShiftLeft
        | BinaryOperator::ShiftRight
        | BinaryOperator::UnsignedShiftRight => 1400,
        BinaryOperator::LessThan
        | BinaryOperator::LessThanOrEqual
        | BinaryOperator::GreaterThan
        | BinaryOperator::GreaterThanOrEqual
        | BinaryOperator::In
        | BinaryOperator::InstanceOf => 1300,
        BinaryOperator::Equal
        | BinaryOperator::NotEqual
        | BinaryOperator::EqualStrict
        | BinaryOperator::NotEqualStrict => 1200,
        BinaryOperator::ElementwiseAnd => 1100,
        BinaryOperator::ElementwiseXor => 1000,
        BinaryOperator::ElementwiseOr => 900,
        BinaryOperator::And => 800,
        BinaryOperator::Coalesce => 700,
        BinaryOperator::Or => 600,
    }
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

/// Return whether a binary operator participates in type union or intersection grouping.
#[inline]
fn is_type_grouping_binary_operator(operator: BinaryOperator) -> bool {
    matches!(
        operator,
        BinaryOperator::ElementwiseOr | BinaryOperator::ElementwiseAnd
    )
}

/// Return whether nested binaries should flatten into one group.
#[inline]
fn should_flatten_binary(parent_operator: BinaryOperator, operator: BinaryOperator) -> bool {
    if binary_operator_expression_precedence(parent_operator)
        != binary_operator_expression_precedence(operator)
    {
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

/// Return whether one expression is a prefix-like left operand of `in` or `instanceof`.
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
        && matches!(
            parent_operator,
            BinaryOperator::In | BinaryOperator::InstanceOf
        )
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

    let parent_precedence = binary_operator_expression_precedence(parent_operator);
    let operand_precedence = binary_operator_expression_precedence(*operand_operator);
    if parent_precedence > operand_precedence {
        return true;
    }

    !operand_is_left
        && parent_precedence == operand_precedence
        && !should_flatten_binary(parent_operator, *operand_operator)
}

/// Flattens a binary expression chain into a list of operands.
///
/// For `a + b + c`, returns [(None, a), (Some(+), b), (Some(+), c)].
pub(crate) fn flatten_binary_expression(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
    target_operator: BinaryOperator,
) -> SmallVec<[(Option<BinaryOperator>, LocalNodeId<Expression>); 8]> {
    let mut operands = SmallVec::new();
    flatten_binary_recursive(
        context,
        expression_id,
        target_operator,
        &mut operands,
        None,
        true,
    );
    operands
}

/// Return the operand count for a flattened binary expression chain.
pub(crate) fn flattened_binary_operand_count(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
    target_operator: BinaryOperator,
) -> usize {
    count_flattened_binary_recursive(context, expression_id, target_operator, true)
}

/// Recursively collect binary expression operands.
fn flatten_binary_recursive(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
    target_operator: BinaryOperator,
    operands: &mut SmallVec<[(Option<BinaryOperator>, LocalNodeId<Expression>); 8]>,
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
        flatten_binary_recursive(context, *left, target_operator, operands, None, false);

        // add the right operand with its operator
        operands.push((Some(*operator), *right));
        return;
    }

    // not a binary expression or different precedence: add as-is
    operands.push((preceding_operator, expression_id));
}

/// Recursively count flattened binary operands without allocating.
fn count_flattened_binary_recursive(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
    target_operator: BinaryOperator,
    is_root: bool,
) -> usize {
    if let Expression::Binary {
        left,
        operator,
        right,
    } = context.tree.get(expression_id)
        && *operator == target_operator
        && (is_root || should_flatten_binary(*operator, target_operator))
        && (!context.has_annotation(expression_id) || is_root)
    {
        let left_count = count_flattened_binary_recursive(context, *left, target_operator, false);
        let right_count = count_flattened_binary_recursive(context, *right, target_operator, false);
        return left_count.saturating_add(right_count);
    }

    1
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

        // type unary
        Expression::TypeUnary { operator, .. } => operator.precedence(),

        // binary
        Expression::Binary { operator, .. } => binary_operator_expression_precedence(*operator),
        Expression::TypeBinary { operator, .. } => match operator {
            TypeBinaryOperator::Cast | TypeBinaryOperator::Satisfies => 500,
            _ => operator.precedence(),
        },

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

/// Return whether a parenthesized closure-cast style operand can drop wrappers.
fn redundant_parenthesized_closure_cast_operand_can_drop(
    context: &DestackFormatContext<'_>,
    parenthesized_id: LocalNodeId<Expression>,
    inner_expression_id: LocalNodeId<Expression>,
) -> bool {
    if !parenthesized_has_leading_inner_trivia(context, parenthesized_id, inner_expression_id) {
        return false;
    }

    let has_doc_like_prefix =
        context
            .annotation_ids(inner_expression_id)
            .iter()
            .any(|annotation_id| {
                matches!(
                    context.annotation(*annotation_id),
                    Annotation::Doc {
                        position: AnnotationPosition::BlockPrefix | AnnotationPosition::LinePrefix,
                        ..
                    }
                )
            });
    if !has_doc_like_prefix {
        return false;
    }

    matches!(
        context.tree.get(inner_expression_id),
        Expression::Path { .. }
            | Expression::Member { .. }
            | Expression::PrivateMember { .. }
            | Expression::ScalarLiteral(_)
            | Expression::TypeLiteral(_)
            | Expression::Call { .. }
            | Expression::Index { .. }
            | Expression::Instantiation { .. }
            | Expression::Maybe { .. }
            | Expression::Must { .. }
    )
}

/// Return whether a parenthesized binary operand can safely drop its wrapper.
fn redundant_parenthesized_binary_operand_can_drop(
    context: &DestackFormatContext<'_>,
    parent_operator: BinaryOperator,
    parenthesized_id: LocalNodeId<Expression>,
    inner_expression_id: LocalNodeId<Expression>,
) -> bool {
    let Expression::Binary {
        operator: inner_operator,
        ..
    } = context.tree.get(inner_expression_id)
    else {
        return false;
    };
    if expression_has_type_grouping_semantics(context, parenthesized_id)
        && is_type_grouping_binary_operator(parent_operator)
        && is_type_grouping_binary_operator(*inner_operator)
        && parent_operator != *inner_operator
    {
        return false;
    }

    let has_inline_closure_cast_prefix =
        parenthesized_has_leading_inner_trivia(context, parenthesized_id, inner_expression_id)
            && context
                .annotation_ids(inner_expression_id)
                .iter()
                .any(|annotation_id| {
                    matches!(
                        context.annotation(*annotation_id),
                        Annotation::Doc {
                            position: AnnotationPosition::BlockPrefix
                                | AnnotationPosition::LinePrefix,
                            ..
                        }
                    )
                });

    if (context.has_annotation(parenthesized_id) || context.has_annotation(inner_expression_id))
        && !has_inline_closure_cast_prefix
    {
        return false;
    }

    if parenthesized_has_leading_inner_trivia(context, parenthesized_id, inner_expression_id)
        && !has_inline_closure_cast_prefix
    {
        return false;
    }

    if !parenthesized_boundary_comments(context, parenthesized_id, inner_expression_id).is_empty() {
        return false;
    }

    !binary_operand_requires_grouping_parentheses(context, parent_operator, parenthesized_id)
}

/// Format a binary operand with grouping parentheses when needed in type-slot grouping.
pub(crate) fn format_binary_operand_with_grouping_parentheses<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    parent_operator: BinaryOperator,
    operand_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let mut operand_id = operand_id;
    let operand_has_annotation = f.context().has_annotation(operand_id);
    if let Expression::Parenthesized {
        expression: inner_expression_id,
    } = f.context().tree.get(operand_id)
    {
        let can_drop_for_binary = redundant_parenthesized_binary_operand_can_drop(
            f.context(),
            parent_operator,
            operand_id,
            *inner_expression_id,
        );
        let can_drop_for_closure_cast = redundant_parenthesized_closure_cast_operand_can_drop(
            f.context(),
            operand_id,
            *inner_expression_id,
        );
        if can_drop_for_binary || can_drop_for_closure_cast {
            operand_id = *inner_expression_id;
        }
    }

    let expression = f.context().tree.get(operand_id);
    let needs_type_grouping_parentheses =
        type_binary_operand_needs_grouping_parentheses(f.context(), parent_operator, operand_id);
    let suppress_precedence_parentheses_for_type_binary = matches!(
        (expression, parent_operator),
        (
            Expression::TypeBinary {
                operator: TypeBinaryOperator::Is
                    | TypeBinaryOperator::In
                    | TypeBinaryOperator::InstanceOf,
                ..
            },
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
        && (expression_precedence(expression)
            < binary_operator_expression_precedence(parent_operator)
            || binary_operand_requires_grouping_parentheses(
                f.context(),
                parent_operator,
                operand_id,
            ))
        && !suppress_precedence_parentheses_for_type_binary;
    let needs_grouping_parentheses = needs_type_grouping_parentheses
        || needs_precedence_parentheses
        || needs_mixed_logical_grouping_parentheses;
    let operand_has_prefix_annotation = f.context().has_prefix_annotation(operand_id);

    if needs_grouping_parentheses {
        if operand_has_prefix_annotation {
            write!(
                f,
                [
                    crate::format::annotation::prefix_annotations(f.context(), operand_id),
                    token("(")
                ]
            )?;
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
            operator: BinaryOperator::In | BinaryOperator::InstanceOf,
            ..
        } if *left == node_id
    ) && matches!(inner_expression, Expression::Unary { .. })
}

/// Return whether one binary operator is logical.
#[inline]
pub(crate) fn is_logical_binary_operator(operator: BinaryOperator) -> bool {
    matches!(
        operator,
        BinaryOperator::And | BinaryOperator::Or | BinaryOperator::Coalesce
    )
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

// union-specific prefix splitting now lives in operator/union.rs

/// Write one logical seam with trailing operator placement.
fn write_trailing_logical_non_head_seam<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    has_postfix: bool,
    left_postfix_comment_allows_space: bool,
    previous_requires_type_grouping_break: bool,
    previous_has_line_postfix_slash_comment: bool,
    root_operator: BinaryOperator,
    operand_operator: BinaryOperator,
    operand_expression: LocalNodeId<Expression>,
) -> FormatResult<()> {
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

    Ok(())
}

fn write_default_flattened_non_head_operand<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    root_operator: BinaryOperator,
    operand_operator: BinaryOperator,
    operand_expression: LocalNodeId<Expression>,
    previous_expression: Option<LocalNodeId<Expression>>,
) -> FormatResult<()> {
    let has_postfix = previous_expression
        .is_some_and(|expression_id| f.context().has_postfix_annotation(expression_id));
    let previous_has_prefix_annotation = previous_expression.is_some_and(|expression_id| {
        expression_has_leading_prefix_comment(f.context(), expression_id)
    });
    let previous_is_parenthesized_multiline = previous_expression.is_some_and(|expression_id| {
        matches!(
            f.context().tree.get(expression_id),
            Expression::Parenthesized { .. }
        ) && f.context().node_has_newline(expression_id)
    });
    let previous_has_line_postfix_slash_comment =
        previous_expression.is_some_and(|expression_id| {
            expression_has_line_postfix_slash_comment(f.context(), expression_id)
        });
    let previous_requires_type_grouping_break = false;
    let current_prefers_trailing_operator = is_logical_binary_operator(operand_operator)
        && expression_has_leading_prefix_comment(f.context(), operand_expression);
    let left_inline_block_postfix_comment_allows_space =
        previous_expression.is_some_and(|expression_id| {
            expression_has_inline_block_postfix_boundary_star_comment(f.context(), expression_id)
        });
    let left_postfix_comment_allows_space =
        left_inline_block_postfix_comment_allows_space && !previous_has_line_postfix_slash_comment;

    // logical operators that should trail current seams
    if current_prefers_trailing_operator {
        write_trailing_logical_non_head_seam(
            f,
            has_postfix,
            left_postfix_comment_allows_space,
            previous_requires_type_grouping_break,
            previous_has_line_postfix_slash_comment,
            root_operator,
            operand_operator,
            operand_expression,
        )?;

        return Ok(());
    }

    // seams after previous prefix annotations
    if previous_has_prefix_annotation {
        if !has_postfix || left_postfix_comment_allows_space {
            write!(f, [space()])?;
        } else if previous_requires_type_grouping_break || previous_has_line_postfix_slash_comment {
            write!(f, [hard_line_break()])?;
        }

        write!(f, [operand_operator, space()])?;
        format_binary_operand_with_grouping_parentheses(f, root_operator, operand_expression)?;

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

    write!(f, [indent(&seam_document)])
}

/// One local binary-like formatting owner.
struct BinaryLikeExpression {
    node_id: LocalNodeId<Expression>,
    operator: BinaryOperator,
    operands: SmallVec<[(Option<BinaryOperator>, LocalNodeId<Expression>); 8]>,
}

impl BinaryLikeExpression {
    /// Build one binary-like formatting owner.
    fn new(
        context: &DestackFormatContext<'_>,
        node_id: LocalNodeId<Expression>,
        operator: BinaryOperator,
    ) -> Self {
        let operands = flatten_binary_like_operands(context, node_id, operator);

        Self {
            node_id,
            operator,
            operands,
        }
    }

    /// Return whether this owner is one type union.
    fn is_type_union(&self, context: &DestackFormatContext<'_>) -> bool {
        binary_like_is_type_union(context, self.node_id, self.operator)
    }

    /// Return whether this owner is one type intersection.
    fn is_type_intersection(&self, context: &DestackFormatContext<'_>) -> bool {
        binary_like_is_type_intersection(context, self.node_id, self.operator)
    }

    /// Return whether this binary can use the clean short-circuit layout.
    fn can_use_clean_short_circuit(&self, context: &DestackFormatContext<'_>) -> bool {
        let has_node_annotation = context.has_annotation(self.node_id);
        let has_operand_annotations = self
            .operands
            .iter()
            .any(|operand| context.has_annotation(operand.1));
        let has_operand_prefix_comments = self
            .operands
            .iter()
            .any(|operand| expression_has_leading_prefix_comment(context, operand.1));

        !self.is_type_union(context)
            && !self.is_type_intersection(context)
            && !has_node_annotation
            && !has_operand_annotations
            && !has_operand_prefix_comments
    }

    /// Format this owner using the clean short-circuit layout.
    fn format_clean_short_circuit_layout<'ast>(
        &self,
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(
            f,
            [group(&format_with(|f: &mut DestackFormatter<'ast, '_>| {
                let Some(first_operand) = self.operands.first() else {
                    return Ok(());
                };
                format_binary_operand_with_grouping_parentheses(f, self.operator, first_operand.1)?;

                for operand in self.operands.iter().skip(1) {
                    let Some(operator) = operand.0 else {
                        continue;
                    };
                    write!(
                        f,
                        [
                            space(),
                            operator,
                            indent(&format_with(|f: &mut DestackFormatter<'ast, '_>| {
                                write!(f, [soft_line_break_or_space()])?;
                                format_binary_operand_with_grouping_parentheses(
                                    f,
                                    self.operator,
                                    operand.1,
                                )
                            }))
                        ]
                    )?;
                }

                Ok(())
            }))]
        )
    }

    /// Format this owner using the default flattened layout.
    fn format_default_flattened_layout<'ast>(
        &self,
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(
            f,
            [group(&format_with(|f: &mut DestackFormatter<'ast, '_>| {
                let mut previous_expression = None;

                for operand in &self.operands {
                    if let Some(operand_operator) = operand.0 {
                        write_default_flattened_non_head_operand(
                            f,
                            self.operator,
                            operand_operator,
                            operand.1,
                            previous_expression,
                        )?;
                    } else {
                        format_binary_operand_with_grouping_parentheses(
                            f,
                            self.operator,
                            operand.1,
                        )?;
                    }

                    previous_expression = Some(operand.1);
                }

                Ok(())
            }))
            .should_expand(false)]
        )?;

        Ok(())
    }

    /// Try formatting `??` using trailing-operator layout.
    fn format_trailing_coalesce_case<'ast>(
        &self,
        f: &mut DestackFormatter<'ast, '_>,
        left: LocalNodeId<Expression>,
        right: LocalNodeId<Expression>,
    ) -> FormatResult<bool> {
        if self.operator != BinaryOperator::Coalesce
            || !should_use_trailing_coalesce(f.context(), self.node_id, left)
        {
            return Ok(false);
        }

        write!(
            f,
            [group(&format_args![
                left,
                indent(&format_with(|f| {
                    write_space_after_binary_left_if_needed(f, left, self.operator)?;
                    write!(
                        f,
                        [
                            self.operator,
                            indent(&format_args![soft_line_break_or_space(), right])
                        ]
                    )
                }))
            ])]
        )?;

        Ok(true)
    }

    /// Try formatting logical operators with a right-side line-prefix comment seam.
    fn format_logical_prefix_line_comment_case<'ast>(
        &self,
        f: &mut DestackFormatter<'ast, '_>,
        left: LocalNodeId<Expression>,
        right: LocalNodeId<Expression>,
    ) -> FormatResult<bool> {
        if !is_logical_binary_operator(self.operator)
            || !expression_has_line_prefix_slash_comment(f.context(), right)
        {
            return Ok(false);
        }

        write!(
            f,
            [group(&format_args![
                left,
                format_with(|f| write_space_after_binary_left_if_needed(f, left, self.operator)),
                self.operator,
                space(),
                indent(&format_args![right])
            ])]
        )?;

        Ok(true)
    }

    /// Try formatting logical operators with an inline right-side block-prefix comment seam.
    fn format_logical_prefix_block_comment_case<'ast>(
        &self,
        f: &mut DestackFormatter<'ast, '_>,
        left: LocalNodeId<Expression>,
        right: LocalNodeId<Expression>,
    ) -> FormatResult<bool> {
        if !is_logical_binary_operator(self.operator)
            || !expression_has_inline_block_prefix_star_comment(f.context(), right)
            || self.operands.len() > 2
        {
            return Ok(false);
        }

        write!(
            f,
            [group(&format_args![
                left,
                format_with(|f| write_space_after_binary_left_if_needed(f, left, self.operator)),
                self.operator,
                indent(&format_args![soft_line_break_or_space(), right])
            ])]
        )?;

        Ok(true)
    }

    /// Try formatting mixed logical precedence pairs with explicit right parentheses.
    fn format_mixed_logical_precedence_case<'ast>(
        &self,
        f: &mut DestackFormatter<'ast, '_>,
        left: LocalNodeId<Expression>,
        right: LocalNodeId<Expression>,
    ) -> FormatResult<bool> {
        let Expression::Binary {
            operator: right_operator,
            ..
        } = f.context().tree.get(right)
        else {
            return Ok(false);
        };
        if !is_mixed_logical_precedence_pair(self.operator, *right_operator) {
            return Ok(false);
        }

        let right_span = f.context().span(right);
        let has_right_comments = !f
            .context()
            .comments_in_range(right_span.start, right_span.end)
            .is_empty();
        let should_preserve_grouping_for_comments =
            has_right_comments || has_comment_between_expressions(f.context(), left, right);
        if !should_preserve_grouping_for_comments {
            return Ok(false);
        }

        write!(
            f,
            [group(&format_args![
                left,
                format_with(|f| write_space_after_binary_left_if_needed(f, left, self.operator)),
                self.operator,
                space(),
                token("("),
                right,
                token(")")
            ])]
        )?;

        Ok(true)
    }

    /// Try formatting logical expressions with parenthesized-tail policies.
    fn format_logical_parenthesized_case<'ast>(
        &self,
        f: &mut DestackFormatter<'ast, '_>,
        left: LocalNodeId<Expression>,
        right: LocalNodeId<Expression>,
    ) -> FormatResult<bool> {
        if !is_logical_binary_operator(self.operator) {
            return Ok(false);
        }

        let left_span = f.context().span(left);
        let left_has_multiline_parenthesized_tail = f.context().has_newline(left_span)
            && f.context()
                .last_non_trivia_token_in_span(left_span)
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
            && self.operator == BinaryOperator::And
        {
            write!(
                f,
                [group(&format_args![
                    left,
                    format_with(|f| {
                        write_space_after_binary_left_if_needed(f, left, self.operator)
                    }),
                    self.operator,
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
                    format_with(|f| {
                        write_space_after_binary_left_if_needed(f, left, self.operator)
                    }),
                    self.operator,
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
                    format_with(|f| {
                        write_space_after_binary_left_if_needed(f, left, self.operator)
                    }),
                    self.operator,
                    space(),
                    right
                ])]
            )?;
            return Ok(true);
        }

        Ok(false)
    }

    /// Format this binary-like owner using the closest local analogue of the OXC owner flow.
    fn fmt<'ast>(
        &self,
        f: &mut DestackFormatter<'ast, '_>,
        left: LocalNodeId<Expression>,
        right: LocalNodeId<Expression>,
    ) -> FormatResult<()> {
        // specialized owner-local paths
        if self.format_trailing_coalesce_case(f, left, right)?
            || self.format_logical_prefix_block_comment_case(f, left, right)?
            || self.format_logical_prefix_line_comment_case(f, left, right)?
            || self.format_mixed_logical_precedence_case(f, left, right)?
            || self.format_logical_parenthesized_case(f, left, right)?
        {
            return Ok(());
        }

        // clean non-type binaries without annotation or prefix trivia
        if self.can_use_clean_short_circuit(f.context()) {
            self.format_clean_short_circuit_layout(f)?;
            return Ok(());
        }

        // union types use one deterministic layout path: leading-pipe group
        if self.is_type_union(f.context()) {
            super::format_type_union_binary_layout(
                f,
                self.node_id,
                &self.operands,
                is_in_type_template_literal_interpolation(f.context(), self.node_id),
                f.context().has_annotation(self.node_id),
                self.operands
                    .iter()
                    .any(|operand| f.context().has_annotation(operand.1)),
                self.operands
                    .iter()
                    .any(|operand| expression_has_leading_prefix_comment(f.context(), operand.1)),
            )?;
            return Ok(());
        }

        // destack intersections use trailing operator layout
        if self.is_type_intersection(f.context()) {
            format_type_intersection_binary_layout(
                f,
                self.node_id,
                &self.operands,
                f.context().options.language_type.is_destack(),
            )?;
            return Ok(());
        }

        // default flattened binary formatting
        self.format_default_flattened_layout(f)?;

        Ok(())
    }
}

/// Format a binary expression with all operator-specific layout policies.
pub(crate) fn format_binary_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    left: LocalNodeId<Expression>,
    operator: &BinaryOperator,
    right: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let binary_like = BinaryLikeExpression::new(f.context(), node_id, *operator);
    binary_like.fmt(f, left, right)
}
