#[cfg(test)]
extern crate self as tspp_rpc;

mod call;
mod connection;
mod protocol;
mod service;
mod transport;

pub use call::*;
pub use connection::*;
pub use protocol::*;
pub use service::*;
pub use transport::*;
pub use tspp_rpc_macros::service;

#[cfg(test)]
mod tests;
