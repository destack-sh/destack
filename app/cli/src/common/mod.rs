pub mod compile;
pub mod diagnostic;
pub mod format;
pub mod input;
pub mod list;
pub mod output;
pub mod program;
pub mod progress;
pub mod report;
pub mod runtime;
pub mod tracing;
pub mod watch;

pub use compile::{CompileResult, CompilerContext, CompilerMode, print_no_input_help};
pub use diagnostic::{DiagnosticArgs, print_diagnostics};
pub use format::{
    DiagnosticFormat, FormatOptions, FormatResult, LineWriter, collect_diagnostics_json,
    format_diagnostics, format_diagnostics_with_writer,
};
pub use input::{InputArgs, InputSource, SingleInputArgs, load_source, load_sources};
pub use list::{
    ListEntry, ListGroup, ListPrinter, ListSpacing, grouped_list_payload, list_payload,
    list_payload_with_count, print_grouped_list, print_grouped_list_with, print_list,
    print_list_with,
};
pub use output::{ArtifactArg, EmitArg, PlatformArg, RuntimeArg, TargetArgs};
pub use program::{FileSystemOverride, ProgramArgs, ensure_no_watch_or_dev};
pub use progress::{ProgressMode, ProgressReporter, is_tty};
pub use report::{
    CommandError, CommandReport, ReportArgs, ReportFormat, parse_command_payload,
    parse_required_command_payload, print_json_payload_report, print_report, report_error,
    report_error_with, report_from_message_payload, report_from_payload, report_no_input,
};
pub use runtime::RuntimeArgs;
pub use tracing::TracingArgs;
pub use watch::{WatchCompileJson, WatchCompileReason, WatchReporter};
