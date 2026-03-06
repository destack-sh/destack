use crate::format::expression::{
    Annotation, AnnotationPosition, Argument, AssignOperator, BinaryOperator, Declaration,
    Declarator, DependencyKind, DestackFormatContext, DestackFormatter, Expression, FormatResult,
    IfKind, ImportAliasTarget, LocalNodeId, Member, NodeTree, NodeType, OperatorPrecedence,
    Parameter, Property, ScalarLiteral, TypeBinaryOperator, TypeUnaryOperator, UnaryOperator,
    WhereClause, block_indent, format_expression_without_prefix_annotations, hard_line_break,
    is_trivial_expression, parenthesized_boundary_comments, parenthesized_has_leading_inner_trivia,
    token, transparent_inner_expression,
};
use destack_fir::format::{Buffer, Format};
use destack_fir::write;
use smallvec::SmallVec;

/// Format unary operators as source tokens.
impl<'ast> Format<DestackFormatContext<'ast>> for UnaryOperator {
    /// Write the token form of the unary operator.
    fn format(&self, f: &mut DestackFormatter<'ast, '_>) -> FormatResult<()> {
        let token = match self {
            UnaryOperator::PostIncrement => token("++"),
            UnaryOperator::PostDecrement => token("--"),
            UnaryOperator::PreIncrement => token("++"),
            UnaryOperator::PreDecrement => token("--"),
            UnaryOperator::Not => token("!"),
            UnaryOperator::Negate => token("-"),
            UnaryOperator::Plus => token("+"),
            UnaryOperator::WrappingNegate => token("-%"),
            UnaryOperator::ElementwiseNot => token("~"),
            UnaryOperator::Typeof => token("typeof"),
            UnaryOperator::Void => token("void"),
            UnaryOperator::Dereference => token("*"),
            UnaryOperator::Spread => token("..."),
        };
        write!(f, [token])
    }
}

/// Format type unary operators as source tokens.
impl<'ast> Format<DestackFormatContext<'ast>> for TypeUnaryOperator {
    /// Write the token form of the type unary operator.
    fn format(&self, f: &mut DestackFormatter<'ast, '_>) -> FormatResult<()> {
        let token = match self {
            TypeUnaryOperator::Not => token("!"),
            TypeUnaryOperator::Must => token("!"),
            TypeUnaryOperator::Newtype => token("newtype"),
            TypeUnaryOperator::Type => token("type"),
            TypeUnaryOperator::Readonly => token("readonly"),
            TypeUnaryOperator::Typeof => token("typeof"),
            TypeUnaryOperator::Keyof => token("keyof"),
            TypeUnaryOperator::AsComptime => token("as comptime"),
            TypeUnaryOperator::AsConst => token("as const"),
        };
        write!(f, [token])
    }
}

/// Format binary operators as source tokens.
impl<'ast> Format<DestackFormatContext<'ast>> for BinaryOperator {
    /// Write the token form of the binary operator.
    fn format(&self, f: &mut DestackFormatter<'ast, '_>) -> FormatResult<()> {
        let token = match self {
            // multiplication
            BinaryOperator::Multiply => token("*"),
            BinaryOperator::WrappingMultiply => token("*%"),
            BinaryOperator::SaturatingMultiply => token("*|"),
            BinaryOperator::Exponent => token("**"),
            BinaryOperator::WrappingExponent => token("**%"),
            BinaryOperator::SaturatingExponent => token("**|"),
            BinaryOperator::Divide => token("/"),
            BinaryOperator::Remainder => token("%"),

            // addition
            BinaryOperator::Add => token("+"),
            BinaryOperator::WrappingAdd => token("+%"),
            BinaryOperator::SaturatingAdd => token("+|"),
            BinaryOperator::Subtract => token("-"),
            BinaryOperator::WrappingSubtract => token("-%"),
            BinaryOperator::SaturatingSubtract => token("-|"),

            // shift
            BinaryOperator::ShiftLeft => token("<<"),
            BinaryOperator::SaturatingShiftLeft => token("<<|"),
            BinaryOperator::ShiftRight => token(">>"),
            BinaryOperator::UnsignedShiftRight => token(">>>"),

            // elementwise
            BinaryOperator::ElementwiseAnd => token("&"),
            BinaryOperator::ElementwiseXor => token("^"),
            BinaryOperator::ElementwiseOr => token("|"),

            // comparison
            BinaryOperator::Equal => token("=="),
            BinaryOperator::NotEqual => token("!="),
            BinaryOperator::EqualStrict => token("==="),
            BinaryOperator::NotEqualStrict => token("!=="),
            BinaryOperator::LessThan => token("<"),
            BinaryOperator::LessThanOrEqual => token("<="),
            BinaryOperator::GreaterThan => token(">"),
            BinaryOperator::GreaterThanOrEqual => token(">="),

            // boolean
            BinaryOperator::And => token("&&"),
            BinaryOperator::Or => token("||"),
            BinaryOperator::Coalesce => token("??"),

            // container
            BinaryOperator::In => token("in"),
            BinaryOperator::InstanceOf => token("instanceof"),
        };
        write!(f, [token])
    }
}

/// Format type binary operators as source tokens.
impl<'ast> Format<DestackFormatContext<'ast>> for TypeBinaryOperator {
    /// Write the token form of the type binary operator.
    fn format(&self, f: &mut DestackFormatter<'ast, '_>) -> FormatResult<()> {
        let token = match self {
            TypeBinaryOperator::Cast => token("as"),
            TypeBinaryOperator::In => token("in"),
            TypeBinaryOperator::Is => token("is"),
            TypeBinaryOperator::InstanceOf => token("instanceof"),
            TypeBinaryOperator::Satisfies => token("satisfies"),
            TypeBinaryOperator::Extends => token("extends"),
            TypeBinaryOperator::Implements => token("implements"),
        };
        write!(f, [token])
    }
}

/// Format assignment operators as source tokens.
impl<'ast> Format<DestackFormatContext<'ast>> for AssignOperator {
    /// Write the token form of the assignment operator.
    fn format(&self, f: &mut DestackFormatter<'ast, '_>) -> FormatResult<()> {
        let token = token(match self {
            AssignOperator::Assign => "=",

            // addition
            AssignOperator::AddAssign => "+=",
            AssignOperator::WrappingAddAssign => "+%=",
            AssignOperator::SaturatingAddAssign => "+|=",
            AssignOperator::SubtractAssign => "-=",
            AssignOperator::WrappingSubtractAssign => "-%=",
            AssignOperator::SaturatingSubtractAssign => "-|=",

            // multiplication
            AssignOperator::MultiplyAssign => "*=",
            AssignOperator::WrappingMultiplyAssign => "*%=",
            AssignOperator::SaturatingMultiplyAssign => "*|=",
            AssignOperator::ExponentAssign => "**=",
            AssignOperator::WrappingExponentAssign => "**%=",
            AssignOperator::SaturatingExponentAssign => "**|=",
            AssignOperator::DivideAssign => "/=",
            AssignOperator::RemainderAssign => "%=",

            // shift
            AssignOperator::ShiftLeftAssign => "<<=",
            AssignOperator::SaturatingShiftLeftAssign => "<<|=",
            AssignOperator::ShiftRightAssign => ">>=",
            AssignOperator::UnsignedShiftRightAssign => ">>>=",

            // elementwise
            AssignOperator::ElementwiseAndAssign => "&=",
            AssignOperator::ElementwiseOrAssign => "|=",
            AssignOperator::ElementwiseXorAssign => "^=",

            // boolean
            AssignOperator::AndAssign => "&&=",
            AssignOperator::OrAssign => "||=",
            AssignOperator::CoalesceAssign => "??=",
        });
        write!(f, [token])
    }
}

/// Return whether an expression is trivial and inline-safe without annotations.
pub(crate) fn expression_is_trivial_inline_without_annotations(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    !context.has_annotation(expression_id)
        && !context.node_has_newline(expression_id)
        && is_trivial_expression(context.tree, context.tree.get(expression_id))
}

/// Return whether an expression is a type-grammar variant.
pub(crate) fn is_type_expression_variant(expression: &Expression) -> bool {
    matches!(
        expression,
        Expression::TypeUnary { .. }
            | Expression::TypeBinary { .. }
            | Expression::TypeConditional { .. }
            | Expression::TypeMapped { .. }
            | Expression::TypeIndex { .. }
            | Expression::TypeTemplateLiteral { .. }
            | Expression::TypeImport { .. }
            | Expression::TypeInfer { .. }
            | Expression::TypePredicate { .. }
    )
}

/// Return static argument slots for expression variants that support type arguments.
pub(crate) fn expression_static_arguments(
    expression: &Expression,
) -> Option<&[LocalNodeId<Argument>]> {
    match expression {
        Expression::Path {
            static_arguments, ..
        }
        | Expression::Member {
            static_arguments, ..
        }
        | Expression::PrivateMember {
            static_arguments, ..
        }
        | Expression::Call {
            static_arguments, ..
        }
        | Expression::New {
            static_arguments, ..
        }
        | Expression::TypeImport {
            static_arguments, ..
        } => static_arguments.as_deref(),
        Expression::Instantiation {
            static_arguments, ..
        } => Some(static_arguments.as_slice()),
        _ => None,
    }
}

/// Whether an expression is used as a static type argument.
pub(crate) fn is_static_type_argument_context(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let Some((argument_id, parent_type)) = context.parent(node_id) else {
        return false;
    };
    if parent_type != NodeType::Argument {
        return false;
    }

    let argument_id = LocalNodeId::<Argument>::new(argument_id);
    let Some((expression_id, expression_type)) = context.parent(argument_id) else {
        return false;
    };
    if expression_type != NodeType::Expression {
        return false;
    }

    let parent_expression_id = LocalNodeId::<Expression>::new(expression_id);
    expression_static_arguments(context.tree.get(parent_expression_id))
        .is_some_and(|arguments| arguments.contains(&argument_id))
}

/// Whether a binary expression is in a type position.
pub(crate) fn is_type_context(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    if let Some(is_type_context) = context.lookup_expression_type_context(node_id) {
        context.increment_counter("cache.type_context.hits", 1);
        return is_type_context;
    }
    context.increment_counter("cache.type_context.misses", 1);

    let is_type_context = is_type_context_uncached(context, node_id);
    context.store_expression_type_context(node_id, is_type_context);
    is_type_context
}

/// Whether a binary expression is in a type position.
fn is_type_context_uncached(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let mut current_id = node_id.id;

    // walk ancestors and check for type slots
    while let Some((parent_id, parent_type)) = context.parent_by_id(current_id) {
        match parent_type {
            // static arguments are always type positions in js/ts syntax
            NodeType::Argument => {
                if argument_parent_is_type_context(context, parent_id) {
                    return true;
                }
            }

            // type specific expressions imply type context
            NodeType::Expression => {
                let (is_type_context, continue_with_parent) =
                    expression_parent_type_context_step(context, parent_id, current_id);
                if is_type_context {
                    return true;
                }

                if continue_with_parent {
                    current_id = parent_id;
                    continue;
                }
            }

            // declarator type annotation
            NodeType::Declarator => {
                if declarator_parent_is_type_context(context, parent_id, current_id) {
                    return true;
                }
            }

            // parameter type annotation
            NodeType::Parameter => {
                if parameter_parent_is_type_context(context, parent_id, current_id) {
                    return true;
                }
            }

            // where clause constraint
            NodeType::WhereClause => {
                if where_clause_parent_is_type_context(context, parent_id, current_id) {
                    return true;
                }
            }

            // property field type
            NodeType::Property => {
                if property_parent_is_type_context(context, parent_id, current_id) {
                    return true;
                }
            }

            // member type slots
            NodeType::Member => {
                if member_parent_is_type_context(context, parent_id, current_id) {
                    return true;
                }
            }

            // declaration type slots
            NodeType::Declaration => {
                if declaration_parent_is_type_context(context, parent_id, current_id) {
                    return true;
                }
            }

            _ => {}
        }

        current_id = parent_id;
    }

    false
}

/// Return whether one argument parent marks a type context.
fn argument_parent_is_type_context(context: &DestackFormatContext<'_>, parent_id: u32) -> bool {
    let argument_id = LocalNodeId::<Argument>::new(parent_id);
    let Some((expression_id, expression_type)) = context.parent_by_id(parent_id) else {
        return false;
    };
    if expression_type != NodeType::Expression {
        return false;
    }

    let parent_expression = context
        .tree
        .get(LocalNodeId::<Expression>::new(expression_id));
    expression_static_arguments(parent_expression)
        .is_some_and(|arguments| arguments.contains(&argument_id))
}

/// Return expression-parent type-context step outcome.
fn expression_parent_type_context_step(
    context: &DestackFormatContext<'_>,
    parent_id: u32,
    current_id: u32,
) -> (bool, bool) {
    let parent_expression = context.tree.get(LocalNodeId::<Expression>::new(parent_id));

    if let Expression::TypeUnary { operator, right } = parent_expression
        && *operator == TypeUnaryOperator::AsConst
        && right.id == current_id
    {
        return (false, true);
    }

    if let Expression::TypeBinary { left, operator, .. } = parent_expression
        && left.id == current_id
        && matches!(
            operator,
            TypeBinaryOperator::Cast
                | TypeBinaryOperator::Satisfies
                | TypeBinaryOperator::Is
                | TypeBinaryOperator::InstanceOf
                | TypeBinaryOperator::In
        )
    {
        return (false, true);
    }

    (is_type_expression_variant(parent_expression), false)
}

/// Return whether one declarator parent marks a type context.
fn declarator_parent_is_type_context(
    context: &DestackFormatContext<'_>,
    parent_id: u32,
    current_id: u32,
) -> bool {
    let declarator = context.tree.get(LocalNodeId::<Declarator>::new(parent_id));
    declarator.ty.is_some_and(|ty| ty.id == current_id)
}

/// Return whether one parameter parent marks a type context.
fn parameter_parent_is_type_context(
    context: &DestackFormatContext<'_>,
    parent_id: u32,
    current_id: u32,
) -> bool {
    let parameter = context.tree.get(LocalNodeId::<Parameter>::new(parent_id));
    let parameter_ty = match parameter {
        Parameter::Named { ty, .. }
        | Parameter::Pattern { ty, .. }
        | Parameter::VariadicNamed { ty, .. }
        | Parameter::VariadicPattern { ty, .. } => *ty,
    };

    parameter_ty.is_some_and(|ty| ty.id == current_id)
}

/// Return whether one where-clause parent marks a type context.
fn where_clause_parent_is_type_context(
    context: &DestackFormatContext<'_>,
    parent_id: u32,
    current_id: u32,
) -> bool {
    let where_clause = context.tree.get(LocalNodeId::<WhereClause>::new(parent_id));
    where_clause.right.id == current_id
}

/// Return whether one property parent marks a type context.
fn property_parent_is_type_context(
    context: &DestackFormatContext<'_>,
    parent_id: u32,
    current_id: u32,
) -> bool {
    let property = context.tree.get(LocalNodeId::<Property>::new(parent_id));
    if let Property::Field { value, .. } = property
        && value.is_some_and(|value| value.id == current_id)
        && let Some((expression_id, expression_type)) = context.parent_by_id(parent_id)
        && expression_type == NodeType::Expression
    {
        return is_type_context(context, LocalNodeId::<Expression>::new(expression_id));
    }

    false
}

/// Return whether one member parent marks a type context.
fn member_parent_is_type_context(
    context: &DestackFormatContext<'_>,
    parent_id: u32,
    current_id: u32,
) -> bool {
    let member = context.tree.get(LocalNodeId::<Member>::new(parent_id));
    match member {
        Member::Type { ty, value, .. } => {
            ty.is_some_and(|ty| ty.id == current_id)
                || value.is_some_and(|value| value.id == current_id)
        }
        Member::Field { value, .. } => value.is_some_and(|value| value.id == current_id),
        Member::ComptimeConst { ty, .. } => ty.is_some_and(|ty| ty.id == current_id),
        Member::Embed { value, .. } => value.id == current_id,
        Member::Method { .. } | Member::StaticBlock { .. } | Member::ComptimeBlock { .. } => false,
    }
}

/// Return whether one declaration parent marks a type context.
fn declaration_parent_is_type_context(
    context: &DestackFormatContext<'_>,
    parent_id: u32,
    current_id: u32,
) -> bool {
    let declaration = context.tree.get(LocalNodeId::<Declaration>::new(parent_id));
    match declaration {
        Declaration::Type { value, .. } => value.id == current_id,
        Declaration::Struct { heritage, .. }
        | Declaration::Interface { heritage, .. }
        | Declaration::Enum { heritage, .. } => {
            heritage
                .extends_types
                .as_ref()
                .is_some_and(|types| types.iter().any(|ty| ty.id == current_id))
                || heritage
                    .implements_types
                    .as_ref()
                    .is_some_and(|types| types.iter().any(|ty| ty.id == current_id))
        }
        Declaration::Class { heritage, .. } => heritage
            .implements_types
            .as_ref()
            .is_some_and(|types| types.iter().any(|ty| ty.id == current_id)),
        Declaration::Extension {
            target_type,
            heritage,
            ..
        } => {
            target_type.id == current_id
                || heritage
                    .extends_types
                    .as_ref()
                    .is_some_and(|types| types.iter().any(|ty| ty.id == current_id))
                || heritage
                    .implements_types
                    .as_ref()
                    .is_some_and(|types| types.iter().any(|ty| ty.id == current_id))
        }
        Declaration::Function { signature, .. } => signature
            .return_type
            .is_some_and(|return_type| return_type.id == current_id),
        Declaration::ImportAlias { kind, target, .. } => match (kind, target) {
            (DependencyKind::Type, ImportAliasTarget::Path { value }) => value.id == current_id,
            _ => false,
        },
        Declaration::Global { .. } | Declaration::Namespace { .. } => false,
    }
}

/// Return whether an expression is the type annotation of a parameter.
pub(crate) fn is_parameter_type_annotation(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let Some((parent_id, parent_type)) = context.parent(expression_id) else {
        return false;
    };
    if parent_type != NodeType::Parameter {
        return false;
    }

    let parameter = context.tree.get(LocalNodeId::<Parameter>::new(parent_id));
    match parameter {
        Parameter::Named { ty, .. }
        | Parameter::Pattern { ty, .. }
        | Parameter::VariadicNamed { ty, .. }
        | Parameter::VariadicPattern { ty, .. } => ty.is_some_and(|ty| ty.id == expression_id.id),
    }
}

/// Get the precedence of an expression.
pub(crate) fn is_simple_type_binary_left_expression(
    tree: &NodeTree,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    match tree.get(expression_id) {
        Expression::Parenthesized { expression } => {
            is_simple_type_binary_left_expression(tree, *expression)
        }
        Expression::TypeBinary { left, operator, .. } => {
            matches!(
                operator,
                TypeBinaryOperator::Cast | TypeBinaryOperator::Satisfies
            ) && is_simple_type_binary_left_expression(tree, *left)
        }
        Expression::Path { .. }
        | Expression::Member { .. }
        | Expression::PrivateMember { .. }
        | Expression::Index { .. }
        | Expression::Call { .. }
        | Expression::Instantiation { .. }
        | Expression::Maybe { .. }
        | Expression::Must { .. }
        | Expression::This
        | Expression::Super
        | Expression::PrivateIdentifier { .. }
        | Expression::ScalarLiteral(_)
        | Expression::TypeLiteral(_) => true,
        _ => false,
    }
}

/// Return whether one type expression is object-like.
pub(crate) fn is_object_like_type_expression(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let expression_id = transparent_inner_expression(context, expression_id);
    matches!(
        context.tree.get(expression_id),
        Expression::ObjectExpression { .. } | Expression::TypeMapped { .. }
    )
}

/// Return whether a binary operator participates in type union or intersection grouping.
pub(crate) fn is_type_grouping_binary_operator(operator: BinaryOperator) -> bool {
    matches!(
        operator,
        BinaryOperator::ElementwiseOr | BinaryOperator::ElementwiseAnd
    )
}

/// Return whether a type binary operand needs grouping parentheses.
pub(crate) fn type_binary_operand_needs_grouping_parentheses(
    context: &DestackFormatContext<'_>,
    parent_operator: BinaryOperator,
    operand_id: LocalNodeId<Expression>,
) -> bool {
    if !is_type_grouping_binary_operator(parent_operator) {
        return false;
    }

    if matches!(
        context.tree.get(operand_id),
        Expression::Parenthesized { .. }
    ) {
        return false;
    }

    let inner_id = transparent_inner_expression(context, operand_id);
    let Expression::Binary {
        operator: inner_operator,
        ..
    } = context.tree.get(inner_id)
    else {
        return false;
    };

    parent_operator == BinaryOperator::ElementwiseAnd
        && *inner_operator == BinaryOperator::ElementwiseOr
        && is_type_context(context, inner_id)
}

/// Format a binary operand with grouping parentheses when needed in type contexts.
pub(crate) fn format_binary_operand_with_grouping_parentheses<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    parent_operator: BinaryOperator,
    operand_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let mut operand_id = operand_id;
    let operand_has_non_blank_annotation = f.context().has_non_blank_annotation(operand_id);
    if let Expression::Parenthesized {
        expression: inner_expression_id,
    } = f.context().tree.get(operand_id)
    {
        let can_drop_for_binary = redundant_parenthesized_binary_operand_can_drop(
            f.context(),
            parent_operator,
            operand_id,
            *inner_expression_id,
        );
        let can_drop_for_closure_cast = redundant_parenthesized_closure_cast_operand_can_drop(
            f.context(),
            operand_id,
            *inner_expression_id,
        );
        if can_drop_for_binary || can_drop_for_closure_cast {
            operand_id = *inner_expression_id;
        }
    }

    let expression = f.context().tree.get(operand_id);
    let needs_type_grouping_parentheses =
        type_binary_operand_needs_grouping_parentheses(f.context(), parent_operator, operand_id);
    let suppress_precedence_parentheses_for_type_binary = matches!(
        (expression, parent_operator),
        (
            Expression::TypeBinary {
                operator: TypeBinaryOperator::Is
                    | TypeBinaryOperator::In
                    | TypeBinaryOperator::InstanceOf,
                ..
            },
            BinaryOperator::And | BinaryOperator::Or | BinaryOperator::Coalesce,
        )
    );
    let needs_mixed_logical_grouping_parentheses = matches!(
        (parent_operator, expression),
        (
            BinaryOperator::Or | BinaryOperator::Coalesce,
            Expression::Binary {
                operator: BinaryOperator::And | BinaryOperator::Coalesce,
                ..
            },
        )
    );
    let needs_precedence_parentheses = !matches!(expression, Expression::Parenthesized { .. })
        && expression_precedence(expression) < parent_operator.precedence()
        && !suppress_precedence_parentheses_for_type_binary;
    let needs_grouping_parentheses = needs_type_grouping_parentheses
        || needs_precedence_parentheses
        || needs_mixed_logical_grouping_parentheses;
    let operand_has_prefix_annotation = f.context().has_prefix_annotation(operand_id);

    if needs_grouping_parentheses {
        if operand_has_prefix_annotation {
            write!(
                f,
                [f.context().any_prefix_annotations(operand_id), token("(")]
            )?;
            format_expression_without_prefix_annotations(f, operand_id)?;
            write!(f, [token(")")])?;
        } else if operand_has_non_blank_annotation {
            write!(
                f,
                [
                    token("("),
                    block_indent(&operand_id),
                    hard_line_break(),
                    token(")")
                ]
            )?;
        } else {
            write!(f, [token("("), operand_id, token(")")])?;
        }
    } else {
        write!(f, [operand_id])?;
    }

    Ok(())
}

/// Return whether a parenthesized closure-cast style operand can drop wrappers.
fn redundant_parenthesized_closure_cast_operand_can_drop(
    context: &DestackFormatContext<'_>,
    parenthesized_id: LocalNodeId<Expression>,
    inner_expression_id: LocalNodeId<Expression>,
) -> bool {
    if !parenthesized_has_leading_inner_trivia(context, parenthesized_id, inner_expression_id) {
        return false;
    }

    let has_doc_like_prefix = context
        .visit_annotations(inner_expression_id, |annotations| {
            annotations.iter().any(|annotation_id| {
                matches!(
                    context.annotation(*annotation_id),
                    Annotation::Doc {
                        position: AnnotationPosition::BlockPrefix | AnnotationPosition::LinePrefix,
                        ..
                    }
                )
            })
        })
        .unwrap_or(false);
    if !has_doc_like_prefix {
        return false;
    }

    matches!(
        context.tree.get(inner_expression_id),
        Expression::Path { .. }
            | Expression::Member { .. }
            | Expression::PrivateMember { .. }
            | Expression::ScalarLiteral(_)
            | Expression::TypeLiteral(_)
            | Expression::Call { .. }
            | Expression::Index { .. }
            | Expression::Instantiation { .. }
            | Expression::Maybe { .. }
            | Expression::Must { .. }
    )
}

/// Return whether a parenthesized binary operand can safely drop its wrapper.
fn redundant_parenthesized_binary_operand_can_drop(
    context: &DestackFormatContext<'_>,
    parent_operator: BinaryOperator,
    parenthesized_id: LocalNodeId<Expression>,
    inner_expression_id: LocalNodeId<Expression>,
) -> bool {
    // mixed type-grouping operators require explicit wrappers:
    // `A | (B & C)` should not drop inner parentheses
    let Expression::Binary {
        operator: inner_operator,
        ..
    } = context.tree.get(inner_expression_id)
    else {
        return false;
    };
    if is_type_context(context, parenthesized_id)
        && is_type_grouping_binary_operator(parent_operator)
        && is_type_grouping_binary_operator(*inner_operator)
        && parent_operator != *inner_operator
    {
        return false;
    }

    // closure style casts use inline prefix docs/comments before the inner expression
    let has_inline_closure_cast_prefix =
        parenthesized_has_leading_inner_trivia(context, parenthesized_id, inner_expression_id)
            && context
                .visit_annotations(inner_expression_id, |annotations| {
                    annotations.iter().any(|annotation_id| {
                        matches!(
                            context.annotation(*annotation_id),
                            Annotation::Doc {
                                position: AnnotationPosition::BlockPrefix
                                    | AnnotationPosition::LinePrefix,
                                ..
                            }
                        )
                    })
                })
                .unwrap_or(false);

    if (context.has_annotation(parenthesized_id) || context.has_annotation(inner_expression_id))
        && !has_inline_closure_cast_prefix
    {
        return false;
    }

    if parenthesized_has_leading_inner_trivia(context, parenthesized_id, inner_expression_id)
        && !has_inline_closure_cast_prefix
    {
        return false;
    }

    if !parenthesized_boundary_comments(context, parenthesized_id, inner_expression_id).is_empty() {
        return false;
    }

    let inner_precedence = expression_precedence(context.tree.get(inner_expression_id));
    inner_precedence > parent_operator.precedence()
}

/// Returns the precedence group for a binary operator.
/// Return precedence group for binary operators.
#[inline]
fn binary_operator_precedence_group(operator: BinaryOperator) -> u8 {
    // first two digits of discriminant encode precedence
    (operator as u16 / 100) as u8
}

/// Checks if two binary operators should be flattened together.
/// Return whether nested binaries should flatten into one group.
#[inline]
fn should_flatten_binary(left_operator: BinaryOperator, right_operator: BinaryOperator) -> bool {
    let both_logical_operators = matches!(
        left_operator,
        BinaryOperator::And | BinaryOperator::Or | BinaryOperator::Coalesce
    ) && matches!(
        right_operator,
        BinaryOperator::And | BinaryOperator::Or | BinaryOperator::Coalesce
    );
    if both_logical_operators && left_operator != right_operator {
        return false;
    }

    binary_operator_precedence_group(left_operator)
        == binary_operator_precedence_group(right_operator)
}

/// Represents a flattened binary expression operand with its preceding operator.
pub(crate) struct BinaryOperand {
    /// The operator before this operand (None for first).
    pub(crate) operator: Option<BinaryOperator>,
    /// The expression node.
    pub(crate) expression: LocalNodeId<Expression>,
}

/// Store flattened binary operands with an inline-first buffer.
pub(crate) type BinaryOperands = SmallVec<[BinaryOperand; 8]>;

/// Flattens a binary expression chain into a list of operands.
///
/// For `a + b + c`, returns [(None, a), (Some(+), b), (Some(+), c)].
pub(crate) fn flatten_binary_expression(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
    target_operator: BinaryOperator,
) -> BinaryOperands {
    let mut operands = BinaryOperands::new();
    flatten_binary_recursive(
        context,
        expression_id,
        target_operator,
        &mut operands,
        None,
        true,
    );
    operands
}

/// Flattens associative type binary chains while unwrapping redundant parentheses.
pub(crate) fn flatten_type_binary_expression(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
    target_operator: BinaryOperator,
) -> BinaryOperands {
    let mut operands = BinaryOperands::new();
    flatten_type_binary_recursive(context, expression_id, target_operator, &mut operands, None);
    operands
}

/// Return the operand count for a flattened binary expression chain.
pub(crate) fn flattened_binary_operand_count(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
    target_operator: BinaryOperator,
) -> usize {
    count_flattened_binary_recursive(context, expression_id, target_operator, true)
}

/// Recursively flatten type binary chains and preserve operand operators.
fn flatten_type_binary_recursive(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
    target_operator: BinaryOperator,
    operands: &mut BinaryOperands,
    preceding_operator: Option<BinaryOperator>,
) {
    let expression_id =
        normalize_type_binary_operand_expression(context, expression_id, target_operator);

    if let Expression::Binary {
        left,
        operator,
        right,
    } = context.tree.get(expression_id)
        && *operator == target_operator
    {
        flatten_type_binary_recursive(
            context,
            *left,
            target_operator,
            operands,
            preceding_operator,
        );
        flatten_type_binary_recursive(context, *right, target_operator, operands, Some(*operator));
        return;
    }

    operands.push(BinaryOperand {
        operator: preceding_operator,
        expression: expression_id,
    });
}

/// Remove redundant parenthesized wrappers around associative type operands.
pub(crate) fn normalize_type_binary_operand_expression(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
    target_operator: BinaryOperator,
) -> LocalNodeId<Expression> {
    let mut current_id = expression_id;

    loop {
        let Expression::Parenthesized { expression } = context.tree.get(current_id) else {
            break;
        };

        if context.has_annotation(current_id) {
            break;
        }

        let inner_id = *expression;
        let inner_is_flattenable = matches!(
            context.tree.get(inner_id),
            Expression::Binary { operator, .. } if *operator == target_operator
        );
        let inner_is_parenthesized =
            matches!(context.tree.get(inner_id), Expression::Parenthesized { .. });
        if !inner_is_flattenable && !inner_is_parenthesized {
            break;
        }

        current_id = inner_id;
    }

    current_id
}

/// Recursively collect binary expression operands.
fn flatten_binary_recursive(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
    target_operator: BinaryOperator,
    operands: &mut BinaryOperands,
    preceding_operator: Option<BinaryOperator>,
    is_root: bool,
) {
    if let Expression::Binary {
        left,
        operator,
        right,
    } = context.tree.get(expression_id)
        && should_flatten_binary(*operator, target_operator)
        && (!context.has_annotation(expression_id) || is_root)
    {
        // recursively flatten the left side
        flatten_binary_recursive(context, *left, target_operator, operands, None, false);

        // add the right operand with its operator
        operands.push(BinaryOperand {
            operator: Some(*operator),
            expression: *right,
        });
        return;
    }

    // not a binary expression or different precedence - add as-is
    operands.push(BinaryOperand {
        operator: preceding_operator,
        expression: expression_id,
    });
}

/// Recursively count flattened binary operands without allocating.
fn count_flattened_binary_recursive(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
    target_operator: BinaryOperator,
    is_root: bool,
) -> usize {
    if let Expression::Binary {
        left,
        operator,
        right,
    } = context.tree.get(expression_id)
        && should_flatten_binary(*operator, target_operator)
        && (!context.has_annotation(expression_id) || is_root)
    {
        let left_count = count_flattened_binary_recursive(context, *left, target_operator, false);
        let right_count = count_flattened_binary_recursive(context, *right, target_operator, false);
        return left_count.saturating_add(right_count);
    }

    1
}

/// Whether an expression variant is type specific.
/// Return precedence value for an expression.
#[inline]
pub(crate) fn expression_precedence(expr: &Expression) -> u16 {
    match expr {
        // postfix operators (2000)
        Expression::Call { .. }
        | Expression::Member { .. }
        | Expression::PrivateMember { .. }
        | Expression::Index { .. }
        | Expression::Instantiation { .. }
        | Expression::Maybe { .. }
        | Expression::Must { .. } => OperatorPrecedence::Postfix as u16,

        // postfix unary (2000)
        Expression::Unary { operator, .. } if operator.is_postfix() => {
            OperatorPrecedence::Postfix as u16
        }

        // prefix unary (1900)
        Expression::Unary { .. } => OperatorPrecedence::Prefix as u16,

        // prefix expressions (1900)
        Expression::Await { .. }
        | Expression::AwaitMaybe { .. }
        | Expression::Yield { .. }
        | Expression::Delete { .. }
        | Expression::Throw { .. }
        | Expression::Return { .. } => OperatorPrecedence::Prefix as u16,

        // type unary: use operator's precedence
        Expression::TypeUnary { operator, .. } => operator.precedence(),

        // binary: use operator's precedence
        Expression::Binary { operator, .. } => operator.precedence(),
        Expression::TypeBinary { operator, .. } => match operator {
            TypeBinaryOperator::Cast | TypeBinaryOperator::Satisfies => 0,
            _ => operator.precedence(),
        },

        // assignment: use operator's precedence
        Expression::Assign { operator, .. } => operator.precedence(),

        // ternary: lower than all binary/assignment operators
        Expression::If {
            kind: IfKind::Ternary,
            ..
        } => OperatorPrecedence::AssignmentBoolean as u16 - 1,

        // atomic/primary expressions: highest precedence (never need parens)
        _ => u16::MAX,
    }
}

/// Returns true if the expression needs parentheses when used as the operand
/// of a postfix operator like `?` or `!`.
///
/// Postfix operators (precedence 2000) bind tighter than all other operators.
/// For example, `await x?` parses as `await (x?)`, not `(await x)?`.
/// So when formatting `Maybe { left: Await { expr } }`, we need to output `(await expr)?`.
/// Return whether postfix formatting requires parentheses.
#[inline]
pub(crate) fn needs_parens_in_postfix_position(
    tree: &NodeTree,
    expr_id: LocalNodeId<Expression>,
) -> bool {
    if matches!(
        tree.get(expr_id),
        Expression::ObjectExpression { .. } | Expression::TreeExpression { .. }
    ) {
        return true;
    }

    if let Expression::Declaration(declaration_id) = tree.get(expr_id)
        && matches!(
            tree.get(*declaration_id),
            Declaration::Function { .. } | Declaration::Class { .. }
        )
    {
        return true;
    }

    expression_precedence(tree.get(expr_id)) < OperatorPrecedence::Postfix as u16
}

/// Format an expression used as the receiver/base of a postfix operation.
pub(crate) fn write_postfix_base_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    expression_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let parent_expression_id = postfix_parent_expression_id(f.context(), expression_id);
    let needs_integer_member_parentheses = parent_expression_id.is_some_and(|parent_id| {
        matches!(
            f.context().tree.get(expression_id),
            Expression::ScalarLiteral(ScalarLiteral::Integer(_))
        ) && matches!(
            f.context().tree.get(parent_id),
            Expression::Member { .. } | Expression::PrivateMember { .. }
        )
    });
    let needs_parentheses = needs_parens_in_postfix_position(f.context().tree, expression_id)
        || needs_integer_member_parentheses;
    if needs_parentheses {
        write!(f, [token("("), expression_id, token(")")])?;
    } else {
        write!(f, [expression_id])?;
    }
    Ok(())
}

/// Return one postfix parent expression id when this expression is used as a chain receiver.
fn postfix_parent_expression_id(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> Option<LocalNodeId<Expression>> {
    // direct parent chain receiver
    if let Some((parent_id, parent_type)) = context.parent(expression_id)
        && parent_type == NodeType::Expression
    {
        let parent_expression_id = LocalNodeId::<Expression>::new(parent_id);
        let parent_expression = context.tree.get(parent_expression_id);
        let uses_expression_as_left = matches!(
            parent_expression,
            Expression::Member { left, .. }
                | Expression::PrivateMember { left, .. }
                | Expression::Call { left, .. }
                | Expression::Index { left, .. }
                | Expression::Instantiation { left, .. }
                | Expression::Maybe { left, .. }
                | Expression::Must { left, .. }
                if *left == expression_id
        );
        if uses_expression_as_left {
            return Some(parent_expression_id);
        }
    }

    // parenthesized wrapper chain receiver
    let (parent_id, parent_type) = context.parent(expression_id)?;
    if parent_type != NodeType::Expression {
        return None;
    }

    let parent_expression_id = LocalNodeId::<Expression>::new(parent_id);
    let Expression::Parenthesized { expression } = context.tree.get(parent_expression_id) else {
        return None;
    };
    if *expression != expression_id {
        return None;
    }

    let (grandparent_id, grandparent_type) = context.parent(parent_expression_id)?;
    if grandparent_type != NodeType::Expression {
        return None;
    }

    let grandparent_expression_id = LocalNodeId::<Expression>::new(grandparent_id);
    let grandparent_expression = context.tree.get(grandparent_expression_id);
    let uses_parenthesized_as_left = matches!(
        grandparent_expression,
        Expression::Member { left, .. }
            | Expression::PrivateMember { left, .. }
            | Expression::Call { left, .. }
            | Expression::Index { left, .. }
            | Expression::Instantiation { left, .. }
            | Expression::Maybe { left, .. }
            | Expression::Must { left, .. }
            if *left == parent_expression_id
    );
    if !uses_parenthesized_as_left {
        return None;
    }

    Some(grandparent_expression_id)
}

/// Check whether a parenthesized cast or satisfies left side is simple enough to unwrap.
pub(crate) fn is_chain_expression(expression: &Expression) -> bool {
    matches!(
        expression,
        Expression::Member { .. }
            | Expression::PrivateMember { .. }
            | Expression::Call { .. }
            | Expression::Index { .. }
            | Expression::Instantiation { .. }
            | Expression::Maybe { .. }
            | Expression::Must { .. }
    )
}
