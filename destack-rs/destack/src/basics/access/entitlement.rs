//! destack.basics.access.entitlement@2025.08.15.1

#![destack::partial(destack.basics.access.entitlement, file)]

#[destack::generated(EntitlementType, enum, block)]
/// A Type of Entitlement.
pub enum EntitlementType {
    /// A Permission
    Permission = 1,
    /// A Role
    Role = 2,
}
