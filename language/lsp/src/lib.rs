#![allow(clippy::module_inception)]

mod query;
mod server;

pub use server::TsppLanguageServer;

#[cfg(test)]
mod tests;
