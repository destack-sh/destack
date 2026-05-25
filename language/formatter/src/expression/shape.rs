use crate::DestackFormatContext;
use crate::operator::assign_pattern_target_expression;
use destack_dir::{
    Argument, Expression, GenericArgument, IfCondition, IfForm, LocalNodeId, NodeType, Pattern,
    Property, ScalarLiteral, TokenType, Tree, TypeExpression, UnaryOperator,
};
use destack_source::Span;

/// One expression together with access to its left spine.
#[derive(Debug, Copy, Clone)]
pub(crate) struct ExpressionLeftSide {
    /// The current expression on the left spine.
    expression_id: LocalNodeId<Expression>,
}

impl ExpressionLeftSide {
    /// Create one left-side cursor for an expression.
    #[inline]
    pub(crate) fn new(expression_id: LocalNodeId<Expression>) -> Self {
        Self { expression_id }
    }

    /// Return the current expression.
    #[inline]
    pub(crate) fn expression_id(self) -> LocalNodeId<Expression> {
        self.expression_id
    }

    /// Return the next expression on the left spine.
    pub(crate) fn left(self, context: &DestackFormatContext<'_>) -> Option<Self> {
        let expression_id = match context.tree.get(self.expression_id) {
            Expression::Parenthesized { expression } => Some(*expression),
            Expression::SequenceExpression { expressions } => expressions.first().copied(),
            Expression::Member { left, .. }
            | Expression::PrivateMember { left, .. }
            | Expression::Index { left, .. }
            | Expression::Call { left, .. }
            | Expression::Instantiation { left, .. }
            | Expression::Maybe { left, .. }
            | Expression::Must { left, .. }
            | Expression::As {
                expression: left, ..
            }
            | Expression::Satisfies {
                expression: left, ..
            }
            | Expression::RangeExpression {
                start: Some(left), ..
            }
            | Expression::Binary { left, .. } => Some(*left),
            Expression::Assign { left, .. } => assign_pattern_target_expression(context, *left),
            Expression::Is { value, .. } | Expression::InstanceOf { value, .. } => Some(*value),
            Expression::TaggedTemplateExpression { tag, .. } => Some(*tag),
            Expression::If {
                form: IfForm::Ternary,
                condition: IfCondition::Expression { condition },
                ..
            } => Some(*condition),
            _ => None,
        }?;

        Some(Self::new(expression_id))
    }
}

/// Return whether a type expression prefers inline layout.
fn is_trivial_type_expression(tree: &Tree, expression_id: LocalNodeId<TypeExpression>) -> bool {
    matches!(
        tree.get(expression_id),
        TypeExpression::ScalarLiteral { .. }
            | TypeExpression::Literal { .. }
            | TypeExpression::Intrinsic
            | TypeExpression::Reference { .. }
            | TypeExpression::Member { .. }
            | TypeExpression::Const
            | TypeExpression::This
    )
}

/// Return whether an expression prefers inline layout.
pub fn is_trivial_expression(tree: &Tree, expression: &Expression) -> bool {
    match expression {
        Expression::ScalarLiteral(_)
        | Expression::Identifier { .. }
        | Expression::ImportMeta
        | Expression::NewTarget
        | Expression::This
        | Expression::Super
        | Expression::PrivateIdentifier { .. } => true,
        Expression::Type { value } => is_trivial_type_expression(tree, *value),
        Expression::ObjectExpression { properties } => {
            properties.len() <= 5
                && properties
                    .iter()
                    .all(|property| is_trivial_property(tree, tree.get(*property)))
        }
        Expression::Unary { right, .. } => is_trivial_expression(tree, tree.get(*right)),
        Expression::Index { left, index, .. } => index
            .as_ref()
            .map_or(is_trivial_expression(tree, tree.get(*left)), |index_id| {
                is_trivial_expression(tree, tree.get(*index_id))
            }),
        Expression::BorrowOf { right, .. } => is_trivial_expression(tree, tree.get(*right)),
        Expression::Member { left, .. } | Expression::PrivateMember { left, .. } => {
            is_trivial_expression(tree, tree.get(*left))
        }
        Expression::QualifiedReference {
            path,
            generic_arguments,
        } => path.segments.len() <= 3 && generic_arguments_are_trivial(tree, generic_arguments),
        Expression::As {
            expression: left,
            target_type: right,
        }
        | Expression::Satisfies {
            expression: left,
            target_type: right,
        } => {
            is_trivial_expression(tree, tree.get(*left)) && is_trivial_type_expression(tree, *right)
        }
        _ => false,
    }
}

/// Return whether generic arguments stay concise when inlined.
fn generic_arguments_are_trivial(
    tree: &Tree,
    generic_arguments: &[LocalNodeId<GenericArgument>],
) -> bool {
    generic_arguments
        .iter()
        .all(|argument_id| match tree.get(*argument_id) {
            GenericArgument::Type { value } | GenericArgument::SpreadType { value } => {
                is_trivial_type_expression(tree, *value)
            }
            GenericArgument::Value { value } | GenericArgument::SpreadValue { value } => {
                is_trivial_expression(tree, tree.get(*value))
            }
            GenericArgument::Error => false,
        })
}

/// Return whether an argument prefers inline layout.
pub fn is_trivial_argument(tree: &Tree, argument: &Argument) -> bool {
    match argument {
        Argument::Named { value, .. }
        | Argument::Labeled { value, .. }
        | Argument::Positional { value, .. }
        | Argument::Spread { value, .. } => is_trivial_expression(tree, tree.get(*value)),
        Argument::Error => false,
    }
}

/// Return whether a property prefers inline layout.
pub fn is_trivial_property(tree: &Tree, property: &Property) -> bool {
    match property {
        Property::Field { value, .. } => is_trivial_expression(tree, tree.get(*value)),
        Property::Method { body, .. } => {
            body.is_none_or(|body| is_trivial_expression(tree, tree.get(body)))
        }
        Property::Spread { value, .. } => is_trivial_expression(tree, tree.get(*value)),
        Property::Error => false,
    }
}

/// Return whether an expression can break across multiple lines.
pub fn is_expression_breakable(tree: &Tree, expression: &Expression) -> bool {
    match expression {
        Expression::ArrayExpression { elements, .. } => !elements.is_empty(),
        Expression::TupleExpression { elements, .. } => !elements.is_empty(),
        Expression::SequenceExpression { expressions, .. } => !expressions.is_empty(),
        Expression::ObjectExpression { properties } => !properties.is_empty(),
        Expression::StructExpression { ty, properties } => {
            is_type_expression_breakable(tree, *ty) || !properties.is_empty()
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
        Expression::Call { arguments, .. } | Expression::New { arguments, .. } => {
            !arguments.is_empty()
        }
        Expression::Match { .. }
        | Expression::If { .. }
        | Expression::Loop { .. }
        | Expression::Try { .. }
        | Expression::Block { .. }
        | Expression::ForEach { .. }
        | Expression::For { .. }
        | Expression::While { .. }
        | Expression::Import { .. }
        | Expression::Export { .. }
        | Expression::RangeExpression { .. }
        | Expression::Binary { .. }
        | Expression::As { .. }
        | Expression::Satisfies { .. }
        | Expression::Is { .. }
        | Expression::InstanceOf { .. } => true,
        Expression::QualifiedReference {
            generic_arguments, ..
        } => !generic_arguments.is_empty(),
        _ => false,
    }
}

/// Return whether a type expression can break across multiple lines.
fn is_type_expression_breakable(tree: &Tree, expression_id: LocalNodeId<TypeExpression>) -> bool {
    match tree.get(expression_id) {
        TypeExpression::Tuple { elements } | TypeExpression::ArrayTuple { elements } => {
            !elements.is_empty()
        }
        TypeExpression::Slice { .. } => true,
        TypeExpression::Object { members } => !members.is_empty(),
        TypeExpression::Union { elements } | TypeExpression::Intersection { elements } => {
            !elements.is_empty()
        }
        TypeExpression::Conditional { .. }
        | TypeExpression::Mapped { .. }
        | TypeExpression::TemplateLiteral { .. } => true,
        _ => false,
    }
}

/// Return whether a pattern can expand to multiple lines.
pub fn is_pattern_breakable(tree: &Tree, pattern_id: LocalNodeId<Pattern>) -> bool {
    match tree.get(pattern_id) {
        Pattern::Object { fields } | Pattern::TaggedObject { fields, .. } => !fields.is_empty(),
        Pattern::Sequence { fields }
        | Pattern::Tuple { fields }
        | Pattern::TaggedTuple { fields, .. } => !fields.is_empty(),
        _ => false,
    }
}

/// Return whether array elements are simple enough for concise fill formatting.
pub(crate) fn array_elements_are_fill_candidates(
    tree: &Tree,
    elements: &[LocalNodeId<Argument>],
) -> bool {
    elements
        .iter()
        .all(|element_id| array_element_is_fill_candidate(tree, *element_id))
}

/// Return whether comments in an array appear only before the first or after the last element.
pub(crate) fn array_has_only_outer_comments(
    context: &DestackFormatContext<'_>,
    array_span: Span,
    elements: &[LocalNodeId<Argument>],
) -> bool {
    let (Some(first_element), Some(last_element)) = (elements.first(), elements.last()) else {
        return false;
    };

    let first_span = context.span(*first_element);
    let last_span = context.span(*last_element);

    let has_internal_comment =
        context
            .tokens
            .iter()
            .chain(context.side_tokens.iter())
            .any(|token| {
                if !array_span.intersects(token.span) || !is_comment_token_type(token.token.ty()) {
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
fn array_element_is_fill_candidate(tree: &Tree, element_id: LocalNodeId<Argument>) -> bool {
    let value_id = match tree.get(element_id) {
        Argument::Positional { value, .. } => *value,
        _ => return false,
    };

    match tree.get(value_id) {
        Expression::ScalarLiteral(
            ScalarLiteral::Integer(_) | ScalarLiteral::Bigint(_) | ScalarLiteral::Float(_),
        ) => true,
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

/// Return whether a sequence expression needs parentheses in its parent context.
pub(crate) fn sequence_expression_needs_parens(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let Some((parent_id, parent_type)) = context.parent(node_id) else {
        return true;
    };

    if parent_type != NodeType::Expression {
        return true;
    }

    let parent_id = LocalNodeId::<Expression>::new(parent_id);
    match context.tree.get(parent_id) {
        Expression::Return { value } => value.is_some_and(|value_id| value_id != node_id),
        Expression::Throw { value } => *value != node_id,
        Expression::For {
            initialization,
            increment,
            ..
        } => {
            !initialization.is_some_and(|value_id| value_id == node_id)
                && !increment.is_some_and(|value_id| value_id == node_id)
        }
        Expression::SequenceExpression { .. } => true,
        _ => true,
    }
}
