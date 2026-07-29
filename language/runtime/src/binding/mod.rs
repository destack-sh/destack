mod access;
mod codec;
pub(crate) mod generated;
mod table;

pub use access::BindingAccess;
pub use codec::{CodecId, DEFAULT_BINDING_CODEC};
pub use table::{Binding, BindingFn, BindingTable, ReplayPayload};
