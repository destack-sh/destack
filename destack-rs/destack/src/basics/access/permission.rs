//! destack.basics.access.permission@2025.08.15.1

#![destack::partial(destack.basics.access.permission, file)]

#[destack::generated(PermissionDefinition, -, block)]
/// Definition of a builtin Permission for a builtin Node.
pub struct PermissionDefinition {
    pub id: u8,
    pub name: String,
    pub description: String,
}
