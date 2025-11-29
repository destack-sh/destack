//! Module resolution for JavaScript/TypeScript.
//!
//! This crate provides Node.js-style module resolution with support for:
//! - CommonJS `require()` resolution algorithm
//! - ECMAScript module resolution with `exports` and `imports` fields
//! - TypeScript path mapping via `tsconfig.json`
//! - Browser field substitution
//! - Aliases and restrictions
//!
//! The resolver is a port of oxc-resolver (itself a port of enhanced-resolve),
//! adapted to work with our `Program` and registry system.
//!
//! ## Usage
//!
//! ```ignore
//! use destack_resolver::{Resolver, ResolveOptions};
//!
//! let resolver = Resolver::blank(ResolveOptions::default());
//! let resolution = resolver.resolve("/project/src", "./utils");
//! ```
//!
//! ## Architecture
//!
//! The resolver integrates with `Program` for storing resolved packages and tsconfigs:
//! - `Program.packages` (PackageRegistry) - stores parsed package.json files
//! - `Program.tsconfigs` (TsConfigRegistry) - stores parsed tsconfig.json files

#![feature(default_field_values)]
#![feature(if_let_guard)]
#![feature(once_cell_try)]

pub mod resolve;

pub use resolve::*;

#[cfg(test)]
mod tests;
