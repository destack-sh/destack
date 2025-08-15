//! destack.basics.entity.constraint@2025.08.15.1

#![destack::partial(destack.basics.entity.constraint, file)]

use crate::PropertyReference;

#[destack::generated(ConstraintDefinition, , block)]
/// Definition of a builtin Constraint.
pub struct ConstraintDefinition {
    id: u8,
    r#type: ConstraintType,
    name: String,
    description: String,
    properties: Vec<PropertyReference>,
}

#[destack::generated(IndexType, , block)]
/// Type of an Index.
pub enum IndexType {
    Btree = 1,
}

#[destack::generated(ConstraintType, , block)]
/// Type of a Constraint.
pub enum ConstraintType {
    Unique = 1,
}
