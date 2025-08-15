//! destack.core.definition.module@2025.08.15.1

#![destack::partial(destack.core.definition.module, file)]

use crate::ConstantDefinition;
use crate::EnumType;
use crate::HandleType;
use crate::MethodDefinition;
use crate::ModuleType;
use crate::NodeType;
use crate::StructType;
use crate::UniverseCategory;
use crate::UniverseDomain;

#[destack::generated(ModuleDefinition, , block)]
/// Definition of a builtin Module.
pub struct ModuleDefinition {
    r#type: ModuleType,
    name: String,
    description: String,
    path: String,
    domain: Option<UniverseDomain>,
    category: Option<UniverseCategory>,
    methods: Vec<MethodDefinition>,
    constants: Vec<ConstantDefinition>,
    node_types: Vec<NodeType>,
    struct_types: Vec<StructType>,
    handle_types: Vec<HandleType>,
    enum_types: Vec<EnumType>,
    parent_path: Option<String>,
    children_paths: Vec<String>,
}
