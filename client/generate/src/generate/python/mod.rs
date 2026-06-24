mod client;
mod codec;
mod defaults;
mod facade;
mod generate;
mod item;
mod module;
mod name;
mod operation;
mod output;
mod package;
mod path;
mod text;

pub(in crate::generate) use generate::{generate, generate_protocol};
pub(in crate::generate) use output::format;
