#![allow(clippy::module_inception)]

pub(crate) mod daemon;
mod server;

pub use server::DestackLanguageServer;
