use super::super::{
    Annotation, AnnotationPosition, BinaryOperator, Declaration, DestackFormatContext, Expression,
    FunctionKind, LocalNodeId, NodeType, TokenType, TypeBinaryOperator,
};
use super::member::parenthesized_wraps_decorated_class_extends_head;
use super::trivia::parenthesized_has_leading_inner_trivia;
use crate::analysis::scan::{
    first_non_trivia_token_in_span, next_non_whitespace_after_span, nth_non_trivia_token_in_span,
    previous_non_whitespace_token_before_span,
};
use crate::analysis::timing::tags;
use crate::operator::{is_simple_type_binary_left_expression, is_type_context};

/// Keep parentheses for cast or satisfies expressions in statement position.
pub(crate) fn type_binary_is_statement_expression(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let Some((parent_id, parent_type)) = context.parent(node_id) else {
        return false;
    };
    if parent_type != NodeType::Expression {
        return false;
    }

    let parent_id = LocalNodeId::<Expression>::new(parent_id);
    matches!(
        context.tree.get(parent_id),
        Expression::Statement(inner) if *inner == node_id
    )
}

/// Return whether a type-binary expression is wrapped by one statement parenthesized node.
pub(crate) fn type_binary_is_parenthesized_statement_expression(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let Some((parent_id, parent_type)) = context.parent(node_id) else {
        return false;
    };
    if parent_type != NodeType::Expression {
        return false;
    }

    let parent_id = LocalNodeId::<Expression>::new(parent_id);
    let Expression::Parenthesized { expression } = context.tree.get(parent_id) else {
        return false;
    };
    if *expression != node_id {
        return false;
    }

    let Some((grandparent_id, grandparent_type)) = context.parent(parent_id) else {
        return false;
    };
    if grandparent_type != NodeType::Expression {
        return false;
    }

    let grandparent_id = LocalNodeId::<Expression>::new(grandparent_id);
    matches!(
        context.tree.get(grandparent_id),
        Expression::Statement(inner_id) if *inner_id == parent_id
    )
}

/// Return whether any parenthesized expression ancestor has leading inner trivia.
pub(crate) fn has_parenthesized_ancestor_with_leading_inner_trivia(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let mut current_id = node_id;

    while let Some((parent_id, parent_type)) = context.parent(current_id) {
        if parent_type != NodeType::Expression {
            break;
        }

        let parent_id = LocalNodeId::<Expression>::new(parent_id);
        if let Expression::Parenthesized { expression } = context.tree.get(parent_id)
            && *expression == current_id
            && parenthesized_has_leading_inner_trivia(context, parent_id, current_id)
        {
            return true;
        }

        current_id = parent_id;
    }

    false
}

/// Return whether cast or satisfies appears as the parenthesized callee of a `new` expression.
pub(crate) fn type_binary_is_parenthesized_new_callee(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let Some((parent_id, parent_type)) = context.parent(node_id) else {
        return false;
    };
    if parent_type != NodeType::Expression {
        return false;
    }

    let parent_id = LocalNodeId::<Expression>::new(parent_id);
    let Expression::Parenthesized { expression } = context.tree.get(parent_id) else {
        return false;
    };
    if *expression != node_id {
        return false;
    }

    let Some((grandparent_id, grandparent_type)) = context.parent(parent_id) else {
        return false;
    };
    if grandparent_type != NodeType::Expression {
        return false;
    }

    let grandparent_id = LocalNodeId::<Expression>::new(grandparent_id);
    matches!(
        context.tree.get(grandparent_id),
        Expression::New { left, .. } if *left == parent_id
    )
}

/// Decide whether cast or satisfies can drop a parenthesized left side.
pub(crate) fn should_drop_type_binary_left_parentheses(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    parenthesized_id: LocalNodeId<Expression>,
    left_id: LocalNodeId<Expression>,
) -> bool {
    let left_is_cast_chain = matches!(
        context.tree.get(left_id),
        Expression::TypeBinary {
            operator: TypeBinaryOperator::Cast | TypeBinaryOperator::Satisfies,
            ..
        }
    );

    if context.has_annotation(parenthesized_id) {
        return false;
    }

    if context.has_annotation(left_id) && !left_is_cast_chain {
        return false;
    }

    if parenthesized_has_leading_inner_trivia(context, parenthesized_id, left_id)
        && !left_is_cast_chain
    {
        return false;
    }

    if type_binary_is_statement_expression(context, node_id)
        || type_binary_is_parenthesized_statement_expression(context, node_id)
    {
        return false;
    }

    if has_parenthesized_ancestor_with_leading_inner_trivia(context, node_id) && !left_is_cast_chain
    {
        return false;
    }

    is_simple_type_binary_left_expression(context.tree, left_id)
}

/// Return whether this parenthesized expression is a top-level type alias value.
pub(crate) fn parenthesized_is_top_level_type_alias_value(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let Some((parent_id, parent_type)) = context.parent(node_id) else {
        return false;
    };
    if parent_type != NodeType::Declaration {
        return false;
    }

    let declaration_id = LocalNodeId::<Declaration>::new(parent_id);
    matches!(
        context.tree.get(declaration_id),
        Declaration::Type { value, .. } if *value == node_id
    )
}

/// Return whether an expression starts with a type union or intersection chain.
pub(crate) fn expression_is_type_binary_chain_head(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    match context.tree.get(expression_id) {
        Expression::Parenthesized { expression } => {
            expression_is_type_binary_chain_head(context, *expression)
        }
        Expression::Binary { operator, .. } => {
            matches!(
                operator,
                BinaryOperator::ElementwiseOr | BinaryOperator::ElementwiseAnd
            ) && is_type_context(context, expression_id)
        }
        _ => false,
    }
}

/// Return whether one parenthesized expression starts with `|` or `&` inside the wrapper.
pub(crate) fn parenthesized_leading_type_grouping_operator(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> Option<BinaryOperator> {
    if !matches!(
        context.tree.get(expression_id),
        Expression::Parenthesized { .. }
    ) {
        return None;
    }

    let span = context.span(expression_id);
    let first_token = first_non_trivia_token_in_span(context, span)?;
    if first_token.token.ty != TokenType::OpenParenthesis {
        return None;
    }

    let second_token = nth_non_trivia_token_in_span(context, span, 1)?;
    match second_token.token.ty {
        TokenType::ElementwiseOr => Some(BinaryOperator::ElementwiseOr),
        TokenType::ElementwiseAnd => Some(BinaryOperator::ElementwiseAnd),
        _ => None,
    }
}

/// Return whether expression annotations are only prefix comments before `|` or `&`.
fn expression_has_only_type_grouping_prefix_annotations(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let Some(annotation_ids) = context.annotations(expression_id) else {
        return false;
    };
    if annotation_ids.is_empty() {
        return false;
    }

    annotation_ids.into_iter().all(|annotation_id| {
        let annotation = context.annotation(annotation_id);
        let Annotation::Comment {
            position: AnnotationPosition::BlockPrefix | AnnotationPosition::LinePrefix,
            ..
        } = annotation
        else {
            return false;
        };

        let annotation_span = context.annotation_span(annotation_id);
        matches!(
            next_non_whitespace_after_span(context, annotation_span),
            Some('/' | '|' | '&')
        )
    })
}

/// Decide whether a parenthesized type expression can drop wrappers.
pub(crate) fn should_drop_parenthesized_type_expression(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    inner_id: LocalNodeId<Expression>,
) -> bool {
    let _timing = context.timing_scope(tags::FORMAT_EXPRESSION_PRIMARY_PARENTHESES_TYPE_DROP);

    // wrapper annotations are semantic boundaries and should keep grouping
    if context.has_annotation(node_id) {
        return false;
    }

    // inner annotations are usually semantic boundaries
    // but leading type-grouping prefix comments are compatible with dropping the wrapper
    if context.has_annotation(inner_id)
        && !expression_has_only_type_grouping_prefix_annotations(context, inner_id)
    {
        return false;
    }

    // decorated class extends heads must preserve explicit grouping
    if parenthesized_wraps_decorated_class_extends_head(context, node_id, inner_id) {
        return false;
    }

    if !is_type_context(context, node_id) {
        return false;
    }

    // drop redundant simple wrappers in array element position: `(number)[]` -> `number[]`
    if let Some((parent_id, parent_type)) = context.parent(node_id)
        && parent_type == NodeType::Expression
    {
        let parent_expression_id = LocalNodeId::<Expression>::new(parent_id);
        if let Expression::Index { left, index, .. } = context.tree.get(parent_expression_id)
            && *left == node_id
            && index.is_none()
            && !context.has_annotation(node_id)
            && !context.has_annotation(inner_id)
            && is_simple_type_binary_left_expression(context.tree, inner_id)
        {
            return true;
        }
    }

    if parenthesized_associative_type_binary_can_drop(context, node_id, inner_id) {
        return true;
    }

    if parenthesized_is_top_level_type_alias_value(context, node_id)
        && expression_is_type_binary_chain_head(context, inner_id)
    {
        return true;
    }

    if !parenthesized_type_grouping_drop_is_safe_in_parent(context, node_id) {
        return false;
    }

    let node_span = context.span(node_id);
    let leading_grouping_operator = parenthesized_leading_type_grouping_operator(context, node_id);
    let has_leading_union_source = first_non_trivia_token_in_span(context, node_span)
        .is_some_and(|token| token.token.ty == TokenType::ElementwiseOr);
    let has_prefix_union_operator = previous_non_whitespace_token_before_span(context, node_span)
        .is_some_and(|token| token.token.ty == TokenType::ElementwiseOr);
    if !has_prefix_union_operator
        && !has_leading_union_source
        && leading_grouping_operator.is_none()
    {
        return false;
    }

    if let Some(leading_operator) = leading_grouping_operator {
        if let Expression::Binary { operator, .. } = context.tree.get(inner_id)
            && *operator == leading_operator
            && is_type_context(context, inner_id)
        {
            return true;
        } else if let Expression::Parenthesized { expression } = context.tree.get(inner_id)
            && (matches!(
                context.tree.get(*expression),
                Expression::Binary { operator, .. }
                    if *operator == leading_operator && is_type_context(context, *expression)
            ) || is_simple_type_binary_left_expression(context.tree, *expression))
        {
            return true;
        } else if is_simple_type_binary_left_expression(context.tree, inner_id) {
            return true;
        }
    }

    match context.tree.get(inner_id) {
        Expression::TypeConditional { .. } => true,
        Expression::Declaration(declaration_id) => matches!(
            context.tree.get(*declaration_id),
            Declaration::Function { signature, .. } if signature.kind == FunctionKind::Lambda
        ),
        _ => false,
    }
}

/// Return whether dropping a parenthesized type grouping is safe in the parent expression context.
fn parenthesized_type_grouping_drop_is_safe_in_parent(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let Some((parent_id, parent_type)) = context.parent(node_id) else {
        return true;
    };
    if parent_type != NodeType::Expression {
        return true;
    }

    let parent_expression_id = LocalNodeId::<Expression>::new(parent_id);
    match context.tree.get(parent_expression_id) {
        Expression::Index { left, .. } | Expression::TypeIndex { left, .. } => *left != node_id,
        _ => true,
    }
}

/// Return whether a binary operator is associative in type contexts.
pub(crate) fn is_associative_type_binary_operator(operator: BinaryOperator) -> bool {
    matches!(
        operator,
        BinaryOperator::ElementwiseOr | BinaryOperator::ElementwiseAnd
    )
}

/// Decide whether an associative type binary can drop redundant wrappers.
pub(crate) fn parenthesized_associative_type_binary_can_drop(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    inner_id: LocalNodeId<Expression>,
) -> bool {
    let Expression::Binary {
        operator: inner_operator,
        ..
    } = context.tree.get(inner_id)
    else {
        return false;
    };
    if !is_associative_type_binary_operator(*inner_operator) {
        return false;
    }

    let Some((parent_id, parent_type)) = context.parent(node_id) else {
        return false;
    };
    if parent_type != NodeType::Expression {
        return false;
    }

    let parent_id = LocalNodeId::<Expression>::new(parent_id);
    match context.tree.get(parent_id) {
        Expression::Parenthesized { expression } => *expression == node_id,
        Expression::Binary {
            left,
            operator,
            right,
        } => {
            (*left == node_id || *right == node_id)
                && *operator == *inner_operator
                && is_type_context(context, parent_id)
        }
        _ => false,
    }
}
