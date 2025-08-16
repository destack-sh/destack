//! destack.basics.access@2025.08.15.1

#![destack::partial(destack.basics.access, file)]
#![allow(unused_imports)]
#![allow(unreachable_pub)]

pub use crate::basics::access::entitlement::{EntitlementType, RoleType};
pub use crate::basics::access::permission::PermissionDefinition;
pub use crate::basics::access::sanction::SanctionType;

pub mod _gen;
pub mod entitlement;
pub mod invite;
pub mod membership;
pub mod permission;
pub mod role;
pub mod sanction;
