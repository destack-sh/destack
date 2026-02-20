use super::super::{
    Annotation, AnnotationPosition, Declaration, DestackFormatContext, Expression, LocalNodeId,
    NodeTree, NodeType, needs_parens_in_postfix_position,
};
use super::trivia::{
    parenthesized_has_leading_inner_comments, parenthesized_has_leading_inner_trivia,
};
use crate::operator::is_type_context;

fn expression_chain_has_optional_maybe(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let mut current_id = expression_id;

    loop {
        match context.tree.get(current_id) {
            Expression::Maybe { .. } => return true,
            Expression::Member { left, .. }
            | Expression::PrivateMember { left, .. }
            | Expression::Index { left, .. }
            | Expression::Call { left, .. }
            | Expression::Must { left, .. }
            | Expression::Instantiation { left, .. } => current_id = *left,
            Expression::Parenthesized { expression } | Expression::Statement(expression) => {
                current_id = *expression;
            }
            _ => return false,
        }
    }
}

/// Return whether one member expression contains optional chaining semantics.
pub(crate) fn member_expression_has_optional_chain(
    context: &DestackFormatContext<'_>,
    member_id: LocalNodeId<Expression>,
) -> bool {
    expression_chain_has_optional_maybe(context, member_id)
}

/// Decide whether a parenthesized expression can be unwrapped in member object position.
pub(crate) fn should_unwrap_parenthesized_member_object(
    context: &DestackFormatContext<'_>,
    parenthesized_id: LocalNodeId<Expression>,
    inner_expression_id: LocalNodeId<Expression>,
) -> bool {
    // object members require explicit grouping: `({}).x`
    if matches!(
        context.tree.get(inner_expression_id),
        Expression::ObjectExpression { .. }
    ) {
        return false;
    }

    // keep nested grouping in type contexts stable across repeated formatting
    if is_type_context(context, parenthesized_id) {
        return false;
    }

    // decorated class expressions require explicit grouping before member access
    if expression_is_decorated_class_declaration(context, inner_expression_id) {
        return false;
    }

    if context.has_annotation(parenthesized_id) || context.has_annotation(inner_expression_id) {
        // allow unwrapping only when inner annotations are prefix comments or docs
        if !expression_has_only_prefix_comment_or_doc_annotations(context, inner_expression_id) {
            return false;
        }
    }

    if parenthesized_has_leading_inner_comments(context, parenthesized_id, inner_expression_id)
        && !expression_has_only_prefix_comment_or_doc_annotations(context, inner_expression_id)
    {
        return false;
    }

    !needs_parens_in_postfix_position(context.tree, inner_expression_id)
}

/// Return whether a member object should keep parentheses as a `new` callee.
pub(crate) fn member_object_prefers_new_callee_parentheses(
    context: &DestackFormatContext<'_>,
    object_id: LocalNodeId<Expression>,
) -> bool {
    let mut current_id = object_id;

    while let Expression::Parenthesized { expression } = context.tree.get(current_id) {
        if context.has_annotation(current_id)
            || parenthesized_has_leading_inner_trivia(context, current_id, *expression)
        {
            return false;
        }
        current_id = *expression;
    }

    matches!(
        context.tree.get(current_id),
        Expression::Call { .. } | Expression::Instantiation { .. }
    )
}

/// Return whether a member object is simple enough for `new a.b()` style callee formatting.
pub(crate) fn is_simple_new_member_object(
    tree: &NodeTree,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    match tree.get(expression_id) {
        Expression::Path { .. }
        | Expression::This
        | Expression::Super
        | Expression::PrivateIdentifier { .. } => true,
        Expression::Member { left, .. } | Expression::PrivateMember { left, .. } => {
            is_simple_new_member_object(tree, *left)
        }
        Expression::Parenthesized { expression } => is_simple_new_member_object(tree, *expression),
        _ => false,
    }
}

/// Decide whether `new (<member>)()` can unwrap outer parentheses.
pub(crate) fn should_unwrap_parenthesized_new_member_callee(
    context: &DestackFormatContext<'_>,
    parenthesized_id: LocalNodeId<Expression>,
    inner_expression_id: LocalNodeId<Expression>,
) -> bool {
    if context.has_annotation(parenthesized_id) || context.has_annotation(inner_expression_id) {
        return false;
    }

    if parenthesized_has_leading_inner_trivia(context, parenthesized_id, inner_expression_id) {
        return false;
    }

    match context.tree.get(inner_expression_id) {
        Expression::Path { .. } => true,
        Expression::Member { left, .. } | Expression::PrivateMember { left, .. } => {
            if member_expression_has_optional_chain(context, inner_expression_id) {
                return false;
            }

            is_simple_new_member_object(context.tree, *left)
        }
        _ => false,
    }
}

/// Return whether a declaration expression is a decorated class declaration.
pub(super) fn expression_is_decorated_class_declaration(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let Expression::Declaration(declaration_id) = context.tree.get(expression_id) else {
        return false;
    };
    let Declaration::Class { .. } = context.tree.get(*declaration_id) else {
        return false;
    };

    let expression_has_decorator = context.visit_annotations(expression_id, |annotations| {
        annotations.iter().any(|annotation_id| {
            matches!(
                context.annotation(*annotation_id),
                Annotation::Decorator { .. }
            )
        })
    });
    if expression_has_decorator.unwrap_or(false) {
        return true;
    }

    context
        .visit_annotations(*declaration_id, |annotations| {
            annotations.iter().any(|annotation_id| {
                matches!(
                    context.annotation(*annotation_id),
                    Annotation::Decorator { .. }
                )
            })
        })
        .unwrap_or(false)
}

/// Return whether a parenthesized expression wraps a decorated class in `extends`.
pub(super) fn parenthesized_wraps_decorated_class_extends_head(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    inner_expression_id: LocalNodeId<Expression>,
) -> bool {
    let Some((parent_id, parent_type)) = context.parent(node_id) else {
        return false;
    };
    if parent_type != NodeType::Declaration {
        return false;
    }

    let declaration_id = LocalNodeId::<Declaration>::new(parent_id);
    let extends_types = match context.tree.get(declaration_id) {
        Declaration::Class { heritage, .. } => heritage.extends_types.as_deref(),
        _ => None,
    };
    let Some(extends_types) = extends_types else {
        return false;
    };

    extends_types.contains(&node_id)
        && expression_is_decorated_class_declaration(context, inner_expression_id)
}

/// Return whether a parenthesized extends head carries prefix comment/doc annotations.
pub(super) fn parenthesized_wraps_prefix_annotated_class_extends_head(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    inner_expression_id: LocalNodeId<Expression>,
) -> bool {
    let Some((parent_id, parent_type)) = context.parent(node_id) else {
        return false;
    };
    if parent_type != NodeType::Declaration {
        return false;
    }

    let declaration_id = LocalNodeId::<Declaration>::new(parent_id);
    let extends_types = match context.tree.get(declaration_id) {
        Declaration::Class { heritage, .. } => heritage.extends_types.as_deref(),
        _ => None,
    };
    let Some(extends_types) = extends_types else {
        return false;
    };
    if !extends_types.contains(&node_id) {
        return false;
    }

    expression_has_prefix_comment_or_doc_annotation_in_left_spine(context, inner_expression_id)
}

/// Return whether annotations are only prefix comment/doc markers for this expression.
fn expression_has_only_prefix_comment_or_doc_annotations(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    context
        .visit_annotations(expression_id, |annotations| {
            !annotations.is_empty()
                && annotations.iter().all(|annotation_id| {
                    matches!(
                        context.annotation(*annotation_id),
                        Annotation::Comment {
                            position: AnnotationPosition::LinePrefix
                                | AnnotationPosition::BlockPrefix,
                            ..
                        } | Annotation::Doc {
                            position: AnnotationPosition::LinePrefix
                                | AnnotationPosition::BlockPrefix,
                            ..
                        }
                    )
                })
        })
        .unwrap_or(false)
}

/// Return whether any expression on the left spine has a prefix comment/doc annotation.
pub(super) fn expression_has_prefix_comment_or_doc_annotation_in_left_spine(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let mut current_id = expression_id;

    loop {
        if expression_has_only_prefix_comment_or_doc_annotations(context, current_id) {
            return true;
        }

        let next_id = match context.tree.get(current_id) {
            Expression::Parenthesized { expression } => Some(*expression),
            Expression::Call { left, .. }
            | Expression::Member { left, .. }
            | Expression::PrivateMember { left, .. }
            | Expression::Index { left, .. }
            | Expression::Instantiation { left, .. }
            | Expression::Maybe { left, .. }
            | Expression::Must { left, .. }
            | Expression::TypeBinary { left, .. }
            | Expression::Binary { left, .. } => Some(*left),
            _ => None,
        };

        let Some(next_id) = next_id else {
            break;
        };
        current_id = next_id;
    }

    false
}
