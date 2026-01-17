#![feature(default_field_values)]
#![feature(if_let_guard)]
#![feature(str_as_str)]
#![feature(thread_id_value)]

mod daemon;
mod diagnostic;
mod repl;
mod watch;

#[cfg(test)]
pub mod tests;

pub use daemon::*;
pub use diagnostic::*;
pub use repl::*;
pub use watch::*;
