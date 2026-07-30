#![allow(clippy::module_inception)]

mod query;
mod server;

pub use server::DestackLanguageServer;

#[cfg(test)]
mod tests;
