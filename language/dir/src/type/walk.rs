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
        Type::Literal(_) => {}
        Type::InferVariable(_) => {}
        Type::Value(value) => {
            visitor.visit_type_id(types, value.value);
        }
        Type::This => {}
        Type::Reference(reference) => {
            if let Some(arguments) = reference.generic_arguments.as_ref() {
                walk_static_arguments(visitor, types, arguments);
            }
        }
        Type::Unevaluated(_) => {}
        Type::Conditional(conditional) => {
            visitor.visit_type_id(types, conditional.left);
            visitor.visit_type_id(types, conditional.right);
            visitor.visit_type_id(types, conditional.then_type);
            visitor.visit_type_id(types, conditional.else_type);
        }
        Type::Mapped(mapped) => {
            walk_type_mapped_parameter(visitor, types, &mapped.parameter);
            visitor.visit_type_id(types, mapped.value);
        }
        Type::Index(index) => {
            visitor.visit_type_id(types, index.left);
            visitor.visit_type_id(types, index.index);
        }
        Type::TemplateLiteral(template) => {
            for span in &template.spans {
                visitor.visit_type_id(types, *span);
            }
        }
        Type::Import(import) => {
            if let Some(arguments) = import.generic_arguments.as_ref() {
                walk_static_arguments(visitor, types, arguments);
            }
        }
        Type::Infer(infer) => {
            if let Some(constraint) = infer.constraint.as_ref() {
                visitor.visit_type_id(types, *constraint);
            }
        }
        Type::Predicate(predicate) => {
            if let Some(target) = predicate.target.as_ref() {
                visitor.visit_type_id(types, *target);
            }
        }
        Type::Form(form) => {
            visitor.visit_type_id(types, form.base);
            visitor.visit_type_id(types, form.ownership);
            visitor.visit_type_id(types, form.place);
            visitor.visit_type_id(types, form.lifetime);
            visitor.visit_type_id(types, form.access);
        }
        Type::KeyOf(unary) | Type::Must(unary) | Type::AsComptime(unary) | Type::Not(unary) => {
            visitor.visit_type_id(types, unary.target_type);
        }
        Type::In(binary) | Type::Extends(binary) | Type::Implements(binary) => {
            visitor.visit_type_id(types, binary.left);
            visitor.visit_type_id(types, binary.right);
        }
        Type::FixedArray(array) => {
            visitor.visit_type_id(types, array.element);
            visitor.visit_type_id(types, array.count);
        }
        Type::Slice(slice) => {
            if let Some(element) = slice.element.as_ref() {
                visitor.visit_type_id(types, *element);
            }
        }
        Type::Tuple(tuple) => {
            for element in &tuple.elements {
                walk_type_element(visitor, types, element);
            }
        }
        Type::Object(object) => {
            for field in &object.fields {
                walk_type_field(visitor, types, field);
            }
            for signature in &object.call_signatures {
                visitor.visit_type_id(types, *signature);
            }
            for signature in &object.construct_signatures {
                visitor.visit_type_id(types, *signature);
            }
            for signature in &object.index_signatures {
                walk_type_index_signature(visitor, types, signature);
            }
        }
        Type::Function(function) => {
            for parameter in &function.generic_parameters {
                visitor.visit_type_id(types, *parameter);
            }
            if let Some(this_parameter) = function.this_parameter.as_ref() {
                visitor.visit_type_id(types, *this_parameter);
            }
            for parameter in &function.parameters {
                visitor.visit_type_id(types, *parameter);
            }
            if let Some(return_type) = function.return_type.as_ref() {
                visitor.visit_type_id(types, *return_type);
            }
        }
        Type::Union(union) => {
            for element in &union.elements {
                visitor.visit_type_id(types, *element);
            }
        }
        Type::Intersection(intersection) => {
            for element in &intersection.elements {
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
            generic_arguments, ..
        } => {
            if let Some(arguments) = generic_arguments.as_ref() {
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
        StaticProperty::Field { value, .. } => visitor.visit_static_expression(types, value),
        StaticProperty::Method { body, .. } => {
            visitor.visit_static_expression(types, body);
        }
        StaticProperty::Spread { value, .. } => visitor.visit_static_expression(types, value),
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
