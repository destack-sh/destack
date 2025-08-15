//! destack.core.builtin.fractional@2025.08.15.1

#![destack::partial(destack.core.builtin.fractional, file)]

use crate::Timestamp;
use crate::Uuid;

#[destack::generated(FractionalIntegerError, -, block)]
/// An Error raised when a fractional integer operation fails.
pub struct FractionalIntegerError {
    pub id: Uuid,
    pub client: i64, /* TODO */
    pub client_nonce: u8,
    pub client_created_at: Timestamp,
    pub client_remote_epoch: u64,
    pub client_local_epoch: u64,
    pub description: Option<String>,
    pub head: Option<String>,
    pub a: Option<String>,
    pub b: Option<String>,
}
