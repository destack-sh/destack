//! destack.core.definition.module@2025.08.15.1

#![destack::partial(destack.core.definition.module, file)]

use crate::{StructType, UniverseDomain, MethodDefinition, HandleType, UniverseCategory, ConstantDefinition, NodeType, EnumType};

#[destack::generated(ModuleDefinition, struct, block)]
/// Definition of a builtin Module.
pub struct ModuleDefinition {
    r#type: ModuleType,
    name: String,
    description: String,
    path: String,
    domain: UniverseDomain,
    category: UniverseCategory,
    methods: Vec<MethodDefinition>,
    constants: Vec<ConstantDefinition>,
    node_types: Vec<NodeType>,
    struct_types: Vec<StructType>,
    handle_types: Vec<HandleType>,
    enum_types: Vec<EnumType>,
    parent_path: String,
    children_paths: Vec<String>
}

#[destack::generated(ModuleType, enum, block)]
/// Built-in module types.
pub enum ModuleType {
    /// Root module for the entire Universe
    Root = 1,
    /// Module for an entire UniverseDomain
    Domain = 2,
    /// Module for an entire UniverseCategory
    Category = 3,
    /// Module for one or more Objects
    Object = 4
}
