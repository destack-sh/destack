//! destack.basics.access.entitlement@2025.08.15.1

#![destack::generated(destack.basics.access.entitlement, file)]

#[destack::generated(EntitlementType, Debug, block)]
impl std::fmt::Debug for EntitlementType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EntitlementType::Permission => write!(f, "PERMISSION"),
            EntitlementType::Role => write!(f, "ROLE"),
        }
    }
}
