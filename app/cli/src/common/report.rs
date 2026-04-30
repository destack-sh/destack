use clap::{Args, ValueEnum};
use destack_daemon::protocol::{BinaryPayload, CommandMessagePayload};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::common::compile::print_no_input_help;
use crate::common::format::DiagnosticOutputJson;
use crate::console;

/// Schema version for command reports.
pub const REPORT_SCHEMA_VERSION: u32 = 4;

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
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum CommandStatus {
    /// Command completed successfully.
    Success,
    /// Command failed.
    Failure,
}

/// Structured error payload for command failures.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct CommandError {
    /// Machine-readable error code.
    pub code: String,
    /// Error category for tooling.
    pub kind: String,
    /// Human-readable error message.
    pub message: String,
}

impl CommandError {
    /// Build a structured command error.
    pub fn new(
        code: impl Into<String>,
        kind: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code: code.into(),
            kind: kind.into(),
            message: message.into(),
        }
    }
}

/// Report output for a command invocation.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
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
    /// Structured error payload when diagnostics are not available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<CommandError>,
    /// Command-specific payload.
    #[cfg_attr(feature = "schema", schemars(schema_with = "schema_any"))]
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
            error: None,
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
            error: None,
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
    report_error_with(
        command,
        report_args,
        CommandError::new("cli_error", "cli", message),
    )
}

/// Report a command error with structured error details.
pub fn report_error_with(command: &str, report_args: &ReportArgs, error: CommandError) -> i32 {
    if report_args.is_json() {
        let mut report = CommandReport::failure(command, 1);
        report.summary = Some(error.message.clone());
        report.error = Some(error);
        print_report(&report, report_args.format());
    } else {
        console::error(&format!("error: {}", error.message));
    }
    1
}

/// Decode a structured daemon payload into a typed payload and raw JSON value.
pub fn parse_command_payload<T: DeserializeOwned>(
    command: &str,
    report_args: &ReportArgs,
    payload: Option<&BinaryPayload>,
    payload_label: &str,
    required: bool,
) -> Result<Option<(T, Value)>, i32> {
    // return early when the payload is optional and missing
    let payload = match payload {
        Some(payload) => payload,
        None => {
            if required {
                let message = format!("{payload_label} payload missing");
                return Err(report_error(command, report_args, &message));
            }
            return Ok(None);
        }
    };

    // decode the payload as json
    let value = match payload.to_json_value() {
        Ok(value) => value,
        Err(error) => {
            let message = format!("invalid {payload_label} payload: {error}");
            return Err(report_error(command, report_args, &message));
        }
    };

    // deserialize into the typed payload
    let parsed = match serde_json::from_value(value.clone()) {
        Ok(payload) => payload,
        Err(error) => {
            let message = format!("invalid {payload_label} payload: {error}");
            return Err(report_error(command, report_args, &message));
        }
    };

    Ok(Some((parsed, value)))
}

/// Decode a required daemon payload into a typed payload and raw JSON value.
pub fn parse_required_command_payload<T: DeserializeOwned>(
    command: &str,
    report_args: &ReportArgs,
    payload: Option<&BinaryPayload>,
    payload_label: &str,
) -> Result<(T, Value), i32> {
    // decode the payload with the required flag
    let payload = parse_command_payload::<T>(command, report_args, payload, payload_label, true)?;

    // guard against missing payloads
    match payload {
        Some(payload) => Ok(payload),
        None => Err(report_error(
            command,
            report_args,
            &format!("{payload_label} payload missing"),
        )),
    }
}

/// Build a command report from optional payload data.
pub fn report_from_payload(
    command: &str,
    exit_code: i32,
    data: Option<Value>,
    summary: Option<String>,
    error: Option<CommandError>,
) -> CommandReport {
    // build base report from the exit code
    let mut report = if exit_code == 0 {
        CommandReport::success(command, exit_code)
    } else {
        CommandReport::failure(command, exit_code)
    };
    report.data = data;
    report.summary = summary;
    report.error = error;
    report
}

/// Build a command report from a standard message payload.
pub fn report_from_message_payload(
    command: &str,
    exit_code: i32,
    payload: Option<(CommandMessagePayload, Value)>,
) -> CommandReport {
    // derive payload-specific report fields
    let (summary, error, data) = match payload {
        Some((payload, value)) => {
            let summary = Some(payload.message.clone());
            let error = if exit_code == 0 {
                None
            } else {
                Some(CommandError::new(
                    format!("{command}_failed"),
                    "command",
                    payload.message,
                ))
            };
            (summary, error, Some(value))
        }
        None => (None, None, None),
    };

    report_from_payload(command, exit_code, data, summary, error)
}

/// Print a JSON report with a serialized payload.
pub fn print_json_payload_report<T: Serialize>(
    command: &str,
    report_args: &ReportArgs,
    exit_code: i32,
    payload: &T,
) -> Result<(), i32> {
    // skip if json output is not requested
    if !report_args.is_json() {
        return Ok(());
    }

    // serialize the payload for the report
    let data = match serde_json::to_value(payload) {
        Ok(data) => data,
        Err(error) => {
            return Err(report_error(
                command,
                report_args,
                &format!("failed to serialize payload: {error}"),
            ));
        }
    };

    // emit the report payload
    let report = report_from_payload(command, exit_code, Some(data), None, None);
    print_report(&report, report_args.format());
    Ok(())
}

/// Report a missing input error with optional usage help.
pub fn report_no_input(command: &str, report_args: &ReportArgs) -> i32 {
    if report_args.is_json() {
        return report_error_with(
            command,
            report_args,
            CommandError::new("no_input", "usage", "no input provided"),
        );
    }

    print_no_input_help(command);
    1
}

#[cfg(feature = "schema")]
fn schema_any(_gen: &mut schemars::SchemaGenerator) -> schemars::Schema {
    true.into()
}
