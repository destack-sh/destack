pub mod app;
pub mod command;
pub mod common;
pub mod console;
pub mod diagnostic;

pub use command::{
    build, cache, check, clean, completions, doc, doctor, explain, fmt, info, lint, lsp,
    query, rewrite, run, settings, targets, task, test, update, version,
};

#[cfg(test)]
pub mod tests;
