mod error;
mod view;

pub use error::Error;
pub use view::DiagnosticView;

pub(crate) use view::diagnostics_by_file;
