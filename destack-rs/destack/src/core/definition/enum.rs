//! destack.core.definition.enum@2025.08.15.1

#![destack::partial(destack.core.definition.enum, file)]

use crate::{OptionDefinition, EnumType};

#[destack::generated(EnumDefinition, struct, block)]
/// Definition of a builtin Enum.
pub struct EnumDefinition {
    id: u32,
    r#type: EnumType,
    name: String,
    description: String,
    taggings: Vec<u8>,
    is_flag: bool,
    options: Vec<OptionDefinition>
}