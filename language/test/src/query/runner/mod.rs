//! Query test runner modules.
//!
//! Contains runners for different LSP query types: navigation, assist,
//! refactor, and diagnostic. These are invoked by the QuerySuite via
//! dispatch_query().

pub mod assist;
pub mod diagnostic;
pub mod navigation;
pub mod refactor;
