//! destack.core.definition.option@2025.08.15.1

#![destack::partial(destack.core.definition.option, file)]

use crate::EnumType;

#[destack::generated(OptionDefinition, struct, block)]
/// Definition of a builtin Enum Option.
pub struct OptionDefinition {
    id: u8,
    r#type: EnumType,
    name: String,
    description: String,
    taggings: Vec<u8>,
    is_internal: bool,
}
