use crate::format::annotation::prefix_annotations;
use crate::format::expression::{
    expression_has_leading_prefix_comment, expression_is_trivial_inline_without_annotations,
    write_expression_without_prefix_annotations,
};
use crate::format::operator::{
    binary_like_is_type_intersection, binary_like_is_type_union, flatten_binary_like_operands,
    format_type_intersection_binary_layout, type_binary_operand_needs_grouping_parentheses,
};
use crate::{DestackFormatContext, DestackFormatter};
use destack_ast::{
    BinaryOperator, Expression, IfKind, LocalNodeId, NodeType, OperatorPrecedence, TokenType,
    TypeBinaryOperator, UnaryOperator,
};
use destack_fir::format::{Buffer, FormatResult};
use destack_fir::prelude::{
    block_indent, format_with, group, hard_line_break, indent, soft_line_break_or_space, space,
    token,
};
use destack_fir::{format_args, write};
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

/// Return the prefix trivia gap before one expression.
fn binary_expression_prefix_gap(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> Option<Span> {
    let expression_span = context.span(expression_id);
    let previous_token = context.previous_non_whitespace_token_before_span(expression_span)?;
    if previous_token.span.file != expression_span.file
        || previous_token.span.end >= expression_span.start
    {
        return None;
    }

    Some(Span::new(
        expression_span.file,
        previous_token.span.end,
        expression_span.start,
    ))
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

/// Return whether one expression starts with a line prefix comment.
fn binary_expression_has_line_prefix_comment(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let Some(gap_span) = binary_expression_prefix_gap(context, expression_id) else {
        return false;
    };

    context
        .comments_in_range(gap_span.start, gap_span.end)
        .iter()
        .copied()
        .any(|comment| context.comment_is_line(comment))
}

/// Return whether one expression starts with an inline block prefix comment.
fn binary_expression_has_inline_block_prefix_comment(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let Some(gap_span) = binary_expression_prefix_gap(context, expression_id) else {
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

    let parent_precedence = parent_operator.precedence_group() as u16;
    let operand_precedence = operand_operator.precedence_group() as u16;
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
        Expression::Binary { operator, .. } => operator.precedence_group() as u16,
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

/// Format a binary operand with grouping parentheses when needed in type-slot grouping.
pub(crate) fn format_binary_operand_with_grouping_parentheses<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    parent_operator: BinaryOperator,
    operand_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let operand_has_annotation = f.context().has_annotation(operand_id);
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
        && (expression_precedence(expression) < parent_operator.precedence_group() as u16
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
                        self.write_flattened_operand(
                            f,
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

    /// Write one non-head operand in the flattened binary chain.
    fn write_flattened_operand<'ast>(
        &self,
        f: &mut DestackFormatter<'ast, '_>,
        operand_operator: BinaryOperator,
        operand_expression: LocalNodeId<Expression>,
        previous_expression: Option<LocalNodeId<Expression>>,
    ) -> FormatResult<()> {
        let has_postfix = previous_expression
            .is_some_and(|expression_id| f.context().has_postfix_annotation(expression_id));
        let previous_has_prefix_annotation = previous_expression.is_some_and(|expression_id| {
            expression_has_leading_prefix_comment(f.context(), expression_id)
        });
        let previous_is_parenthesized_multiline =
            previous_expression.is_some_and(|expression_id| {
                matches!(
                    f.context().tree.get(expression_id),
                    Expression::Parenthesized { .. }
                ) && f.context().node_has_newline(expression_id)
            });
        let previous_has_line_suffix_slash_comment =
            previous_expression.is_some_and(|expression_id| {
                binary_expression_has_line_suffix_comment(f.context(), expression_id)
            });
        let current_prefers_trailing_operator = is_logical_binary_operator(operand_operator)
            && expression_has_leading_prefix_comment(f.context(), operand_expression);
        let left_inline_block_postfix_comment_allows_space =
            previous_expression.is_some_and(|expression_id| {
                binary_expression_has_inline_block_postfix_comment(f.context(), expression_id)
            });
        let left_postfix_comment_allows_space = left_inline_block_postfix_comment_allows_space
            && !previous_has_line_suffix_slash_comment;

        // logical operators that should trail the current operand
        if current_prefers_trailing_operator {
            if !has_postfix || left_postfix_comment_allows_space {
                write!(f, [space()])?;
            } else if previous_has_line_suffix_slash_comment {
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
                                self.operator,
                                operand_expression,
                            )
                        })
                    ])
                ]
            )?;

            return Ok(());
        }

        // spacing after previous prefix annotations
        if previous_has_prefix_annotation {
            if !has_postfix || left_postfix_comment_allows_space {
                write!(f, [space()])?;
            } else if previous_has_line_suffix_slash_comment {
                write!(f, [hard_line_break()])?;
            }

            write!(f, [operand_operator, space()])?;
            format_binary_operand_with_grouping_parentheses(f, self.operator, operand_expression)?;

            return Ok(());
        }

        // generic boundary rendering
        let operator_document = format_with(|f: &mut DestackFormatter<'ast, '_>| {
            if !has_postfix {
                if previous_is_parenthesized_multiline
                    && is_logical_binary_operator(operand_operator)
                {
                    write!(f, [space()])?;
                } else {
                    write!(f, [soft_line_break_or_space()])?;
                }
            } else if left_postfix_comment_allows_space {
                write!(f, [space()])?;
            } else if previous_has_line_suffix_slash_comment {
                write!(f, [hard_line_break()])?;
            }

            write!(f, [operand_operator, space()])?;
            format_binary_operand_with_grouping_parentheses(f, self.operator, operand_expression)
        });

        write!(f, [indent(&operator_document)])
    }
    /// Try formatting logical operators with a right-side line-prefix comment boundary.
    fn format_logical_prefix_line_comment_case<'ast>(
        &self,
        f: &mut DestackFormatter<'ast, '_>,
        left: LocalNodeId<Expression>,
        right: LocalNodeId<Expression>,
    ) -> FormatResult<bool> {
        if !is_logical_binary_operator(self.operator)
            || !binary_expression_has_line_prefix_comment(f.context(), right)
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

    /// Try formatting logical operators with an inline right-side block-prefix comment boundary.
    fn format_logical_prefix_block_comment_case<'ast>(
        &self,
        f: &mut DestackFormatter<'ast, '_>,
        left: LocalNodeId<Expression>,
        right: LocalNodeId<Expression>,
    ) -> FormatResult<bool> {
        if !is_logical_binary_operator(self.operator)
            || !binary_expression_has_inline_block_prefix_comment(f.context(), right)
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

    /// Try formatting multiline parenthesized `&&` tails with the rhs on the next line.
    fn format_logical_multiline_parenthesized_tail<'ast>(
        &self,
        f: &mut DestackFormatter<'ast, '_>,
        left: LocalNodeId<Expression>,
        right: LocalNodeId<Expression>,
    ) -> FormatResult<bool> {
        if self.operator != BinaryOperator::And {
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

        if !left_has_multiline_parenthesized_tail || !right_is_inline_trivial || right_has_prefix {
            return Ok(false);
        }

        write!(
            f,
            [group(&format_args![
                left,
                format_with(|f| write_space_after_binary_left_if_needed(f, left, self.operator)),
                self.operator,
                indent(&format_args![hard_line_break(), right])
            ])]
        )?;

        Ok(true)
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

    if binary_like.format_logical_multiline_parenthesized_tail(f, left, right)?
        || binary_like.format_logical_prefix_block_comment_case(f, left, right)?
        || binary_like.format_logical_prefix_line_comment_case(f, left, right)?
    {
        return Ok(());
    }

    if binary_like.is_type_union(f.context()) {
        super::format_type_union_binary_layout(f, node_id, &binary_like.operands)?;
        return Ok(());
    }

    if binary_like.is_type_intersection(f.context()) {
        format_type_intersection_binary_layout(f, &binary_like.operands)?;
        return Ok(());
    }

    binary_like.format_default_flattened_layout(f)
}
