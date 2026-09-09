#![feature(default_field_values)]
#![allow(clippy::result_large_err)]
#![allow(clippy::too_many_arguments)]

mod analyze;
mod bind;
mod compile;
mod elaborate;
mod emit;
mod expand;
mod export;
mod import;
mod instantiate;
mod link;
mod lower;
mod optimize;
mod resolve;
mod sema;
mod r#static;
mod verify;

pub use analyze::*;
pub use bind::*;
pub use compile::*;
pub use elaborate::*;
pub use emit::*;
pub use expand::*;
pub use export::*;
pub use import::*;
pub use instantiate::*;
pub use link::*;
pub use lower::*;
pub use optimize::*;
pub use resolve::*;
pub use sema::*;
pub use verify::*;

#[cfg(test)]
mod tests;
#[cfg(test)]
#[allow(unused_imports)]
pub(crate) use tests::*;
