mod error;
mod dependency;
mod execute;
mod lower;
mod patch;
mod process;
mod r#static;
mod warning;

pub use error::*;
pub(crate) use dependency::collect_comptime_dependencies;
pub(crate) use patch::ComptimePatch;
pub use process::*;
pub use warning::*;

#[cfg(test)]
mod tests;
