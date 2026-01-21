//! This library contains code that is common to both the `cranelift-codegen` and
//! `cranelift-codegen-meta` libraries.
// NOTE #Cleanup: fix clippy lints once fully vendored
#![allow(clippy::all)]
#![allow(warnings)]

#![deny(missing_docs)]

pub mod constant_hash;
pub mod constants;

/// Version number of this crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
