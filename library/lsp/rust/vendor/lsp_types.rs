//! Language Server Protocol data types.
//!
//! NOTE @Incomplete: For now we re-export the `lsp-types` crate to ensure we
//! have complete coverage for all LSP structures. If/when we want to vendor
//! them completely to remove the dependency, we can copy the relevant modules
//! here in a structured way.

pub use lsp_types::*;
