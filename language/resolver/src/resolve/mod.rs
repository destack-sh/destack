//! # Oxc Resolver
//!
//! Node.js [CommonJS][cjs] and [ECMAScript][esm] Module Resolution.
//!
//! A module resolution is the process of finding the file referenced by a module specifier in
//! `import "specifier"` or `require("specifier")`.
//!
//! All [configuration options](ResolveOptions) are aligned with webpack's [enhanced-resolve].
//!
//! ## Terminology
//!
//! ### Specifier
//!
//! For [CommonJS modules][cjs],
//! the specifier is the string passed to the `require` function. e.g. `"id"` in `require("id")`.
//!
//! For [ECMAScript modules][esm],
//! the specifier of an `import` statement is the string after the `from` keyword,
//! e.g. `'specifier'` in `import 'specifier'` or `import { sep } from 'specifier'`.
//! Specifiers are also used in export from statements, and as the argument to an `import()` expression.
//!
//! This is also named "request" in some places.

mod builtins;
mod context;
mod diagnostic;
mod options;
mod package_json;
mod resolution;
mod resolve;
mod specifier;
mod tsconfig;

pub use builtins::*;
pub use context::*;
pub use diagnostic::*;
pub use options::*;
pub use package_json::*;
pub use resolution::*;
pub use resolve::*;
pub use specifier::*;
pub use tsconfig::*;
