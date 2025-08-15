//! destack.core.definition.object@2025.08.15.1

#![destack::partial(destack.core.definition.object, file)]

#[destack::generated(ObjectDefinition, , block)]
/// Definition of a builtin Trait, Node or Struct.
pub struct ObjectDefinition {
    id: u32,
    name: String,
    description: String,
}
