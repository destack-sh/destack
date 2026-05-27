#![feature(default_field_values)]
#![feature(str_as_str)]
#![feature(thread_id_value)]
#![allow(clippy::result_large_err)]

mod bind;
mod check;
mod common;
mod compile;
mod elaborate;
mod expand;
mod export;
mod generate;
mod import;
mod link;
mod lower;
mod materialize;
mod optimize;
mod resolve;
mod verify;

pub use bind::*;
pub use check::*;
pub use compile::*;
pub use elaborate::*;
pub use expand::*;
pub use export::*;
pub use generate::*;
pub use import::*;
pub use link::*;
pub use lower::*;
pub use materialize::*;
pub use optimize::*;
pub use resolve::*;
pub use verify::*;

#[cfg(test)]
mod tests;
#[cfg(test)]
#[allow(unused_imports)]
pub(crate) use tests::*;
