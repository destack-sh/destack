//! Destack CLI public API – command handlers and runtime integration.

pub mod ast;
pub mod dir;
pub mod format;
pub mod lsp;
pub mod print;
pub mod source;
pub mod tokei;
pub mod token;
pub mod version;

pub use destack_terminal as terminal;
