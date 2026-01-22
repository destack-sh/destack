#![feature(default_field_values)]
#![feature(if_let_guard)]
#![feature(str_as_str)]
#![feature(thread_id_value)]

mod command;
mod daemon;
mod diagnostic;
pub mod protocol;
mod repl;
mod watch;

#[cfg(test)]
pub mod tests;

pub use command::*;
pub use daemon::*;
pub use diagnostic::*;
pub use repl::*;
pub use watch::*;
