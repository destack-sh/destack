//! destack.core.definition.schema

#![destack::partial(destack.core.definition.schema, file)]

use crate::{EnumDefinition, HandleDefinition, ModuleDefinition, NodeDefinition, StructDefinition};

#[destack::generated(SchemaDefinition, -, block)]
/// Definition of the entire Destack Schema ("language definition").
pub struct SchemaDefinition {
    pub name: String,
    pub description: String,
    pub version: String,
    pub modules: Vec<ModuleDefinition>,
    pub nodes: Vec<NodeDefinition>,
    pub structs: Vec<StructDefinition>,
    pub handles: Vec<HandleDefinition>,
    pub enums: Vec<EnumDefinition>,
}
