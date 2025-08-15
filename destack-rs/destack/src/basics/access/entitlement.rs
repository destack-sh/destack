//! destack.basics.access.entitlement@2025.08.15.1

#![destack::partial(destack.basics.access.entitlement, file)]

#[destack::generated(RoleType, -, block)]
/// RoleType
pub enum RoleType {
    System = 1,
    Owner = 2,
    Admin = 3,
    Developer = 5,
    User = 7,
    Spectator = 10,
}

#[destack::generated(EntitlementType, -, block)]
/// A Type of Entitlement.
pub enum EntitlementType {
    /// A Permission
    Permission = 1,
    /// A Role
    Role = 2,
}
