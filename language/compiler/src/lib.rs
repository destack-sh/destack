#![feature(default_field_values)]
#![feature(if_let_guard)]

mod bind;
mod build;
mod compile;
mod execute;
mod import;
mod link;
mod lower;
mod optimize;
mod resolve;
mod validate;

pub use bind::*;
pub use build::*;
pub use compile::*;
pub use execute::*;
pub use import::*;
pub use link::*;
pub use lower::*;
pub use optimize::*;
pub use resolve::*;
pub use validate::*;

#[cfg(test)]
mod tests;
#[cfg(test)]
#[allow(unused_imports)]
pub(crate) use tests::*;
