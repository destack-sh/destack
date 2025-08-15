//! destack.basics.entity.tag@2025.08.15.1

#![destack::partial(destack.basics.entity.tag, file)]

#[destack::generated(TagDefinition, , block)]
/// Definition of a builtin Tag to associate builtin definitions to.
pub struct TagDefinition {
    id: u8,
    name: String,
    description: Option<String>,
    is_internal: bool,
}
