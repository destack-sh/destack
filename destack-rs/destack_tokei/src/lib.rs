//! High-performance source line counter core re-exports.

pub mod count;
pub mod glob;
pub mod ignore;

pub use count::{CommentStyle, LanguageConfiguration, Options, Statistics, count};
