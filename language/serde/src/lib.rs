extern crate self as tspp_serde;

mod codec;
mod decode;
mod encode;
mod error;
mod schema;
mod value;

pub use codec::*;
pub use decode::*;
pub use encode::*;
pub use error::*;
pub use schema::*;
pub use tspp_serde_macros::{Reflect, SectionEntry};
pub use value::*;

#[cfg(test)]
mod tests;
