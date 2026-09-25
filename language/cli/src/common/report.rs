use clap::{Args, ValueEnum};
use tspp_repository::TraceSnapshot;
use tspp_serde as serde;
use tspp_workspace::CommandMessagePayload;

use ::serde::Serialize;
use ::serde::de::DeserializeOwned;
use serde_json as json;

use crate::common::format::DiagnosticOutputJson;
use crate::console;

/// Reflect version for command reports.
pub const REPORT_SCHEMA_VERSION: u32 = 6;

/// Output format for command reports.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, ValueEnum)]
pub enum ReportFormat {
    /// Rich human-readable output.
    #[default]
    Human,
    /// Plain line-oriented text output.
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
    /// Output format for the command.
    #[arg(long = "output-format", value_enum)]
    pub output_format: Option<ReportFormat>,

    /// Emit JSON output (shorthand for --output-format json).
    #[arg(long, conflicts_with = "output_format")]
    pub json: bool,
}

impl ReportArgs {
    /// Resolve the requested report format.
    pub fn format(&self) -> ReportFormat {
        // honor the json flag first
        if self.json {
            return ReportFormat::Json;
        }

        // fall back to rich human output
        self.output_format.unwrap_or(ReportFormat::Human)
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<json::Value>,
    /// Command timing trace when requested.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "schema", schemars(with = "Option<json::Value>"))]
    pub trace: Option<TraceSnapshot>,
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
            trace: None,
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
            trace: None,
        }
    }
}

/// Print a command report in the requested format.
pub fn print_report(report: &CommandReport, format: ReportFormat) {
    // format and emit the report payload
    match format {
        ReportFormat::Human | ReportFormat::Text => {
            if let Some(summary) = report.summary.as_ref() {
                match report.status {
                    CommandStatus::Success => console::success(summary),
                    CommandStatus::Failure => console::error(summary),
                }
            }
        }
        ReportFormat::Json => {
            // serialize as pretty json
            if let Ok(output) = json::to_string_pretty(report) {
                println!("{output}");
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

/// Decode a structured workspace payload into a typed payload and raw JSON value.
pub fn parse_command_payload<T: DeserializeOwned>(
    command: &str,
    report_args: &ReportArgs,
    payload: Option<&serde::Value>,
    payload_label: &str,
    required: bool,
) -> Result<Option<(T, json::Value)>, i32> {
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

    // convert the protocol payload into report JSON
    let value = match payload.clone().into_json() {
        Ok(value) => value,
        Err(error) => {
            let message = format!("invalid {payload_label} payload: {error}");
            return Err(report_error(command, report_args, &message));
        }
    };

    // deserialize into the typed payload
    let parsed = match json::from_value(value.clone()) {
        Ok(payload) => payload,
        Err(error) => {
            let message = format!("invalid {payload_label} payload: {error}");
            return Err(report_error(command, report_args, &message));
        }
    };

    Ok(Some((parsed, value)))
}

/// Decode a required workspace payload into a typed payload and raw JSON value.
pub fn parse_required_command_payload<T: DeserializeOwned>(
    command: &str,
    report_args: &ReportArgs,
    payload: Option<&serde::Value>,
    payload_label: &str,
) -> Result<(T, json::Value), i32> {
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

/// Convert optional command payload data into report JSON.
pub fn command_data_json(
    command: &str,
    report_args: &ReportArgs,
    data: Option<&serde::Value>,
) -> Result<Option<json::Value>, i32> {
    // return early when the command has no payload
    let Some(data) = data else {
        return Ok(None);
    };

    // convert protocol JSON into CLI report JSON
    data.clone().into_json().map(Some).map_err(|error| {
        report_error(
            command,
            report_args,
            &format!("invalid command payload: {error}"),
        )
    })
}

/// Build a command report from optional payload data.
pub fn report_from_payload(
    command: &str,
    exit_code: i32,
    data: Option<json::Value>,
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
    payload: Option<(CommandMessagePayload, json::Value)>,
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
    let data = match json::to_value(payload) {
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
