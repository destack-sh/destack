//! destack.core.builtin.fractional@2025.08.15.1

#![destack::partial(destack.core.builtin.fractional, file)]

use crate::Timestamp;
use crate::Uuid;

#[destack::generated(FractionalIntegerError, struct, block)]
/// An Error raised when a fractional integer operation fails.
pub struct FractionalIntegerError {
    id: Uuid,
    client: i64, /* TODO */
    client_nonce: u8,
    client_created_at: Timestamp,
    client_remote_epoch: u64,
    client_local_epoch: u64,
    description: String,
    head: String,
    a: String,
    b: String,
}
