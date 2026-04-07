mod content;
mod document;
mod printer;

pub use document::{
    print_document, print_document_with_options, print_fragment, print_fragment_with_options,
};
pub(crate) use printer::Printer;
pub use printer::RenderOptions;
