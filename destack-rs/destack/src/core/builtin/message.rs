//! destack.core.builtin.message@2025.08.15.1

#![destack::partial(destack.core.builtin.message, file)]

#[destack::generated(Message, struct, block)]
/// A Message contains data for communicating with Nodes via Actions.
/// Because Message are as-is provided by Clients, they only contain client-authority data.
pub struct Message {}
