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

#[destack::generated(ModuleDefinition, -, block)]
/// Definition of a builtin Module.
pub struct ModuleDefinition {
    pub r#type: ModuleType,
    pub name: String,
    pub description: String,
    pub path: String,
    pub domain: Option<UniverseDomain>,
    pub category: Option<UniverseCategory>,
    pub methods: Vec<MethodDefinition>,
    pub constants: Vec<ConstantDefinition>,
    pub node_types: Vec<NodeType>,
    pub struct_types: Vec<StructType>,
    pub handle_types: Vec<HandleType>,
    pub enum_types: Vec<EnumType>,
    pub parent_path: Option<String>,
    pub children_paths: Vec<String>,
}
