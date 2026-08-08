extern crate self as destack_serde;

mod codec;
mod decode;
mod encode;
mod error;
mod schema;
mod value;

pub use codec::*;
pub use decode::*;
pub use destack_serde_macros::{Reflect, SectionEntry};
pub use encode::*;
pub use error::*;
pub use schema::*;
pub use value::*;

#[cfg(test)]
mod tests;
