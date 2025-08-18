//! destack.core.common.type

#![destack::partial(destack.core.common.type, file)]

use crate::{
    EnumType, HandleType, NodeType, PrimitiveType, ScalarType, StructType, TypeCardinality,
};

#[destack::generated(Type, -, block)]
/// A Type in the type system.
/// Types compose like a tree with scalars at the leaves:
/// - Scalar: a single value (self, Type.scalar_type)
/// - List: a dynamic sequence of homogeneous values (Type.value_type)
/// - Tuple: a fixed sequence of heterogeneous values (Type.element_types)
/// - Array: a fixed n-dimensional sequence of homogeneous values (Type.value_type * Type.dimensions)
/// - Map: a dynamic mapping of homogenous keys to homogeneous values (Type.key_type->Type.value_type)
/// - Union: a union of heterogeneous values (Type.element_types)
pub struct Type {
    pub cardinality: TypeCardinality,
    pub key_type: Box<Option<Type>>,
    pub value_type: Box<Option<Type>>,
    pub element_types: Option<Vec<Type>>,
    pub length: Option<u32>,
    pub dimensions: Option<Vec<u32>>,
    pub is_required: bool,
    pub scalar_type: Option<ScalarType>,
    pub primitive_type: Option<PrimitiveType>,
    pub enum_type: Option<EnumType>,
    pub node_types: Option<Vec<NodeType>>,
    pub struct_type: Option<StructType>,
    pub handle_type: Option<HandleType>,
}

#[destack::generated(NumberConstraint, -, block)]
/// The constraint of a number.
pub struct NumberConstraint {
    pub min_value: Option<f32>,
    pub max_value: Option<f32>,
    pub step_value: Option<f32>,
}

#[destack::generated(StringConstraint, -, block)]
/// The constraint of a string.
pub struct StringConstraint {
    pub regex: Option<String>,
    pub starts_with: Option<String>,
    pub ends_with: Option<String>,
}

#[destack::generated(CollectionConstraint, -, block)]
/// The constraint of a collection.
pub struct CollectionConstraint {
    pub min_length: Option<u32>,
    pub max_length: Option<u32>,
}
