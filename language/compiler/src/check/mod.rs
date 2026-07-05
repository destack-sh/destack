mod auto;
mod decorator;
mod diagnostic;
mod dump;
mod error;
mod flow;
mod infer;
mod language;
mod obligation;
mod provide;
mod reduce;
mod reify;
mod relate;
mod select;
mod solve;
mod state;
mod walk;
mod warning;
mod write;

pub use error::*;
pub use warning::*;

pub(in crate::check) use decorator::*;
pub(in crate::check) use dump::*;
pub(in crate::check) use flow::*;
pub(in crate::check) use language::*;
pub(in crate::check) use obligation::*;
pub(in crate::check) use reduce::*;
pub(in crate::check) use reify::*;
pub(in crate::check) use relate::*;
pub(in crate::check) use select::*;
pub(in crate::check) use solve::*;
pub(in crate::check) use state::*;
pub(in crate::check) use walk::*;

#[cfg(test)]
mod tests;
