//! destack.core.universe@2025.08.15.1

#![destack::partial(destack.core.universe, file)]
#![allow(unused_imports)]
#![allow(unreachable_pub)]

pub use crate::core::universe::universe::{
    UniverseSignupRequest, UniverseSignupResponse, UniverseSpawnRequest, UniverseSpawnResponse,
};
pub use crate::core::universe::user::ClientType;

pub mod _gen;
pub mod client;
pub mod organization;
pub mod team;
pub mod universe;
pub mod user;
