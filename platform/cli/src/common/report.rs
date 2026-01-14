use std::time::Duration;

use clap::{Args, ValueEnum};
use destack_compiler::StatsSnapshot;
use serde::Serialize;
use serde_json::Value;

use crate::common::compile::print_no_input_help;
use crate::common::format::DiagnosticOutputJson;
use crate::console;

/// Schema version for command reports.
pub const REPORT_SCHEMA_VERSION: u32 = 1;

/// Output format for command reports.
#[derive(Clone, Copy, Debug, Default, ValueEnum)]
pub enum ReportFormat {
    /// Human-readable text output.
    #[default]
    Text,
    /// JSON output for tooling integration.
    Json,
}

impl ReportFormat {
    /// Whether this report format is JSON.
    pub fn is_json(self) -> bool {
        // check for json enum variant
        matches!(self, Self::Json)
    }
}

/// Common report formatting arguments for commands.
#[derive(Args, Debug, Clone, Default)]
pub struct ReportArgs {
    /// Output format for the command (text or json).
    #[arg(long = "output-format", value_enum)]
    pub output_format: Option<ReportFormat>,

    /// Emit JSON output (shorthand for --output-format json).
    #[arg(long)]
    pub json: bool,
}

impl ReportArgs {
    /// Resolve the requested report format.
    pub fn format(&self) -> ReportFormat {
        // honor the json flag first
        if self.json {
            return ReportFormat::Json;
        }

        // fall back to the explicit format or text
        self.output_format.unwrap_or(ReportFormat::Text)
    }

    /// Whether JSON output was requested.
    pub fn is_json(&self) -> bool {
        // compare the resolved format
        matches!(self.format(), ReportFormat::Json)
    }
}

/// Result status for a command report.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandStatus {
    /// Command completed successfully.
    Success,
    /// Command failed.
    Failure,
}

/// Summary statistics for command execution.
#[derive(Debug, Clone, Serialize)]
pub struct CommandStats {
    /// Elapsed time in milliseconds.
    pub elapsed_ms: u64,
    /// Number of tasks completed by the compiler.
    pub tasks_completed: usize,
    /// Number of tasks failed by the compiler.
    pub tasks_failed: usize,
    /// Number of modules processed.
    pub modules_processed: usize,
    /// Number of lines processed.
    pub lines_processed: usize,
    /// Number of slow tasks detected.
    pub slow_tasks: usize,
}

impl CommandStats {
    /// Build stats from a compiler snapshot.
    pub fn from_snapshot(snapshot: &StatsSnapshot) -> Self {
        // map snapshot fields into the report summary
        let elapsed_ms = duration_to_ms(snapshot.elapsed);
        let modules_processed = snapshot.modules_processed();
        Self {
            elapsed_ms,
            tasks_completed: snapshot.tasks_completed,
            tasks_failed: snapshot.tasks_failed,
            modules_processed,
            lines_processed: snapshot.lines_processed,
            slow_tasks: snapshot.slow_tasks,
        }
    }
}

/// Report output for a command invocation.
#[derive(Debug, Serialize)]
pub struct CommandReport {
    /// Report schema version.
    pub schema_version: u32,
    /// Command name.
    pub command: String,
    /// Result status.
    pub status: CommandStatus,
    /// Exit code returned by the command.
    pub exit_code: i32,
    /// Short summary for text output.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    /// Diagnostics payload for tooling.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diagnostics: Option<DiagnosticOutputJson>,
    /// Compiler stats snapshot for tooling.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stats: Option<CommandStats>,
    /// Command-specific payload.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl CommandReport {
    /// Build a success report with the given exit code.
    pub fn success(command: &str, exit_code: i32) -> Self {
        // create a report with success status
        Self {
            schema_version: REPORT_SCHEMA_VERSION,
            command: command.to_string(),
            status: CommandStatus::Success,
            exit_code,
            summary: None,
            diagnostics: None,
            stats: None,
            data: None,
        }
    }

    /// Build a failure report with the given exit code.
    pub fn failure(command: &str, exit_code: i32) -> Self {
        // create a report with failure status
        Self {
            schema_version: REPORT_SCHEMA_VERSION,
            command: command.to_string(),
            status: CommandStatus::Failure,
            exit_code,
            summary: None,
            diagnostics: None,
            stats: None,
            data: None,
        }
    }
}

/// Print a command report in the requested format.
pub fn print_report(report: &CommandReport, format: ReportFormat) {
    // format and emit the report payload
    match format {
        ReportFormat::Text => {
            if let Some(summary) = report.summary.as_ref() {
                match report.status {
                    CommandStatus::Success => console::success(summary),
                    CommandStatus::Failure => console::error(summary),
                }
            }
        }
        ReportFormat::Json => {
            // serialize as pretty json
            if let Ok(json) = serde_json::to_string_pretty(report) {
                println!("{json}");
            }
        }
    }
}

/// Report a command error using text or JSON output.
pub fn report_error(command: &str, report_args: &ReportArgs, message: &str) -> i32 {
    if report_args.is_json() {
        let mut report = CommandReport::failure(command, 1);
        report.summary = Some(message.to_string());
        print_report(&report, report_args.format());
    } else {
        console::error(&format!("error: {message}"));
    }
    1
}

/// Report a missing input error with optional usage help.
pub fn report_no_input(command: &str, report_args: &ReportArgs) -> i32 {
    if report_args.is_json() {
        return report_error(command, report_args, "no input provided");
    }

    print_no_input_help(command);
    1
}

/// Convert a duration to milliseconds with saturation.
fn duration_to_ms(duration: Duration) -> u64 {
    // guard against overflow on large durations
    let millis = duration.as_millis();
    u64::try_from(millis).unwrap_or(u64::MAX)
}
