#[cfg(test)]
extern crate self as destack_rpc;

mod call;
mod connection;
mod protocol;
mod service;
mod transport;

pub use call::*;
pub use connection::*;
pub use destack_rpc_macros::service;
pub use protocol::*;
pub use service::*;
pub use transport::*;

#[cfg(test)]
mod tests;
