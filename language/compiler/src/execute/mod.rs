mod dependency;
mod error;
mod execute;
mod lower;
mod patch;
mod process;
mod r#static;
mod warning;

pub(crate) use dependency::collect_comptime_dependencies;
pub use error::*;
pub(crate) use patch::ComptimePatch;
pub use process::*;
pub use warning::*;

#[cfg(test)]
mod tests;
