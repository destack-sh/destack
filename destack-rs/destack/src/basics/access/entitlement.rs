//! destack.basics.access.entitlement@2025.08.14.0

#![destack::partial(destack.basics.access.entitlement, file)]

#[destack::generated(EntitlementType, enum, block)]
/// A Type of Entitlement.
pub enum EntitlementType {
    /// A Permission
    PERMISSION = 1,
    /// A Role
    ROLE = 2
}