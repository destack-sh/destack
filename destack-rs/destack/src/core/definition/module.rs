//! destack.core.definition.module@2025.08.15.1

#![destack::partial(destack.core.definition.module, file)]

#[destack::generated(ModuleDefinition, struct, block)]
/// Definition of a builtin Module.
pub struct ModuleDefinition {

}

#[destack::generated(ModuleType, enum, block)]
/// Built-in module types.
pub enum ModuleType {
    /// Root module for the entire Universe
    ROOT = 1,
    /// Module for an entire UniverseDomain
    DOMAIN = 2,
    /// Module for an entire UniverseCategory
    CATEGORY = 3,
    /// Module for one or more Objects
    OBJECT = 4
}