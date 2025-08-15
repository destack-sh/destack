//! destack.core.universe.universe@2025.08.15.1

#![destack::partial(destack.core.universe.universe, file)]

use crate::Region;
use crate::Timestamp;
use crate::Uuid;

#[destack::generated(UniverseSignupRequest, struct, block)]
/// UniverseSignupRequest
pub struct UniverseSignupRequest {
    id: Uuid,
    client: i64, /* TODO */
    client_nonce: u8,
    client_created_at: Timestamp,
    client_remote_epoch: u64,
    client_local_epoch: u64,
    name: String,
    email: String,
    password: String,
}

#[destack::generated(UniverseSignupResponse, struct, block)]
/// UniverseSignupResponse
pub struct UniverseSignupResponse {
    id: Uuid,
    client: i64, /* TODO */
    client_nonce: u8,
    client_created_at: Timestamp,
    client_remote_epoch: u64,
    client_local_epoch: u64,
    user: i64, /* TODO */
}

#[destack::generated(UniverseSpawnRequest, struct, block)]
/// UniverseSpawnRequest
pub struct UniverseSpawnRequest {
    id: Uuid,
    client: i64, /* TODO */
    client_nonce: u8,
    client_created_at: Timestamp,
    client_remote_epoch: u64,
    client_local_epoch: u64,
    name: String,
    region: Region,
    slug: String,
}

#[destack::generated(UniverseSpawnResponse, struct, block)]
/// UniverseSpawnResponse
pub struct UniverseSpawnResponse {
    id: Uuid,
    client: i64, /* TODO */
    client_nonce: u8,
    client_created_at: Timestamp,
    client_remote_epoch: u64,
    client_local_epoch: u64,
    space: i64, /* TODO */
}
