#![feature(default_field_values)]
#![feature(if_let_guard)]
#![feature(str_as_str)]
#![feature(thread_id_value)]

mod analyze;
mod bind;
mod compile;
mod elaborate;
mod generate;
mod import;
mod link;
mod lower;
mod optimize;
mod resolve;
mod verify;

pub use analyze::*;
pub use bind::*;
pub use compile::*;
pub use elaborate::*;
pub use generate::*;
pub use import::*;
pub use link::*;
pub use lower::*;
pub use optimize::*;
pub use resolve::*;
pub use verify::*;

#[cfg(test)]
mod tests;
#[cfg(test)]
#[allow(unused_imports)]
pub use tests::*;
