mod directive;
mod emit;
mod error;
mod item;
mod module;
mod process;
mod table;
mod r#type;
mod warning;

pub(crate) use emit::*;
pub use error::*;
pub(crate) use module::ModuleLowerer;
pub use process::*;
pub(crate) use r#type::*;
pub use warning::*;

#[cfg(test)]
mod tests;
