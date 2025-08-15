//! destack.core.builtin.error@2025.08.15.1

#![destack::partial(destack.core.builtin.error, file)]

use crate::Timestamp;
use crate::Uuid;

#[destack::generated(Error, , block)]
/// An Error is a structured error message.
/// Errors are used to communicate failure states.
/// Like Messages, Errors are as-is provided by Clients and tagged with client-authority tracking.
pub struct Error {
    id: Uuid,
    client: i64, /* TODO */
    client_nonce: u8,
    client_created_at: Timestamp,
    client_remote_epoch: u64,
    client_local_epoch: u64,
    description: Option<String>,
}
