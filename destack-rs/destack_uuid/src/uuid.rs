//! Universally Unique Identifier (UUID) as a u128 wrapper.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// A Universally Unique Identifier (UUID) is a 128-bit identifier.
pub struct Uuid(pub u128);
