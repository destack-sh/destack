//! destack.core.definition.enum@2025.08.15.1

#![destack::partial(destack.core.definition.enum, file)]

use crate::{EnumType, OptionDefinition};

#[destack::generated(EnumDefinition, -, block)]
/// Definition of a builtin Enum.
pub struct EnumDefinition {
    pub id: u32,
    pub r#type: EnumType,
    pub name: String,
    pub description: String,
    pub taggings: Vec<u8>,
    pub is_flag: bool,
    pub options: Vec<OptionDefinition>,
}
