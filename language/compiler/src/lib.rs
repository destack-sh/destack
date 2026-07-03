#![feature(default_field_values)]
#![allow(clippy::result_large_err)]
#![allow(clippy::too_many_arguments)]

mod analyze;
mod bind;
mod check;
mod compile;
mod emit;
mod expand;
mod export;
mod import;
mod link;
mod lower;
mod materialize;
mod optimize;
mod resolve;
mod r#static;
mod verify;

pub use analyze::*;
pub use bind::*;
pub use check::*;
pub use compile::*;
pub use emit::*;
pub use expand::*;
pub use export::*;
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
