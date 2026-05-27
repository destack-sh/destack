#![feature(default_field_values)]
#![feature(str_as_str)]
#![feature(thread_id_value)]

mod command;
mod daemon;
mod diagnostic;
mod ipc;
pub mod protocol;
mod repl;
mod watch;

#[cfg(test)]
pub mod tests;

pub use command::*;
pub use daemon::*;
pub use diagnostic::*;
pub use ipc::*;
pub use repl::*;
pub use watch::*;
