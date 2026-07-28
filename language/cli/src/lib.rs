pub mod app;
pub mod command;
pub mod common;
pub mod console;
pub mod diagnostic;

pub use command::{
    bench, build, cache, check, clean, completions, doc, doctor, eval, explain, fmt, info, init,
    lint, lsp, query, rewrite, run, settings, targets, task, test, update, version,
};

#[cfg(test)]
pub mod tests;
