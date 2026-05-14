use super::transparent_inner_expression;
use crate::DestackFormatContext;
use destack_dir::{
    Argument, Expression, GenericArgument, Key, LocalNodeId, Property, ScalarLiteral,
    TemplateLiteral, UnaryOperator,
};

const MAX_SIMPLE_ARGUMENT_DEPTH: u8 = 2;

/// One call or member-chain argument candidate.
pub(crate) enum SimpleArgument {
    /// One expression candidate.
    Expression(LocalNodeId<Expression>),
    /// One argument node candidate.
    Argument(LocalNodeId<Argument>),
}

impl SimpleArgument {
    /// Build one simple-argument candidate from an argument node.
    pub(crate) fn new(argument_id: LocalNodeId<Argument>) -> Self {
        Self::Argument(argument_id)
    }

    /// Return whether the candidate is simple.
    pub(crate) fn is_simple(&self, context: &DestackFormatContext<'_>) -> bool {
        self.is_simple_with_depth(context, 0)
    }

    /// Return whether the candidate is simple at one recursion depth.
    pub(crate) fn is_simple_with_depth(
        &self,
        context: &DestackFormatContext<'_>,
        depth: u8,
    ) -> bool {
        self.is_simple_impl(context, depth)
    }

    /// Return whether the candidate is simple at one recursion depth.
    fn is_simple_impl(&self, context: &DestackFormatContext<'_>, depth: u8) -> bool {
        // recursion limit
        if depth >= MAX_SIMPLE_ARGUMENT_DEPTH {
            return false;
        }

        match self {
            Self::Expression(expression_id) => expression_is_simple(context, *expression_id, depth),
            Self::Argument(argument_id) => argument_is_simple(context, *argument_id, depth),
        }
    }
}

impl From<LocalNodeId<Expression>> for SimpleArgument {
    fn from(expression_id: LocalNodeId<Expression>) -> Self {
        Self::Expression(expression_id)
    }
}

/// Return whether one argument node is simple at one recursion depth.
fn argument_is_simple(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
    depth: u8,
) -> bool {
    match context.tree.get(argument_id) {
        // named and labeled arguments delegate to their value expression
        Argument::Named { value, .. }
        | Argument::Labeled { value, .. }
        | Argument::Positional { value, .. } => {
            SimpleArgument::from(*value).is_simple_with_depth(context, depth)
        }

        // spread arguments always break the simple layout
        Argument::Spread { .. } | Argument::Error => false,
    }
}

/// Return whether one generic argument is simple at one recursion depth.
fn generic_argument_is_simple(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<GenericArgument>,
    depth: u8,
) -> bool {
    match context.tree.get(argument_id) {
        GenericArgument::Type { .. } => false,
        GenericArgument::Value { value } => {
            SimpleArgument::from(*value).is_simple_with_depth(context, depth)
        }
        GenericArgument::Error => false,
    }
}

/// Return whether one expression node is simple at one recursion depth.
fn expression_is_simple(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
    depth: u8,
) -> bool {
    // transparent wrappers
    let expression_id = transparent_inner_expression(context, expression_id);

    match context.tree.get(expression_id) {
        // literals and trivial references
        Expression::ScalarLiteral(ScalarLiteral::RegexString { content, .. }) => {
            context.strings.get(*content).chars().count() <= 5
        }
        Expression::ScalarLiteral(_)
        | Expression::Identifier { .. }
        | Expression::PrivateIdentifier { .. }
        | Expression::ImportMeta
        | Expression::NewTarget
        | Expression::This
        | Expression::Super => true,

        // templates and collection literals
        Expression::TemplateExpression { value } => {
            template_literal_is_simple(context, value, depth + 1)
        }
        Expression::ObjectExpression { ty, properties } => {
            ty.is_none() && object_expression_is_simple(context, properties, depth)
        }
        Expression::ArrayExpression { elements } => {
            array_expression_is_simple(context, elements, depth)
        }

        // unary wrappers
        Expression::Unary { operator, right } => {
            matches!(
                operator,
                UnaryOperator::Not
                    | UnaryOperator::Negate
                    | UnaryOperator::Plus
                    | UnaryOperator::ElementwiseNot
                    | UnaryOperator::PreIncrement
                    | UnaryOperator::PreDecrement
                    | UnaryOperator::PostIncrement
                    | UnaryOperator::PostDecrement
            ) && SimpleArgument::from(*right).is_simple_with_depth(context, depth)
        }
        Expression::Must { left, .. } | Expression::Maybe { left, .. } => {
            SimpleArgument::from(*left).is_simple_with_depth(context, depth)
        }

        // member and call like expressions
        Expression::Member { left, .. } | Expression::PrivateMember { left, .. } => {
            SimpleArgument::from(*left).is_simple_with_depth(context, depth)
        }
        Expression::Index { left, index, .. } => {
            SimpleArgument::from(*left).is_simple_with_depth(context, depth)
                && index.is_some_and(|index_id| {
                    SimpleArgument::from(index_id).is_simple_with_depth(context, depth)
                })
        }
        Expression::QualifiedReference {
            generic_arguments, ..
        } => generic_arguments
            .iter()
            .copied()
            .all(|argument_id| generic_argument_is_simple(context, argument_id, depth + 1)),
        Expression::Call {
            left,
            generic_arguments,
            arguments,
            ..
        }
        | Expression::New {
            left,
            generic_arguments,
            arguments,
        } => call_like_is_simple(context, *left, generic_arguments, arguments, depth),
        // anything else is too complex
        _ => false,
    }
}

/// Return whether one call-like expression is simple at one recursion depth.
fn call_like_is_simple(
    context: &DestackFormatContext<'_>,
    callee_id: LocalNodeId<Expression>,
    generic_arguments: &[LocalNodeId<GenericArgument>],
    arguments: &[LocalNodeId<Argument>],
    depth: u8,
) -> bool {
    SimpleArgument::from(callee_id).is_simple_with_depth(context, depth)
        && generic_arguments
            .iter()
            .copied()
            .all(|argument_id| generic_argument_is_simple(context, argument_id, depth + 1))
        && arguments.len() + usize::from(depth) <= 2
        && arguments.iter().copied().all(|argument_id| {
            SimpleArgument::new(argument_id).is_simple_with_depth(context, depth + 1)
        })
}

/// Return whether one template literal is simple at one recursion depth.
pub(crate) fn template_literal_is_simple(
    context: &DestackFormatContext<'_>,
    template: &TemplateLiteral,
    depth: u8,
) -> bool {
    match template {
        // plain template contents
        TemplateLiteral::String { string } => !context.strings.get(*string).contains('\n'),

        // interpolated template contents
        TemplateLiteral::InterpolatedString { strings, arguments } => {
            strings
                .iter()
                .all(|string| !context.strings.get(*string).contains('\n'))
                && arguments.iter().copied().all(|argument_id| {
                    SimpleArgument::new(argument_id).is_simple_with_depth(context, depth)
                })
        }
    }
}

/// Return whether one object expression is simple at one recursion depth.
fn object_expression_is_simple(
    context: &DestackFormatContext<'_>,
    properties: &[LocalNodeId<Property>],
    depth: u8,
) -> bool {
    properties
        .iter()
        .copied()
        .all(|property_id| property_is_simple(context, property_id, depth + 1))
}

/// Return whether one property is simple at one recursion depth.
fn property_is_simple(
    context: &DestackFormatContext<'_>,
    property_id: LocalNodeId<Property>,
    depth: u8,
) -> bool {
    match context.tree.get(property_id) {
        Property::Field { key, value, .. } => {
            key_is_simple(key) && SimpleArgument::from(*value).is_simple_with_depth(context, depth)
        }
        Property::Method { .. } | Property::Spread { .. } | Property::Error => false,
    }
}

/// Return whether one property key is simple.
fn key_is_simple(key: &Key) -> bool {
    matches!(key, Key::Name(_) | Key::Private(_))
}

/// Return whether one array expression is simple at one recursion depth.
fn array_expression_is_simple(
    context: &DestackFormatContext<'_>,
    elements: &[LocalNodeId<Argument>],
    depth: u8,
) -> bool {
    elements.iter().copied().all(|argument_id| {
        SimpleArgument::new(argument_id).is_simple_with_depth(context, depth + 1)
    })
}
