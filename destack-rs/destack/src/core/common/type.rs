//! destack.core.common.type@2025.08.15.1

#![destack::partial(destack.core.common.type, file)]

use crate::EnumType;
use crate::HandleType;
use crate::NodeType;
use crate::PrimitiveType;
use crate::ScalarType;
use crate::StructType;
use crate::TypeCardinality;

#[destack::generated(Type, , block)]
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
    key_type: Box<Option<Type>>,
    value_type: Box<Option<Type>>,
    element_types: Option<Vec<Type>>,
    length: Option<u32>,
    dimensions: Option<Vec<u32>>,
    is_required: bool,
    scalar_type: Option<ScalarType>,
    primitive_type: Option<PrimitiveType>,
    enum_type: Option<EnumType>,
    node_types: Option<Vec<NodeType>>,
    struct_type: Option<StructType>,
    handle_type: Option<HandleType>,
}

#[destack::generated(NumberConstraint, , block)]
/// The constraint of a number.
pub struct NumberConstraint {
    min_value: Option<f32>,
    max_value: Option<f32>,
    step_value: Option<f32>,
}

#[destack::generated(StringConstraint, , block)]
/// The constraint of a string.
pub struct StringConstraint {
    regex: Option<String>,
    starts_with: Option<String>,
    ends_with: Option<String>,
}

#[destack::generated(CollectionConstraint, , block)]
/// The constraint of a collection.
pub struct CollectionConstraint {
    min_length: Option<u32>,
    max_length: Option<u32>,
}
