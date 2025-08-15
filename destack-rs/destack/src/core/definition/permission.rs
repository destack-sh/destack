//! destack.core.definition.permission@2025.08.15.1

#![destack::partial(destack.core.definition.permission, file)]

#[destack::generated(PermissionDefinition, struct, block)]
/// Definition of a builtin Permission for a builtin Node.
pub struct PermissionDefinition {
    id: u8,
    name: String,
    description: String
}