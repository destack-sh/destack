//! destack.basics.entity.index

#![destack::partial(destack.basics.entity.index, file)]

use crate::{IndexType, PropertyReference};

#[destack::generated(IndexDefinition, -, block)]
/// Definition of a builtin Index.
pub struct IndexDefinition {
    pub id: u8,
    pub r#type: IndexType,
    pub name: String,
    pub description: String,
    pub properties: Vec<PropertyReference>,
    pub cover: Vec<PropertyReference>,
}
