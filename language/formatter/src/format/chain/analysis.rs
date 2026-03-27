use super::{chain_node_left_id, transparent_inner_expression};
use crate::format::expression::is_expression_breakable;
use crate::format::operator::{expression_static_arguments, is_chain_expression};
use crate::format::tree::tree_literal_should_break;
use crate::{Annotation, DestackFormatContext};
use destack_ast::{
    AnnotationPosition, Argument, Comment, CommentStyle, Declaration, Expression, FunctionKind,
    Key, LocalNodeId, NodeTree, NodeType, Property, ScalarLiteral, TokenType, TypeBinaryOperator,
    UnaryOperator,
};
use destack_source::Span;

// static argument hugging thresholds
const HUG_STATIC_ARGUMENT_MAX_COUNT: usize = 3;

/// Get the value expression of any argument variant.
pub(crate) fn argument_value_id_if_present(
    tree: &NodeTree,
    argument_id: LocalNodeId<Argument>,
) -> Option<LocalNodeId<Expression>> {
    match tree.get(argument_id) {
        Argument::Named { value, .. }
        | Argument::Labeled { value, .. }
        | Argument::Positional { value, .. }
        | Argument::Spread { value, .. } => Some(*value),
        Argument::Error => None,
    }
}

/// Check whether source contains a comment between two expression nodes.
pub(crate) fn has_comment_between_expressions(
    context: &DestackFormatContext<'_>,
    left_id: LocalNodeId<Expression>,
    right_id: LocalNodeId<Expression>,
) -> bool {
    let left_span = context.span(left_id);
    let right_span = context.span(right_id);
    let Some(between_span) = left_span.gap_to(right_span) else {
        return false;
    };

    context.has_comment(between_span)
}

/// Return whether a `//` comment exists between two expression nodes.
pub(crate) fn has_line_comment_between_expressions(
    context: &DestackFormatContext<'_>,
    left_id: LocalNodeId<Expression>,
    right_id: LocalNodeId<Expression>,
) -> bool {
    let left_span = context.span(left_id);
    let right_span = context.span(right_id);
    let Some(between_span) = left_span.gap_to(right_span) else {
        return false;
    };

    context
        .line_comment_spans
        .iter()
        .copied()
        .any(|comment_span| {
            comment_span.file == between_span.file
                && comment_span.start >= between_span.start
                && comment_span.end <= between_span.end
        })
}

/// Check whether an argument is simple enough to stay inline in chains.
pub(crate) fn is_simple_chain_argument(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    if context.has_annotation(argument_id) {
        return false;
    }

    let Some(value_id) = argument_value_id_if_present(context.tree, argument_id) else {
        return false;
    };

    expression_is_simple_chain_argument(context, value_id, 2)
}

/// Check whether one expression is simple enough for chain-call heuristics.
fn expression_is_simple_chain_argument(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
    depth: usize,
) -> bool {
    if depth == 0 {
        return false;
    }

    let expression_id = transparent_inner_expression(context, expression_id);
    if context.has_annotation(expression_id) {
        return false;
    }

    match context.tree.get(expression_id) {
        Expression::ScalarLiteral(ScalarLiteral::RegexString { content, .. }) => {
            context.strings.get(*content).chars().count() <= 5
        }
        Expression::ScalarLiteral(_) | Expression::This | Expression::Super => true,
        Expression::TemplateExpression { .. } => !context.node_has_newline(expression_id),
        Expression::Import { arguments, .. } => arguments.as_ref().is_none_or(|arguments| {
            arguments.len() <= depth
                && arguments
                    .iter()
                    .copied()
                    .all(|argument_id| simple_argument_is_simple(context, argument_id, depth - 1))
        }),
        Expression::Path {
            static_arguments, ..
        } => static_arguments.as_ref().is_none_or(|arguments| {
            arguments.is_empty()
                || (arguments.len() <= depth
                    && arguments.iter().copied().all(|argument_id| {
                        simple_argument_is_simple(context, argument_id, depth - 1)
                    }))
        }),
        Expression::Member {
            left,
            static_arguments,
            ..
        }
        | Expression::PrivateMember {
            left,
            static_arguments,
            ..
        } => {
            expression_is_simple_chain_argument(context, *left, depth)
                && static_arguments.as_ref().is_none_or(|arguments| {
                    arguments.is_empty()
                        || (arguments.len() <= depth
                            && arguments.iter().copied().all(|argument_id| {
                                simple_argument_is_simple(context, argument_id, depth - 1)
                            }))
                })
        }
        Expression::Index { left, index, .. } => {
            expression_is_simple_chain_argument(context, *left, depth)
                && index.is_some_and(|index_id| {
                    expression_is_simple_chain_argument(context, index_id, depth)
                })
        }
        Expression::Call {
            left,
            static_arguments,
            dynamic_arguments,
            ..
        }
        | Expression::New {
            left,
            static_arguments,
            dynamic_arguments,
        } => {
            expression_is_simple_chain_argument(context, *left, depth)
                && dynamic_arguments.len() <= depth
                && dynamic_arguments
                    .iter()
                    .copied()
                    .all(|argument_id| simple_argument_is_simple(context, argument_id, depth - 1))
                && static_arguments.as_ref().is_none_or(|arguments| {
                    arguments.is_empty()
                        || (arguments.len() <= depth
                            && arguments.iter().copied().all(|argument_id| {
                                simple_argument_is_simple(context, argument_id, depth - 1)
                            }))
                })
        }
        Expression::Maybe { left, .. } | Expression::Must { left, .. } => {
            expression_is_simple_chain_argument(context, *left, depth)
        }
        Expression::Unary { operator, right } => {
            let operator_is_simple = matches!(
                operator,
                UnaryOperator::PreIncrement
                    | UnaryOperator::PreDecrement
                    | UnaryOperator::Not
                    | UnaryOperator::Plus
                    | UnaryOperator::Negate
                    | UnaryOperator::WrappingNegate
                    | UnaryOperator::ElementwiseNot
            ) || operator.is_postfix();

            operator_is_simple && expression_is_simple_chain_argument(context, *right, depth)
        }
        Expression::ObjectExpression { properties, .. } => {
            properties.iter().copied().all(|property_id| {
                depth > 0
                    && !context.has_annotation(property_id)
                    && match context.tree.get(property_id) {
                        Property::Field {
                            key,
                            value,
                            default,
                            ..
                        } => {
                            !matches!(key, Some(Key::Expression(_) | Key::NamedExpression { .. }))
                                && default.is_none()
                                && value.is_none_or(|value_id| {
                                    expression_is_simple_chain_argument(
                                        context,
                                        value_id,
                                        depth - 1,
                                    )
                                })
                        }
                        Property::Method { .. } | Property::Spread { .. } | Property::Error => {
                            false
                        }
                    }
            })
        }
        Expression::ArrayExpression { elements } => elements
            .iter()
            .copied()
            .all(|argument_id| simple_argument_is_simple(context, argument_id, depth - 1)),
        Expression::TypeBinary {
            operator,
            left,
            right,
        } => {
            matches!(
                operator,
                TypeBinaryOperator::Cast | TypeBinaryOperator::Satisfies
            ) && expression_is_simple_chain_argument(context, *left, depth)
                && !is_expression_breakable(context.tree, context.tree.get(*right))
        }
        _ => false,
    }
}

/// Check whether one child argument is simple.
fn simple_argument_is_simple(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
    depth: usize,
) -> bool {
    if depth == 0 || context.has_annotation(argument_id) {
        return false;
    }

    match context.tree.get(argument_id) {
        Argument::Named { value, .. }
        | Argument::Labeled { value, .. }
        | Argument::Positional { value, .. } => {
            expression_is_simple_chain_argument(context, *value, depth)
        }
        Argument::Spread { .. } | Argument::Error => false,
    }
}

/// Check whether static arguments are simple enough for chain heads.
pub(crate) fn is_simple_chain_static_arguments(
    context: &DestackFormatContext<'_>,
    static_arguments: &Option<Vec<LocalNodeId<Argument>>>,
) -> bool {
    match static_arguments {
        None => true,
        Some(arguments) => {
            arguments.len() <= 1
                && arguments
                    .iter()
                    .copied()
                    .all(|argument_id| is_simple_chain_argument(context, argument_id))
        }
    }
}

/// Get the root head expression of a chain.
pub(crate) fn chain_head_id(
    tree: &NodeTree,
    node_id: LocalNodeId<Expression>,
) -> LocalNodeId<Expression> {
    let mut current = node_id;

    loop {
        let next = chain_node_left_id(tree, current);

        match next {
            Some(next_id) if is_chain_expression(tree.get(next_id)) => {
                current = next_id;
            }
            Some(next_id) => return next_id,
            None => return current,
        }
    }
}

/// Check whether an expression is a numeric scalar literal.
pub(crate) fn is_numeric_scalar_literal(expression: &Expression) -> bool {
    matches!(
        expression,
        Expression::ScalarLiteral(
            ScalarLiteral::Integer(_) | ScalarLiteral::Bigint(_) | ScalarLiteral::Float(_)
        )
    )
}

/// Check whether an index expression is a numeric literal without annotations.
pub(crate) fn is_numeric_index_expression(
    context: &DestackFormatContext<'_>,
    index_id: LocalNodeId<Expression>,
) -> bool {
    // annotated indexes are never numeric simple
    if context.has_annotation(index_id) {
        return false;
    }

    let expression = context.tree.get(index_id);
    is_numeric_scalar_literal(expression)
}

/// Check whether an optional index is numeric-simple.
pub(crate) fn is_numeric_index(
    context: &DestackFormatContext<'_>,
    index: &Option<LocalNodeId<Expression>>,
) -> bool {
    match index {
        Some(index_id) => is_numeric_index_expression(context, *index_id),
        None => false,
    }
}

/// Check whether an expression is a lambda declaration.
pub(crate) fn is_lambda_expression(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let tree = context.tree;

    let Expression::Declaration(declaration_id) = tree.get(expression_id) else {
        return false;
    };

    matches!(
        tree.get(*declaration_id),
        Declaration::Function { signature, .. } if signature.kind == FunctionKind::Lambda
    )
}

/// Check whether an expression is a lambda whose body is another lambda.
pub(crate) fn is_nested_lambda_expression(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let tree = context.tree;

    let Expression::Declaration(declaration_id) = tree.get(expression_id) else {
        return false;
    };

    let Declaration::Function {
        signature,
        body: Some(body_id),
        ..
    } = tree.get(*declaration_id)
    else {
        return false;
    };

    if signature.kind != FunctionKind::Lambda {
        return false;
    }

    is_lambda_expression(context, *body_id)
}

/// Check whether an expression is nested inside a tree literal argument.
pub(crate) fn expression_is_in_tree_literal_child(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let mut current_id = expression_id.id;

    // walk up the parent chain looking for tree element arguments
    while let Some((parent_id, parent_type)) = context.parent_by_id(current_id) {
        if parent_type == NodeType::Argument {
            let argument_id = LocalNodeId::<Argument>::new(parent_id);

            // tree literals store both attributes and children as arguments
            if let Some((grand_id, grand_type)) = context.parent_by_id(parent_id)
                && grand_type == NodeType::Expression
            {
                let parent_expression = context.tree.get(LocalNodeId::<Expression>::new(grand_id));
                if let Expression::TreeExpression {
                    arguments,
                    elements,
                    ..
                } = parent_expression
                    && (arguments
                        .as_ref()
                        .is_some_and(|arguments| arguments.contains(&argument_id))
                        || elements
                            .as_ref()
                            .is_some_and(|elements| elements.contains(&argument_id)))
                {
                    return true;
                }
            }
        }

        current_id = parent_id;
    }

    false
}

/// Check whether a lambda expression body should break across lines.
pub(crate) fn lambda_expression_should_break(
    context: &DestackFormatContext<'_>,
    declaration_id: LocalNodeId<Declaration>,
) -> bool {
    let tree = context.tree;

    let Declaration::Function {
        signature,
        body: Some(body_id),
        ..
    } = tree.get(declaration_id)
    else {
        return false;
    };

    if signature.kind != FunctionKind::Lambda {
        return false;
    }

    let body_expr_id = transparent_inner_expression(context, *body_id);
    let body_expr = tree.get(body_expr_id);
    if matches!(body_expr, Expression::Block(_)) {
        return false;
    }

    if let Expression::TreeExpression {
        arguments,
        elements,
        ..
    } = body_expr
    {
        if tree_literal_should_break(context, arguments, elements) {
            return true;
        }

        // tree returning callbacks in tree literals should break for readability
        if let Some((parent_id, parent_type)) = context.parent(declaration_id)
            && parent_type == NodeType::Expression
        {
            let expression_id = LocalNodeId::<Expression>::new(parent_id);
            if expression_is_in_tree_literal_child(context, expression_id) {
                return true;
            }
        }
    }

    false
}

/// Return whether a member has only one promotable boundary comment annotation.
pub(crate) fn chain_member_has_promotable_boundary_comment(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let Some(annotations) = context.annotations(node_id) else {
        return false;
    };

    let mut found_promotable_boundary_comment = false;
    for annotation_id in annotations {
        match context.annotation(annotation_id) {
            Annotation::Blank { .. } => {}
            Annotation::Comment {
                node,
                position: AnnotationPosition::LinePostfixBoundary,
            } => {
                let comment = context.tree.get::<Comment>(node);
                match comment.style {
                    CommentStyle::Slash => {}
                    CommentStyle::Star => {
                        let comment_span = context.span(node);
                        if context.has_newline(comment_span) {
                            return false;
                        }
                    }
                }
                found_promotable_boundary_comment = true;
            }
            _ => return false,
        }
    }

    found_promotable_boundary_comment
}

/// Check if a member access has an intervening comment between receiver and property.
pub(crate) fn member_has_intervening_comment(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let Some((left, property_span)) = (match context.tree.get(node_id) {
        Expression::Member { left, .. } | Expression::PrivateMember { left, .. } => context
            .tree
            .get_main_span(node_id)
            .map(|property_span| (*left, property_span)),
        _ => None,
    }) else {
        return false;
    };
    let left_span = context.span(left);
    let left_anchor_end = expression_trivia_anchor_end(context, left);
    if property_span.start <= left_anchor_end {
        return false;
    }

    let span = Span::new(left_span.file, left_anchor_end, property_span.start);
    context.has_comment(span)
}

/// Return an expression end anchor used for chain trivia checks.
pub(crate) fn expression_trivia_anchor_end(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> u32 {
    let expression = context.tree.get(expression_id);
    let span = context.span(expression_id);

    // member like nodes often include trailing boundary comments in their full spans
    // so anchor at the property token to inspect the comment gap before parent operators
    match expression {
        Expression::Member { .. } | Expression::PrivateMember { .. } => context
            .tree
            .get_main_span(expression_id)
            .map_or(span.end, |member_span| member_span.end),
        Expression::Path { path, .. } if path.segments.len() == 1 => context
            .tree
            .get_main_span(expression_id)
            .map_or(span.end, |path_span| path_span.end),
        _ => span.end,
    }
}

/// Find the first non-trivia parent operator token after one chain node.
pub(crate) fn chain_parent_operator_start(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    parent_id: LocalNodeId<Expression>,
) -> Option<u32> {
    let node_span = context.span(node_id);
    let parent_span = context.span(parent_id);
    if node_span.file != parent_span.file {
        return None;
    }

    let node_anchor_end = expression_trivia_anchor_end(context, node_id);
    if parent_span.end <= node_anchor_end {
        return None;
    }

    context
        .first_non_trivia_token_between(node_anchor_end, parent_span.end)
        .map(|token| token.span.start)
}

/// Check if a chain node has source breaks or comments before its parent operator.
pub(crate) fn chain_has_parent_intervening_break_or_comment(
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
    let parent = context.tree.get(parent_id);
    let parent_uses_node_as_left = match parent {
        Expression::Member { left, .. }
        | Expression::PrivateMember { left, .. }
        | Expression::Call { left, .. }
        | Expression::Index { left, .. }
        | Expression::Instantiation { left, .. }
        | Expression::Maybe { left, .. }
        | Expression::Must { left, .. } => *left == node_id,
        _ => false,
    };
    if !parent_uses_node_as_left {
        return false;
    }

    let should_check_parent_gap = match parent {
        Expression::Member { .. } | Expression::PrivateMember { .. } => true,
        Expression::Call { .. }
        | Expression::Index { .. }
        | Expression::Instantiation { .. }
        | Expression::Maybe { .. }
        | Expression::Must { .. } => matches!(
            context.tree.get(node_id),
            Expression::Member { .. } | Expression::PrivateMember { .. } | Expression::Path { .. }
        ),
        _ => false,
    };
    if !should_check_parent_gap {
        return false;
    }

    let node_span = context.span(node_id);
    let node_anchor_end = expression_trivia_anchor_end(context, node_id);
    let Some(parent_operator_start) = chain_parent_operator_start(context, node_id, parent_id)
    else {
        return false;
    };
    if parent_operator_start <= node_anchor_end {
        return false;
    }
    let between_span = Span::new(node_span.file, node_anchor_end, parent_operator_start);

    context.has_comment(between_span)
}

/// Check whether a member access uses a private hash (`.#name`).
pub(crate) fn member_is_private_hash(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    if matches!(context.tree.get(node_id), Expression::PrivateMember { .. }) {
        return true;
    }

    let Some(property_span) = context.tree.get_main_span(node_id) else {
        return false;
    };

    let Some(prev_token) = context.token_before_token_start(property_span.start) else {
        return false;
    };

    prev_token.token.ty == TokenType::Hash
}

/// Decide whether postfix annotations on a path belong after the last segment.
pub(crate) fn path_postfix_annotations_emit_on_tail(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    segments_len: usize,
) -> bool {
    let Some(last_segment_start) = path_last_segment_start(context, node_id, segments_len) else {
        return false;
    };

    context
        .visit_annotations(node_id, |annotations| {
            let mut has_postfix = false;
            for annotation_id in annotations {
                let annotation = context.annotation(*annotation_id);
                let position = annotation.position();
                let is_postfix = matches!(
                    position,
                    AnnotationPosition::LinePostfix
                        | AnnotationPosition::LinePostfixBoundary
                        | AnnotationPosition::BlockInfix
                        | AnnotationPosition::BlockPostfix
                );
                if !is_postfix {
                    continue;
                }

                has_postfix = true;
                let span = context.annotation_span(*annotation_id);
                if span.start < last_segment_start {
                    return false;
                }
            }

            if !has_postfix {
                return true;
            }

            true
        })
        .unwrap_or(true)
}

/// Find the start byte of the last path segment token.
pub(crate) fn path_last_segment_start(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    segments_len: usize,
) -> Option<u32> {
    if segments_len == 0 {
        return None;
    }

    let span = context.span(node_id);
    context.nth_token_type_start_in_span(span, TokenType::Identifier, segments_len)
}

/// Decide whether static argument lists should expand at the list level.
pub(crate) fn should_expand_static_argument_list(
    context: &DestackFormatContext<'_>,
    static_arguments: &[LocalNodeId<Argument>],
) -> bool {
    if static_arguments.len() != 1 {
        return false;
    }

    // keep single direct object-like type arguments hugged as `<{ ... }>`
    let Some(value_id) = argument_value_id_if_present(context.tree, static_arguments[0]) else {
        return true;
    };
    let value_id = transparent_inner_expression(context, value_id);
    if matches!(
        context.tree.get(value_id),
        Expression::ObjectExpression { .. } | Expression::TypeMapped { .. }
    ) {
        return false;
    }

    // expand list-level wrappers only when nested type arguments are structurally multiline
    expression_static_arguments(context.tree.get(value_id)).is_some_and(|nested_arguments| {
        nested_arguments.iter().copied().any(|nested_argument_id| {
            let Some(nested_value_id) =
                argument_value_id_if_present(context.tree, nested_argument_id)
            else {
                return true;
            };
            let nested_value_id = transparent_inner_expression(context, nested_value_id);
            static_argument_expression_has_complex_nested_structure(context, nested_value_id)
        })
    })
}

/// Return whether one static argument expression has nested structure that should force expansion.
fn static_argument_expression_has_complex_nested_structure(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    match context.tree.get(expression_id) {
        Expression::ObjectExpression { properties, .. } => properties.len() > 1,
        Expression::TypeMapped { .. } => true,
        _ => expression_static_arguments(context.tree.get(expression_id)).is_some_and(
            |nested_arguments| {
                nested_arguments.iter().copied().any(|nested_argument_id| {
                    let Some(nested_value_id) =
                        argument_value_id_if_present(context.tree, nested_argument_id)
                    else {
                        return true;
                    };
                    let nested_value_id = transparent_inner_expression(context, nested_value_id);
                    static_argument_expression_has_complex_nested_structure(
                        context,
                        nested_value_id,
                    )
                })
            },
        ),
    }
}

/// Return whether static arguments are structurally safe for hugged inline formatting.
pub(crate) fn static_argument_list_is_hug_safe(
    context: &DestackFormatContext<'_>,
    static_arguments: &[LocalNodeId<Argument>],
) -> bool {
    if static_arguments.is_empty() || static_arguments.len() > HUG_STATIC_ARGUMENT_MAX_COUNT {
        return false;
    }

    static_arguments.iter().copied().all(|argument_id| {
        if context.has_non_blank_annotation(argument_id) {
            return false;
        }

        let Some(argument_value_id) = argument_value_id_if_present(context.tree, argument_id)
        else {
            return false;
        };
        let argument_value_id = transparent_inner_expression(context, argument_value_id);
        if context.node_has_newline(argument_value_id) {
            return false;
        }

        if let Expression::Path { path, .. } = context.tree.get(argument_value_id)
            && path.segments.len() == 1
            && context.strings.get(path.segments[0]).chars().count() > 12
        {
            return false;
        }

        static_argument_expression_is_hug_safe(context, argument_value_id)
    })
}

/// Return whether one static argument expression is structurally safe for hugged inline formatting.
fn static_argument_expression_is_hug_safe(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    match context.tree.get(expression_id) {
        Expression::ScalarLiteral(_)
        | Expression::TypeLiteral(_)
        | Expression::PrivateIdentifier { .. } => true,
        Expression::Path {
            static_arguments, ..
        } => static_arguments.as_deref().is_none_or(|static_arguments| {
            static_argument_list_is_hug_safe(context, static_arguments)
        }),
        Expression::Member {
            left,
            static_arguments,
            ..
        }
        | Expression::PrivateMember {
            left,
            static_arguments,
            ..
        } => {
            static_argument_expression_is_hug_safe(context, *left)
                && static_arguments.as_deref().is_none_or(|static_arguments| {
                    static_argument_list_is_hug_safe(context, static_arguments)
                })
        }
        // allow generic index arguments to break with line width
        Expression::Index { .. } | Expression::TypeIndex { .. } => false,
        Expression::Parenthesized { expression } | Expression::Statement(expression) => {
            static_argument_expression_is_hug_safe(context, *expression)
        }
        Expression::Binary {
            operator:
                destack_ast::BinaryOperator::ElementwiseAnd
                | destack_ast::BinaryOperator::ElementwiseOr
                | destack_ast::BinaryOperator::ElementwiseXor,
            left,
            right,
        } => {
            static_argument_expression_is_hug_safe(context, *left)
                && static_argument_expression_is_hug_safe(context, *right)
        }
        _ => false,
    }
}

/// Decide whether a mapped type should force multiline formatting.
pub(crate) fn should_force_multiline_mapped_type(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    value_id: LocalNodeId<Expression>,
) -> bool {
    if context.has_annotation(node_id) {
        return true;
    }

    let span = context.span(node_id);
    if context.has_comment(span) {
        return true;
    }

    if context.span_has_token_type(span, TokenType::Semicolon) {
        return true;
    }

    is_expression_breakable(context.tree, context.tree.get(value_id))
}
