pub mod compile;
pub mod diagnostic;
pub mod lex;
pub mod parse;
pub mod program;
pub mod resolve;
pub mod source;
pub mod tracing;
pub mod transpile;
pub mod version;

pub(crate) use diagnostic::{DiagnosticOptionsArgs, print_diagnostics};
pub(crate) use program::{ProgramArgs, WorkerOptionsArgs};
pub(crate) use source::{SourceArg, get_string_or_file};
pub(crate) use tracing::TracingArgs;
