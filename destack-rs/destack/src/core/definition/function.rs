//! destack.core.definition.function@2025.08.15.1

#![destack::partial(destack.core.definition.function, file)]

use crate::RuntimeLanguage;
use crate::RuntimePlatform;
use crate::RuntimeType;

#[destack::generated(FunctionDefinition, struct, block)]
/// Definition of a builtin Function.
pub struct FunctionDefinition {
    id: u16,
    name: String,
    description: String,
    is_async: bool,
    is_managed: bool,
    platforms: Vec<RuntimePlatform>,
    languages: Vec<RuntimeLanguage>,
    runtimes: Vec<RuntimeType>,
}
