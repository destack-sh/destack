pub mod format;
pub mod input;
pub mod list;
pub mod node;
pub mod output;
pub mod program;
pub mod progress;
pub mod report;
pub mod target;
pub mod watch;

mod command;

pub(crate) use command::{
    CommandOptionsBuilder, CommandResult, CommandSummary, WatchCompileContext, WatchCycle,
    WorkspaceWatch, command_error, emit_workspace_text_output, finish_diagnostic_command,
    finish_workspace_message_command, overrides_from_program, run_workspace_command,
    run_workspace_command_or_report, run_workspace_payload_command_or_report,
};
pub use format::{
    DiagnosticFormat, FormatOptions, FormatResult, LineWriter, collect_diagnostics_json,
    format_diagnostics, format_diagnostics_with_writer,
};
pub use input::{InputArgs, InputSource};
pub use list::{
    ListEntry, ListGroup, ListPrinter, ListSpacing, grouped_list_payload, list_payload,
    list_payload_with_count, print_grouped_list, print_grouped_list_with, print_list,
    print_list_with,
};
pub use node::NodeTypeArg;
pub use output::TargetArgs;
pub use program::{FileSystemOverride, ProgramArgs, ensure_no_watch_or_dev};
pub use progress::{ProgressMode, ProgressReporter, is_tty};
pub use report::{
    CommandError, CommandReport, ReportArgs, ReportFormat, command_data_json,
    parse_command_payload, parse_required_command_payload, print_json_payload_report, print_report,
    report_error, report_error_with, report_from_message_payload, report_from_payload,
};
pub(crate) use target::target_overrides_from_args;
pub use watch::{WatchCompileJson, WatchCompileReason, WatchReporter};
