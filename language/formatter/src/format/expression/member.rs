use super::*;
use destack_fir::{format_args, write};
use smallvec::SmallVec;

/// Format a member expression.
pub(crate) fn format_member_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    match f.context().tree.get(node_id) {
        Expression::Member {
            left,
            name,
            static_arguments,
        } => {
            let left = if let Expression::Parenthesized { expression } = f.context().tree.get(*left)
                && parenthesized_should_unwrap(
                    f.context(),
                    *left,
                    *expression,
                    ParenthesizedUnwrapPolicy::MemberObject,
                ) {
                *expression
            } else {
                *left
            };
            let is_breakable_member_receiver = matches!(
                f.context().tree.get(left),
                Expression::Call { .. } | Expression::Instantiation { .. }
            );

            if is_breakable_member_receiver {
                write!(
                    f,
                    [group(&format_args![
                        format_with(|f| write_postfix_base_expression(f, left)),
                        indent(&format_with(|f| {
                            write!(f, [soft_line_break(), token("."), *name])?;
                            if let Some(static_arguments) = static_arguments {
                                format_static_argument_list(f, static_arguments)?;
                            }
                            Ok(())
                        }))
                    ])]
                )?;
            } else {
                write_postfix_base_expression(f, left)?;
                write!(f, [token(".")])?;
                write!(f, [*name])?;
                if let Some(static_arguments) = static_arguments {
                    format_static_argument_list(f, static_arguments)?;
                }
            }
        }
        Expression::PrivateMember {
            left,
            name,
            static_arguments,
        } => {
            let left = if let Expression::Parenthesized { expression } = f.context().tree.get(*left)
                && parenthesized_should_unwrap(
                    f.context(),
                    *left,
                    *expression,
                    ParenthesizedUnwrapPolicy::MemberObject,
                ) {
                *expression
            } else {
                *left
            };
            let is_breakable_member_receiver = matches!(
                f.context().tree.get(left),
                Expression::Call { .. } | Expression::Instantiation { .. }
            );

            if is_breakable_member_receiver {
                write!(
                    f,
                    [group(&format_args![
                        format_with(|f| write_postfix_base_expression(f, left)),
                        indent(&format_with(|f| {
                            write!(f, [soft_line_break(), token("."), token("#"), *name])?;
                            if let Some(static_arguments) = static_arguments {
                                format_static_argument_list(f, static_arguments)?;
                            }
                            Ok(())
                        }))
                    ])]
                )?;
            } else {
                write_postfix_base_expression(f, left)?;
                write!(f, [token("."), token("#"), *name])?;
                if let Some(static_arguments) = static_arguments {
                    format_static_argument_list(f, static_arguments)?;
                }
            }
        }
        _ => {
            debug_assert!(false, "unexpected expression kind for member formatter");
        }
    }
    Ok(())
}

/// Format a type index expression without considering chaining.
#[inline]
pub(crate) fn format_type_index_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    left: LocalNodeId<Expression>,
    index: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let needs_parentheses = matches!(
        f.context().tree.get(left),
        Expression::TypeBinary { .. }
            | Expression::TypeConditional { .. }
            | Expression::TypeMapped { .. }
    );
    if needs_parentheses {
        write!(f, [token("("), left, token(")")])?;
    } else {
        write!(f, [left])?;
    }
    write!(f, [token("["), index, token("]")])?;
    Ok(())
}

/// Format a type template literal expression.
pub(crate) fn format_type_template_literal<'ast>(
    strings: &[StringId],
    spans: &[LocalNodeId<Expression>],
    f: &mut DestackFormatter<'ast, '_>,
) -> FormatResult<()> {
    debug_assert_eq!(strings.len(), spans.len().saturating_add(1));

    write!(f, [token("`")])?;

    let mut string_segments = strings.iter();
    if let Some(first_segment) = string_segments.next() {
        write!(f, [*first_segment])?;
    }

    for (span, segment) in spans.iter().zip(string_segments) {
        let should_expand_span = span_has_comment(f.context(), f.context().span(*span));

        write!(
            f,
            [
                group(&format_args![
                    token("${"),
                    group(span).should_expand(should_expand_span),
                    token("}")
                ]),
                *segment,
            ]
        )?;
    }

    write!(f, [token("`")])
}

/// Format an index expression without considering chaining.
#[inline]
pub(crate) fn format_index_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    if let Expression::Index {
        position,
        left,
        index,
    } = f.context().tree.get(node_id)
    {
        write_postfix_base_expression(f, *left)?;
        if *position == PostfixPosition::Indirect {
            write!(f, [token(".")])?;
        }
        if let Some(index) = index {
            let should_parenthesize = should_parenthesize_index_expression(f.context(), *index);
            let left_span = f.context().span(*left);
            let index_span = f.context().span(*index);
            let has_break_after_open = left_span.file == index_span.file
                && left_span.end < index_span.start
                && f.context().has_newline(Span::new(
                    left_span.file,
                    left_span.end,
                    index_span.start,
                ));
            let should_break_index = has_break_after_open
                || f.context().has_newline(index_span)
                || f.context().has_annotation(*index);

            if should_break_index {
                write!(
                    f,
                    [group(&format_with(|f| {
                        write!(f, [token("[")])?;
                        if should_parenthesize {
                            write!(
                                f,
                                [block_indent(&format_args![token("("), *index, token(")")])]
                            )?;
                        } else {
                            write!(f, [block_indent(index)])?;
                        }
                        write!(f, [token("]")])
                    }))
                    .should_expand(true)]
                )?;
            } else if should_parenthesize {
                write!(f, [token("["), token("("), *index, token(")"), token("]")])?;
            } else {
                write!(f, [token("["), *index, token("]")])?;
            }
        } else {
            write!(f, [token("[]")])?;
        }
    } else {
        debug_assert!(false, "unexpected expression kind for index formatter");
    }
    Ok(())
}

/// Hugging configuration for different delimiter contexts.
pub(crate) struct HugOptions {
    /// The opening delimiter.
    pub(crate) open: &'static str,
    /// The closing delimiter.
    pub(crate) close: &'static str,
    /// Whether to force a trailing comma.
    pub(crate) force_trailing: bool,
    /// Whether to include a trailing comma when the group breaks.
    pub(crate) trailing_if_breaks: bool,
    /// Whether to allow arrow functions.
    pub(crate) allow_arrow_functions: bool,
    /// Whether to handle annotations.
    pub(crate) handle_annotations: bool,
    /// Whether multiline object and array values can still use hugging.
    pub(crate) allow_multiline_collection: bool,
}

impl HugOptions {
    pub(crate) const CALL: Self = Self {
        open: "(",
        close: ")",
        force_trailing: false,
        trailing_if_breaks: false,
        allow_arrow_functions: true,
        handle_annotations: true,
        allow_multiline_collection: true,
    };

    pub(crate) const ARRAY: Self = Self {
        open: "[",
        close: "]",
        force_trailing: false,
        trailing_if_breaks: false,
        allow_arrow_functions: false,
        handle_annotations: false,
        allow_multiline_collection: false,
    };

    pub(crate) const TUPLE: Self = Self {
        open: "(",
        close: ")",
        force_trailing: true,
        trailing_if_breaks: false,
        allow_arrow_functions: false,
        handle_annotations: false,
        allow_multiline_collection: false,
    };
}

/// Returns the precedence group for a binary operator.
/// Return precedence group for binary operators.
#[inline]
fn binary_operator_precedence_group(operator: BinaryOperator) -> u8 {
    // first two digits of discriminant encode precedence
    (operator as u16 / 100) as u8
}

/// Checks if two binary operators should be flattened together.
/// Return whether nested binaries should flatten into one group.
#[inline]
fn should_flatten_binary(left_operator: BinaryOperator, right_operator: BinaryOperator) -> bool {
    binary_operator_precedence_group(left_operator)
        == binary_operator_precedence_group(right_operator)
}

/// Represents a flattened binary expression operand with its preceding operator.
pub(crate) struct BinaryOperand {
    /// The operator before this operand (None for first).
    pub(crate) operator: Option<BinaryOperator>,
    /// The expression node.
    pub(crate) expression: LocalNodeId<Expression>,
}

/// Store flattened binary operands with an inline-first buffer.
pub(crate) type BinaryOperands = SmallVec<[BinaryOperand; 8]>;

/// Flattens a binary expression chain into a list of operands.
///
/// For `a + b + c`, returns [(None, a), (Some(+), b), (Some(+), c)].
pub(crate) fn flatten_binary_expression(
    tree: &NodeTree,
    expression_id: LocalNodeId<Expression>,
    target_operator: BinaryOperator,
) -> BinaryOperands {
    let mut operands = BinaryOperands::new();
    flatten_binary_recursive(tree, expression_id, target_operator, &mut operands, None);
    operands
}

/// Flattens associative type binary chains while unwrapping redundant parentheses.
pub(crate) fn flatten_type_binary_expression(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
    target_operator: BinaryOperator,
) -> BinaryOperands {
    let mut operands = BinaryOperands::new();
    flatten_type_binary_recursive(context, expression_id, target_operator, &mut operands, None);
    operands
}

/// Return the operand count for a flattened binary expression chain.
pub(crate) fn flattened_binary_operand_count(
    tree: &NodeTree,
    expression_id: LocalNodeId<Expression>,
    target_operator: BinaryOperator,
) -> usize {
    count_flattened_binary_recursive(tree, expression_id, target_operator)
}

/// Recursively flatten type binary chains and preserve operand operators.
fn flatten_type_binary_recursive(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
    target_operator: BinaryOperator,
    operands: &mut BinaryOperands,
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

    operands.push(BinaryOperand {
        operator: preceding_operator,
        expression: expression_id,
    });
}

/// Remove redundant parenthesized wrappers around associative type operands.
pub(crate) fn normalize_type_binary_operand_expression(
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

/// Recursively collect binary expression operands.
fn flatten_binary_recursive(
    tree: &NodeTree,
    expression_id: LocalNodeId<Expression>,
    target_operator: BinaryOperator,
    operands: &mut BinaryOperands,
    preceding_operator: Option<BinaryOperator>,
) {
    if let Expression::Binary {
        left,
        operator,
        right,
    } = tree.get(expression_id)
        && should_flatten_binary(*operator, target_operator)
    {
        // recursively flatten the left side
        flatten_binary_recursive(tree, *left, target_operator, operands, None);

        // add the right operand with its operator
        operands.push(BinaryOperand {
            operator: Some(*operator),
            expression: *right,
        });
        return;
    }

    // not a binary expression or different precedence - add as-is
    operands.push(BinaryOperand {
        operator: preceding_operator,
        expression: expression_id,
    });
}

/// Recursively count flattened binary operands without allocating.
fn count_flattened_binary_recursive(
    tree: &NodeTree,
    expression_id: LocalNodeId<Expression>,
    target_operator: BinaryOperator,
) -> usize {
    if let Expression::Binary {
        left,
        operator,
        right,
    } = tree.get(expression_id)
        && should_flatten_binary(*operator, target_operator)
    {
        let left_count = count_flattened_binary_recursive(tree, *left, target_operator);
        let right_count = count_flattened_binary_recursive(tree, *right, target_operator);
        return left_count.saturating_add(right_count);
    }

    1
}

/// Whether an expression variant is type specific.
/// Return precedence value for an expression.
#[inline]
pub(crate) fn expression_precedence(expr: &Expression) -> u16 {
    match expr {
        // postfix operators (2000)
        Expression::Call { .. }
        | Expression::Member { .. }
        | Expression::PrivateMember { .. }
        | Expression::Index { .. }
        | Expression::Instantiation { .. }
        | Expression::Maybe { .. }
        | Expression::Must { .. } => OperatorPrecedence::Postfix as u16,

        // postfix unary (2000)
        Expression::Unary { operator, .. } if operator.is_postfix() => {
            OperatorPrecedence::Postfix as u16
        }

        // prefix unary (1900)
        Expression::Unary { .. } => OperatorPrecedence::Prefix as u16,

        // prefix expressions (1900)
        Expression::Await { .. }
        | Expression::AwaitMaybe { .. }
        | Expression::Yield { .. }
        | Expression::Delete { .. }
        | Expression::Throw { .. }
        | Expression::Return { .. } => OperatorPrecedence::Prefix as u16,

        // type unary: use operator's precedence
        Expression::TypeUnary { operator, .. } => operator.precedence(),

        // binary: use operator's precedence
        Expression::Binary { operator, .. } => operator.precedence(),
        Expression::TypeBinary { operator, .. } => match operator {
            TypeBinaryOperator::Cast | TypeBinaryOperator::Satisfies => 0,
            _ => operator.precedence(),
        },

        // assignment: use operator's precedence
        Expression::Assign { operator, .. } => operator.precedence(),

        // ternary: lower than all binary/assignment operators
        Expression::If {
            kind: IfKind::Ternary,
            ..
        } => OperatorPrecedence::AssignmentBoolean as u16 - 1,

        // atomic/primary expressions: highest precedence (never need parens)
        _ => u16::MAX,
    }
}

/// Returns true if the expression needs parentheses when used as the operand
/// of a postfix operator like `?` or `!`.
///
/// Postfix operators (precedence 2000) bind tighter than all other operators.
/// For example, `await x?` parses as `await (x?)`, not `(await x)?`.
/// So when formatting `Maybe { left: Await { expr } }`, we need to output `(await expr)?`.
/// Return whether postfix formatting requires parentheses.
#[inline]
pub(crate) fn needs_parens_in_postfix_position(
    tree: &NodeTree,
    expr_id: LocalNodeId<Expression>,
) -> bool {
    expression_precedence(tree.get(expr_id)) < OperatorPrecedence::Postfix as u16
}

/// Format an expression used as the receiver/base of a postfix operation.
pub(crate) fn write_postfix_base_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    expression_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let parent_expression_id = postfix_parent_expression_id(f.context(), expression_id);
    let needs_integer_member_parentheses = parent_expression_id.is_some_and(|parent_id| {
        matches!(
            f.context().tree.get(expression_id),
            Expression::ScalarLiteral(ScalarLiteral::Integer(_))
        ) && matches!(
            f.context().tree.get(parent_id),
            Expression::Member { .. } | Expression::PrivateMember { .. }
        )
    });
    let needs_parentheses = needs_parens_in_postfix_position(f.context().tree, expression_id)
        || needs_integer_member_parentheses;
    if needs_parentheses {
        let line_width = usize::from(f.context().options.line_width);
        let parenthesized_chain_overflows = parent_expression_id.is_some_and(|parent_id| {
            let available_width =
                assignment_like_remaining_width(f.context(), parent_id).unwrap_or(line_width);
            expression_source_len(f.context(), parent_id) > available_width
        });

        if parenthesized_chain_overflows {
            write!(
                f,
                [
                    token("("),
                    block_indent(&expression_id),
                    hard_line_break(),
                    token(")")
                ]
            )?;
        } else {
            write!(f, [token("("), expression_id, token(")")])?;
        }
    } else {
        write!(f, [expression_id])?;
    }
    Ok(())
}

/// Return one postfix parent expression id when this expression is used as a chain receiver.
fn postfix_parent_expression_id(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> Option<LocalNodeId<Expression>> {
    // direct parent chain receiver
    if let Some((parent_id, parent_type)) = context.parent(expression_id)
        && parent_type == NodeType::Expression
    {
        let parent_expression_id = LocalNodeId::<Expression>::new(parent_id);
        let parent_expression = context.tree.get(parent_expression_id);
        let uses_expression_as_left = matches!(
            parent_expression,
            Expression::Member { left, .. }
                | Expression::PrivateMember { left, .. }
                | Expression::Call { left, .. }
                | Expression::Index { left, .. }
                | Expression::Instantiation { left, .. }
                | Expression::Maybe { left, .. }
                | Expression::Must { left, .. }
                if *left == expression_id
        );
        if uses_expression_as_left {
            return Some(parent_expression_id);
        }
    }

    // parenthesized wrapper chain receiver
    let Some((parent_id, parent_type)) = context.parent(expression_id) else {
        return None;
    };
    if parent_type != NodeType::Expression {
        return None;
    }

    let parent_expression_id = LocalNodeId::<Expression>::new(parent_id);
    let Expression::Parenthesized { expression } = context.tree.get(parent_expression_id) else {
        return None;
    };
    if *expression != expression_id {
        return None;
    }

    let Some((grandparent_id, grandparent_type)) = context.parent(parent_expression_id) else {
        return None;
    };
    if grandparent_type != NodeType::Expression {
        return None;
    }

    let grandparent_expression_id = LocalNodeId::<Expression>::new(grandparent_id);
    let grandparent_expression = context.tree.get(grandparent_expression_id);
    let uses_parenthesized_as_left = matches!(
        grandparent_expression,
        Expression::Member { left, .. }
            | Expression::PrivateMember { left, .. }
            | Expression::Call { left, .. }
            | Expression::Index { left, .. }
            | Expression::Instantiation { left, .. }
            | Expression::Maybe { left, .. }
            | Expression::Must { left, .. }
            if *left == parent_expression_id
    );
    if !uses_parenthesized_as_left {
        return None;
    }

    Some(grandparent_expression_id)
}

/// Check whether a parenthesized cast or satisfies left side is simple enough to unwrap.
pub(crate) fn is_chain_expression(expression: &Expression) -> bool {
    matches!(
        expression,
        Expression::Member { .. }
            | Expression::PrivateMember { .. }
            | Expression::Call { .. }
            | Expression::Index { .. }
            | Expression::Instantiation { .. }
            | Expression::Maybe { .. }
            | Expression::Must { .. }
    )
}
