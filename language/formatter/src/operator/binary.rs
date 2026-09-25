use crate::chain::transparent_inner_expression;
use crate::context::{TsppFormatterSpeculationExt, with_following_span_start};
use crate::expression::is_control_expression;
use crate::{TsppFormatContext, TsppFormatter};
use smallvec::SmallVec;
use tspp_dir::{
    Argument, BinaryOperator, Expression, IfForm, LocalNodeId, Member, NodeType, Property,
};
use tspp_fir::format::{Format, FormatResult, Formatter as FirFormatter};
use tspp_fir::prelude::{
    format_with, group, soft_block_indent, soft_line_break_or_space, soft_line_indent_or_space,
    space,
};
use tspp_fir::write;
use tspp_source::{NodeSpanBoundary, NodeSpanType, Span};

type BinarySideList = SmallVec<[BinarySide; 8]>;

/// Return the trailing trivia gap after one expression.
fn binary_expression_postfix_gap(
    context: &TsppFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> Option<Span> {
    let expression_span = context.span(expression_id);
    let gap_end = if let Some(next_token) = context.next_token_after_span(expression_span) {
        if next_token.span.file != expression_span.file
            || next_token.span.start <= expression_span.end
        {
            return None;
        }

        next_token.span.start
    } else {
        let (line_index, _) = context.file.get_position(expression_span.end)?;
        let line_span = context.file.get_line_span(line_index)?;
        line_span.end
    };

    (gap_end > expression_span.end)
        .then(|| Span::new(expression_span.file, expression_span.end, gap_end))
}

/// Return whether one expression ends with a line postfix comment.
fn binary_expression_has_line_suffix_comment(
    context: &TsppFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let Some(gap_span) = binary_expression_postfix_gap(context, expression_id) else {
        return false;
    };

    context
        .source_comments_in_range(gap_span.start, gap_span.end)
        .iter()
        .copied()
        .any(|comment| comment.is_line())
}

/// Return whether one expression ends with an inline block postfix comment.
fn binary_expression_has_inline_block_postfix_comment(
    context: &TsppFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let Some(gap_span) = binary_expression_postfix_gap(context, expression_id) else {
        return false;
    };
    if context.has_newline(gap_span) {
        return false;
    }

    context
        .source_comments_in_range(gap_span.start, gap_span.end)
        .iter()
        .any(|comment| comment.is_block())
}

/// Return whether an internal line comment should keep the operator beside the left operand.
fn binary_left_keeps_operator_inline(
    context: &TsppFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    if matches!(context.tree.get(expression_id), Expression::Binary { .. }) {
        return false;
    }

    let Some(leading_span) = context.tree.get_side_span(
        expression_id,
        NodeSpanType::Boundary(NodeSpanBoundary::Leading),
    ) else {
        return false;
    };
    let expression_start = context.expression_token_start(expression_id);

    context
        .source_comments_in_range(leading_span.start, expression_start)
        .iter()
        .any(|comment| comment.is_line())
}

/// Return whether one operand is a control expression with an expanded body.
fn binary_operand_is_control(
    context: &TsppFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let expression_id = transparent_inner_expression(context, expression_id);

    is_control_expression(context.tree.get(expression_id))
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
        BinaryOperator::Multiply | BinaryOperator::Divide | BinaryOperator::Remainder
    )
}

/// Return whether one operator belongs to the shift family.
#[inline]
fn binary_operator_is_shift(operator: BinaryOperator) -> bool {
    matches!(
        operator,
        BinaryOperator::ShiftLeft | BinaryOperator::ShiftRight | BinaryOperator::UnsignedShiftRight
    )
}

/// Return whether one operator is a remainder operator.
#[inline]
fn binary_operator_is_remainder(operator: BinaryOperator) -> bool {
    operator == BinaryOperator::Remainder
}

/// Return whether nested binaries should flatten into one chain.
#[inline]
fn should_flatten_binary(parent_operator: BinaryOperator, operator: BinaryOperator) -> bool {
    let parent_precedence = parent_operator.precedence();
    let precedence = operator.precedence();

    if parent_precedence != precedence {
        return false;
    }

    if matches!(parent_operator, BinaryOperator::Exponent) {
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

/// Return whether one binary operator is logical.
#[inline]
pub(crate) fn is_logical_binary_operator(operator: BinaryOperator) -> bool {
    matches!(
        operator,
        BinaryOperator::And | BinaryOperator::Or | BinaryOperator::Coalesce
    )
}

/// Return whether one expression is the same binary kind as the current expression.
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

/// Return whether an infix operator needs its own separator after the left operand.
fn binary_operator_needs_separator(
    context: &TsppFormatContext<'_>,
    left: LocalNodeId<Expression>,
    operator: BinaryOperator,
) -> bool {
    let allow_inline_block_postfix_space =
        binary_expression_has_inline_block_postfix_comment(context, left);
    let allow_logical_space_after_line_comment = is_logical_binary_operator(operator)
        && binary_expression_has_line_suffix_comment(context, left);

    if context.has_postfix_annotation(left)
        && !allow_logical_space_after_line_comment
        && !allow_inline_block_postfix_space
    {
        return false;
    }

    true
}

/// One binary operation.
#[derive(Debug, Clone, Copy)]
struct BinaryOperation {
    /// The binary expression node.
    node_id: LocalNodeId<Expression>,
    /// The left operand.
    left: LocalNodeId<Expression>,
    /// The operator.
    operator: BinaryOperator,
    /// The right operand.
    right: LocalNodeId<Expression>,
}

impl BinaryOperation {
    /// Create one binary operation.
    #[inline]
    fn new(
        node_id: LocalNodeId<Expression>,
        left: LocalNodeId<Expression>,
        operator: BinaryOperator,
        right: LocalNodeId<Expression>,
    ) -> Self {
        Self {
            node_id,
            left,
            operator,
            right,
        }
    }

    /// Return the left operand.
    fn left(self) -> LocalNodeId<Expression> {
        self.left
    }

    /// Return the right operand.
    fn right(self) -> LocalNodeId<Expression> {
        self.right
    }

    /// Return the operator.
    fn operator(self) -> BinaryOperator {
        self.operator
    }

    /// Return the flattenable binary expression on the left.
    fn flattened_left(self, context: &TsppFormatContext<'_>) -> Option<Self> {
        let Expression::Binary {
            left,
            operator,
            right,
        } = context.tree.get(self.left)
        else {
            return None;
        };

        should_flatten_binary(self.operator, *operator)
            .then(|| Self::new(self.left, *left, *operator, *right))
    }

    /// Return whether this expression is inside one test condition.
    fn is_inside_condition(self, context: &TsppFormatContext<'_>) -> bool {
        let Some(parent_id) = context.expression_parent(self.node_id) else {
            return false;
        };

        match context.tree.get(parent_id) {
            Expression::If {
                form: IfForm::If,
                condition,
                ..
            } => condition.as_expression() == Some(self.node_id),
            Expression::While { condition, .. } => condition.as_expression() == Some(self.node_id),
            Expression::For { condition, .. } => {
                condition.is_some_and(|condition| condition == self.node_id)
            }
            Expression::Switch { value, .. } => *value == self.node_id,
            _ => false,
        }
    }

    /// Return whether a logical chain should keep its right side inline.
    fn should_inline_logical_expression(self, context: &TsppFormatContext<'_>) -> bool {
        if !is_logical_binary_operator(self.operator()) {
            return false;
        }

        match context.tree.get(self.right()) {
            Expression::ObjectExpression { properties, .. }
            | Expression::StructExpression { properties, .. } => !properties.is_empty(),
            Expression::ArrayExpression { elements } => !elements.is_empty(),
            Expression::TreeExpression { .. } => true,
            _ => false,
        }
    }

    /// Return whether one ternary parent already owns indentation.
    fn ternary_parent_owns_indentation(
        context: &TsppFormatContext<'_>,
        ternary_expression_id: LocalNodeId<Expression>,
    ) -> bool {
        let Some((parent_id, parent_type)) = context.parent(ternary_expression_id) else {
            return true;
        };

        if parent_type == NodeType::Argument {
            let argument_id = LocalNodeId::<Argument>::new(parent_id);
            let argument_is_value = matches!(
                context.tree.get(argument_id),
                Argument::Positional { value } if *value == ternary_expression_id
            );
            if !argument_is_value {
                return true;
            }

            let Some((parent_id, NodeType::Expression)) = context.parent(argument_id) else {
                return true;
            };

            let parent_expression_id = LocalNodeId::<Expression>::new(parent_id);

            return !matches!(
                context.tree.get(parent_expression_id),
                Expression::Call { .. }
                    | Expression::New { .. }
                    | Expression::Import { .. }
                    | Expression::ImportMeta
                    | Expression::ImportSource
            );
        }

        if parent_type != NodeType::Expression {
            return true;
        }

        let parent_expression_id = LocalNodeId::<Expression>::new(parent_id);

        !matches!(
            context.tree.get(parent_expression_id),
            Expression::Return { .. }
                | Expression::Call { .. }
                | Expression::New { .. }
                | Expression::Import { .. }
                | Expression::ImportMeta
                | Expression::ImportSource
        )
    }

    /// Return whether the parent already owns indentation.
    fn should_not_indent_if_parent_indents(self, context: &TsppFormatContext<'_>) -> bool {
        let Some(parent_id) = context.expression_parent(self.node_id) else {
            return false;
        };

        match context.tree.get(parent_id) {
            Expression::Return { value } => value.is_some_and(|value| value == self.node_id),
            Expression::For { condition, .. } => {
                condition.is_some_and(|condition| condition == self.node_id)
            }
            Expression::Call {
                left, arguments, ..
            } => {
                *left == self.node_id
                    || (arguments.len() == 1
                        && arguments.iter().copied().any(|argument_id| {
                            matches!(
                                context.tree.get(argument_id),
                                Argument::Positional { value } if *value == self.node_id
                            )
                        })
                        && matches!(
                            context.tree.get(*left),
                            Expression::Identifier { name } if context.strings.get(*name) == "Boolean"
                        ))
            }
            Expression::If {
                form: IfForm::Ternary,
                ..
            } => Self::ternary_parent_owns_indentation(context, parent_id),
            _ => false,
        }
    }
}

/// One side in a flattened binary chain.
#[derive(Debug, Clone, Copy)]
enum BinarySide {
    /// The terminal left side.
    Left {
        /// The current parent chain node.
        parent: BinaryOperation,
    },

    /// One operator plus right operand.
    Right {
        /// The current parent chain node.
        parent: BinaryOperation,
        /// Whether the containing chain is in condition position.
        inside_condition: bool,
    },
}

impl BinarySide {
    /// Return whether this side is a tree expression.
    fn is_tree(self, context: &TsppFormatContext<'_>) -> bool {
        let expression_id = match self {
            Self::Left { parent } => parent.left(),
            Self::Right { parent, .. } => parent.right(),
        };

        matches!(
            context.tree.get(expression_id),
            Expression::TreeExpression { .. }
        )
    }
}

impl<'a> Format<'a, TsppFormatContext<'a>> for BinarySide {
    fn format(&self, f: &mut FirFormatter<'_, 'a, TsppFormatContext<'a>>) -> FormatResult<()> {
        match self {
            // left side
            Self::Left { parent } => {
                let (left, following_span_start) = {
                    let context = f.context();
                    let left = parent.left();
                    let right = parent.right();

                    (left, context.span(right).start)
                };

                with_following_span_start(f, Some(following_span_start), |f| {
                    write!(f, [group(&left)])
                })
            }

            // operator and right side
            Self::Right {
                parent,
                inside_condition,
            } => {
                // source shape
                let (
                    right,
                    operator,
                    left_is_same_kind,
                    right_is_same_kind,
                    parent_is_same_kind,
                    should_break,
                ) = {
                    let context = f.context();
                    let left = parent.left();
                    let right = parent.right();
                    let operator = parent.operator();
                    let left_is_same_kind =
                        expression_is_same_binary_kind(context.tree.get(left), operator);
                    let right_is_same_kind =
                        expression_is_same_binary_kind(context.tree.get(right), operator);
                    let parent_is_same_kind = context.parent(parent.node_id).is_some_and(
                        |(grand_parent_id, grand_parent_type)| {
                            grand_parent_type == NodeType::Expression
                                && expression_is_same_binary_kind(
                                    context
                                        .tree
                                        .get(LocalNodeId::<Expression>::new(grand_parent_id)),
                                    operator,
                                )
                        },
                    );
                    let should_break = binary_expression_has_line_suffix_comment(context, left);

                    (
                        right,
                        operator,
                        left_is_same_kind,
                        right_is_same_kind,
                        parent_is_same_kind,
                        should_break,
                    )
                };

                let left = parent.left();
                let right_will_break = f.speculate_will_break_after(
                    f.context().expression_token_start(right),
                    &right,
                )?;
                let right_is_control = binary_operand_is_control(f.context(), right);
                let needs_separator = binary_operator_needs_separator(f.context(), left, operator);
                let separator_can_break = !binary_left_keeps_operator_inline(f.context(), left);
                let operator_and_right = format_with(|f: &mut TsppFormatter<'a, '_>| {
                    write!(f, [operator, space()])?;

                    // inline logical rhs
                    let should_inline = parent.should_inline_logical_expression(f.context())
                        && !matches!(
                            f.context().tree.get(right),
                            Expression::TreeExpression { .. }
                        )
                        && f.context()
                            .comments()
                            .has_leading_own_line_comment(f.context().span(right).start);
                    if should_inline {
                        write!(f, [soft_line_indent_or_space(&right)])?;

                        return Ok(());
                    }

                    write!(f, [right])
                });

                // group boundaries
                let should_group = !(parent_is_same_kind
                    || left_is_same_kind
                    || right_is_same_kind
                    || (*inside_condition && is_logical_binary_operator(operator)));

                // grouped operator layout
                if should_group {
                    let operator_and_right = group(&operator_and_right)
                        .should_expand(should_break || right_will_break || right_is_control);

                    if needs_separator && separator_can_break {
                        return write!(f, [soft_line_indent_or_space(&operator_and_right)]);
                    }

                    if needs_separator {
                        return write!(f, [space(), operator_and_right]);
                    }

                    return write!(f, [operator_and_right]);
                }

                if needs_separator && separator_can_break {
                    return write!(f, [soft_line_indent_or_space(&operator_and_right)]);
                }

                if needs_separator {
                    return write!(f, [space(), operator_and_right]);
                }

                write!(f, [operator_and_right])
            }
        }
    }
}

/// Collect one left-associative binary chain into printable sides.
fn collect_binary_chain_sides(
    binary: BinaryOperation,
    inside_condition: bool,
    context: &TsppFormatContext<'_>,
    items: &mut BinarySideList,
) {
    let mut ancestors = SmallVec::<[BinaryOperation; 8]>::new();
    let mut current = binary;

    // walk to the terminal left operand
    while let Some(left) = current.flattened_left(context) {
        ancestors.push(current);
        current = left;
    }

    // emit the chain in source order
    items.reserve(ancestors.len() + 2);
    items.push(BinarySide::Left { parent: current });
    items.push(BinarySide::Right {
        parent: current,
        inside_condition,
    });

    while let Some(parent) = ancestors.pop() {
        items.push(BinarySide::Right {
            parent,
            inside_condition,
        });
    }
}

/// Write one collected binary chain.
fn write_binary_chain_sides<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    sides: &[BinarySide],
) -> FormatResult<()> {
    for side in sides {
        write!(f, [*side])?;
    }

    Ok(())
}

/// Return whether this binary root is already owned by an outer indentation layout.
fn binary_parent_inlines_flattened_layout(
    context: &TsppFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let Some((parent_id, parent_type)) = context.parent(expression_id) else {
        return false;
    };

    match parent_type {
        // declarators own the outer indent
        NodeType::Declarator => true,

        // assignment rhs owns the outer indent
        NodeType::Expression => matches!(
            context.tree.get(LocalNodeId::<Expression>::new(parent_id)),
            Expression::Assign { right, .. } if right.id == expression_id.id
        ),

        // object field values own the outer indent
        NodeType::Property => matches!(
            context.tree.get(LocalNodeId::<Property>::new(parent_id)),
            Property::Field { value, .. } if *value == expression_id
        ),

        // class field initializers own the outer indent
        NodeType::Member => matches!(
            context.tree.get(LocalNodeId::<Member>::new(parent_id)),
            Member::Field { default, .. } if default.is_some_and(|default| default == expression_id)
        ),

        _ => false,
    }
}

/// Return whether the current expression sits in a parenthesized callee or object position.
fn binary_expression_is_inside_parenthesis_context(
    context: &TsppFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let Some(parent_id) = context.expression_parent(expression_id) else {
        return false;
    };

    match context.tree.get(parent_id) {
        Expression::Unary { right, .. } => *right == expression_id,
        Expression::Member { left, .. }
        | Expression::Index { left, .. }
        | Expression::Call { left, .. }
        | Expression::Instantiation { left, .. }
        | Expression::Maybe { left, .. }
        | Expression::Must { left, .. }
        | Expression::Chain { expression: left } => *left == expression_id,
        Expression::TaggedTemplateExpression { tag, .. } => *tag == expression_id,
        _ => false,
    }
}

/// Format one binary expression.
pub(crate) fn format_binary_expression<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    left: LocalNodeId<Expression>,
    operator: &BinaryOperator,
    right: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let binary = BinaryOperation::new(node_id, left, *operator, right);
    let is_inside_condition = binary.is_inside_condition(f.context());
    let mut sides = BinarySideList::new();
    collect_binary_chain_sides(binary, is_inside_condition, f.context(), &mut sides);

    // condition position
    if is_inside_condition {
        return write_binary_chain_sides(f, &sides);
    }

    // parenthesized callee or object position
    if binary_expression_is_inside_parenthesis_context(f.context(), node_id) {
        return write!(
            f,
            [group(&soft_block_indent(&format_with(
                |f: &mut TsppFormatter<'ast, '_>| { write_binary_chain_sides(f, &sides) }
            )))]
        );
    }

    // parent-owned indent
    if binary.should_not_indent_if_parent_indents(f.context()) {
        return write!(
            f,
            [group(&format_with(|f: &mut TsppFormatter<'ast, '_>| {
                write_binary_chain_sides(f, &sides)
            }))]
        );
    }

    let should_inline_logical = binary.should_inline_logical_expression(f.context());
    let should_indent_if_parent_inlines =
        binary_parent_inlines_flattened_layout(f.context(), node_id);
    let is_flattened = sides.len() > 2;

    // direct grouped layout
    if (should_inline_logical && !is_flattened)
        || (!should_inline_logical && should_indent_if_parent_inlines)
    {
        return write!(
            f,
            [group(&format_with(|f: &mut TsppFormatter<'ast, '_>| {
                write_binary_chain_sides(f, &sides)
            }))]
        );
    }

    let first = sides[0];
    let last_is_tree = sides.last().is_some_and(|side| side.is_tree(f.context()));
    let tail_end = if last_is_tree {
        sides.len().saturating_sub(1)
    } else {
        sides.len()
    };
    let tail = &sides[1..tail_end];
    let group_id = f.group_id();

    let format_non_tree_parts = format_with(|f: &mut TsppFormatter<'ast, '_>| {
        write!(
            f,
            [group(&format_with(|f: &mut TsppFormatter<'ast, '_>| {
                write!(f, [first])?;

                for part in tail {
                    write!(f, [*part])?;
                }

                Ok(())
            }))
            .with_id(Some(group_id))]
        )
    });

    // tree tail
    if last_is_tree {
        let tree_tail = sides[sides.len() - 1];

        return write!(
            f,
            [group(&format_with(|f: &mut TsppFormatter<'ast, '_>| {
                write!(f, [format_non_tree_parts])?;
                write!(f, [soft_line_break_or_space(), tree_tail])?;

                Ok(())
            }))]
        );
    }

    write!(f, [format_non_tree_parts])
}
