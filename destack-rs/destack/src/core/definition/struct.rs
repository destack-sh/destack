//! destack.core.definition.struct@2025.08.15.1

#![destack::partial(destack.core.definition.struct, file)]

use crate::{
    ConstantDefinition, MethodDefinition, NodeType, ObjectStability, PropertyDefinition,
    StructType, TagDefinition, UniverseCategory, UniverseDomain,
};

#[destack::generated(StructDefinition, -, block)]
/// Definition of a builtin Struct.
pub struct StructDefinition {
    pub id: u32,
    pub r#type: StructType,
    pub name: String,
    pub description: String,
    pub domain: UniverseDomain,
    pub category: UniverseCategory,
    pub stability: ObjectStability,
    pub taggings: Vec<u8>,
    pub is_immutable: bool,
    pub is_abstract: bool,
    pub is_interned: bool,
    pub properties: Vec<PropertyDefinition>,
    pub methods: Vec<MethodDefinition>,
    pub constants: Vec<ConstantDefinition>,
    pub tags: Vec<TagDefinition>,
    pub base_type: Option<StructType>,
    pub extended_by: Vec<StructType>,
    pub inherits: Vec<StructType>,
    pub inherited_by: Vec<StructType>,
    pub into_node_types: Vec<NodeType>,
}
