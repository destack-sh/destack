use crate::TsppFormatContext;
use crate::operator::assign_pattern_target_expression;
use tspp_dir::{
    Argument, Declaration, Expression, FunctionForm, IfForm, Literal, LocalNodeId, Pattern,
    Property, Tree, TypeExpression, UnaryOperator,
};
use tspp_source::Span;

/// Return whether one expression is a lambda declaration.
pub(crate) fn expression_is_lambda_declaration(
    context: &TsppFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let Expression::Declaration(declaration_id) = context.tree.get(expression_id) else {
        return false;
    };

    matches!(
        context.tree.get(*declaration_id),
        Declaration::Function(function) if function.signature.form == FunctionForm::Lambda
    )
}

/// Return whether one expression is a multiline template starting on its opening line.
pub(crate) fn expression_is_multiline_template_starting_on_same_line(
    context: &TsppFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    if !matches!(
        context.tree.get(expression_id),
        Expression::TemplateExpression { .. } | Expression::TaggedTemplateExpression { .. }
    ) {
        return false;
    }

    let expression_span = context.span(expression_id);
    context.source_text().contains_newline(expression_span)
        && !context
            .source_text()
            .has_newline_before(expression_span.start)
}

/// A cursor over the successive left operands of one expression.
#[derive(Debug, Copy, Clone)]
pub(crate) struct ExpressionLeftPath {
    /// The current expression.
    expression_id: LocalNodeId<Expression>,
}

impl ExpressionLeftPath {
    /// Create one cursor at an expression.
    #[inline]
    pub(crate) fn new(expression_id: LocalNodeId<Expression>) -> Self {
        Self { expression_id }
    }

    /// Return the current expression.
    #[inline]
    pub(crate) fn expression_id(self) -> LocalNodeId<Expression> {
        self.expression_id
    }

    /// Advance to the current expression's left operand.
    pub(crate) fn next(self, context: &TsppFormatContext<'_>) -> Option<Self> {
        let expression_id = match context.tree.get(self.expression_id) {
            Expression::Member { left, .. }
            | Expression::Index { left, .. }
            | Expression::Call { left, .. }
            | Expression::Instantiation { left, .. }
            | Expression::Maybe { left, .. }
            | Expression::Must { left, .. }
            | Expression::Chain { expression: left }
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
                condition,
                ..
            } => condition.as_expression(),
            _ => None,
        }?;

        Some(Self::new(expression_id))
    }

    /// Return the leftmost expression reachable from this expression.
    pub(crate) fn leftmost(mut self, context: &TsppFormatContext<'_>) -> Self {
        while let Some(left) = self.next(context) {
            self = left;
        }

        self
    }
}

/// Return whether a type expression prefers inline layout.
fn is_trivial_type_expression(tree: &Tree, expression_id: LocalNodeId<TypeExpression>) -> bool {
    matches!(
        tree.get(expression_id),
        TypeExpression::Literal { .. }
            | TypeExpression::Keyword { .. }
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
        Expression::Literal(_)
        | Expression::Identifier { .. }
        | Expression::ImportMeta
        | Expression::ImportSource
        | Expression::This
        | Expression::Super
        | Expression::Infer { .. } => true,
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
        Expression::Member { left, .. } => is_trivial_expression(tree, tree.get(*left)),
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

/// Return whether an argument prefers inline layout.
pub fn is_trivial_argument(tree: &Tree, argument: &Argument) -> bool {
    match argument {
        Argument::Positional { value } | Argument::Spread { value } => {
            is_trivial_expression(tree, tree.get(*value))
        }
        Argument::Elision | Argument::Error => false,
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
    if is_control_expression(expression) {
        return true;
    }

    match expression {
        Expression::ArrayExpression { elements, .. } => !elements.is_empty(),
        Expression::TupleExpression { elements, .. } => !elements.is_empty(),
        Expression::ObjectExpression { properties } => !properties.is_empty(),
        Expression::StructExpression { ty, properties } => {
            is_type_expression_breakable(tree, *ty) || !properties.is_empty()
        }
        Expression::TreeExpression {
            attributes,
            children,
            ..
        } => {
            attributes
                .as_ref()
                .is_some_and(|attributes| !attributes.is_empty())
                || children
                    .as_ref()
                    .is_some_and(|children| !children.is_empty())
        }
        Expression::Call { arguments, .. } | Expression::New { arguments, .. } => {
            !arguments.is_empty()
        }
        Expression::Import { .. }
        | Expression::Export { .. }
        | Expression::RangeExpression { .. }
        | Expression::Binary { .. }
        | Expression::As { .. }
        | Expression::Satisfies { .. }
        | Expression::Is { .. }
        | Expression::InstanceOf { .. } => true,
        _ => false,
    }
}

/// Return whether an expression is a control expression.
pub(crate) fn is_control_expression(expression: &Expression) -> bool {
    matches!(
        expression,
        Expression::Match { .. }
            | Expression::Switch { .. }
            | Expression::If { .. }
            | Expression::Loop { .. }
            | Expression::Try { .. }
            | Expression::Block { .. }
            | Expression::ForEach { .. }
            | Expression::For { .. }
            | Expression::While { .. }
    )
}

/// Return whether a type expression can break across multiple lines.
fn is_type_expression_breakable(tree: &Tree, expression_id: LocalNodeId<TypeExpression>) -> bool {
    match tree.get(expression_id) {
        TypeExpression::Tuple { elements, .. } => !elements.is_empty(),
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
        Pattern::Object { fields } | Pattern::NominalObject { fields, .. } => !fields.is_empty(),
        Pattern::Sequence { fields }
        | Pattern::Tuple { fields }
        | Pattern::NominalTuple { fields, .. } => !fields.is_empty(),
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
    context: &TsppFormatContext<'_>,
    array_span: Span,
    elements: &[LocalNodeId<Argument>],
) -> bool {
    let (Some(first_element), Some(last_element)) = (elements.first(), elements.last()) else {
        return false;
    };

    let first_span = context.span(*first_element);
    let last_span = context.span(*last_element);

    let has_internal_comment = context
        .source_comments_intersecting_span(array_span)
        .iter()
        .any(|comment| {
            let is_before_first = comment.span.end <= first_span.start;
            let is_after_last = comment.span.start >= last_span.end;
            !(is_before_first || is_after_last)
        });

    !has_internal_comment
}

/// Return whether a single array element is a concise fill candidate.
fn array_element_is_fill_candidate(tree: &Tree, element_id: LocalNodeId<Argument>) -> bool {
    let value_id = match tree.get(element_id) {
        Argument::Positional { value, .. } => *value,
        _ => return false,
    };

    match tree.get(value_id) {
        Expression::Literal(Literal::Integer(_) | Literal::Bigint(_) | Literal::Float(_)) => true,
        Expression::Unary { operator, right } => {
            matches!(operator, UnaryOperator::Plus | UnaryOperator::Negate)
                && matches!(
                    tree.get(*right),
                    Expression::Literal(
                        Literal::Integer(_) | Literal::Bigint(_) | Literal::Float(_)
                    )
                )
        }
        _ => false,
    }
}
