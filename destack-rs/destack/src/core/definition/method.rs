//! destack.core.definition.method@2025.08.15.1

#![destack::partial(destack.core.definition.method, file)]

use crate::MethodType;
use crate::PropertyDefinition;
use crate::RuntimeLanguage;
use crate::RuntimePlatform;
use crate::RuntimeType;

#[destack::generated(MethodDefinition, struct, block)]
/// Definition of a builtin Method.
pub struct MethodDefinition {
    id: u16,
    r#type: MethodType,
    name: String,
    description: String,
    is_async: bool,
    is_managed: bool,
    alias_of: u16,
    input_properties: Vec<PropertyDefinition>,
    output_property: PropertyDefinition,
    platforms: Vec<RuntimePlatform>,
    languages: Vec<RuntimeLanguage>,
    runtimes: Vec<RuntimeType>,
}
