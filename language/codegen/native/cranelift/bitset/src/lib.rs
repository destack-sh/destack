//! Bitsets for Cranelift.
//!
//! This module provides two bitset implementations:
//!
//! 1. [`ScalarBitSet`]: A small bitset built on top of a single integer.
//!
//! 2. [`CompoundBitSet`]: A bitset that can store more bits than fit in a
//!    single integer, but which internally has heap allocations.
// NOTE #Cleanup: fix clippy lints once fully vendored
#![allow(clippy::all)]
#![allow(warnings)]

#![deny(missing_docs)]
#![no_std]

extern crate alloc;

pub mod compound;
pub mod scalar;

pub use compound::CompoundBitSet;
pub use scalar::ScalarBitSet;
