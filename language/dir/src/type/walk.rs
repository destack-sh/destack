use crate::{
    LocalTypeId, MappedTypeParameter, StaticArgument, StaticExpression, StaticProperty, Type,
    TypeElement, TypeField, TypeIndexSignature, TypeTable, TypeVisitor,
};

/// Walk a type id.
pub fn walk_type_id<V: TypeVisitor + ?Sized>(
    visitor: &mut V,
    types: &TypeTable,
    type_id: LocalTypeId,
) {
    let ty = types.get_type(type_id);
    visitor.visit_type(types, type_id, ty);
}

/// Walk a type.
pub fn walk_type<V: TypeVisitor + ?Sized>(
    visitor: &mut V,
    types: &TypeTable,
    type_id: LocalTypeId,
    ty: &Type,
) {
    visitor.visit_any(types, type_id, ty);
    match ty {
        Type::TypeLiteral { .. } => {}
        Type::InferVar { .. } => {}
        Type::Value { value } => {
            visitor.visit_type_id(types, *value);
        }
        Type::This => {}
        Type::Reference {
            static_arguments, ..
        } => {
            if let Some(arguments) = static_arguments.as_ref() {
                walk_static_arguments(visitor, types, arguments);
            }
        }
        Type::Unevaluated(_) => {}
        Type::Conditional {
            left,
            right,
            then_type,
            else_type,
            ..
        } => {
            visitor.visit_type_id(types, *left);
            visitor.visit_type_id(types, *right);
            visitor.visit_type_id(types, *then_type);
            visitor.visit_type_id(types, *else_type);
        }
        Type::Mapped {
            parameter, value, ..
        } => {
            walk_type_mapped_parameter(visitor, types, parameter);
            visitor.visit_type_id(types, *value);
        }
        Type::Index { left, index } => {
            visitor.visit_type_id(types, *left);
            visitor.visit_type_id(types, *index);
        }
        Type::TemplateLiteral { spans, .. } => {
            for span in spans {
                visitor.visit_type_id(types, *span);
            }
        }
        Type::Import {
            static_arguments, ..
        } => {
            if let Some(arguments) = static_arguments.as_ref() {
                walk_static_arguments(visitor, types, arguments);
            }
        }
        Type::Infer { constraint, .. } => {
            if let Some(constraint) = constraint.as_ref() {
                visitor.visit_type_id(types, *constraint);
            }
        }
        Type::Predicate { target, .. } => {
            if let Some(target) = target.as_ref() {
                visitor.visit_type_id(types, *target);
            }
        }
        Type::Unary { right, .. } => {
            visitor.visit_type_id(types, *right);
        }
        Type::ValueOf { right, .. } => {
            visitor.visit_type_id(types, *right);
        }
        Type::ReferenceOf { right, .. } => {
            visitor.visit_type_id(types, *right);
        }
        Type::PointerOf { right, .. } => {
            visitor.visit_type_id(types, *right);
        }
        Type::Binary { left, right, .. } => {
            visitor.visit_type_id(types, *left);
            visitor.visit_type_id(types, *right);
        }
        Type::ArraySized { element, count, .. } => {
            visitor.visit_type_id(types, *element);
            visitor.visit_type_id(types, *count);
        }
        Type::Array { element, .. } => {
            if let Some(element) = element.as_ref() {
                visitor.visit_type_id(types, *element);
            }
        }
        Type::Tuple { elements, .. } => {
            for element in elements {
                walk_type_element(visitor, types, element);
            }
        }
        Type::Object {
            fields,
            call_signatures,
            construct_signatures,
            index_signatures,
        } => {
            for field in fields {
                walk_type_field(visitor, types, field);
            }
            for signature in call_signatures {
                visitor.visit_type_id(types, *signature);
            }
            for signature in construct_signatures {
                visitor.visit_type_id(types, *signature);
            }
            for signature in index_signatures {
                walk_type_index_signature(visitor, types, signature);
            }
        }
        Type::Function {
            static_parameters,
            this_parameter,
            dynamic_parameters,
            return_type,
            ..
        } => {
            for parameter in static_parameters {
                visitor.visit_type_id(types, *parameter);
            }
            if let Some(this_parameter) = this_parameter.as_ref() {
                visitor.visit_type_id(types, *this_parameter);
            }
            for parameter in dynamic_parameters {
                visitor.visit_type_id(types, *parameter);
            }
            if let Some(return_type) = return_type.as_ref() {
                visitor.visit_type_id(types, *return_type);
            }
        }
        Type::Union { elements } => {
            for element in elements {
                visitor.visit_type_id(types, *element);
            }
        }
        Type::Intersection { elements } => {
            for element in elements {
                visitor.visit_type_id(types, *element);
            }
        }
        Type::Error => {}
    }
}

/// Walk static arguments.
fn walk_static_arguments<V: TypeVisitor + ?Sized>(
    visitor: &mut V,
    types: &TypeTable,
    arguments: &[StaticArgument],
) {
    for argument in arguments {
        visitor.visit_static_argument(types, argument);
    }
}

/// Walk a static argument.
pub fn walk_static_argument<V: TypeVisitor + ?Sized>(
    visitor: &mut V,
    types: &TypeTable,
    argument: &StaticArgument,
) {
    match argument {
        StaticArgument::Unevaluated { .. } => {}
        StaticArgument::Evaluated { value, .. } => {
            visitor.visit_static_expression(types, value);
        }
    }
}

/// Walk a static expression.
pub fn walk_static_expression<V: TypeVisitor + ?Sized>(
    visitor: &mut V,
    types: &TypeTable,
    expression: &StaticExpression,
) {
    match expression {
        StaticExpression::Unevaluated { .. } => {}
        StaticExpression::ScalarLiteral { .. } => {}
        StaticExpression::TypeLiteral { .. } => {}
        StaticExpression::Declaration {
            static_arguments, ..
        } => {
            if let Some(arguments) = static_arguments.as_ref() {
                walk_static_arguments(visitor, types, arguments);
            }
        }
        StaticExpression::Type { ty } => {
            visitor.visit_type_id(types, *ty);
        }
        StaticExpression::ArrayExpression { elements } => {
            for element in elements {
                visitor.visit_static_expression(types, element);
            }
        }
        StaticExpression::TupleExpression { elements } => {
            for element in elements {
                visitor.visit_static_expression(types, element);
            }
        }
        StaticExpression::ObjectExpression { properties } => {
            for property in properties {
                visitor.visit_static_property(types, property);
            }
        }
    }
}

/// Walk a static property.
pub fn walk_static_property<V: TypeVisitor + ?Sized>(
    visitor: &mut V,
    types: &TypeTable,
    property: &StaticProperty,
) {
    match property {
        StaticProperty::Unevaluated { .. } => {}
        StaticProperty::Field { value, default, .. } => {
            visitor.visit_static_expression(types, value);
            if let Some(default) = default.as_ref() {
                visitor.visit_static_expression(types, default);
            }
        }
        StaticProperty::Method { body, .. } => {
            visitor.visit_static_expression(types, body);
        }
    }
}

/// Walk a mapped type parameter.
fn walk_type_mapped_parameter<V: TypeVisitor + ?Sized>(
    visitor: &mut V,
    types: &TypeTable,
    parameter: &MappedTypeParameter,
) {
    visitor.visit_type_id(types, parameter.constraint);
    if let Some(remap) = parameter.key_remap.as_ref() {
        visitor.visit_type_id(types, *remap);
    }
}

/// Walk a type element.
fn walk_type_element<V: TypeVisitor + ?Sized>(
    visitor: &mut V,
    types: &TypeTable,
    element: &TypeElement,
) {
    visitor.visit_type_id(types, element.ty);
}

/// Walk a type field.
fn walk_type_field<V: TypeVisitor + ?Sized>(visitor: &mut V, types: &TypeTable, field: &TypeField) {
    visitor.visit_type_id(types, field.ty);
}

/// Walk a type index signature.
fn walk_type_index_signature<V: TypeVisitor + ?Sized>(
    visitor: &mut V,
    types: &TypeTable,
    signature: &TypeIndexSignature,
) {
    visitor.visit_type_id(types, signature.key_type);
    visitor.visit_type_id(types, signature.value_type);
}
