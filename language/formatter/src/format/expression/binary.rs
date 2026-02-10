use super::*;
use destack_fir::write;

/// Return whether an expression is a type-grammar variant.
pub(super) fn is_type_expression_variant(expression: &Expression) -> bool {
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
pub(super) fn expression_static_arguments(
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
pub(super) fn is_static_type_argument_context(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let Some((argument_id, parent_type)) = context.get_parent(node_id) else {
        return false;
    };
    if parent_type != NodeType::Argument {
        return false;
    }

    let argument_id = LocalNodeId::<Argument>::new(argument_id);
    let Some((expression_id, expression_type)) = context.get_parent(argument_id) else {
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
pub(super) fn is_type_context(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    if let Some(is_type_context) = context.cached_expression_type_context(node_id) {
        context.increment_counter("cache.type_context.hits", 1);
        return is_type_context;
    }
    context.increment_counter("cache.type_context.misses", 1);

    let is_type_context = is_type_context_uncached(context, node_id);
    context.set_cached_expression_type_context(node_id, is_type_context);
    is_type_context
}

/// Whether a binary expression is in a type position.
fn is_type_context_uncached(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let mut current_id = node_id.id;

    // walk ancestors and check for type slots
    while let Some((parent_id, parent_type)) = context.get_parent_by_id(current_id) {
        match parent_type {
            // static arguments are always type positions in js/ts syntax
            NodeType::Argument => {
                let argument_id = LocalNodeId::<Argument>::new(parent_id);
                if let Some((expression_id, expression_type)) = context.get_parent_by_id(parent_id)
                    && expression_type == NodeType::Expression
                {
                    let parent_expression = context
                        .tree
                        .get(LocalNodeId::<Expression>::new(expression_id));
                    if expression_static_arguments(parent_expression)
                        .is_some_and(|arguments| arguments.contains(&argument_id))
                    {
                        return true;
                    }
                }
            }

            // type specific expressions imply type context
            NodeType::Expression => {
                let parent_expr = context.tree.get(LocalNodeId::<Expression>::new(parent_id));
                if let Expression::TypeUnary { operator, right } = parent_expr
                    && *operator == TypeUnaryOperator::AsConst
                    && right.id == current_id
                {
                    current_id = parent_id;
                    continue;
                }
                if let Expression::TypeBinary { left, operator, .. } = parent_expr
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
                    current_id = parent_id;
                    continue;
                }
                if is_type_expression_variant(parent_expr) {
                    return true;
                }
            }

            // declarator type annotation
            NodeType::Declarator => {
                let declarator = context.tree.get(LocalNodeId::<Declarator>::new(parent_id));
                if declarator.ty.is_some_and(|ty| ty.id == current_id) {
                    return true;
                }
            }

            // parameter type annotation
            NodeType::Parameter => {
                let parameter = context.tree.get(LocalNodeId::<Parameter>::new(parent_id));
                let parameter_ty = match parameter {
                    Parameter::Named { ty, .. }
                    | Parameter::Pattern { ty, .. }
                    | Parameter::VariadicNamed { ty, .. }
                    | Parameter::VariadicPattern { ty, .. } => *ty,
                };
                if parameter_ty.is_some_and(|ty| ty.id == current_id) {
                    return true;
                }
            }

            // where clause constraint
            NodeType::WhereClause => {
                let where_clause = context.tree.get(LocalNodeId::<WhereClause>::new(parent_id));
                if where_clause.right.id == current_id {
                    return true;
                }
            }

            // property field type
            NodeType::Property => {
                let property = context.tree.get(LocalNodeId::<Property>::new(parent_id));
                if let Property::Field { value, .. } = property
                    && value.is_some_and(|value| value.id == current_id)
                {
                    return true;
                }
            }

            // member type slots
            NodeType::Member => {
                let member = context.tree.get(LocalNodeId::<Member>::new(parent_id));
                let is_type_slot = match member {
                    Member::Type { ty, value, .. } => {
                        ty.is_some_and(|ty| ty.id == current_id)
                            || value.is_some_and(|value| value.id == current_id)
                    }
                    Member::Field { value, .. } => {
                        value.is_some_and(|value| value.id == current_id)
                    }
                    Member::ComptimeConst { ty, .. } => ty.is_some_and(|ty| ty.id == current_id),
                    Member::Embed { value, .. } => value.id == current_id,
                    Member::Method { .. }
                    | Member::StaticBlock { .. }
                    | Member::ComptimeBlock { .. } => false,
                };
                if is_type_slot {
                    return true;
                }
            }

            // declaration type slots
            NodeType::Declaration => {
                let declaration = context.tree.get(LocalNodeId::<Declaration>::new(parent_id));
                let in_type_slot = match declaration {
                    Declaration::Type { value, .. } => value.id == current_id,
                    Declaration::Struct { heritage, .. }
                    | Declaration::Class { heritage, .. }
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
                        (DependencyKind::Type, ImportAliasTarget::Path { value }) => {
                            value.id == current_id
                        }
                        _ => false,
                    },
                    Declaration::Global { .. } | Declaration::Namespace { .. } => false,
                };
                if in_type_slot {
                    return true;
                }
            }

            _ => {}
        }

        current_id = parent_id;
    }

    false
}

/// Return whether an expression is the type annotation of a parameter.
pub(super) fn is_parameter_type_annotation(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let Some((parent_id, parent_type)) = context.get_parent(expression_id) else {
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
pub(super) fn is_simple_type_binary_left_expression(
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

/// Return the previous non whitespace character before a span.
pub(super) fn is_object_like_type_expression(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let expression_id = transparent_inner_expression(context, expression_id);
    matches!(
        context.tree.get(expression_id),
        Expression::ObjectExpression { .. } | Expression::TypeMapped { .. }
    )
}

/// Whether a type expression is nullable (null, undefined, or void).
pub(super) fn is_nullable_union_member(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let expression_id = transparent_inner_expression(context, expression_id);
    matches!(
        context.tree.get(expression_id),
        Expression::TypeLiteral(TypeLiteral::Null | TypeLiteral::Undefined | TypeLiteral::Void)
    )
}

/// Whether a type expression is a simple reference.
pub(super) fn is_type_reference_expression(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let expression_id = transparent_inner_expression(context, expression_id);
    matches!(
        context.tree.get(expression_id),
        Expression::Path { .. } | Expression::Member { .. } | Expression::PrivateMember { .. }
    )
}

/// Decide whether a nullable union type should stay inline with `|` separators.
pub(super) fn should_hug_nullable_union_type(
    context: &DestackFormatContext<'_>,
    operands: &[BinaryOperand],
) -> bool {
    if operands.len() < 2 {
        return false;
    }

    if operands
        .iter()
        .any(|operand| context.has_annotation(operand.expression))
    {
        return false;
    }

    let mut nullable_count = 0;
    let mut has_object_or_ref = false;
    let mut non_nullable_count = 0;

    for operand in operands {
        let expression_id = operand.expression;
        if is_nullable_union_member(context, expression_id) {
            nullable_count += 1;
            continue;
        }

        non_nullable_count += 1;
        if is_object_like_type_expression(context, expression_id)
            || is_type_reference_expression(context, expression_id)
        {
            has_object_or_ref = true;
        } else {
            return false;
        }
    }

    has_object_or_ref && non_nullable_count == 1 && nullable_count == operands.len() - 1
}

/// Decide whether a union type in static arguments should stay inline.
pub(super) fn should_hug_static_argument_union_type(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    operands: &[BinaryOperand],
) -> bool {
    if !is_static_type_argument_context(context, node_id) || operands.len() < 2 {
        return false;
    }

    if union_source_has_leading_pipe(context, node_id) {
        return false;
    }

    if operands
        .iter()
        .any(|operand| context.has_annotation(operand.expression))
    {
        return false;
    }

    let mut previous_expression: Option<LocalNodeId<Expression>> = None;
    for operand in operands {
        if let Some(previous_expression) = previous_expression
            && has_comment_between_expressions(context, previous_expression, operand.expression)
        {
            return false;
        }

        let span = context.get_span(operand.expression);
        if context.has_newline(span) {
            return false;
        }

        let operand_expression = transparent_inner_expression(context, operand.expression);
        if matches!(
            context.tree.get(operand_expression),
            Expression::ObjectExpression { .. }
                | Expression::ArrayExpression { .. }
                | Expression::TypeMapped { .. }
        ) {
            return false;
        }

        previous_expression = Some(operand.expression);
    }

    true
}

/// Return whether a union expression source starts with a leading `|`.
pub(super) fn union_source_has_leading_pipe(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let span = context.get_span(node_id);
    let source = context.get_span_str(span);
    source.trim_start().starts_with('|')
        || previous_non_whitespace_before_span(context, span) == Some('|')
}

/// Return whether a binary operator participates in type union or intersection grouping.
pub(super) fn is_type_grouping_binary_operator(operator: BinaryOperator) -> bool {
    matches!(
        operator,
        BinaryOperator::ElementwiseOr | BinaryOperator::ElementwiseAnd
    )
}

/// Return whether a long binary rhs should break immediately after an assignment operator.
pub(super) fn binary_rhs_prefers_break_after_operator(
    value_source_len: usize,
    line_width: usize,
) -> bool {
    value_source_len > line_width
}

/// Return whether a type binary operand needs grouping parentheses.
pub(super) fn type_binary_operand_needs_grouping_parentheses(
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

    is_type_grouping_binary_operator(*inner_operator)
        && *inner_operator != parent_operator
        && is_type_context(context, inner_id)
}

/// Format a binary operand with grouping parentheses when needed in type contexts.
pub(super) fn format_binary_operand_with_grouping_parentheses<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    parent_operator: BinaryOperator,
    operand_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
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
    let needs_precedence_parentheses = !matches!(expression, Expression::Parenthesized { .. })
        && expression_precedence(expression) < parent_operator.precedence()
        && !suppress_precedence_parentheses_for_type_binary;
    if needs_type_grouping_parentheses || needs_precedence_parentheses {
        write!(f, [token("("), operand_id, token(")")])?;
    } else {
        write!(f, [operand_id])?;
    }

    Ok(())
}
