#![feature(default_field_values)]
#![feature(if_let_guard)]
#![feature(str_as_str)]

mod analyze;
mod bind;
mod build;
mod compile;
mod elaborate;
mod execute;
mod import;
mod link;
mod lower;
mod optimize;
mod resolve;
mod validate;

pub use analyze::*;
pub use bind::*;
pub use build::*;
pub use compile::*;
pub use elaborate::*;
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
