use crate::{
    LocalTypeId, MappedTypeParameter, StaticArgument, StaticExpression, StaticProperty, Type,
    TypeElement, TypeField, TypeIndexSignature, TypeTable,
};

/// Options for type rewriters.
#[derive(Debug, Clone, Copy, Default)]
pub struct TypeRewriterOptions {
    cache_key: u64,
}

impl TypeRewriterOptions {
    /// Create rewriter options with the given cache key.
    pub fn new(cache_key: u64) -> Self {
        Self { cache_key }
    }

    /// Return the cache key for this rewriter.
    pub fn cache_key(self) -> u64 {
        self.cache_key
    }
}

/// Rewrite the type graph.
pub trait TypeRewriter {
    /// Return the rewrite options.
    fn options(&self) -> &TypeRewriterOptions;

    /// Rewrite any type.
    fn rewrite_any(
        &mut self,
        _types: &mut TypeTable,
        _id: LocalTypeId,
        _ty: &Type,
    ) -> Option<LocalTypeId> {
        None
    }

    /// Rewrite a type id.
    fn rewrite_type_id(&mut self, types: &mut TypeTable, id: LocalTypeId) -> LocalTypeId {
        let ty = types.get_type(id).clone();
        self.rewrite_type(types, id, &ty)
    }

    /// Rewrite a type.
    fn rewrite_type(&mut self, types: &mut TypeTable, id: LocalTypeId, ty: &Type) -> LocalTypeId {
        destack_core::ensure_sufficient_stack(|| rewrite_type(self, types, id, ty))
    }

    /// Rewrite a static argument.
    fn rewrite_static_argument(
        &mut self,
        types: &mut TypeTable,
        argument: &StaticArgument,
    ) -> StaticArgument {
        rewrite_static_argument(self, types, argument)
    }

    /// Rewrite a static expression.
    fn rewrite_static_expression(
        &mut self,
        types: &mut TypeTable,
        expression: &StaticExpression,
    ) -> StaticExpression {
        destack_core::ensure_sufficient_stack(|| rewrite_static_expression(self, types, expression))
    }

    /// Rewrite a static property.
    fn rewrite_static_property(
        &mut self,
        types: &mut TypeTable,
        property: &StaticProperty,
    ) -> StaticProperty {
        rewrite_static_property(self, types, property)
    }
}

/// Rewrite a type using the rewriter callbacks.
pub fn rewrite_type<V: TypeRewriter + ?Sized>(
    rewriter: &mut V,
    types: &mut TypeTable,
    type_id: LocalTypeId,
    ty: &Type,
) -> LocalTypeId {
    if let Some(mapped) = rewriter.rewrite_any(types, type_id, ty) {
        return mapped;
    }

    match ty {
        Type::TypeLiteral { .. }
        | Type::InferVar { .. }
        | Type::Unevaluated(_)
        | Type::Error
        | Type::This => type_id,
        Type::Value { value } => {
            let mapped_value = rewriter.rewrite_type_id(types, *value);
            if mapped_value == *value {
                type_id
            } else {
                types.insert_type_from_type(
                    Type::Value {
                        value: mapped_value,
                    },
                    type_id,
                )
            }
        }
        Type::Reference {
            symbol,
            generic_arguments,
        } => {
            let Some(generic_arguments) = generic_arguments.as_ref() else {
                return type_id;
            };
            let (mapped_arguments, changed) =
                rewrite_static_arguments(rewriter, types, generic_arguments);
            if !changed {
                type_id
            } else {
                types.insert_type_from_type(
                    Type::Reference {
                        symbol: *symbol,
                        generic_arguments: Some(mapped_arguments),
                    },
                    type_id,
                )
            }
        }
        Type::Import {
            target,
            qualifier,
            generic_arguments,
        } => {
            let Some(generic_arguments) = generic_arguments.as_ref() else {
                return type_id;
            };
            let (mapped_arguments, changed) =
                rewrite_static_arguments(rewriter, types, generic_arguments);
            if !changed {
                type_id
            } else {
                types.insert_type_from_type(
                    Type::Import {
                        target: *target,
                        qualifier: qualifier.clone(),
                        generic_arguments: Some(mapped_arguments),
                    },
                    type_id,
                )
            }
        }
        Type::Conditional {
            distributive_symbol,
            left,
            right,
            then_type,
            else_type,
        } => {
            let mapped_left = rewriter.rewrite_type_id(types, *left);
            let mapped_right = rewriter.rewrite_type_id(types, *right);
            let mapped_then = rewriter.rewrite_type_id(types, *then_type);
            let mapped_else = rewriter.rewrite_type_id(types, *else_type);
            if mapped_left == *left
                && mapped_right == *right
                && mapped_then == *then_type
                && mapped_else == *else_type
            {
                type_id
            } else {
                types.insert_type_from_type(
                    Type::Conditional {
                        distributive_symbol: *distributive_symbol,
                        left: mapped_left,
                        right: mapped_right,
                        then_type: mapped_then,
                        else_type: mapped_else,
                    },
                    type_id,
                )
            }
        }
        Type::Mapped {
            parameter,
            value,
            modifiers,
        } => {
            let (mapped_parameter, parameter_changed) =
                rewrite_type_mapped_parameter(rewriter, types, parameter);
            let mapped_value = rewriter.rewrite_type_id(types, *value);
            if !parameter_changed && mapped_value == *value {
                type_id
            } else {
                types.insert_type_from_type(
                    Type::Mapped {
                        parameter: mapped_parameter,
                        value: mapped_value,
                        modifiers: *modifiers,
                    },
                    type_id,
                )
            }
        }
        Type::Index { left, index } => {
            let mapped_left = rewriter.rewrite_type_id(types, *left);
            let mapped_index = rewriter.rewrite_type_id(types, *index);
            if mapped_left == *left && mapped_index == *index {
                type_id
            } else {
                types.insert_type_from_type(
                    Type::Index {
                        left: mapped_left,
                        index: mapped_index,
                    },
                    type_id,
                )
            }
        }
        Type::TemplateLiteral { strings, spans } => {
            let (mapped_spans, changed) = rewrite_type_ids(rewriter, types, spans);
            if !changed {
                type_id
            } else {
                types.insert_type_from_type(
                    Type::TemplateLiteral {
                        strings: strings.clone(),
                        spans: mapped_spans,
                    },
                    type_id,
                )
            }
        }
        Type::Infer { name, constraint } => {
            let (mapped_constraint, changed) = rewrite_type_id_option(rewriter, types, *constraint);
            if !changed {
                type_id
            } else {
                types.insert_type_from_type(
                    Type::Infer {
                        name: *name,
                        constraint: mapped_constraint,
                    },
                    type_id,
                )
            }
        }
        Type::Predicate {
            asserts,
            subject,
            target,
        } => {
            let (mapped_target, changed) = rewrite_type_id_option(rewriter, types, *target);
            if !changed {
                type_id
            } else {
                types.insert_type_from_type(
                    Type::Predicate {
                        asserts: *asserts,
                        subject: *subject,
                        target: mapped_target,
                    },
                    type_id,
                )
            }
        }
        Type::Readonly { target_type } => {
            let mapped_target_type = rewriter.rewrite_type_id(types, *target_type);
            if mapped_target_type == *target_type {
                type_id
            } else {
                types.insert_type_from_type(
                    Type::Readonly {
                        target_type: mapped_target_type,
                    },
                    type_id,
                )
            }
        }
        Type::KeyOf { target_type } => {
            let mapped_target_type = rewriter.rewrite_type_id(types, *target_type);
            if mapped_target_type == *target_type {
                type_id
            } else {
                types.insert_type_from_type(
                    Type::KeyOf {
                        target_type: mapped_target_type,
                    },
                    type_id,
                )
            }
        }
        Type::Must { target_type } => {
            let mapped_target_type = rewriter.rewrite_type_id(types, *target_type);
            if mapped_target_type == *target_type {
                type_id
            } else {
                types.insert_type_from_type(
                    Type::Must {
                        target_type: mapped_target_type,
                    },
                    type_id,
                )
            }
        }
        Type::AsComptime { target_type } => {
            let mapped_target_type = rewriter.rewrite_type_id(types, *target_type);
            if mapped_target_type == *target_type {
                type_id
            } else {
                types.insert_type_from_type(
                    Type::AsComptime {
                        target_type: mapped_target_type,
                    },
                    type_id,
                )
            }
        }
        Type::Not { target_type } => {
            let mapped_target_type = rewriter.rewrite_type_id(types, *target_type);
            if mapped_target_type == *target_type {
                type_id
            } else {
                types.insert_type_from_type(
                    Type::Not {
                        target_type: mapped_target_type,
                    },
                    type_id,
                )
            }
        }
        Type::ValueOf {
            mutability,
            variance,
            right,
        } => {
            let mapped_right = rewriter.rewrite_type_id(types, *right);
            if mapped_right == *right {
                type_id
            } else {
                types.insert_type_from_type(
                    Type::ValueOf {
                        mutability: *mutability,
                        variance: *variance,
                        right: mapped_right,
                    },
                    type_id,
                )
            }
        }
        Type::ReferenceOf {
            mutability,
            variance,
            right,
        } => {
            let mapped_right = rewriter.rewrite_type_id(types, *right);
            if mapped_right == *right {
                type_id
            } else {
                types.insert_type_from_type(
                    Type::ReferenceOf {
                        mutability: *mutability,
                        variance: *variance,
                        right: mapped_right,
                    },
                    type_id,
                )
            }
        }
        Type::PointerOf { mutability, right } => {
            let mapped_right = rewriter.rewrite_type_id(types, *right);
            if mapped_right == *right {
                type_id
            } else {
                types.insert_type_from_type(
                    Type::PointerOf {
                        mutability: *mutability,
                        right: mapped_right,
                    },
                    type_id,
                )
            }
        }
        Type::In { left, right } => {
            let mapped_left = rewriter.rewrite_type_id(types, *left);
            let mapped_right = rewriter.rewrite_type_id(types, *right);
            if mapped_left == *left && mapped_right == *right {
                type_id
            } else {
                types.insert_type_from_type(
                    Type::In {
                        left: mapped_left,
                        right: mapped_right,
                    },
                    type_id,
                )
            }
        }
        Type::Extends { left, right } => {
            let mapped_left = rewriter.rewrite_type_id(types, *left);
            let mapped_right = rewriter.rewrite_type_id(types, *right);
            if mapped_left == *left && mapped_right == *right {
                type_id
            } else {
                types.insert_type_from_type(
                    Type::Extends {
                        left: mapped_left,
                        right: mapped_right,
                    },
                    type_id,
                )
            }
        }
        Type::Implements { left, right } => {
            let mapped_left = rewriter.rewrite_type_id(types, *left);
            let mapped_right = rewriter.rewrite_type_id(types, *right);
            if mapped_left == *left && mapped_right == *right {
                type_id
            } else {
                types.insert_type_from_type(
                    Type::Implements {
                        left: mapped_left,
                        right: mapped_right,
                    },
                    type_id,
                )
            }
        }
        Type::ArraySized {
            element,
            count,
            is_readonly,
        } => {
            let mapped_element = rewriter.rewrite_type_id(types, *element);
            let mapped_count = rewriter.rewrite_type_id(types, *count);
            if mapped_element == *element && mapped_count == *count {
                type_id
            } else {
                types.insert_type_from_type(
                    Type::ArraySized {
                        element: mapped_element,
                        count: mapped_count,
                        is_readonly: *is_readonly,
                    },
                    type_id,
                )
            }
        }
        Type::Array {
            element,
            is_readonly,
        } => {
            let (mapped_element, changed) = rewrite_type_id_option(rewriter, types, *element);
            if !changed {
                type_id
            } else {
                types.insert_type_from_type(
                    Type::Array {
                        element: mapped_element,
                        is_readonly: *is_readonly,
                    },
                    type_id,
                )
            }
        }
        Type::Tuple {
            elements,
            is_readonly,
        } => {
            let (mapped_elements, changed) = rewrite_type_elements(rewriter, types, elements);
            if !changed {
                type_id
            } else {
                types.insert_type_from_type(
                    Type::Tuple {
                        elements: mapped_elements,
                        is_readonly: *is_readonly,
                    },
                    type_id,
                )
            }
        }
        Type::Object {
            fields,
            call_signatures,
            construct_signatures,
            index_signatures,
        } => {
            let (mapped_fields, fields_changed) = rewrite_type_fields(rewriter, types, fields);
            let (mapped_calls, calls_changed) = rewrite_type_ids(rewriter, types, call_signatures);
            let (mapped_constructs, constructs_changed) =
                rewrite_type_ids(rewriter, types, construct_signatures);
            let (mapped_indexes, indexes_changed) =
                rewrite_type_index_signatures(rewriter, types, index_signatures);
            if !fields_changed && !calls_changed && !constructs_changed && !indexes_changed {
                type_id
            } else {
                types.insert_type_from_type(
                    Type::Object {
                        fields: mapped_fields,
                        call_signatures: mapped_calls,
                        construct_signatures: mapped_constructs,
                        index_signatures: mapped_indexes,
                    },
                    type_id,
                )
            }
        }
        Type::Function {
            asynchrony,
            cardinality,
            generic_parameters,
            this_parameter,
            parameters,
            return_type,
        } => {
            let (mapped_static, static_changed) =
                rewrite_type_ids(rewriter, types, generic_parameters);
            let (mapped_this, this_changed) =
                rewrite_type_id_option(rewriter, types, *this_parameter);
            let (mapped_parameters, parameters_changed) =
                rewrite_type_ids(rewriter, types, parameters);
            let (mapped_return, return_changed) =
                rewrite_type_id_option(rewriter, types, *return_type);
            if !static_changed && !this_changed && !parameters_changed && !return_changed {
                type_id
            } else {
                types.insert_type_from_type(
                    Type::Function {
                        asynchrony: *asynchrony,
                        cardinality: *cardinality,
                        generic_parameters: mapped_static,
                        this_parameter: mapped_this,
                        parameters: mapped_parameters,
                        return_type: mapped_return,
                    },
                    type_id,
                )
            }
        }
        Type::Union { elements } => {
            let (mapped_elements, changed) = rewrite_type_ids(rewriter, types, elements);
            if !changed {
                type_id
            } else {
                types.insert_type_from_type(
                    Type::Union {
                        elements: mapped_elements,
                    },
                    type_id,
                )
            }
        }
        Type::Intersection { elements } => {
            let (mapped_elements, changed) = rewrite_type_ids(rewriter, types, elements);
            if !changed {
                type_id
            } else {
                types.insert_type_from_type(
                    Type::Intersection {
                        elements: mapped_elements,
                    },
                    type_id,
                )
            }
        }
    }
}

/// Rewrite a static argument using the rewriter callbacks.
pub fn rewrite_static_argument<V: TypeRewriter + ?Sized>(
    rewriter: &mut V,
    types: &mut TypeTable,
    argument: &StaticArgument,
) -> StaticArgument {
    let (mapped, _) = rewrite_static_argument_inner(rewriter, types, argument);
    mapped
}

/// Rewrite a static expression using the rewriter callbacks.
pub fn rewrite_static_expression<V: TypeRewriter + ?Sized>(
    rewriter: &mut V,
    types: &mut TypeTable,
    expression: &StaticExpression,
) -> StaticExpression {
    let (mapped, _) = rewrite_static_expression_inner(rewriter, types, expression);
    mapped
}

/// Rewrite a static property using the rewriter callbacks.
pub fn rewrite_static_property<V: TypeRewriter + ?Sized>(
    rewriter: &mut V,
    types: &mut TypeTable,
    property: &StaticProperty,
) -> StaticProperty {
    let (mapped, _) = rewrite_static_property_inner(rewriter, types, property);
    mapped
}

/// Rewrite a list of type ids.
fn rewrite_type_ids<V: TypeRewriter + ?Sized>(
    rewriter: &mut V,
    types: &mut TypeTable,
    elements: &[LocalTypeId],
) -> (Vec<LocalTypeId>, bool) {
    let mut changed = false;
    let mut mapped = Vec::with_capacity(elements.len());
    for element in elements {
        let mapped_element = rewriter.rewrite_type_id(types, *element);
        if mapped_element != *element {
            changed = true;
        }
        mapped.push(mapped_element);
    }
    (mapped, changed)
}

/// Rewrite an optional type id.
fn rewrite_type_id_option<V: TypeRewriter + ?Sized>(
    rewriter: &mut V,
    types: &mut TypeTable,
    value: Option<LocalTypeId>,
) -> (Option<LocalTypeId>, bool) {
    let Some(value) = value else {
        return (None, false);
    };
    let mapped = rewriter.rewrite_type_id(types, value);
    if mapped == value {
        (Some(value), false)
    } else {
        (Some(mapped), true)
    }
}

/// Rewrite a mapped type parameter.
fn rewrite_type_mapped_parameter<V: TypeRewriter + ?Sized>(
    rewriter: &mut V,
    types: &mut TypeTable,
    parameter: &MappedTypeParameter,
) -> (MappedTypeParameter, bool) {
    let mapped_constraint = rewriter.rewrite_type_id(types, parameter.constraint);
    let (mapped_remap, remap_changed) =
        rewrite_type_id_option(rewriter, types, parameter.key_remap);
    let changed = mapped_constraint != parameter.constraint || remap_changed;
    if !changed {
        (parameter.clone(), false)
    } else {
        (
            MappedTypeParameter {
                name: parameter.name,
                symbol: parameter.symbol,
                constraint: mapped_constraint,
                key_remap: mapped_remap,
            },
            true,
        )
    }
}

/// Rewrite a list of tuple elements.
fn rewrite_type_elements<V: TypeRewriter + ?Sized>(
    rewriter: &mut V,
    types: &mut TypeTable,
    elements: &[TypeElement],
) -> (Vec<TypeElement>, bool) {
    let mut changed = false;
    let mut mapped = Vec::with_capacity(elements.len());
    for element in elements {
        let (mapped_element, element_changed) = rewrite_type_element(rewriter, types, element);
        if element_changed {
            changed = true;
        }
        mapped.push(mapped_element);
    }
    (mapped, changed)
}

/// Rewrite a tuple element.
fn rewrite_type_element<V: TypeRewriter + ?Sized>(
    rewriter: &mut V,
    types: &mut TypeTable,
    element: &TypeElement,
) -> (TypeElement, bool) {
    let mapped_ty = rewriter.rewrite_type_id(types, element.ty);
    if mapped_ty == element.ty {
        (element.clone(), false)
    } else {
        let mut mapped = element.clone();
        mapped.ty = mapped_ty;
        (mapped, true)
    }
}

/// Rewrite a list of fields.
fn rewrite_type_fields<V: TypeRewriter + ?Sized>(
    rewriter: &mut V,
    types: &mut TypeTable,
    fields: &[TypeField],
) -> (Vec<TypeField>, bool) {
    let mut changed = false;
    let mut mapped = Vec::with_capacity(fields.len());
    for field in fields {
        let (mapped_field, field_changed) = rewrite_type_field(rewriter, types, field);
        if field_changed {
            changed = true;
        }
        mapped.push(mapped_field);
    }
    (mapped, changed)
}

/// Rewrite a field.
fn rewrite_type_field<V: TypeRewriter + ?Sized>(
    rewriter: &mut V,
    types: &mut TypeTable,
    field: &TypeField,
) -> (TypeField, bool) {
    let mapped_ty = rewriter.rewrite_type_id(types, field.ty);
    if mapped_ty == field.ty {
        (field.clone(), false)
    } else {
        let mut mapped = field.clone();
        mapped.ty = mapped_ty;
        (mapped, true)
    }
}

/// Rewrite a list of index signatures.
fn rewrite_type_index_signatures<V: TypeRewriter + ?Sized>(
    rewriter: &mut V,
    types: &mut TypeTable,
    signatures: &[TypeIndexSignature],
) -> (Vec<TypeIndexSignature>, bool) {
    let mut changed = false;
    let mut mapped = Vec::with_capacity(signatures.len());
    for signature in signatures {
        let (mapped_signature, signature_changed) =
            rewrite_type_index_signature(rewriter, types, signature);
        if signature_changed {
            changed = true;
        }
        mapped.push(mapped_signature);
    }
    (mapped, changed)
}

/// Rewrite an index signature.
fn rewrite_type_index_signature<V: TypeRewriter + ?Sized>(
    rewriter: &mut V,
    types: &mut TypeTable,
    signature: &TypeIndexSignature,
) -> (TypeIndexSignature, bool) {
    let mapped_key = rewriter.rewrite_type_id(types, signature.key_type);
    let mapped_value = rewriter.rewrite_type_id(types, signature.value_type);
    if mapped_key == signature.key_type && mapped_value == signature.value_type {
        (signature.clone(), false)
    } else {
        (
            TypeIndexSignature {
                name: signature.name,
                key_type: mapped_key,
                value_type: mapped_value,
                is_optional: signature.is_optional,
                is_readonly: signature.is_readonly,
            },
            true,
        )
    }
}

/// Rewrite a list of static arguments.
fn rewrite_static_arguments<V: TypeRewriter + ?Sized>(
    rewriter: &mut V,
    types: &mut TypeTable,
    arguments: &[StaticArgument],
) -> (Vec<StaticArgument>, bool) {
    let mut changed = false;
    let mut mapped = Vec::with_capacity(arguments.len());
    for argument in arguments {
        let (mapped_argument, argument_changed) =
            rewrite_static_argument_inner(rewriter, types, argument);
        if argument_changed {
            changed = true;
        }
        mapped.push(mapped_argument);
    }
    (mapped, changed)
}

/// Rewrite a static argument and report changes.
fn rewrite_static_argument_inner<V: TypeRewriter + ?Sized>(
    rewriter: &mut V,
    types: &mut TypeTable,
    argument: &StaticArgument,
) -> (StaticArgument, bool) {
    match argument {
        StaticArgument::Unevaluated { .. } => (argument.clone(), false),
        StaticArgument::Evaluated { name, value } => {
            let (mapped_value, changed) = rewrite_static_expression_inner(rewriter, types, value);
            if !changed {
                (argument.clone(), false)
            } else {
                (
                    StaticArgument::Evaluated {
                        name: *name,
                        value: mapped_value,
                    },
                    true,
                )
            }
        }
    }
}

/// Rewrite a static expression and report changes.
fn rewrite_static_expression_inner<V: TypeRewriter + ?Sized>(
    rewriter: &mut V,
    types: &mut TypeTable,
    expression: &StaticExpression,
) -> (StaticExpression, bool) {
    match expression {
        StaticExpression::Unevaluated { .. }
        | StaticExpression::ScalarLiteral { .. }
        | StaticExpression::TypeLiteral { .. } => (expression.clone(), false),
        StaticExpression::Declaration {
            declaration,
            generic_arguments,
        } => {
            let Some(generic_arguments) = generic_arguments.as_ref() else {
                return (expression.clone(), false);
            };
            let (mapped_arguments, changed) =
                rewrite_static_arguments(rewriter, types, generic_arguments);
            if !changed {
                (expression.clone(), false)
            } else {
                (
                    StaticExpression::Declaration {
                        declaration: *declaration,
                        generic_arguments: Some(mapped_arguments),
                    },
                    true,
                )
            }
        }
        StaticExpression::Type { ty } => {
            let mapped_ty = rewriter.rewrite_type_id(types, *ty);
            if mapped_ty == *ty {
                (expression.clone(), false)
            } else {
                (StaticExpression::Type { ty: mapped_ty }, true)
            }
        }
        StaticExpression::ArrayExpression { elements } => {
            let (mapped_elements, changed) = rewrite_static_expressions(rewriter, types, elements);
            if !changed {
                (expression.clone(), false)
            } else {
                (
                    StaticExpression::ArrayExpression {
                        elements: mapped_elements,
                    },
                    true,
                )
            }
        }
        StaticExpression::TupleExpression { elements } => {
            let (mapped_elements, changed) = rewrite_static_expressions(rewriter, types, elements);
            if !changed {
                (expression.clone(), false)
            } else {
                (
                    StaticExpression::TupleExpression {
                        elements: mapped_elements,
                    },
                    true,
                )
            }
        }
        StaticExpression::ObjectExpression { properties } => {
            let (mapped_properties, changed) =
                rewrite_static_properties(rewriter, types, properties);
            if !changed {
                (expression.clone(), false)
            } else {
                (
                    StaticExpression::ObjectExpression {
                        properties: mapped_properties,
                    },
                    true,
                )
            }
        }
    }
}

/// Rewrite a list of static expressions.
fn rewrite_static_expressions<V: TypeRewriter + ?Sized>(
    rewriter: &mut V,
    types: &mut TypeTable,
    expressions: &[StaticExpression],
) -> (Vec<StaticExpression>, bool) {
    let mut changed = false;
    let mut mapped = Vec::with_capacity(expressions.len());
    for expression in expressions {
        let (mapped_expression, expression_changed) =
            rewrite_static_expression_inner(rewriter, types, expression);
        if expression_changed {
            changed = true;
        }
        mapped.push(mapped_expression);
    }
    (mapped, changed)
}

/// Rewrite a list of static properties.
fn rewrite_static_properties<V: TypeRewriter + ?Sized>(
    rewriter: &mut V,
    types: &mut TypeTable,
    properties: &[StaticProperty],
) -> (Vec<StaticProperty>, bool) {
    let mut changed = false;
    let mut mapped = Vec::with_capacity(properties.len());
    for property in properties {
        let (mapped_property, property_changed) =
            rewrite_static_property_inner(rewriter, types, property);
        if property_changed {
            changed = true;
        }
        mapped.push(mapped_property);
    }
    (mapped, changed)
}

/// Rewrite a static property and report changes.
fn rewrite_static_property_inner<V: TypeRewriter + ?Sized>(
    rewriter: &mut V,
    types: &mut TypeTable,
    property: &StaticProperty,
) -> (StaticProperty, bool) {
    match property {
        StaticProperty::Unevaluated { .. } => (property.clone(), false),
        StaticProperty::Field { key, value, symbol } => {
            let (mapped_value, value_changed) =
                rewrite_static_expression_inner(rewriter, types, value);
            if !value_changed {
                (property.clone(), false)
            } else {
                (
                    StaticProperty::Field {
                        key: *key,
                        value: mapped_value,
                        symbol: *symbol,
                    },
                    true,
                )
            }
        }
        StaticProperty::Method {
            key,
            signature,
            body,
            symbol,
        } => {
            let (mapped_body, body_changed) =
                rewrite_static_expression_inner(rewriter, types, body);
            if !body_changed {
                (property.clone(), false)
            } else {
                (
                    StaticProperty::Method {
                        key: *key,
                        signature: signature.clone(),
                        body: mapped_body,
                        symbol: *symbol,
                    },
                    true,
                )
            }
        }
        StaticProperty::Spread { value, symbol } => {
            let (mapped_value, value_changed) =
                rewrite_static_expression_inner(rewriter, types, value);
            if !value_changed {
                (property.clone(), false)
            } else {
                (
                    StaticProperty::Spread {
                        value: mapped_value,
                        symbol: *symbol,
                    },
                    true,
                )
            }
        }
    }
}
