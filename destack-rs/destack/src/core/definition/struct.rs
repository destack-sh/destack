//! destack.core.definition.struct@2025.08.15.1

#![destack::partial(destack.core.definition.struct, file)]

use crate::{StructType, MethodDefinition, ObjectStability, ConstantDefinition, TagDefinition, NodeType, PropertyDefinition};

#[destack::generated(StructDefinition, struct, block)]
/// Definition of a builtin Struct.
pub struct StructDefinition {
    id: u32,
    r#type: StructType,
    name: String,
    description: String,
    stability: ObjectStability,
    taggings: Vec<u8>,
    is_immutable: bool,
    is_abstract: bool,
    is_interned: bool,
    properties: Vec<PropertyDefinition>,
    methods: Vec<MethodDefinition>,
    constants: Vec<ConstantDefinition>,
    tags: Vec<TagDefinition>,
    base_type: StructType,
    extended_by: Vec<StructType>,
    inherits: Vec<StructType>,
    inherited_by: Vec<StructType>,
    into_node_types: Vec<NodeType>
}

