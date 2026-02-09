use crate::DestackFormatContext;
use destack_ast::{
    Argument, Expression, LocalNodeId, NodeTree, Pattern, Property, ScalarLiteral, TokenType,
    TypeBinaryOperator, UnaryOperator,
};
use destack_source::Span;

/// Whether an expression is "trivial" (prefers to be fully inline).
pub fn is_trivial_expression(tree: &NodeTree, expression: &Expression) -> bool {
    match expression {
        Expression::ScalarLiteral(_)
        | Expression::TypeLiteral(_)
        | Expression::PrivateIdentifier { .. } => true,
        Expression::ObjectExpression { ty, properties, .. } => {
            ty.is_none()
                && properties.len() <= 5
                && properties
                    .iter()
                    .all(|property| is_trivial_property(tree, tree.get(*property)))
        }
        Expression::Unary { operator: _, right } => is_trivial_expression(tree, tree.get(*right)),
        Expression::Index { left, index, .. } => index
            .as_ref()
            .map_or(is_trivial_expression(tree, tree.get(*left)), |index_id| {
                is_trivial_expression(tree, tree.get(*index_id))
            }),
        Expression::ReferenceOf {
            mutability: _,
            variance: _,
            right,
        } => is_trivial_expression(tree, tree.get(*right)),
        Expression::PointerOf {
            mutability: _,
            right,
        } => is_trivial_expression(tree, tree.get(*right)),
        Expression::Member { left, .. } | Expression::PrivateMember { left, .. } => {
            is_trivial_expression(tree, tree.get(*left))
        }
        Expression::Path {
            path,
            static_arguments,
        } => {
            path.segments.len() <= 3
                && static_arguments.as_deref().is_none_or(|static_arguments| {
                    static_arguments_are_trivial(tree, static_arguments)
                })
        }
        Expression::TypeBinary {
            left,
            operator: TypeBinaryOperator::Cast | TypeBinaryOperator::Satisfies,
            right,
        } => {
            is_trivial_expression(tree, tree.get(*left))
                && is_trivial_expression(tree, tree.get(*right))
        }
        _ => false,
    }
}

/// Return whether static arguments stay concise when inlined.
fn static_arguments_are_trivial(
    tree: &NodeTree,
    static_arguments: &[LocalNodeId<Argument>],
) -> bool {
    static_arguments
        .iter()
        .all(|argument_id| is_trivial_argument(tree, tree.get(*argument_id)))
}

/// Whether an expression is "complex" (prefers to be multiline).
pub fn is_complex_expression(_tree: &NodeTree, expression: &Expression) -> bool {
    match expression {
        Expression::Statement { .. } => true,
        // only expand objects with many (>3) properties by default
        // let best_fitting handle the rest based on line width
        Expression::ObjectExpression { properties, .. } => properties.len() > 3,
        Expression::TreeExpression { .. } => true,
        _ => false,
    }
}

/// Whether an argument is "trivial" (prefers to be inline).
pub fn is_trivial_argument(tree: &NodeTree, argument: &Argument) -> bool {
    match argument {
        Argument::Named { name: _, value, .. }
        | Argument::Labeled {
            label: _, value, ..
        }
        | Argument::Positional { value, .. }
        | Argument::Spread { value, .. } => is_trivial_expression(tree, tree.get(*value)),
    }
}

/// Whether a property is "trivial" (prefers to be inline).
pub fn is_trivial_property(tree: &NodeTree, property: &Property) -> bool {
    match property {
        Property::Field { value, default, .. } => {
            value.is_none_or(|value| is_trivial_expression(tree, tree.get(value)))
                && default.is_none_or(|default| is_trivial_expression(tree, tree.get(default)))
        }
        Property::Method { body, .. } => {
            body.is_none_or(|body| is_trivial_expression(tree, tree.get(body)))
        }
        Property::Spread { value, .. } => is_trivial_expression(tree, tree.get(*value)),
    }
}

/// Whether an argument is "complex" (prefers to be multiline).
pub fn is_complex_argument(tree: &NodeTree, argument: &Argument) -> bool {
    match argument {
        Argument::Named { name: _, value, .. }
        | Argument::Labeled {
            label: _, value, ..
        }
        | Argument::Positional { value, .. }
        | Argument::Spread { value, .. } => is_complex_expression(tree, tree.get(*value)),
    }
}

/// Whether the expression can break itself across multiple lines.
pub fn is_expression_breakable(tree: &NodeTree, expression: &Expression) -> bool {
    match expression {
        Expression::ArrayExpression { elements, .. } => !elements.is_empty(),
        Expression::TupleExpression { elements, .. } => !elements.is_empty(),
        Expression::SequenceExpression { expressions, .. } => !expressions.is_empty(),
        Expression::ObjectExpression { ty, properties, .. } => {
            ty.is_some_and(|ty| is_expression_breakable(tree, tree.get(ty)))
                || !properties.is_empty()
        }
        Expression::TreeExpression {
            arguments,
            elements,
            ..
        } => {
            arguments
                .as_ref()
                .is_some_and(|arguments| !arguments.is_empty())
                || elements
                    .as_ref()
                    .is_some_and(|elements| !elements.is_empty())
        }
        Expression::Call {
            dynamic_arguments, ..
        } => !dynamic_arguments.is_empty(),
        Expression::New {
            dynamic_arguments, ..
        } => !dynamic_arguments.is_empty(),
        Expression::Match { .. } => true,
        Expression::Path {
            static_arguments, ..
        } => static_arguments
            .as_ref()
            .is_some_and(|static_arguments| !static_arguments.is_empty()),
        Expression::If { .. }
        | Expression::Loop { .. }
        | Expression::Try { .. }
        | Expression::Block { .. }
        | Expression::ForEach { .. }
        | Expression::For { .. }
        | Expression::While { .. }
        | Expression::Import { .. }
        | Expression::Export { .. } => true,
        Expression::Binary { .. }
        | Expression::TypeBinary { .. }
        | Expression::TypeConditional { .. }
        | Expression::TypeMapped { .. }
        | Expression::TypeTemplateLiteral { .. } => true,
        _ => false,
    }
}

/// Check if a pattern can expand to multiple lines (object, array, tuple patterns).
pub fn is_pattern_breakable(tree: &NodeTree, pattern_id: LocalNodeId<Pattern>) -> bool {
    let pattern = tree.get(pattern_id);
    match pattern {
        Pattern::Object { fields } | Pattern::TaggedObject { fields, .. } => !fields.is_empty(),
        Pattern::Array { fields }
        | Pattern::Tuple { fields }
        | Pattern::TaggedTuple { fields, .. } => !fields.is_empty(),
        _ => false,
    }
}

/// Check if a span includes any comment tokens.
pub(super) fn span_has_comment(context: &DestackFormatContext<'_>, span: Span) -> bool {
    context.has_comment(span)
}

/// Return whether array elements are simple enough for concise fill formatting.
pub(super) fn array_elements_are_fill_candidates(
    tree: &NodeTree,
    elements: &[LocalNodeId<Argument>],
) -> bool {
    elements
        .iter()
        .all(|element_id| array_element_is_fill_candidate(tree, *element_id))
}

/// Return whether comments in an array appear only before the first or after the last element.
pub(super) fn array_has_only_boundary_comments(
    context: &DestackFormatContext<'_>,
    array_span: Span,
    elements: &[LocalNodeId<Argument>],
) -> bool {
    let (Some(first_element), Some(last_element)) = (elements.first(), elements.last()) else {
        return false;
    };

    let first_span = context.get_span(*first_element);
    let last_span = context.get_span(*last_element);

    let has_internal_comment =
        context
            .tokens
            .iter()
            .chain(context.side_tokens.iter())
            .any(|token| {
                if !array_span.intersects(token.span) || !is_comment_token_type(token.token.ty) {
                    return false;
                }

                let is_before_first = token.span.end <= first_span.start;
                let is_after_last = token.span.start >= last_span.end;
                !(is_before_first || is_after_last)
            });

    !has_internal_comment
}

/// Return whether a token type is a comment token.
fn is_comment_token_type(token_type: TokenType) -> bool {
    matches!(
        token_type,
        TokenType::LineComment
            | TokenType::BlockComment
            | TokenType::DocLineComment
            | TokenType::DocBlockComment
    )
}

/// Return whether a single array element is a concise fill candidate.
fn array_element_is_fill_candidate(tree: &NodeTree, element_id: LocalNodeId<Argument>) -> bool {
    let value_id = match tree.get(element_id) {
        Argument::Positional { value, .. } => *value,
        _ => return false,
    };

    match tree.get(value_id) {
        Expression::ScalarLiteral(_) => true,
        Expression::Unary { operator, right } => {
            matches!(operator, UnaryOperator::Plus | UnaryOperator::Negate)
                && matches!(
                    tree.get(*right),
                    Expression::ScalarLiteral(
                        ScalarLiteral::Integer(_)
                            | ScalarLiteral::Bigint(_)
                            | ScalarLiteral::Float(_)
                    )
                )
        }
        _ => false,
    }
}
