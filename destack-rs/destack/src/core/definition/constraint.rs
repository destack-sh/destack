//! destack.core.definition.constraint@2025.08.15.1

#![destack::partial(destack.core.definition.constraint, file)]

use crate::ConstraintType;
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
