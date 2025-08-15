//! destack.core.universe.universe@2025.08.15.1

#![destack::partial(destack.core.universe.universe, file)]

use crate::Region;
use crate::Timestamp;
use crate::Uuid;

#[destack::generated(UniverseSignupRequest, -, block)]
/// UniverseSignupRequest
pub struct UniverseSignupRequest {
    pub id: Uuid,
    pub client: i64, /* TODO */
    pub client_nonce: u8,
    pub client_created_at: Timestamp,
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
    pub client: i64, /* TODO */
    pub client_nonce: u8,
    pub client_created_at: Timestamp,
    pub client_remote_epoch: u64,
    pub client_local_epoch: u64,
    pub user: i64, /* TODO */
}

#[destack::generated(UniverseSpawnRequest, -, block)]
/// UniverseSpawnRequest
pub struct UniverseSpawnRequest {
    pub id: Uuid,
    pub client: i64, /* TODO */
    pub client_nonce: u8,
    pub client_created_at: Timestamp,
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
    pub client: i64, /* TODO */
    pub client_nonce: u8,
    pub client_created_at: Timestamp,
    pub client_remote_epoch: u64,
    pub client_local_epoch: u64,
    pub space: i64, /* TODO */
}
