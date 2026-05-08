use crate::context::with_following_span_start;
use crate::{DestackFormatContext, DestackFormatter};
use destack_ast::{
    Argument, BinaryOperator, Expression, IfCondition, IfForm, LocalNodeId, MatchForm, Member,
    NodeType, OperatorPrecedence, Property, TokenType,
};
use destack_fir::format::{Buffer, Format, FormatResult, Formatter as FirFormatter};
use destack_fir::prelude::{
    format_with, group, indent, soft_block_indent, soft_line_break_or_space,
    soft_line_indent_or_space, space,
};
use destack_fir::write;
use destack_source::Span;

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
            let (line_index, _) = context.file.get_position(expression_span.end)?;
            let line_span = context.file.get_line_span(line_index)?;
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
        .comment_tokens_in_range(gap_span.start, gap_span.end)
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

/// Return the precedence value used by binary expression formatting.
#[inline]
pub(crate) fn binary_operator_format_precedence(operator: BinaryOperator) -> u16 {
    if is_logical_binary_operator(operator) {
        return operator.precedence();
    }

    operator.precedence_group() as u16
}

/// Return whether nested binaries should flatten into one chain.
#[inline]
pub(crate) fn should_flatten_binary(
    parent_operator: BinaryOperator,
    operator: BinaryOperator,
) -> bool {
    let parent_precedence = binary_operator_format_precedence(parent_operator);
    let precedence = binary_operator_format_precedence(operator);

    if parent_precedence != precedence {
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
        | Expression::Comptime { .. }
        | Expression::Yield { .. }
        | Expression::MoveOf { .. }
        | Expression::BorrowOf { .. }
        | Expression::Delete { .. }
        | Expression::Throw { .. }
        | Expression::Return { .. } => OperatorPrecedence::Prefix as u16,

        // binary
        Expression::Binary { operator, .. } => binary_operator_format_precedence(*operator),
        Expression::As { .. } | Expression::Satisfies { .. } => u16::MAX,
        Expression::Is { .. } | Expression::InstanceOf { .. } => {
            OperatorPrecedence::Comparison as u16
        }

        // assignment
        Expression::Assign { operator, .. } => operator.precedence(),

        // ternary
        Expression::If {
            form: IfForm::Ternary,
            ..
        } => OperatorPrecedence::AssignmentBoolean as u16 - 1,

        // atomic/primary expressions
        _ => u16::MAX,
    }
}

/// Write one separating space after the left operand when no postfix trivia exists.
pub(crate) fn write_space_after_binary_left_if_needed<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    left: LocalNodeId<Expression>,
    operator: BinaryOperator,
) -> FormatResult<()> {
    let allow_inline_block_postfix_space =
        binary_expression_has_inline_block_postfix_comment(f.context(), left);
    let allow_logical_space_after_line_comment = is_logical_binary_operator(operator)
        && binary_expression_has_line_suffix_comment(f.context(), left);

    if f.context().has_postfix_annotation(left)
        && !allow_logical_space_after_line_comment
        && !allow_inline_block_postfix_space
    {
        return Ok(());
    }

    write!(f, [space()])
}

/// One binary-like expression wrapper.
#[derive(Debug, Clone, Copy)]
struct BinaryLikeExpression {
    /// The wrapped binary expression node.
    node_id: LocalNodeId<Expression>,
}

impl BinaryLikeExpression {
    /// Create one binary-like wrapper.
    #[inline]
    fn new(node_id: LocalNodeId<Expression>) -> Self {
        Self { node_id }
    }

    /// Return the left operand.
    fn left(self, context: &DestackFormatContext<'_>) -> LocalNodeId<Expression> {
        let Expression::Binary { left, .. } = context.tree.get(self.node_id) else {
            unreachable!("binary-like wrapper requires Expression::Binary");
        };

        *left
    }

    /// Return the right operand.
    fn right(self, context: &DestackFormatContext<'_>) -> LocalNodeId<Expression> {
        let Expression::Binary { right, .. } = context.tree.get(self.node_id) else {
            unreachable!("binary-like wrapper requires Expression::Binary");
        };

        *right
    }

    /// Return the operator.
    fn operator(self, context: &DestackFormatContext<'_>) -> BinaryOperator {
        let Expression::Binary { operator, .. } = context.tree.get(self.node_id) else {
            unreachable!("binary-like wrapper requires Expression::Binary");
        };

        *operator
    }

    /// Return whether this expression is inside one test condition.
    fn is_inside_condition(self, context: &DestackFormatContext<'_>) -> bool {
        let Some((parent_id, parent_type)) = context.parent(self.node_id) else {
            return false;
        };
        if parent_type != NodeType::Expression {
            return false;
        }

        let parent_id = LocalNodeId::<Expression>::new(parent_id);

        match context.tree.get(parent_id) {
            Expression::If {
                form: IfForm::If,
                condition,
                ..
            } => matches!(
                condition,
                IfCondition::Expression { condition } if *condition == self.node_id
            ),
            Expression::While { condition, .. } => *condition == self.node_id,
            Expression::For { condition, .. } => {
                condition.is_some_and(|condition| condition == self.node_id)
            }
            Expression::Match { value, form, .. } => {
                *value == self.node_id && matches!(form, MatchForm::Switch)
            }
            _ => false,
        }
    }

    /// Return whether this expression can flatten with its left child.
    fn can_flatten(self, context: &DestackFormatContext<'_>) -> bool {
        let left = self.left(context);
        let Expression::Binary {
            operator: left_operator,
            ..
        } = context.tree.get(left)
        else {
            return false;
        };

        should_flatten_binary(self.operator(context), *left_operator)
    }

    /// Return whether a logical chain should keep its right side inline.
    fn should_inline_logical_expression(self, context: &DestackFormatContext<'_>) -> bool {
        if !is_logical_binary_operator(self.operator(context)) {
            return false;
        }

        match context.tree.get(self.right(context)) {
            Expression::ObjectExpression { properties, .. } => !properties.is_empty(),
            Expression::ArrayExpression { elements } => !elements.is_empty(),
            Expression::TreeExpression { .. } => true,
            _ => false,
        }
    }

    /// Return whether one ternary parent already owns indentation.
    fn ternary_parent_owns_indentation(
        context: &DestackFormatContext<'_>,
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
                    | Expression::NewTarget
            );
        }

        if parent_type != NodeType::Expression {
            return true;
        }

        let parent_expression_id = LocalNodeId::<Expression>::new(parent_id);

        !matches!(
            context.tree.get(parent_expression_id),
            Expression::Return { .. }
                | Expression::Throw { .. }
                | Expression::Call { .. }
                | Expression::New { .. }
                | Expression::Import { .. }
                | Expression::ImportMeta
                | Expression::NewTarget
        )
    }

    /// Return whether the parent already owns indentation.
    fn should_not_indent_if_parent_indents(self, context: &DestackFormatContext<'_>) -> bool {
        let Some((parent_id, parent_type)) = context.parent(self.node_id) else {
            return false;
        };
        if parent_type != NodeType::Expression {
            return false;
        }

        let parent_id = LocalNodeId::<Expression>::new(parent_id);

        match context.tree.get(parent_id) {
            Expression::Return { value } => value.is_some_and(|value| value == self.node_id),
            Expression::Throw { value } => *value == self.node_id,
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
        parent: BinaryLikeExpression,
    },

    /// One operator plus right operand.
    Right {
        /// The current parent chain node.
        parent: BinaryLikeExpression,
        /// Whether the containing chain is in condition position.
        inside_condition: bool,
    },
}

impl BinarySide {
    /// Return whether this side is a tree expression.
    fn is_tree(self, context: &DestackFormatContext<'_>) -> bool {
        let expression_id = match self {
            Self::Left { parent } => parent.left(context),
            Self::Right { parent, .. } => parent.right(context),
        };

        matches!(
            context.tree.get(expression_id),
            Expression::TreeExpression { .. }
        )
    }
}

impl<'a> Format<DestackFormatContext<'a>> for BinarySide {
    fn format(&self, f: &mut FirFormatter<'_, DestackFormatContext<'a>>) -> FormatResult<()> {
        match self {
            // left side
            Self::Left { parent } => {
                let (left, following_span_start) = {
                    let context = f.context();
                    let left = parent.left(context);
                    let right = parent.right(context);

                    (left, context.span(right).start)
                };

                with_following_span_start(f, following_span_start, |f| write!(f, [group(&left)]))
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
                    let left = parent.left(context);
                    let right = parent.right(context);
                    let operator = parent.operator(context);
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

                // separator space after left side
                write_space_after_binary_left_if_needed(f, parent.left(f.context()), operator)?;

                let operator_and_right = format_with(|f: &mut DestackFormatter<'a, '_>| {
                    write!(f, [operator])?;

                    // inline logical rhs
                    if parent.should_inline_logical_expression(f.context()) {
                        write!(f, [space()])?;

                        if !matches!(
                            f.context().tree.get(right),
                            Expression::TreeExpression { .. }
                        ) && f
                            .context()
                            .comments()
                            .has_leading_own_line_comment(f.context().span(right).start)
                        {
                            write!(f, [soft_line_indent_or_space(&right)])?;

                            return Ok(());
                        }
                    }
                    // standard separator
                    else {
                        write!(f, [soft_line_break_or_space()])?;
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
                    return write!(f, [group(&operator_and_right).should_expand(should_break)]);
                }

                write!(f, [operator_and_right])
            }
        }
    }
}

/// Recursively format one flattened binary expression.
fn format_flattened_binary_expression<'ast>(
    binary: BinaryLikeExpression,
    inside_condition: bool,
    f: &mut DestackFormatter<'ast, '_>,
) -> FormatResult<()> {
    let left = binary.left(f.context());

    // flattened left side
    if binary.can_flatten(f.context()) {
        format_flattened_binary_expression(BinaryLikeExpression::new(left), inside_condition, f)?;
    }
    // terminal left side
    else {
        write!(f, [BinarySide::Left { parent: binary }])?;
    }

    // right side
    write!(
        f,
        [BinarySide::Right {
            parent: binary,
            inside_condition
        }]
    )
}

/// Split one binary chain into left and right sides.
fn split_into_left_and_right_sides(
    binary: BinaryLikeExpression,
    inside_condition: bool,
    context: &DestackFormatContext<'_>,
    items: &mut Vec<BinarySide>,
) {
    let left = binary.left(context);

    // flattened left side
    if binary.can_flatten(context) {
        split_into_left_and_right_sides(
            BinaryLikeExpression::new(left),
            inside_condition,
            context,
            items,
        );
    }
    // terminal left side
    else {
        items.push(BinarySide::Left { parent: binary });
    }

    // right side
    items.push(BinarySide::Right {
        parent: binary,
        inside_condition,
    });
}

/// Return whether this binary root is already owned by an outer indentation layout.
fn binary_parent_inlines_flattened_layout(
    context: &DestackFormatContext<'_>,
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
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let Some((parent_id, parent_type)) = context.parent(expression_id) else {
        return false;
    };
    if parent_type != NodeType::Expression {
        return false;
    }

    let parent_id = LocalNodeId::<Expression>::new(parent_id);

    match context.tree.get(parent_id) {
        Expression::Unary { right, .. } => *right == expression_id,
        Expression::Member { left, .. }
        | Expression::PrivateMember { left, .. }
        | Expression::Index { left, .. }
        | Expression::Call { left, .. }
        | Expression::New { left, .. }
        | Expression::Instantiation { left, .. }
        | Expression::Maybe { left, .. }
        | Expression::Must { left, .. } => *left == expression_id,
        Expression::TaggedTemplateExpression { tag, .. } => *tag == expression_id,
        _ => false,
    }
}

/// Format a binary expression with one binary-like printer.
pub(crate) fn format_binary_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    _left: LocalNodeId<Expression>,
    _operator: &BinaryOperator,
    _right: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let binary = BinaryLikeExpression::new(node_id);
    let is_inside_condition = binary.is_inside_condition(f.context());

    // condition position
    if is_inside_condition {
        return format_flattened_binary_expression(binary, true, f);
    }

    // parenthesized callee or object position
    if binary_expression_is_inside_parenthesis_context(f.context(), node_id) {
        return write!(
            f,
            [group(&soft_block_indent(&format_with(
                |f: &mut DestackFormatter<'ast, '_>| {
                    format_flattened_binary_expression(binary, false, f)
                }
            )))]
        );
    }

    // parent-owned indent
    if binary.should_not_indent_if_parent_indents(f.context()) {
        return write!(
            f,
            [group(&format_with(|f: &mut DestackFormatter<'ast, '_>| {
                format_flattened_binary_expression(binary, false, f)
            }))]
        );
    }

    let should_inline_logical = binary.should_inline_logical_expression(f.context());
    let should_indent_if_parent_inlines =
        binary_parent_inlines_flattened_layout(f.context(), node_id);

    let mut parts = Vec::with_capacity(2);
    split_into_left_and_right_sides(binary, false, f.context(), &mut parts);
    let is_flattened = parts.len() > 2;

    // direct grouped layout
    if (should_inline_logical && !is_flattened)
        || (!should_inline_logical && should_indent_if_parent_inlines)
    {
        return write!(
            f,
            [group(&format_with(|f: &mut DestackFormatter<'ast, '_>| {
                for part in &parts {
                    write!(f, [*part])?;
                }

                Ok(())
            }))]
        );
    }

    let first = parts[0];
    let last_is_tree = parts.last().is_some_and(|part| part.is_tree(f.context()));
    let tail_end = if last_is_tree {
        parts.len().saturating_sub(1)
    } else {
        parts.len()
    };
    let tail = &parts[1..tail_end];
    let group_id = f.group_id("logicalChain");

    let format_non_tree_parts = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        write!(
            f,
            [group(&format_with(|f: &mut DestackFormatter<'ast, '_>| {
                write!(f, [first])?;

                if !tail.is_empty() {
                    write!(
                        f,
                        [indent(&format_with(
                            |f: &mut DestackFormatter<'ast, '_>| {
                                for part in tail {
                                    write!(f, [*part])?;
                                }

                                Ok(())
                            }
                        ))]
                    )?;
                }

                Ok(())
            }))
            .with_id(Some(group_id))]
        )
    });

    // tree tail
    if last_is_tree {
        let tree_tail = *parts.last().expect("tree tail requires one final part");

        return write!(
            f,
            [group(&format_with(|f: &mut DestackFormatter<'ast, '_>| {
                write!(f, [format_non_tree_parts])?;
                write!(f, [soft_line_break_or_space(), tree_tail])?;

                Ok(())
            }))]
        );
    }

    write!(f, [format_non_tree_parts])
}
