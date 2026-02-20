use super::{
    Argument, DestackFormatContext, Expression, LocalNodeId, NodeTree, Pattern, PatternField,
    PostfixPosition, Property, ScalarLiteral, StringId, TypeLiteral, YieldCardinality,
    argument_value_id, assign_operator_len, binary_operator_len, transparent_inner_expression,
};
use destack_ast::{
    AbstractionModifier, AccessorKind, Asynchrony, BindingAnchor, BindingKind, BindingModifier,
    BindingOperator, Declaration, DeclarationKind, FunctionAbstraction, FunctionCardinality,
    FunctionKind, FunctionMode, IntrinsicType, Key, Mutability, Name, Node, NodeTreeImpl,
    Parameter, Path, TemplateLiteral, Timing, VarianceModifier, Visibility,
};

/// Return the display width of one interned string.
fn string_len(context: &DestackFormatContext<'_>, string_id: StringId) -> usize {
    context.strings.get(string_id).chars().count()
}

/// Return the display width of one named key.
fn name_len(context: &DestackFormatContext<'_>, name: Name) -> usize {
    match name {
        Name::Identifier(string_id) | Name::Number(string_id) => string_len(context, string_id),
        Name::String(string_id) => string_len(context, string_id).saturating_add(2),
    }
}

/// Return the display width of one path.
fn path_len(context: &DestackFormatContext<'_>, path: &Path) -> usize {
    let mut total_len = 0usize;
    for (index, segment) in path.segments.iter().enumerate() {
        if index > 0 {
            total_len = total_len.saturating_add(1);
        }
        total_len = total_len.saturating_add(string_len(context, *segment));
    }
    total_len
}

/// Return the display width of one scalar literal.
fn scalar_literal_len(context: &DestackFormatContext<'_>, value: &ScalarLiteral) -> usize {
    match value {
        ScalarLiteral::Boolean(value) => {
            if *value {
                4
            } else {
                5
            }
        }
        ScalarLiteral::Integer(value) | ScalarLiteral::Bigint(value) => value.to_string().len(),
        ScalarLiteral::Float(value) => value.to_string().len(),
        ScalarLiteral::Character(_) => 3,
        ScalarLiteral::String(string_id) => string_len(context, *string_id).saturating_add(2),
        ScalarLiteral::RegexString { content, flags } => {
            let flags_len = flags.map_or(0usize, |flags| string_len(context, flags));
            string_len(context, *content)
                .saturating_add(flags_len)
                .saturating_add(2)
        }
    }
}

/// Return the display width of one type literal.
fn type_literal_len(value: &TypeLiteral) -> usize {
    match value {
        TypeLiteral::Never => 5,
        TypeLiteral::Any => 3,
        TypeLiteral::Infer => 1,
        TypeLiteral::Undefined => 9,
        TypeLiteral::Unknown => 7,
        TypeLiteral::Object => 6,
        TypeLiteral::Void => 4,
        TypeLiteral::Null => 4,
        TypeLiteral::Boolean => 7,
        TypeLiteral::Character => 9,
        TypeLiteral::String => 6,
        TypeLiteral::Bigint => 6,
        TypeLiteral::Number => 6,
        TypeLiteral::Int(value) => value.as_str().len(),
        TypeLiteral::Float(value) => value.as_str().len(),
        TypeLiteral::Symbol => 6,
        TypeLiteral::UniqueSymbol => 13,
        TypeLiteral::Intrinsic(value) => match value {
            IntrinsicType::Uppercase => 9,
            IntrinsicType::Lowercase => 9,
            IntrinsicType::Capitalize => 10,
            IntrinsicType::Uncapitalize => 12,
            IntrinsicType::NoInfer => 7,
            IntrinsicType::BuiltinIteratorReturn => 21,
        },
    }
}

/// Return the display width of one argument.
fn argument_inline_width_hint(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> usize {
    match context.tree.get(argument_id) {
        Argument::Named { name, value, .. } => name_len(context, *name)
            .saturating_add(1)
            .saturating_add(expression_inline_width_hint(context, *value)),
        Argument::Labeled { label, value, .. } => string_len(context, *label)
            .saturating_add(2)
            .saturating_add(expression_inline_width_hint(context, *value)),
        Argument::Positional { value, .. } => expression_inline_width_hint(context, *value),
        Argument::Spread { label, value, .. } => {
            let label_len = label.map_or(0usize, |label| string_len(context, label));
            let label_separator_len = usize::from(label.is_some()) * 2;
            3usize
                .saturating_add(label_len)
                .saturating_add(label_separator_len)
                .saturating_add(expression_inline_width_hint(context, *value))
        }
    }
}

/// Return the display width of one comma-separated argument list.
fn argument_list_inline_width_hint(
    context: &DestackFormatContext<'_>,
    arguments: &[LocalNodeId<Argument>],
) -> usize {
    let mut total_len = 0usize;
    for (index, argument_id) in arguments.iter().enumerate() {
        if index > 0 {
            total_len = total_len.saturating_add(2);
        }
        total_len = total_len.saturating_add(argument_inline_width_hint(context, *argument_id));
    }
    total_len
}

/// Return one approximate inline width contribution for annotations on one node.
fn annotation_inline_width_hint_for_node<T>(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<T>,
) -> usize
where
    T: Node,
    NodeTree: NodeTreeImpl<T>,
{
    context
        .annotations(node_id)
        .map(|annotation_ids| {
            annotation_ids
                .iter()
                .fold(0usize, |total_len, annotation_id| {
                    let annotation_span = context.annotation_span(*annotation_id);
                    let annotation_len = context.span_char_len(annotation_span);

                    total_len.saturating_add(annotation_len).saturating_add(1)
                })
        })
        .unwrap_or(0)
}

/// Return the display width contribution of one visibility prefix.
fn visibility_inline_width_hint(visibility: Visibility) -> usize {
    match visibility {
        Visibility::Public => 7,
        Visibility::Protected => 10,
        Visibility::Private => 8,
    }
}

/// Return the display width contribution of one binding prefix modifier list.
fn binding_modifiers_prefix_inline_width_hint(modifiers: Option<BindingModifier>) -> usize {
    let Some(modifiers) = modifiers else {
        return 0;
    };

    let mut total_len = 0usize;
    if let Some(variance) = modifiers.variance {
        total_len = total_len.saturating_add(match variance {
            VarianceModifier::In => 3,
            VarianceModifier::Out => 4,
            VarianceModifier::InOut => 7,
        });
    }

    if let Some(visibility) = modifiers.visibility {
        total_len = total_len.saturating_add(visibility_inline_width_hint(visibility));
    }

    if modifiers.declaration == Some(DeclarationKind::Declaration) {
        total_len = total_len.saturating_add(8);
    }

    if let Some(abstraction) = modifiers.abstraction {
        total_len = total_len.saturating_add(match abstraction {
            AbstractionModifier::Abstract => 9,
            AbstractionModifier::Override => 9,
            AbstractionModifier::AbstractOverride => 18,
        });
    }

    if modifiers.anchor == Some(BindingAnchor::Static) {
        total_len = total_len.saturating_add(7);
    }

    if modifiers.mutability == Some(Mutability::Immutable) {
        total_len = total_len.saturating_add(9);
    }

    if modifiers.operator == Some(BindingOperator::AsConst) {
        total_len = total_len.saturating_add(6);
    }

    if modifiers.accessor == Some(AccessorKind::Accessor) {
        total_len = total_len.saturating_add(9);
    }

    if modifiers.timing == Some(Timing::Comptime) {
        total_len = total_len.saturating_add(9);
    }

    total_len
}

/// Return the display width contribution of one binding postfix modifier list.
fn binding_modifiers_postfix_inline_width_hint(modifiers: Option<BindingModifier>) -> usize {
    let Some(modifiers) = modifiers else {
        return 0;
    };

    if matches!(modifiers.kind, Some(BindingKind::Must | BindingKind::Maybe)) {
        1
    } else {
        0
    }
}

/// Return the display width contribution of one parameter type clause.
fn parameter_type_inline_width_hint(
    context: &DestackFormatContext<'_>,
    ty: Option<LocalNodeId<Expression>>,
    is_static_parameter: bool,
) -> usize {
    let Some(type_id) = ty else {
        return 0;
    };

    let separator_len: usize = if is_static_parameter { 9 } else { 2 };
    separator_len.saturating_add(expression_inline_width_hint(context, type_id))
}

/// Return the display width of one function parameter.
fn parameter_inline_width_hint(
    context: &DestackFormatContext<'_>,
    parameter_id: LocalNodeId<Parameter>,
    is_static_parameter: bool,
) -> usize {
    let annotation_len = annotation_inline_width_hint_for_node(context, parameter_id);
    let base_len = match context.tree.get(parameter_id) {
        Parameter::Named {
            modifiers,
            name,
            ty,
            default,
        } => {
            let name_len = string_len(context, *name);
            let prefix_modifiers_len = binding_modifiers_prefix_inline_width_hint(*modifiers);
            let postfix_modifiers_len = binding_modifiers_postfix_inline_width_hint(*modifiers);
            let type_len = parameter_type_inline_width_hint(context, *ty, is_static_parameter);
            let default_len = default.map_or(0usize, |expression_id| {
                expression_inline_width_hint(context, expression_id).saturating_add(3)
            });

            prefix_modifiers_len
                .saturating_add(name_len)
                .saturating_add(postfix_modifiers_len)
                .saturating_add(type_len)
                .saturating_add(default_len)
        }
        Parameter::Pattern {
            modifiers,
            pattern,
            ty,
            default,
        } => {
            let pattern_len = pattern_inline_width_hint(context, *pattern);
            let prefix_modifiers_len = binding_modifiers_prefix_inline_width_hint(*modifiers);
            let postfix_modifiers_len = binding_modifiers_postfix_inline_width_hint(*modifiers);
            let type_len = parameter_type_inline_width_hint(context, *ty, is_static_parameter);
            let default_len = default.map_or(0usize, |expression_id| {
                expression_inline_width_hint(context, expression_id).saturating_add(3)
            });

            prefix_modifiers_len
                .saturating_add(pattern_len)
                .saturating_add(postfix_modifiers_len)
                .saturating_add(type_len)
                .saturating_add(default_len)
        }
        Parameter::VariadicNamed {
            modifiers,
            name,
            ty,
        } => {
            let name_len = string_len(context, *name);
            let prefix_modifiers_len = binding_modifiers_prefix_inline_width_hint(*modifiers);
            let type_len = parameter_type_inline_width_hint(context, *ty, is_static_parameter);

            prefix_modifiers_len
                .saturating_add(3)
                .saturating_add(name_len)
                .saturating_add(type_len)
        }
        Parameter::VariadicPattern {
            modifiers,
            pattern,
            ty,
        } => {
            let pattern_len = pattern_inline_width_hint(context, *pattern);
            let prefix_modifiers_len = binding_modifiers_prefix_inline_width_hint(*modifiers);
            let type_len = parameter_type_inline_width_hint(context, *ty, is_static_parameter);

            prefix_modifiers_len
                .saturating_add(3)
                .saturating_add(pattern_len)
                .saturating_add(type_len)
        }
    };

    base_len.saturating_add(annotation_len)
}

/// Return the display width of one static parameter list.
fn static_parameter_list_inline_width_hint(
    context: &DestackFormatContext<'_>,
    static_parameters: Option<&[LocalNodeId<Parameter>]>,
) -> usize {
    let Some(static_parameters) = static_parameters else {
        return 0;
    };
    if static_parameters.is_empty() {
        return 0;
    }

    let mut total_len = 2usize;
    for (index, parameter_id) in static_parameters.iter().copied().enumerate() {
        if index > 0 {
            total_len = total_len.saturating_add(2);
        }
        total_len =
            total_len.saturating_add(parameter_inline_width_hint(context, parameter_id, true));
    }

    total_len
}

/// Return the display width of one dynamic parameter list.
fn dynamic_parameter_list_inline_width_hint(
    context: &DestackFormatContext<'_>,
    this_parameter: Option<LocalNodeId<Parameter>>,
    dynamic_parameters: &[LocalNodeId<Parameter>],
) -> usize {
    let mut total_len = 2usize;
    let mut has_parameter = false;

    if let Some(this_parameter) = this_parameter {
        total_len =
            total_len.saturating_add(parameter_inline_width_hint(context, this_parameter, false));
        has_parameter = true;
    }

    for parameter_id in dynamic_parameters {
        if has_parameter {
            total_len = total_len.saturating_add(2);
        }
        total_len =
            total_len.saturating_add(parameter_inline_width_hint(context, *parameter_id, false));
        has_parameter = true;
    }

    total_len
}

/// Return the display width contribution of one function abstraction prefix.
fn function_abstraction_inline_width_hint(abstraction: FunctionAbstraction) -> usize {
    match abstraction {
        FunctionAbstraction::Abstract => 9,
        FunctionAbstraction::AbstractOverride => 18,
        FunctionAbstraction::ConcreteOverride => 9,
        FunctionAbstraction::Concrete => 0,
    }
}

/// Return the display width contribution of one function asynchrony prefix.
fn function_asynchrony_inline_width_hint(asynchrony: Asynchrony) -> usize {
    if asynchrony == Asynchrony::Async {
        6
    } else {
        0
    }
}

/// Return the display width contribution of one function mode prefix.
fn function_mode_inline_width_hint(mode: Option<FunctionMode>) -> usize {
    let Some(mode) = mode else {
        return 0;
    };

    match mode {
        FunctionMode::Getter => 3,
        FunctionMode::Setter => 3,
        FunctionMode::Constructor => 11,
        FunctionMode::New => 4,
        FunctionMode::Call => 0,
    }
}

/// Return a structural inline width for one lambda declaration expression.
fn lambda_declaration_inline_width_hint(
    context: &DestackFormatContext<'_>,
    declaration_id: LocalNodeId<Declaration>,
) -> Option<usize> {
    let Declaration::Function {
        signature, body, ..
    } = context.tree.get(declaration_id)
    else {
        return None;
    };
    if signature.kind != FunctionKind::Lambda {
        return None;
    }
    if signature
        .generics
        .as_ref()
        .and_then(|generics| generics.where_clauses.as_ref())
        .is_some_and(|where_clauses| !where_clauses.is_empty())
    {
        return None;
    }

    let Some(body_id) = *body else {
        return None;
    };
    if matches!(context.tree.get(body_id), Expression::Block(_)) {
        return None;
    }

    let static_parameter_len = static_parameter_list_inline_width_hint(
        context,
        signature
            .generics
            .as_ref()
            .and_then(|generics| generics.static_parameters.as_deref()),
    );
    let dynamic_parameter_len = dynamic_parameter_list_inline_width_hint(
        context,
        signature.this_parameter,
        &signature.dynamic_parameters,
    );
    let return_type_len = signature.return_type.map_or(0usize, |return_type| {
        expression_inline_width_hint(context, return_type).saturating_add(2)
    });
    let generator_len = if signature.cardinality == FunctionCardinality::Generator {
        2
    } else {
        0
    };

    Some(
        function_abstraction_inline_width_hint(signature.abstraction)
            .saturating_add(function_asynchrony_inline_width_hint(signature.asynchrony))
            .saturating_add(function_mode_inline_width_hint(signature.mode))
            .saturating_add(generator_len)
            .saturating_add(static_parameter_len)
            .saturating_add(dynamic_parameter_len)
            .saturating_add(return_type_len)
            .saturating_add(4)
            .saturating_add(expression_inline_width_hint(context, body_id)),
    )
}

/// Return the display width of one static argument list.
fn static_argument_list_inline_width_hint(
    context: &DestackFormatContext<'_>,
    static_arguments: &[LocalNodeId<Argument>],
) -> usize {
    argument_list_inline_width_hint(context, static_arguments).saturating_add(2)
}

/// Return the display width of one key.
fn key_inline_width_hint(context: &DestackFormatContext<'_>, key: &Key) -> usize {
    match key {
        Key::Name(name) => name_len(context, *name),
        Key::Private(name) => string_len(context, *name).saturating_add(1),
        Key::Expression(expression_id) => {
            expression_inline_width_hint(context, *expression_id).saturating_add(2)
        }
        Key::NamedExpression { name, key } => string_len(context, *name)
            .saturating_add(4)
            .saturating_add(expression_inline_width_hint(context, *key)),
    }
}

/// Return the display width of one object property.
fn property_inline_width_hint(
    context: &DestackFormatContext<'_>,
    property_id: LocalNodeId<Property>,
) -> usize {
    match context.tree.get(property_id) {
        Property::Field {
            key,
            value,
            default,
            ..
        } => {
            let key_len = key
                .as_ref()
                .map_or(0usize, |key| key_inline_width_hint(context, key));
            let value_len = value.map_or(0usize, |value| {
                expression_inline_width_hint(context, value).saturating_add(2)
            });
            let default_len = default.map_or(0usize, |value| {
                expression_inline_width_hint(context, value).saturating_add(3)
            });
            key_len
                .saturating_add(value_len)
                .saturating_add(default_len)
        }
        Property::Method { key, .. } => key
            .as_ref()
            .map_or(0usize, |key| key_inline_width_hint(context, key))
            .saturating_add(4),
        Property::Spread { value, .. } => {
            3usize.saturating_add(expression_inline_width_hint(context, *value))
        }
    }
}

/// Return the display width of one pattern field.
fn pattern_field_inline_width_hint(
    context: &DestackFormatContext<'_>,
    field_id: LocalNodeId<PatternField>,
) -> usize {
    let annotation_len = annotation_inline_width_hint_for_node(context, field_id);
    let base_len = match context.tree.get(field_id) {
        PatternField::Named {
            name,
            pattern,
            default,
            ..
        } => {
            let name_len = name_len(context, *name);
            let pattern_len = pattern.map_or(0usize, |pattern| {
                pattern_inline_width_hint(context, pattern).saturating_add(2)
            });
            let default_len = default.map_or(0usize, |default| {
                expression_inline_width_hint(context, default).saturating_add(3)
            });
            name_len
                .saturating_add(pattern_len)
                .saturating_add(default_len)
        }
        PatternField::Computed {
            key,
            pattern,
            default,
            ..
        } => {
            let key_len = expression_inline_width_hint(context, *key).saturating_add(2);
            let pattern_len = pattern.map_or(0usize, |pattern| {
                pattern_inline_width_hint(context, pattern).saturating_add(2)
            });
            let default_len = default.map_or(0usize, |default| {
                expression_inline_width_hint(context, default).saturating_add(3)
            });
            key_len
                .saturating_add(pattern_len)
                .saturating_add(default_len)
        }
        PatternField::Alias {
            name,
            alias,
            default,
            ..
        } => {
            let name_len = name_len(context, *name);
            let alias_len = string_len(context, *alias);
            let default_len = default.map_or(0usize, |default| {
                expression_inline_width_hint(context, default).saturating_add(3)
            });
            name_len
                .saturating_add(2)
                .saturating_add(alias_len)
                .saturating_add(default_len)
        }
        PatternField::Positional { pattern, default } => {
            let pattern_len = pattern_inline_width_hint(context, *pattern);
            let default_len = default.map_or(0usize, |default| {
                expression_inline_width_hint(context, default).saturating_add(3)
            });
            pattern_len.saturating_add(default_len)
        }
        PatternField::Spread { pattern, .. } => {
            3usize.saturating_add(pattern.map_or(0usize, |pattern| {
                pattern_inline_width_hint(context, pattern)
            }))
        }
        PatternField::Elision => 0,
    };

    base_len.saturating_add(annotation_len)
}

/// Return the display width of one binding pattern.
pub(crate) fn pattern_inline_width_hint(
    context: &DestackFormatContext<'_>,
    pattern_id: LocalNodeId<Pattern>,
) -> usize {
    let annotation_len = annotation_inline_width_hint_for_node(context, pattern_id);
    let base_len =
        match context.tree.get(pattern_id) {
            Pattern::Wildcard => 1,
            Pattern::Must(pattern) => {
                pattern_inline_width_hint(context, *pattern).saturating_add(1)
            }
            Pattern::ReferenceOf { right, .. } | Pattern::ValueOf { right, .. } => {
                pattern_inline_width_hint(context, *right).saturating_add(1)
            }
            Pattern::Binding { name, pattern, .. } => {
                let name_len = string_len(context, *name);
                if let Some(pattern) = pattern {
                    name_len
                        .saturating_add(2)
                        .saturating_add(pattern_inline_width_hint(context, *pattern))
                } else {
                    name_len
                }
            }
            Pattern::Expression { value } => expression_inline_width_hint(context, *value),
            Pattern::Tuple { fields } => {
                fields
                    .iter()
                    .enumerate()
                    .fold(2usize, |total_len, (index, field)| {
                        let separator_len = if index > 0 { 2 } else { 0 };
                        total_len
                            .saturating_add(separator_len)
                            .saturating_add(pattern_field_inline_width_hint(context, *field))
                    })
            }
            Pattern::TaggedTuple { ty, fields } => expression_inline_width_hint(context, *ty)
                .saturating_add(fields.iter().enumerate().fold(
                    2usize,
                    |total_len, (index, field)| {
                        let separator_len = if index > 0 { 2 } else { 0 };
                        total_len
                            .saturating_add(separator_len)
                            .saturating_add(pattern_field_inline_width_hint(context, *field))
                    },
                )),
            Pattern::Array { fields } => {
                fields
                    .iter()
                    .enumerate()
                    .fold(2usize, |total_len, (index, field)| {
                        let separator_len = if index > 0 { 2 } else { 0 };
                        total_len
                            .saturating_add(separator_len)
                            .saturating_add(pattern_field_inline_width_hint(context, *field))
                    })
            }
            Pattern::Object { fields } => {
                fields
                    .iter()
                    .enumerate()
                    .fold(2usize, |total_len, (index, field)| {
                        let separator_len = if index > 0 { 2 } else { 0 };
                        total_len
                            .saturating_add(separator_len)
                            .saturating_add(pattern_field_inline_width_hint(context, *field))
                    })
            }
            Pattern::TaggedObject { ty, fields } => expression_inline_width_hint(context, *ty)
                .saturating_add(1)
                .saturating_add(fields.iter().enumerate().fold(
                    2usize,
                    |total_len, (index, field)| {
                        let separator_len = if index > 0 { 2 } else { 0 };
                        total_len
                            .saturating_add(separator_len)
                            .saturating_add(pattern_field_inline_width_hint(context, *field))
                    },
                )),
            Pattern::Union { patterns } => {
                patterns
                    .iter()
                    .enumerate()
                    .fold(0usize, |total_len, (index, pattern)| {
                        let separator_len = if index > 0 { 3 } else { 0 };
                        total_len
                            .saturating_add(separator_len)
                            .saturating_add(pattern_inline_width_hint(context, *pattern))
                    })
            }
        };

    base_len.saturating_add(annotation_len)
}

/// Get a structural inline length signal for one expression.
pub(crate) fn expression_inline_width_hint(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> usize {
    let expression_id = transparent_inner_expression(context, expression_id);
    let expression = context.tree.get(expression_id);
    let forced_multiline_len = usize::from(context.options.line_width).saturating_add(1);

    match expression {
        Expression::Declaration(declaration_id) => {
            lambda_declaration_inline_width_hint(context, *declaration_id)
                .unwrap_or(forced_multiline_len)
        }
        Expression::Block(_) => forced_multiline_len,
        Expression::Labelled { label, body } => string_len(context, *label)
            .saturating_add(2)
            .saturating_add(expression_inline_width_hint(context, *body)),
        Expression::Import { .. }
        | Expression::Export { .. }
        | Expression::ExportNamespace { .. }
        | Expression::Let { .. }
        | Expression::Using { .. }
        | Expression::If { .. }
        | Expression::While { .. }
        | Expression::ForEach { .. }
        | Expression::For { .. }
        | Expression::Loop { .. }
        | Expression::Try { .. }
        | Expression::Match { .. } => forced_multiline_len,
        Expression::Break { label, value } => {
            let label_len =
                label.map_or(0usize, |label| string_len(context, label).saturating_add(1));
            let value_len = value.map_or(0usize, |value| {
                expression_inline_width_hint(context, value).saturating_add(1)
            });
            5usize.saturating_add(label_len).saturating_add(value_len)
        }
        Expression::Continue { label } => {
            let label_len =
                label.map_or(0usize, |label| string_len(context, label).saturating_add(1));
            8usize.saturating_add(label_len)
        }
        Expression::Yield { cardinality, value } => {
            let cardinality_len = usize::from(*cardinality == YieldCardinality::Generator);
            let value_len = value.map_or(0usize, |value| {
                expression_inline_width_hint(context, value).saturating_add(1)
            });
            5usize
                .saturating_add(cardinality_len)
                .saturating_add(value_len)
        }
        Expression::Throw { value } => {
            expression_inline_width_hint(context, *value).saturating_add(6)
        }
        Expression::Return { value } => {
            let value_len = value.map_or(0usize, |value| {
                expression_inline_width_hint(context, value).saturating_add(1)
            });
            6usize.saturating_add(value_len)
        }
        Expression::Path {
            path,
            static_arguments,
        } => {
            let static_arguments_len = static_arguments.as_deref().map_or(0usize, |arguments| {
                static_argument_list_inline_width_hint(context, arguments)
            });
            path_len(context, path).saturating_add(static_arguments_len)
        }
        Expression::PrivateIdentifier { name } => string_len(context, *name).saturating_add(1),
        Expression::This => 4,
        Expression::Super => 5,
        Expression::ScalarLiteral(value) => scalar_literal_len(context, value),
        Expression::TypeLiteral(value) => type_literal_len(value),
        Expression::TemplateExpression { value } => match value {
            TemplateLiteral::String { string } => string_len(context, *string).saturating_add(2),
            TemplateLiteral::InterpolatedString { strings, arguments } => {
                let strings_len = strings.iter().fold(0usize, |total_len, string_id| {
                    total_len.saturating_add(string_len(context, *string_id))
                });
                let argument_len = arguments.iter().fold(0usize, |total_len, argument_id| {
                    total_len
                        .saturating_add(expression_inline_width_hint(
                            context,
                            argument_value_id(context.tree, *argument_id),
                        ))
                        .saturating_add(3)
                });
                2usize
                    .saturating_add(strings_len)
                    .saturating_add(argument_len)
            }
        },
        Expression::TaggedTemplateExpression { tag, value } => {
            expression_inline_width_hint(context, *tag).saturating_add(match value {
                TemplateLiteral::String { string } => {
                    string_len(context, *string).saturating_add(2)
                }
                TemplateLiteral::InterpolatedString { strings, arguments } => {
                    let strings_len = strings.iter().fold(0usize, |total_len, string_id| {
                        total_len.saturating_add(string_len(context, *string_id))
                    });
                    let argument_len = arguments.iter().fold(0usize, |total_len, argument_id| {
                        total_len
                            .saturating_add(expression_inline_width_hint(
                                context,
                                argument_value_id(context.tree, *argument_id),
                            ))
                            .saturating_add(3)
                    });
                    2usize
                        .saturating_add(strings_len)
                        .saturating_add(argument_len)
                }
            })
        }
        Expression::ArrayExpression { elements } | Expression::TupleExpression { elements } => {
            argument_list_inline_width_hint(context, elements).saturating_add(2)
        }
        Expression::SequenceExpression { expressions } => {
            expressions
                .iter()
                .enumerate()
                .fold(0usize, |total_len, (index, expression_id)| {
                    let separator_len = if index > 0 { 2 } else { 0 };
                    total_len
                        .saturating_add(separator_len)
                        .saturating_add(expression_inline_width_hint(context, *expression_id))
                })
        }
        Expression::ObjectExpression { ty, properties } => {
            let type_len = ty.map_or(0usize, |type_id| {
                expression_inline_width_hint(context, type_id).saturating_add(1)
            });
            let properties_len =
                properties
                    .iter()
                    .enumerate()
                    .fold(0usize, |total_len, (index, property_id)| {
                        let separator_len = if index > 0 { 2 } else { 0 };
                        total_len
                            .saturating_add(separator_len)
                            .saturating_add(property_inline_width_hint(context, *property_id))
                    });
            type_len.saturating_add(2).saturating_add(properties_len)
        }
        Expression::TreeExpression {
            left,
            arguments,
            elements,
        } => {
            let left_len = left.map_or(0usize, |left_id| {
                expression_inline_width_hint(context, left_id)
            });
            let argument_len = arguments.as_ref().map_or(0usize, |arguments| {
                if arguments.is_empty() {
                    0
                } else {
                    1usize.saturating_add(argument_list_inline_width_hint(context, arguments))
                }
            });
            let has_elements = elements
                .as_ref()
                .is_some_and(|elements| !elements.is_empty());
            let closing_len = if has_elements {
                left_len.saturating_add(3)
            } else {
                2
            };
            1usize
                .saturating_add(left_len)
                .saturating_add(argument_len)
                .saturating_add(closing_len)
        }
        Expression::Parenthesized { expression } => {
            expression_inline_width_hint(context, *expression).saturating_add(2)
        }
        Expression::TypeUnary { right, .. } | Expression::Unary { right, .. } => {
            expression_inline_width_hint(context, *right).saturating_add(2)
        }
        Expression::TypeBinary { left, right, .. } => expression_inline_width_hint(context, *left)
            .saturating_add(3)
            .saturating_add(expression_inline_width_hint(context, *right)),
        Expression::TypeConditional {
            left,
            right,
            then_type,
            else_type,
        } => expression_inline_width_hint(context, *left)
            .saturating_add(10)
            .saturating_add(expression_inline_width_hint(context, *right))
            .saturating_add(expression_inline_width_hint(context, *then_type))
            .saturating_add(expression_inline_width_hint(context, *else_type)),
        Expression::TypeMapped { value, .. } => {
            expression_inline_width_hint(context, *value).saturating_add(8)
        }
        Expression::TypeIndex { left, index } => expression_inline_width_hint(context, *left)
            .saturating_add(expression_inline_width_hint(context, *index))
            .saturating_add(2),
        Expression::TypeTemplateLiteral { strings, spans } => {
            let strings_len = strings.iter().fold(0usize, |total_len, string_id| {
                total_len.saturating_add(string_len(context, *string_id))
            });
            let spans_len = spans.iter().fold(0usize, |total_len, span| {
                total_len
                    .saturating_add(expression_inline_width_hint(context, *span))
                    .saturating_add(3)
            });
            2usize.saturating_add(strings_len).saturating_add(spans_len)
        }
        Expression::TypeImport {
            target,
            arguments,
            qualifier,
            static_arguments,
        } => {
            let target_len = expression_inline_width_hint(context, *target);
            let arguments_len =
                argument_list_inline_width_hint(context, arguments).saturating_add(2);
            let qualifier_len = qualifier.as_ref().map_or(0usize, |qualifier| {
                path_len(context, qualifier).saturating_add(1)
            });
            let static_arguments_len = static_arguments.as_deref().map_or(0usize, |arguments| {
                static_argument_list_inline_width_hint(context, arguments)
            });

            6usize
                .saturating_add(target_len)
                .saturating_add(arguments_len)
                .saturating_add(qualifier_len)
                .saturating_add(static_arguments_len)
        }
        Expression::TypeInfer { name, constraint } => {
            let base_len = string_len(context, *name).saturating_add(6);
            if let Some(constraint) = constraint {
                base_len
                    .saturating_add(9)
                    .saturating_add(expression_inline_width_hint(context, *constraint))
            } else {
                base_len
            }
        }
        Expression::TypePredicate { target, .. } => target.map_or(7usize, |target| {
            expression_inline_width_hint(context, target).saturating_add(7)
        }),
        Expression::Member {
            left,
            name,
            static_arguments,
        } => {
            let static_arguments_len = static_arguments.as_deref().map_or(0usize, |arguments| {
                static_argument_list_inline_width_hint(context, arguments)
            });
            expression_inline_width_hint(context, *left)
                .saturating_add(1)
                .saturating_add(string_len(context, *name))
                .saturating_add(static_arguments_len)
        }
        Expression::PrivateMember {
            left,
            name,
            static_arguments,
        } => {
            let static_arguments_len = static_arguments.as_deref().map_or(0usize, |arguments| {
                static_argument_list_inline_width_hint(context, arguments)
            });
            expression_inline_width_hint(context, *left)
                .saturating_add(2)
                .saturating_add(string_len(context, *name))
                .saturating_add(static_arguments_len)
        }
        Expression::Index { left, index, .. } => expression_inline_width_hint(context, *left)
            .saturating_add(2)
            .saturating_add(
                index.map_or(0usize, |index| expression_inline_width_hint(context, index)),
            ),
        Expression::Instantiation {
            left,
            static_arguments,
        } => expression_inline_width_hint(context, *left).saturating_add(
            static_argument_list_inline_width_hint(context, static_arguments),
        ),
        Expression::Call {
            left,
            static_arguments,
            dynamic_arguments,
            ..
        } => {
            let static_arguments_len = static_arguments.as_deref().map_or(0usize, |arguments| {
                static_argument_list_inline_width_hint(context, arguments)
            });
            expression_inline_width_hint(context, *left)
                .saturating_add(static_arguments_len)
                .saturating_add(2)
                .saturating_add(argument_list_inline_width_hint(context, dynamic_arguments))
        }
        Expression::New {
            left,
            static_arguments,
            dynamic_arguments,
        } => {
            let static_arguments_len = static_arguments.as_deref().map_or(0usize, |arguments| {
                static_argument_list_inline_width_hint(context, arguments)
            });
            4usize
                .saturating_add(expression_inline_width_hint(context, *left))
                .saturating_add(static_arguments_len)
                .saturating_add(2)
                .saturating_add(argument_list_inline_width_hint(context, dynamic_arguments))
        }
        Expression::Maybe { left, position } | Expression::Must { left, position } => {
            let position_len = match position {
                PostfixPosition::Direct => 1,
                PostfixPosition::Indirect => 2,
            };
            expression_inline_width_hint(context, *left).saturating_add(position_len)
        }
        Expression::Binary {
            left,
            operator,
            right,
        } => expression_inline_width_hint(context, *left)
            .saturating_add(binary_operator_len(operator))
            .saturating_add(2)
            .saturating_add(expression_inline_width_hint(context, *right)),
        Expression::Assign {
            left,
            operator,
            right,
        } => expression_inline_width_hint(context, *left)
            .saturating_add(assign_operator_len(operator))
            .saturating_add(2)
            .saturating_add(expression_inline_width_hint(context, *right)),
        Expression::Delete { value } => {
            expression_inline_width_hint(context, *value).saturating_add(7)
        }
        Expression::Await { expression } => {
            expression_inline_width_hint(context, *expression).saturating_add(6)
        }
        Expression::AwaitMaybe { expression } => {
            expression_inline_width_hint(context, *expression).saturating_add(7)
        }
        Expression::Comptime { body } => {
            expression_inline_width_hint(context, *body).saturating_add(9)
        }
        Expression::ValueOf { right, .. }
        | Expression::ReferenceOf { right, .. }
        | Expression::PointerOf { right, .. } => {
            expression_inline_width_hint(context, *right).saturating_add(2)
        }
        Expression::Statement(expression) => expression_inline_width_hint(context, *expression),
        Expression::Debugger | Expression::Stub | Expression::Error => forced_multiline_len,
    }
}
