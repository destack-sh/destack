//! High-performance source line counter core re-exports.

#![feature(default_field_values)]

pub mod count;
pub mod glob;
pub mod ignore;

pub use count::{CommentStyle, LanguageConfiguration, Options, Statistics, count};
