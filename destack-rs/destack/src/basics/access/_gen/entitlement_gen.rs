//! destack.basics.access.entitlement@2025.08.15.1

#![destack::generated(destack.basics.access.entitlement, file)]

use crate::EntitlementType;
use crate::RoleType;

#[destack::generated(RoleType, Debug, block)]
impl std::fmt::Debug for RoleType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RoleType::System => write!(f, "SYSTEM"),
            RoleType::Owner => write!(f, "OWNER"),
            RoleType::Admin => write!(f, "ADMIN"),
            RoleType::Developer => write!(f, "DEVELOPER"),
            RoleType::User => write!(f, "USER"),
            RoleType::Spectator => write!(f, "SPECTATOR"),
        }
    }
}

#[destack::generated(EntitlementType, Debug, block)]
impl std::fmt::Debug for EntitlementType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EntitlementType::Permission => write!(f, "PERMISSION"),
            EntitlementType::Role => write!(f, "ROLE"),
        }
    }
}
