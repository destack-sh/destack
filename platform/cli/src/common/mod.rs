pub mod compile;
pub mod diagnostic;
pub mod fix;
pub mod format;
pub mod input;
pub mod output;
pub mod program;
pub mod progress;
pub mod report;
pub mod tracing;

pub use compile::{CompileResult, CompilerContext, CompilerMode, print_no_input_help};
pub use diagnostic::{DiagnosticArgs, print_diagnostics};
pub use format::{
    DiagnosticFormat, FormatOptions, FormatResult, LineWriter, collect_diagnostics_json,
    format_diagnostics, format_diagnostics_with_writer,
};
pub use input::{InputArgs, InputSource, SingleInputArgs, load_source, load_sources};
pub use output::{OutputArg, PlatformArg, RuntimeArg, TargetArgs};
pub use program::{ProgramArgs, ensure_no_watch_or_dev};
pub use progress::{ProgressMode, ProgressReporter, is_tty};
pub use report::{
    CommandReport, CommandStats, ReportArgs, ReportFormat, print_report, report_error,
    report_no_input,
};
pub use tracing::TracingArgs;
