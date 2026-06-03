#![feature(thread_id_value)]

pub mod cli;
pub mod command;
pub mod common;
pub mod console;
pub mod entry;
pub mod error;
pub mod pipeline;
#[cfg(test)]
pub mod tests;

pub use command::{
    bench, build, cache, check, clean, completions, daemon, doc, doctor, eval, explain, fmt, info,
    init, inspect, lint, lsp, manifest, run, settings, targets, task, test, update, version,
};
