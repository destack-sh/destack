//! destack.core.definition.node@2025.08.15.1

#![destack::partial(destack.core.definition.node, file)]

use crate::ActionDefinition;
use crate::ConstantDefinition;
use crate::ConstraintDefinition;
use crate::IndexDefinition;
use crate::MethodDefinition;
use crate::NodeType;
use crate::ObjectStability;
use crate::PermissionDefinition;
use crate::PropertyDefinition;
use crate::StructType;
use crate::TagDefinition;
use crate::TraitType;

#[destack::generated(NodeDefinition, struct, block)]
/// Definition of a builtin Node.
pub struct NodeDefinition {
    id: u32,
    r#type: NodeType,
    name: String,
    description: String,
    stability: ObjectStability,
    is_abstract: bool,
    is_final: bool,
    is_singleton: bool,
    properties: Vec<PropertyDefinition>,
    indexes: Vec<IndexDefinition>,
    constraints: Vec<ConstraintDefinition>,
    permissions: Vec<PermissionDefinition>,
    methods: Vec<MethodDefinition>,
    actions: Vec<ActionDefinition>,
    constants: Vec<ConstantDefinition>,
    tags: Vec<TagDefinition>,
    base_type: NodeType,
    extended_by: Vec<NodeType>,
    inherits: Vec<NodeType>,
    inherited_by: Vec<NodeType>,
    traits: Vec<TraitType>,
    self_traits: Vec<TraitType>,
    parent_types: Vec<NodeType>,
    child_types: Vec<NodeType>,
    ancestor_types: Vec<NodeType>,
    descendant_types: Vec<NodeType>,
    expected_parent_types: Vec<NodeType>,
    expected_child_types: Vec<NodeType>,
    expected_ancestor_types: Vec<NodeType>,
    expected_descendant_types: Vec<NodeType>,
    event_types: Vec<NodeType>,
    self_event_types: Vec<NodeType>,
    base_struct_type: StructType,
}
