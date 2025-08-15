//! destack.core.definition.schema@2025.08.15.1

#![destack::partial(destack.core.definition.schema, file)]

use crate::{EnumDefinition, NodeDefinition, StructDefinition, HandleDefinition, ModuleDefinition};

#[destack::generated(SchemaDefinition, struct, block)]
/// Definition of the entire Destack Schema ("language definition").
pub struct SchemaDefinition {
    name: String,
    description: String,
    version: String,
    modules: Vec<ModuleDefinition>,
    nodes: Vec<NodeDefinition>,
    structs: Vec<StructDefinition>,
    handles: Vec<HandleDefinition>,
    enums: Vec<EnumDefinition>
}