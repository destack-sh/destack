//! destack.basics.script.method@2025.08.15.1

#![destack::partial(destack.basics.script.method, file)]

use crate::MethodType;
use crate::PropertyDefinition;
use crate::RuntimeLanguage;
use crate::RuntimePlatform;
use crate::RuntimeType;

#[destack::generated(MethodDefinition, , block)]
/// Definition of a builtin Method.
pub struct MethodDefinition {
    id: u16,
    r#type: MethodType,
    name: String,
    description: String,
    is_async: bool,
    is_managed: bool,
    alias_of: Option<u16>,
    input_properties: Vec<PropertyDefinition>,
    output_property: Option<PropertyDefinition>,
    platforms: Option<Vec<RuntimePlatform>>,
    languages: Option<Vec<RuntimeLanguage>>,
    runtimes: Option<Vec<RuntimeType>>,
}
