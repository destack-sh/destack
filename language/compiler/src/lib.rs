#![feature(default_field_values)]
#![feature(if_let_guard)]
#![feature(str_as_str)]
#![feature(thread_id_value)]
#![allow(clippy::result_large_err)]

mod analyze;
mod cache;
mod common;
mod compile;
mod elaborate;
mod emit;
mod execute;
mod generate;
mod import;
mod link;
mod lower;
mod optimize;
mod resolve;
mod unbind;

pub use analyze::*;
pub use compile::*;
pub use elaborate::*;
pub use emit::*;
pub use execute::*;
pub use generate::*;
pub use import::*;
pub use link::*;
pub use lower::*;
pub use optimize::*;
pub use resolve::*;
pub use unbind::*;

#[cfg(test)]
mod tests;
#[cfg(test)]
#[allow(unused_imports)]
pub(crate) use tests::*;
