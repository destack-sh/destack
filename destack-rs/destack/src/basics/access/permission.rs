//! destack.basics.access.permission

#![destack::partial(destack.basics.access.permission, file)]

#[destack::generated(PermissionDefinition, -, block)]
/// Definition of a builtin Permission for a builtin Node.
pub struct PermissionDefinition {
    pub id: u8,
    pub name: String,
    pub description: String,
}
