//! destack.core.common.type@2025.08.15.1

#![destack::partial(destack.core.common.type, file)]

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

}

#[destack::generated(NumberConstraint, struct, block)]
/// The constraint of a number.
pub struct NumberConstraint {

}

#[destack::generated(StringConstraint, struct, block)]
/// The constraint of a string.
pub struct StringConstraint {

}

#[destack::generated(CollectionConstraint, struct, block)]
/// The constraint of a collection.
pub struct CollectionConstraint {

}