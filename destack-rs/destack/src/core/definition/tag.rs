//! destack.core.definition.tag@2025.08.15.1

#![destack::partial(destack.core.definition.tag, file)]

#[destack::generated(TagDefinition, struct, block)]
/// Definition of a builtin Tag to associate builtin definitions to.
pub struct TagDefinition {
    id: u8,
    name: String,
    description: String,
    is_internal: bool,
}
