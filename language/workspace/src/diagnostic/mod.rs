mod error;
mod file;

pub use error::Error;
pub use file::{DiagnosticsRequest, FileDiagnostics};

pub(crate) use file::diagnostics_by_file;
