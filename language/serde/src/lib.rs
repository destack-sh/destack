#[cfg(test)]
extern crate self as destack_serde;

mod codec;
mod decode;
mod encode;
mod error;
mod schema;

pub use codec::*;
pub use decode::*;
pub use destack_serde_macros::Schema;
pub use encode::*;
pub use error::*;
pub use schema::*;

#[cfg(test)]
mod tests;
