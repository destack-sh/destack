mod common;
mod dispatch;
mod error;
mod function;
mod instance;
mod key;
mod module;
mod process;
mod runtime;
mod r#type;
mod warning;

pub(crate) use common::*;
pub(crate) use dispatch::*;
pub use error::*;
pub(crate) use function::*;
pub(crate) use instance::*;
pub(crate) use module::{
    ModuleLowerer, lower_mutability, static_key_to_field_name,
    string_literal_global_name_for_content,
};
pub(crate) use runtime::*;
pub(crate) use r#type::*;
pub use warning::*;

#[cfg(test)]
mod tests;
