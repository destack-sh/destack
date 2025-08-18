//! destack.core.universe.universe

#![destack::partial(destack.core.universe.universe, file)]

use crate::{Region, Uuid};

#[destack::generated(UniverseSignupRequest, -, block)]
/// UniverseSignupRequest
pub struct UniverseSignupRequest {
    pub id: Uuid,
    pub client: i64,
    pub client_nonce: u8,
    pub client_remote_epoch: u64,
    pub client_local_epoch: u64,
    pub name: String,
    pub email: String,
    pub password: String,
}

#[destack::generated(UniverseSignupResponse, -, block)]
/// UniverseSignupResponse
pub struct UniverseSignupResponse {
    pub id: Uuid,
    pub client: i64,
    pub client_nonce: u8,
    pub client_remote_epoch: u64,
    pub client_local_epoch: u64,
    pub user: i64,
}

#[destack::generated(UniverseSpawnRequest, -, block)]
/// UniverseSpawnRequest
pub struct UniverseSpawnRequest {
    pub id: Uuid,
    pub client: i64,
    pub client_nonce: u8,
    pub client_remote_epoch: u64,
    pub client_local_epoch: u64,
    pub name: String,
    pub region: Region,
    pub slug: String,
}

#[destack::generated(UniverseSpawnResponse, -, block)]
/// UniverseSpawnResponse
pub struct UniverseSpawnResponse {
    pub id: Uuid,
    pub client: i64,
    pub client_nonce: u8,
    pub client_remote_epoch: u64,
    pub client_local_epoch: u64,
    pub space: i64,
}
