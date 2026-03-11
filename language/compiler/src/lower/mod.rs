mod emit;
mod error;
mod item;
mod module;
mod process;
mod result;
mod table;
mod r#type;
mod warning;

pub(crate) use emit::*;
pub use error::*;
pub(crate) use module::{
    ModuleLowerer, RuntimeStatusLayout, collect_expression_string_literals,
    static_key_to_field_name, string_literal_global_name_for_content,
};
pub(crate) use result::*;
pub(crate) use r#type::*;
pub use warning::*;

#[cfg(test)]
mod tests;
