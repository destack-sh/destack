//! destack.core.definition.option

#![destack::partial(destack.core.definition.option, file)]

use crate::EnumType;

#[destack::generated(OptionDefinition, -, block)]
/// Definition of a builtin Enum Option.
pub struct OptionDefinition {
    pub id: u8,
    pub r#type: EnumType,
    pub name: String,
    pub description: String,
    pub taggings: Vec<u8>,
    pub is_internal: bool,
}
