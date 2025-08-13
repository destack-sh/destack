//! Universally Unique Identifier (UUID) as a u128 wrapper.
//! We provide the `Uuid` type with relevant operators and conversions.
//! We also provide uuid4, uuid5, and uuid7 generation functions.

mod uuid;
mod uuid4;
mod uuid5;
mod uuid7;

pub use uuid::Uuid;