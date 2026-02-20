use super::super::{
    Declaration, DestackFormatContext, Expression, FunctionKind, LocalNodeId, NodeTree, NodeType,
    TypeBinaryOperator,
};
use super::member::{
    expression_has_prefix_comment_or_doc_annotation_in_left_spine,
    expression_is_decorated_class_declaration, parenthesized_wraps_decorated_class_extends_head,
    parenthesized_wraps_prefix_annotated_class_extends_head,
};
use super::trivia::{
    parenthesized_has_leading_inner_newline, parenthesized_has_leading_inner_trivia,
};
use super::type_drop::should_drop_parenthesized_type_expression;
use crate::analysis::timing::tags;

/// Return whether a parenthesized call callee wrapper can drop safely.
fn parenthesized_call_callee_wrapper_can_drop(
    context: &DestackFormatContext<'_>,
    parent_expression: &Expression,
    node_id: LocalNodeId<Expression>,
    inner_expression_id: LocalNodeId<Expression>,
) -> bool {
    let Expression::Call { left, .. } = parent_expression else {
        return false;
    };
    if *left != node_id {
        return false;
    }

    if !expression_has_trailing_static_instantiation(context.tree, inner_expression_id) {
        return false;
    }

    if context.has_annotation(node_id) || context.has_annotation(inner_expression_id) {
        return false;
    }

    !parenthesized_has_leading_inner_trivia(context, node_id, inner_expression_id)
}

/// Return whether one expression ends in static instantiation arguments.
fn expression_has_trailing_static_instantiation(
    tree: &NodeTree,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    match tree.get(expression_id) {
        Expression::Instantiation {
            static_arguments, ..
        } => !static_arguments.is_empty(),
        Expression::Path {
            static_arguments, ..
        }
        | Expression::Member {
            static_arguments, ..
        }
        | Expression::PrivateMember {
            static_arguments, ..
        } => static_arguments
            .as_ref()
            .is_some_and(|arguments| !arguments.is_empty()),
        Expression::Parenthesized { expression } => {
            expression_has_trailing_static_instantiation(tree, *expression)
        }
        _ => false,
    }
}

/// Decide whether a parenthesized expression should drop wrappers in generic expression contexts.
pub(crate) fn should_drop_parenthesized_expression_wrapper(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    inner_expression_id: LocalNodeId<Expression>,
) -> bool {
    let _timing = context.timing_scope(tags::FORMAT_EXPRESSION_PRIMARY_PARENTHESES_DROP_POLICY);
    // decorated class extends heads must keep explicit grouping
    if parenthesized_wraps_decorated_class_extends_head(context, node_id, inner_expression_id) {
        return false;
    }

    // closure-style cast wrappers in class heritage should stay explicit
    if parenthesized_wraps_prefix_annotated_class_extends_head(
        context,
        node_id,
        inner_expression_id,
    ) {
        return false;
    }

    let should_drop_type_parentheses =
        should_drop_parenthesized_type_expression(context, node_id, inner_expression_id);

    let Some((parent_id, parent_type)) = context.parent(node_id) else {
        return should_drop_type_parentheses;
    };
    if parent_type != NodeType::Expression {
        // call/new arguments can unwrap decorated class expressions
        let should_drop_argument_decorated_class_wrapper = parent_type == NodeType::Argument
            && !context.has_annotation(node_id)
            && !parenthesized_has_leading_inner_newline(context, node_id, inner_expression_id)
            && expression_is_decorated_class_declaration(context, inner_expression_id);
        if should_drop_argument_decorated_class_wrapper {
            return true;
        }

        // declarator wrappers can drop when left spine carries prefix comment/doc annotations
        let should_drop_declarator_prefix_wrapper = parent_type == NodeType::Declarator
            && !context.has_annotation(node_id)
            && !parenthesized_has_leading_inner_newline(context, node_id, inner_expression_id)
            && expression_has_prefix_comment_or_doc_annotation_in_left_spine(
                context,
                inner_expression_id,
            );
        if should_drop_declarator_prefix_wrapper {
            return true;
        }

        return should_drop_type_parentheses;
    }

    let parent_id = LocalNodeId::<Expression>::new(parent_id);
    let parent_expression = context.tree.get(parent_id);
    let inner_expression = context.tree.get(inner_expression_id);
    let should_drop_statement_type_binary_wrapper = matches!(
        parent_expression,
        Expression::Statement(inner_id) if inner_id.id == node_id.id
    ) && matches!(
        inner_expression,
        Expression::TypeBinary {
            operator: TypeBinaryOperator::Cast | TypeBinaryOperator::Satisfies,
            ..
        }
    ) && !context.has_annotation(node_id);
    let should_drop_assignment_must = matches!(
        parent_expression,
        Expression::Assign { left, .. } if *left == node_id
    ) && matches!(inner_expression, Expression::Must { .. });
    let should_drop_statement_lambda = matches!(
        parent_expression,
        Expression::Statement(inner_id) if inner_id.id == node_id.id
    ) && matches!(
        inner_expression,
        Expression::Declaration(declaration_id)
            if matches!(
                context.tree.get(*declaration_id),
                Declaration::Function { signature, .. } if signature.kind == FunctionKind::Lambda
            )
    ) && !context.has_annotation(node_id);
    let should_drop_call_callee_instantiation_wrapper = parenthesized_call_callee_wrapper_can_drop(
        context,
        parent_expression,
        node_id,
        inner_expression_id,
    );
    should_drop_statement_type_binary_wrapper
        || should_drop_assignment_must
        || should_drop_statement_lambda
        || should_drop_call_callee_instantiation_wrapper
        || should_drop_type_parentheses
}
