#![allow(clippy::module_inception)]

mod daemon;
mod server;

#[cfg(test)]
pub(crate) use daemon::LspDaemonClient;
pub use server::DestackLanguageServer;
