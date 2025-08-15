//! destack.core.common.type@2025.08.15.1

#![destack::partial(destack.core.common.type, file)]

use crate::{
    EnumType, HandleType, NodeType, PrimitiveType, ScalarType, StructType, TypeCardinality,
};

#[destack::generated(Type, struct, block)]
/// A Type in the type system.
/// Types compose like a tree with scalars at the leaves:
/// - Scalar: a single value (self, Type.scalar_type)
/// - List: a dynamic sequence of homogeneous values (Type.value_type)
/// - Tuple: a fixed sequence of heterogeneous values (Type.element_types)
/// - Array: a fixed n-dimensional sequence of homogeneous values (Type.value_type * Type.dimensions)
/// - Map: a dynamic mapping of homogenous keys to homogeneous values (Type.key_type->Type.value_type)
/// - Union: a union of heterogeneous values (Type.element_types)
pub struct Type {
    cardinality: TypeCardinality,
    key_type: Type,
    value_type: Type,
    element_types: Vec<Type>,
    length: u32,
    dimensions: Vec<u32>,
    is_required: bool,
    scalar_type: ScalarType,
    primitive_type: PrimitiveType,
    enum_type: EnumType,
    node_types: Vec<NodeType>,
    struct_type: StructType,
    handle_type: HandleType,
}

#[destack::generated(NumberConstraint, struct, block)]
/// The constraint of a number.
pub struct NumberConstraint {
    min_value: f32,
    max_value: f32,
    step_value: f32,
}

#[destack::generated(StringConstraint, struct, block)]
/// The constraint of a string.
pub struct StringConstraint {
    regex: String,
    starts_with: String,
    ends_with: String,
}

#[destack::generated(CollectionConstraint, struct, block)]
/// The constraint of a collection.
pub struct CollectionConstraint {
    min_length: u32,
    max_length: u32,
}
