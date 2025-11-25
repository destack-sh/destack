//! Module resolution implementation.
//!
//! This module provides Node.js-style module resolution with extensions for:
//! - TypeScript path mapping (`tsconfig.json` paths)
//! - Package exports/imports (ESM)
//! - Browser field substitution
//! - Custom aliases

mod alias;
mod context;
mod error;
mod file;
mod load;
mod options;
mod package;
mod require;
mod resolution;
mod resolve;
mod resolver;
mod tsconfig;

pub use context::*;
pub use error::*;
pub use options::*;
pub use resolution::*;
pub use resolver::*;
