//! destack.core.definition.node

#![destack::partial(destack.core.definition.node, file)]

use crate::{
    ActionDefinition, ConstantDefinition, ConstraintDefinition, EncoderStability, IndexDefinition,
    MethodDefinition, NodeType, PermissionDefinition, PropertyDefinition, StructType,
    TagDefinition, TraitType, UniverseCategory, UniverseDomain,
};

#[destack::generated(NodeDefinition, -, block)]
/// Definition of a builtin Node.
pub struct NodeDefinition {
    pub id: u32,
    pub r#type: NodeType,
    pub name: String,
    pub description: String,
    pub domain: UniverseDomain,
    pub category: UniverseCategory,
    pub stability: EncoderStability,
    pub is_abstract: bool,
    pub is_final: bool,
    pub is_singleton: bool,
    pub properties: Vec<PropertyDefinition>,
    pub indexes: Vec<IndexDefinition>,
    pub constraints: Vec<ConstraintDefinition>,
    pub permissions: Vec<PermissionDefinition>,
    pub methods: Vec<MethodDefinition>,
    pub actions: Vec<ActionDefinition>,
    pub constants: Vec<ConstantDefinition>,
    pub tags: Vec<TagDefinition>,
    pub base_type: Option<NodeType>,
    pub extended_by: Vec<NodeType>,
    pub inherits: Vec<NodeType>,
    pub inherited_by: Vec<NodeType>,
    pub traits: Vec<TraitType>,
    pub self_traits: Vec<TraitType>,
    pub parent_types: Vec<NodeType>,
    pub child_types: Vec<NodeType>,
    pub ancestor_types: Vec<NodeType>,
    pub descendant_types: Vec<NodeType>,
    pub expected_parent_types: Vec<NodeType>,
    pub expected_child_types: Vec<NodeType>,
    pub expected_ancestor_types: Vec<NodeType>,
    pub expected_descendant_types: Vec<NodeType>,
    pub event_types: Vec<NodeType>,
    pub self_event_types: Vec<NodeType>,
    pub base_struct_type: Option<StructType>,
}
