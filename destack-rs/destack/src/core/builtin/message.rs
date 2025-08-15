//! destack.core.builtin.message@2025.08.15.1

#![destack::partial(destack.core.builtin.message, file)]

use crate::Timestamp;
use crate::Uuid;

#[destack::generated(Message, struct, block)]
/// A Message contains data for communicating with Nodes via Actions.
/// Because Message are as-is provided by Clients, they only contain client-authority data.
pub struct Message {
    id: Uuid,
    client: i64, /* TODO */
    client_nonce: u8,
    client_created_at: Timestamp,
    client_remote_epoch: u64,
    client_local_epoch: u64,
}
