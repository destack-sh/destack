pub mod compile;
pub mod diagnostic;
pub mod dump;
pub mod format;
pub mod lex;
pub mod parse;
pub mod program;
pub mod resolve;
pub mod source;
pub mod tracing;
pub mod version;

pub(crate) use diagnostic::{DiagnosticArgs, print_diagnostics};
pub(crate) use dump::{DumpFormat, DumpKind};
pub(crate) use program::ProgramArgs;
pub(crate) use source::{SourceArg, get_string_or_file};
pub(crate) use tracing::TracingArgs;
