use crate::format::expression::{
    Argument, Declarator, DestackFormatContext, DestackFormatter, Expression, FormatError,
    FormatResult, LocalNodeId, NodeTree, NodeType, PostfixPosition, StringId, is_chain_expression,
    needs_parens_in_postfix_position, token, write_postfix_base_expression,
};
use destack_fir::format::Buffer;
use destack_fir::write;

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
pub(crate) fn chain_nodes(
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
        } => {
            let Some(name) = *name else {
                return Err(FormatError::SyntaxError {
                    message: "missing member name in chain expression",
                });
            };
            ChainExpression::Member {
                node_id: expression_id,
                segment: name,
                static_arguments: static_arguments.clone(),
                emit_prefix_annotations: true,
                emit_postfix_annotations: true,
            }
        }
        Expression::PrivateMember {
            name,
            static_arguments,
            ..
        } => {
            let Some(name) = *name else {
                return Err(FormatError::SyntaxError {
                    message: "missing private member name in chain expression",
                });
            };
            ChainExpression::Member {
                node_id: expression_id,
                segment: name,
                static_arguments: static_arguments.clone(),
                emit_prefix_annotations: true,
                emit_postfix_annotations: true,
            }
        }
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
