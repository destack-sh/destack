//! destack.basics.entity.index@2025.08.15.1

#![destack::partial(destack.basics.entity.index, file)]

use crate::IndexType;
use crate::PropertyReference;

#[destack::generated(IndexDefinition, , block)]
/// Definition of a builtin Index.
pub struct IndexDefinition {
    id: u8,
    r#type: IndexType,
    name: String,
    description: String,
    properties: Vec<PropertyReference>,
    cover: Vec<PropertyReference>,
}
