mod content;
mod document;
mod printer;

pub use document::{print_document, print_fragment};
pub(crate) use printer::Printer;
