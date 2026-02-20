use crate::expression::{
    Argument, AssignOperator, Asynchrony, BinaryOperator, DeclarationKind, Declarator,
    DestackFormatContext, DestackFormatter, Expression, FormatError, FormatResult, LetKind,
    LocalNodeId, NodeTree, NodeType, PostfixPosition, StringId, expression_inline_width_hint,
    is_chain_expression, needs_parens_in_postfix_position, pattern_inline_width_hint, token,
    write_postfix_base_expression,
};
use destack_fir::format::Buffer;
use destack_fir::write;

// assignment/declarator inline width constants
const ASSIGNMENT_OPERATOR_PADDING_WIDTH: usize = 2;
const DECLARATOR_TYPE_SEPARATOR_INLINE_WIDTH: usize = 2;
const DECLARATOR_ASSIGNMENT_SEPARATOR_INLINE_WIDTH: usize = 3;

// declaration prefix lengths with trailing space
const EXPORT_PREFIX_LEN: usize = 7;
const DECLARE_PREFIX_LEN: usize = 8;
const ASYNC_PREFIX_LEN: usize = 6;
const USING_PREFIX_LEN: usize = 6;
const LET_PREFIX_LEN: usize = 4;
const VAR_PREFIX_LEN: usize = 4;
const CONST_PREFIX_LEN: usize = 6;

/// Extract a parenthesized base with a direct index chain.
pub(crate) fn extract_parenthesized_index_chain(
    tree: &NodeTree,
    expression_id: LocalNodeId<Expression>,
) -> Option<(LocalNodeId<Expression>, Vec<LocalNodeId<Expression>>)> {
    // collect direct index operations from the outside in
    let mut indices: Vec<LocalNodeId<Expression>> = Vec::new();
    let mut current = expression_id;

    while let Expression::Index {
        position: PostfixPosition::Direct,
        left,
        index: Some(index),
    } = tree.get(current)
    {
        indices.push(*index);
        current = *left;
    }

    if indices.is_empty() {
        return None;
    }

    let Expression::Parenthesized { expression } = tree.get(current) else {
        return None;
    };

    if needs_parens_in_postfix_position(tree, *expression) {
        return None;
    }

    indices.reverse();
    Some((*expression, indices))
}

/// Format a maybe expression without considering chaining.
pub(crate) fn format_maybe_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    if let Expression::Maybe { left, position } = f.context().tree.get(node_id) {
        write_postfix_base_expression(f, *left)?;
        match position {
            PostfixPosition::Direct => write!(f, [token("?")])?,
            PostfixPosition::Indirect => write!(f, [token("."), token("?")])?,
        }
    } else {
        debug_assert!(false, "unexpected expression kind for maybe formatter");
    }
    Ok(())
}

/// The head of a chain before any postfix operations.
#[derive(Clone)]
pub(crate) enum ChainExpressionBaseHead {
    Path {
        node_id: LocalNodeId<Expression>,
        segment: StringId,
        static_arguments: Option<Vec<LocalNodeId<Argument>>>,
        emit_postfix_annotations: bool,
    },
    Expression(LocalNodeId<Expression>),
}

/// The initial portion of the chain including direct postfix ops.
#[derive(Clone)]
pub(crate) struct ChainExpressionBase {
    pub(crate) head: ChainExpressionBaseHead,
    pub(crate) body: Vec<ChainExpression>,
}

/// One operation in an expression chain.
#[derive(Clone)]
pub(crate) enum ChainExpression {
    /// Member expression.
    Member {
        node_id: LocalNodeId<Expression>,
        segment: StringId,
        static_arguments: Option<Vec<LocalNodeId<Argument>>>,
        emit_prefix_annotations: bool,
        emit_postfix_annotations: bool,
    },
    /// Instantiation expression.
    Instantiation {
        node_id: LocalNodeId<Expression>,
        static_arguments: Vec<LocalNodeId<Argument>>,
    },
    /// Call expression.
    Call {
        node_id: LocalNodeId<Expression>,
        position: PostfixPosition,
        static_arguments: Option<Vec<LocalNodeId<Argument>>>,
        dynamic_arguments: Vec<LocalNodeId<Argument>>,
    },
    /// Index expression.
    Index {
        node_id: LocalNodeId<Expression>,
        position: PostfixPosition,
        index: Option<LocalNodeId<Expression>>,
    },
    /// Maybe expression.
    Maybe {
        node_id: LocalNodeId<Expression>,
        position: PostfixPosition,
    },
    /// Must expression.
    Must {
        node_id: LocalNodeId<Expression>,
        position: PostfixPosition,
    },
}

/// Return the left operand for one chain node.
pub(crate) fn chain_node_left_id(
    tree: &NodeTree,
    node_id: LocalNodeId<Expression>,
) -> Option<LocalNodeId<Expression>> {
    match tree.get(node_id) {
        Expression::Member { left, .. }
        | Expression::PrivateMember { left, .. }
        | Expression::Call { left, .. }
        | Expression::Index { left, .. }
        | Expression::Instantiation { left, .. }
        | Expression::Maybe { left, .. }
        | Expression::Must { left, .. } => Some(*left),
        _ => None,
    }
}

/// Collect all chain nodes from root to leaf.
pub(crate) fn collect_chain_nodes(
    tree: &NodeTree,
    node_id: LocalNodeId<Expression>,
) -> Vec<LocalNodeId<Expression>> {
    // walk from leaf to root through chain links
    let mut chain = Vec::new();
    let mut current = node_id;
    loop {
        chain.push(current);

        let Some(next_id) = chain_node_left_id(tree, current) else {
            break;
        };
        current = next_id;
    }
    chain.reverse();

    chain
}

/// Convert one chain expression node into a chain operation.
pub(crate) fn chain_expression_from_node(
    tree: &NodeTree,
    expression_id: LocalNodeId<Expression>,
) -> FormatResult<ChainExpression> {
    let chain_expression = match tree.get(expression_id) {
        Expression::Member {
            name,
            static_arguments,
            ..
        } => ChainExpression::Member {
            node_id: expression_id,
            segment: *name,
            static_arguments: static_arguments.clone(),
            emit_prefix_annotations: true,
            emit_postfix_annotations: true,
        },
        Expression::PrivateMember {
            name,
            static_arguments,
            ..
        } => ChainExpression::Member {
            node_id: expression_id,
            segment: *name,
            static_arguments: static_arguments.clone(),
            emit_prefix_annotations: true,
            emit_postfix_annotations: true,
        },
        Expression::Call {
            position,
            static_arguments,
            dynamic_arguments,
            ..
        } => ChainExpression::Call {
            node_id: expression_id,
            position: *position,
            static_arguments: static_arguments.clone(),
            dynamic_arguments: dynamic_arguments.clone(),
        },
        Expression::Instantiation {
            static_arguments, ..
        } => ChainExpression::Instantiation {
            node_id: expression_id,
            static_arguments: static_arguments.clone(),
        },
        Expression::Index {
            position, index, ..
        } => ChainExpression::Index {
            node_id: expression_id,
            position: *position,
            index: *index,
        },
        Expression::Maybe { position, .. } => ChainExpression::Maybe {
            node_id: expression_id,
            position: *position,
        },
        Expression::Must { position, .. } => ChainExpression::Must {
            node_id: expression_id,
            position: *position,
        },
        _ => {
            return Err(FormatError::SyntaxError {
                message: "unexpected expression kind for chain expression",
            });
        }
    };

    Ok(chain_expression)
}

/// Check whether the expression is part of a member/call/maybe/index chain.
pub(crate) fn is_expression_chain(tree: &NodeTree, node_id: LocalNodeId<Expression>) -> bool {
    chain_node_left_id(tree, node_id).is_some_and(|left_id| is_chain_expression(tree.get(left_id)))
}

/// Check whether an expression is the root of a chain.
pub(crate) fn is_chain_root(tree: &NodeTree, node_id: LocalNodeId<Expression>) -> bool {
    chain_node_left_id(tree, node_id).is_some_and(|left_id| !is_chain_expression(tree.get(left_id)))
}

/// Check whether this expression is used as the receiver in a chain parent.
pub(crate) fn has_chain_parent(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let Some((parent_id, parent_type)) = context.parent_by_id(node_id.id) else {
        return false;
    };
    if parent_type != NodeType::Expression {
        return false;
    }

    let parent_expr = context.tree.get(LocalNodeId::<Expression>::new(parent_id));
    match parent_expr {
        Expression::Member { left, .. }
        | Expression::PrivateMember { left, .. }
        | Expression::Call { left, .. }
        | Expression::Instantiation { left, .. }
        | Expression::Maybe { left, .. }
        | Expression::Must { left, .. } => left.id == node_id.id,
        Expression::Index { .. } => false,
        _ => false,
    }
}

/// Walk upward through transparent wrappers to find an assignment-like parent rhs.
pub(crate) fn assignment_like_parent(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> Option<(NodeType, u32)> {
    let mut current_id = node_id.id;

    // walk through transparent wrappers until we reach an assignment-like parent
    while let Some((parent_id, parent_type)) = context.parent_by_id(current_id) {
        match parent_type {
            NodeType::Expression => {
                let parent_expr = context.tree.get(LocalNodeId::<Expression>::new(parent_id));

                // assignment rhs
                if let Expression::Assign { right, .. } = parent_expr
                    && right.id == current_id
                {
                    return Some((NodeType::Expression, parent_id));
                }

                // transparent wrappers around the rhs
                let is_wrapper_parent = matches!(
                    parent_expr,
                    Expression::Await { expression }
                        | Expression::AwaitMaybe { expression }
                        | Expression::Parenthesized { expression }
                        if expression.id == current_id
                );

                if is_wrapper_parent {
                    current_id = parent_id;
                    continue;
                }

                return None;
            }
            NodeType::Declarator => {
                let declarator = context.tree.get(LocalNodeId::<Declarator>::new(parent_id));
                if declarator.value.is_some_and(|value| value.id == current_id) {
                    return Some((NodeType::Declarator, parent_id));
                }
                return None;
            }
            _ => return None,
        }
    }

    None
}

/// Unwrap transparent wrappers around an expression for classification.
pub(crate) fn transparent_inner_expression(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> LocalNodeId<Expression> {
    context.transparent_inner_expression(node_id)
}

/// Get the display width of an assignment operator token.
pub(crate) fn assign_operator_len(operator: &AssignOperator) -> usize {
    match operator {
        AssignOperator::Assign => 1,
        AssignOperator::MultiplyAssign
        | AssignOperator::DivideAssign
        | AssignOperator::RemainderAssign
        | AssignOperator::AddAssign
        | AssignOperator::SubtractAssign
        | AssignOperator::ElementwiseAndAssign
        | AssignOperator::ElementwiseXorAssign
        | AssignOperator::ElementwiseOrAssign => 2,
        AssignOperator::WrappingMultiplyAssign
        | AssignOperator::SaturatingMultiplyAssign
        | AssignOperator::WrappingAddAssign
        | AssignOperator::SaturatingAddAssign
        | AssignOperator::WrappingSubtractAssign
        | AssignOperator::SaturatingSubtractAssign
        | AssignOperator::ShiftLeftAssign
        | AssignOperator::ShiftRightAssign
        | AssignOperator::AndAssign
        | AssignOperator::OrAssign
        | AssignOperator::CoalesceAssign
        | AssignOperator::ExponentAssign => 3,
        AssignOperator::WrappingExponentAssign
        | AssignOperator::SaturatingExponentAssign
        | AssignOperator::SaturatingShiftLeftAssign
        | AssignOperator::UnsignedShiftRightAssign => 4,
    }
}

/// Get the display width of a binary operator token.
pub(crate) fn binary_operator_len(operator: &BinaryOperator) -> usize {
    match operator {
        BinaryOperator::Multiply
        | BinaryOperator::WrappingMultiply
        | BinaryOperator::SaturatingMultiply
        | BinaryOperator::Divide
        | BinaryOperator::Remainder
        | BinaryOperator::Add
        | BinaryOperator::WrappingAdd
        | BinaryOperator::SaturatingAdd
        | BinaryOperator::Subtract
        | BinaryOperator::WrappingSubtract
        | BinaryOperator::SaturatingSubtract
        | BinaryOperator::ElementwiseAnd
        | BinaryOperator::ElementwiseXor
        | BinaryOperator::ElementwiseOr
        | BinaryOperator::Equal
        | BinaryOperator::NotEqual
        | BinaryOperator::LessThan
        | BinaryOperator::GreaterThan => 1,
        BinaryOperator::Exponent
        | BinaryOperator::ShiftLeft
        | BinaryOperator::ShiftRight
        | BinaryOperator::EqualStrict
        | BinaryOperator::NotEqualStrict
        | BinaryOperator::LessThanOrEqual
        | BinaryOperator::GreaterThanOrEqual
        | BinaryOperator::And
        | BinaryOperator::Or
        | BinaryOperator::Coalesce
        | BinaryOperator::In => 2,
        BinaryOperator::UnsignedShiftRight => 3,
        BinaryOperator::SaturatingShiftLeft => 3,
        BinaryOperator::WrappingExponent | BinaryOperator::SaturatingExponent => 3,
        BinaryOperator::InstanceOf => 10,
    }
}

/// Decide whether a nullish coalescing operator should trail on a new line.
pub(crate) fn should_use_trailing_coalesce(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    left: LocalNodeId<Expression>,
) -> bool {
    let Some((parent_id, parent_type)) = context.parent(node_id) else {
        return false;
    };

    if parent_type != NodeType::Expression {
        return false;
    }

    let parent_expression = context.tree.get(LocalNodeId::<Expression>::new(parent_id));
    if !matches!(parent_expression, Expression::Parenthesized { .. }) {
        return false;
    }

    let left_is_chain = matches!(
        context.tree.get(left),
        Expression::Member { .. }
            | Expression::PrivateMember { .. }
            | Expression::Index { .. }
            | Expression::Call { .. }
            | Expression::Maybe { .. }
            | Expression::Must { .. }
    );
    if !left_is_chain {
        return false;
    }

    let expression_span = context.span(node_id);
    if context.has_newline(expression_span) {
        return true;
    }

    false
}

/// Estimate the remaining inline width for a rhs in an assignment-like parent.
pub(crate) fn assignment_like_remaining_width(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> Option<usize> {
    let line_width = usize::from(context.options.line_width);
    let (parent_type, parent_id) = assignment_like_parent(context, node_id)?;

    match parent_type {
        NodeType::Expression => {
            let parent_expression = context.tree.get(LocalNodeId::<Expression>::new(parent_id));
            let Expression::Assign { left, operator, .. } = parent_expression else {
                return Some(line_width);
            };

            let left_len = expression_inline_width_hint(context, *left);
            let operator_len = assign_operator_len(operator);
            let consumed_len = left_len
                .saturating_add(operator_len)
                .saturating_add(ASSIGNMENT_OPERATOR_PADDING_WIDTH);

            Some(line_width.saturating_sub(consumed_len))
        }
        NodeType::Declarator => {
            let declarator = context.tree.get(LocalNodeId::<Declarator>::new(parent_id));
            let prefix_len =
                declarator_leading_prefix_len(context, LocalNodeId::<Declarator>::new(parent_id));
            let pattern_len = pattern_inline_width_hint(context, declarator.pattern);
            let type_len = declarator.ty.map_or(0usize, |type_id| {
                expression_inline_width_hint(context, type_id)
                    .saturating_add(DECLARATOR_TYPE_SEPARATOR_INLINE_WIDTH)
            });
            let consumed_len = prefix_len
                .saturating_add(pattern_len)
                .saturating_add(type_len)
                .saturating_add(DECLARATOR_ASSIGNMENT_SEPARATOR_INLINE_WIDTH);

            Some(line_width.saturating_sub(consumed_len))
        }
        _ => Some(line_width),
    }
}

/// Estimate the leading declaration header width before a declarator.
pub(crate) fn declarator_leading_prefix_len(
    context: &DestackFormatContext<'_>,
    declarator_id: LocalNodeId<Declarator>,
) -> usize {
    let Some((parent_id, parent_type)) = context.parent(declarator_id) else {
        return 0;
    };

    if parent_type != NodeType::Expression {
        return 0;
    }

    let parent_id = LocalNodeId::<Expression>::new(parent_id);
    match context.tree.get(parent_id) {
        Expression::Let {
            kind,
            descriptor,
            declarators,
            ..
        } => {
            if declarators
                .first()
                .is_none_or(|id| id.id != declarator_id.id)
            {
                return 0;
            }

            let mut prefix_len = 0usize;
            if descriptor.export.is_some() {
                prefix_len = prefix_len.saturating_add(EXPORT_PREFIX_LEN);
            }
            if descriptor.kind == DeclarationKind::Declaration {
                prefix_len = prefix_len.saturating_add(DECLARE_PREFIX_LEN);
            }
            prefix_len = prefix_len.saturating_add(match kind {
                LetKind::Let => LET_PREFIX_LEN,
                LetKind::Var => VAR_PREFIX_LEN,
                LetKind::Const => CONST_PREFIX_LEN,
            });
            prefix_len
        }
        Expression::Using {
            asynchrony,
            descriptor,
            declarators,
        } => {
            if declarators
                .first()
                .is_none_or(|id| id.id != declarator_id.id)
            {
                return 0;
            }

            let mut prefix_len = 0usize;
            if descriptor.export.is_some() {
                prefix_len = prefix_len.saturating_add(EXPORT_PREFIX_LEN);
            }
            if descriptor.kind == DeclarationKind::Declaration {
                prefix_len = prefix_len.saturating_add(DECLARE_PREFIX_LEN);
            }
            if *asynchrony == Asynchrony::Async {
                prefix_len = prefix_len.saturating_add(ASYNC_PREFIX_LEN);
            }
            prefix_len.saturating_add(USING_PREFIX_LEN)
        }
        _ => 0,
    }
}
