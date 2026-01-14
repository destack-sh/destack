#![feature(thread_id_value)]

pub mod cli;
pub mod command;
pub mod common;
pub mod console;
pub mod entry;
pub mod pipeline;

pub use command::{
    bench, build, check, clean, completions, config, doc, doctor, explain, fmt, info, init, lint,
    lsp, run, targets, task, test, version,
};
