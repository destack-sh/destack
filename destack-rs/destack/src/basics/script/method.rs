//! destack.basics.script.method@2025.08.15.1

#![destack::partial(destack.basics.script.method, file)]

use crate::{MethodType, PropertyDefinition, RuntimeLanguage, RuntimePlatform, RuntimeType};

#[destack::generated(MethodDefinition, -, block)]
/// Definition of a builtin Method.
pub struct MethodDefinition {
    pub id: u16,
    pub r#type: MethodType,
    pub name: String,
    pub description: String,
    pub is_async: bool,
    pub is_managed: bool,
    pub alias_of: Option<u16>,
    pub input_properties: Vec<PropertyDefinition>,
    pub output_property: Option<PropertyDefinition>,
    pub platforms: Option<Vec<RuntimePlatform>>,
    pub languages: Option<Vec<RuntimeLanguage>>,
    pub runtimes: Option<Vec<RuntimeType>>,
}
