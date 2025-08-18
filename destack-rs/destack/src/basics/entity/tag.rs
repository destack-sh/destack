//! destack.basics.entity.tag

#![destack::partial(destack.basics.entity.tag, file)]

#[destack::generated(TagDefinition, -, block)]
/// Definition of a builtin Tag to associate builtin definitions to.
pub struct TagDefinition {
    pub id: u8,
    pub name: String,
    pub description: Option<String>,
    pub is_internal: bool,
}
