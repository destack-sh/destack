use std::io::Write;
use std::ops::AsyncFnOnce;
use std::time::Duration;

use futures::{FutureExt, pin_mut, select_biased};
use serde::de::DeserializeOwned;
use serde_json::Value;
use tspp_workspace::{
    CommandMessagePayload, CommandOutputChunk, CommandProgress, Message, MessageKind, OutputStream,
    Workspace,
};

use super::result::CommandResult;
use crate::common::{
    CommandReport, DiagnosticFormat, FormatOptions, LineWriter, ProgramArgs, ProgressReporter,
    ReportArgs, collect_diagnostics_json, format_diagnostics_with_writer, parse_command_payload,
    parse_required_command_payload, print_report, report_error, report_from_message_payload,
    report_from_payload,
};
use crate::console;
use crate::diagnostic::{ConsoleError, ConsoleResult};

/// Run a workspace command.
pub(crate) async fn run_workspace_command<Run>(
    program: &ProgramArgs,
    run: Run,
    progress: Option<&ProgressReporter>,
) -> ConsoleResult<CommandResult>
where
    for<'a> Run:
        AsyncFnOnce(&'a Workspace, Option<CommandProgress>) -> ConsoleResult<CommandResult>,
{
    let workspace = program.workspace()?;
    let host = workspace.session().repository().host().clone();
    let result = run_with_progress(workspace.as_ref(), progress, run).await;

    // release workspace revisions before waiting for their queued writes
    workspace.close();
    drop(workspace);
    let cache_result = host.flush_artifact_cache();

    // preserve the command and cache outcomes
    match (result, cache_result) {
        (Ok(result), Ok(())) => Ok(result),
        (Err(error), Ok(())) => Err(error),
        (Ok(_), Err(error)) => Err(ConsoleError::message(format!(
            "artifact cache flush failed: {error}"
        ))),
        (Err(error), Err(cache_error)) => Err(ConsoleError::message(format!(
            "{error}; artifact cache flush failed: {cache_error}"
        ))),
    }
}

/// Execute a workspace command or emit a CLI error report.
pub(crate) async fn run_workspace_command_or_report<Run>(
    command: &str,
    report_args: &ReportArgs,
    program: &ProgramArgs,
    run: Run,
) -> Result<CommandResult, i32>
where
    for<'a> Run:
        AsyncFnOnce(&'a Workspace, Option<CommandProgress>) -> ConsoleResult<CommandResult>,
{
    run_workspace_command(program, run, None)
        .await
        .map_err(|error| report_error(command, report_args, &error.to_string()))
}

/// Shared metadata for one command completion summary.
#[derive(Debug, Clone, Copy)]
pub(crate) struct CommandSummary<'a> {
    /// Verb used for the final summary.
    pub(crate) verb: &'a str,
    /// Number of modules processed.
    pub(crate) modules: usize,
    /// Number of targets processed.
    pub(crate) targets: usize,
    /// Complete command wall time.
    pub(crate) duration: Duration,
}

/// Run a workspace command that returns a required typed payload.
pub(crate) async fn run_workspace_payload_command_or_report<T, JsonFn, TextFn, Run>(
    command: &str,
    report_args: &ReportArgs,
    program: &ProgramArgs,
    run: Run,
    payload_label: &str,
    json_report: JsonFn,
    text_report: TextFn,
) -> i32
where
    T: DeserializeOwned,
    JsonFn: FnOnce(i32, T, Value) -> CommandReport,
    TextFn: FnOnce(i32, T),
    for<'a> Run:
        AsyncFnOnce(&'a Workspace, Option<CommandProgress>) -> ConsoleResult<CommandResult>,
{
    // execute the command and decode the payload
    let result = match run_workspace_command_or_report(command, report_args, program, run).await {
        Ok(result) => result,
        Err(code) => return code,
    };
    let (payload, payload_value) = match parse_required_command_payload::<T>(
        command,
        report_args,
        Some(&result.response.data),
        payload_label,
    ) {
        Ok(payload) => payload,
        Err(code) => return code,
    };

    // emit workspace output before command specific rendering
    emit_workspace_text_output(
        report_args,
        &result.response.messages,
        &result.response.output,
    );

    let exit_code = result.response.exit_code;

    // emit the structured report when requested
    if report_args.is_json() {
        let mut report = json_report(exit_code, payload, payload_value);
        report.trace = result.response.trace.clone();
        print_report(&report, report_args.format());
        return exit_code;
    }

    // otherwise render the payload in text mode
    text_report(exit_code, payload);
    result.emit_timings(None);
    exit_code
}

/// Run one command with optional workspace progress reporting.
async fn run_with_progress<Run>(
    workspace: &Workspace,
    progress: Option<&ProgressReporter>,
    run: Run,
) -> ConsoleResult<CommandResult>
where
    for<'a> Run:
        AsyncFnOnce(&'a Workspace, Option<CommandProgress>) -> ConsoleResult<CommandResult>,
{
    // connect workspace progress to the CLI progress reporter
    if let Some(reporter) = progress {
        let (progress, mut events) = CommandProgress::channel();
        let command = run(workspace, Some(progress)).fuse();
        pin_mut!(command);

        loop {
            let event = events.receive().fuse();
            pin_mut!(event);

            // drain ready progress before accepting terminal command completion
            select_biased! {
                event = event => {
                    let Some(event) = event else {
                        let result = command.await;
                        reporter.stop();

                        return result;
                    };

                    reporter.update_workspace(&event.task, event.message.as_deref());
                },
                result = command => {
                    reporter.stop();

                    return result;
                },
            }
        }
    }

    run(workspace, None).await
}

/// Finish a workspace command that primarily reports diagnostics.
pub(crate) fn finish_diagnostic_command(
    command: &str,
    report_args: &ReportArgs,
    result: &CommandResult,
    json_format_options: &FormatOptions,
    text_format_options: &FormatOptions,
    line_writer: Option<&LineWriter>,
    summary: Option<CommandSummary<'_>>,
    data: Option<Value>,
) -> i32 {
    // emit workspace output only for text mode
    if !report_args.is_json() {
        emit_workspace_text_output(
            report_args,
            &result.response.messages,
            &result.response.output,
        );
    }

    // build a structured diagnostics report when requested
    if report_args.is_json() {
        let (output, format_result) = collect_diagnostics_json(
            &|file_id| result.files.get(&file_id).cloned(),
            &result.diagnostics,
            json_format_options,
        );
        let diagnostic_exit_code = format_result.exit_code();
        let exit_code = if diagnostic_exit_code == 0 {
            result.response.exit_code
        } else {
            diagnostic_exit_code
        };
        let mut report = report_from_payload(command, exit_code, data, None, None);
        report.trace = result.response.trace.clone();
        report.diagnostics = Some(output);
        print_report(&report, report_args.format());

        return exit_code;
    }

    // render diagnostics for text oriented output
    let format_result = format_diagnostics_with_writer(
        &|file_id| result.files.get(&file_id).cloned(),
        &result.diagnostics,
        text_format_options,
        result.response.module_count,
        line_writer,
    );

    // keep warning threshold failures loud in text mode
    if format_result.max_warnings_exceeded {
        let warning = format!(
            "warning count ({}) exceeds --max-warnings ({})",
            format_result.warning_count,
            text_format_options.max_warnings.unwrap_or(0)
        );
        let warning = console::yellow(&warning);

        // print above an active progress line when present
        if let Some(line_writer) = line_writer {
            line_writer(&warning);
        } else {
            eprintln!("{warning}");
        }
    }

    // render requested timings before the final command status
    result.emit_timings(summary.map(|summary| summary.duration));

    // finish text diagnostics with the compact command status
    if matches!(text_format_options.format, DiagnosticFormat::Text)
        && let Some(summary) = summary
    {
        print_command_summary(
            &summary,
            format_result.error_count,
            format_result.warning_count,
            format_result.max_warnings_exceeded,
            line_writer,
        );
    }

    if format_result.max_warnings_exceeded {
        return 1;
    }

    let diagnostic_exit_code = format_result.exit_code();
    if diagnostic_exit_code == 0 {
        result.response.exit_code
    } else {
        diagnostic_exit_code
    }
}

/// Emit output for a workspace command that returns a message payload.
pub(crate) fn finish_workspace_message_command(
    command: &str,
    report_args: &ReportArgs,
    result: &CommandResult,
) -> i32 {
    // emit text output for non json modes
    if !report_args.is_json() {
        emit_workspace_text_output(
            report_args,
            &result.response.messages,
            &result.response.output,
        );
        result.emit_timings(None);
        return result.response.exit_code;
    }

    // decode the message payload for structured output
    let payload = match parse_command_payload::<CommandMessagePayload>(
        command,
        report_args,
        Some(&result.response.data),
        command,
        false,
    ) {
        Ok(payload) => payload,
        Err(code) => return code,
    };

    // emit json report output
    let exit_code = result.response.exit_code;
    let mut report = report_from_message_payload(command, exit_code, payload);
    report.trace = result.response.trace.clone();
    print_report(&report, report_args.format());
    exit_code
}

/// Emit workspace messages and output for text mode.
pub(crate) fn emit_workspace_text_output(
    report_args: &ReportArgs,
    messages: &[Message],
    output: &[CommandOutputChunk],
) {
    // skip emission when json output is enabled
    if report_args.is_json() {
        return;
    }

    // emit message records first
    emit_workspace_messages(messages);

    // emit command output when available
    if let Err(error) = emit_command_output(output) {
        console::error(&error.to_string());
    }
}

/// Emit command output chunks to stdout and stderr.
pub(crate) fn emit_command_output(output: &[CommandOutputChunk]) -> ConsoleResult<()> {
    // write buffered chunks to stdout/stderr
    let mut stdout = std::io::stdout();
    let mut stderr = std::io::stderr();
    for chunk in output {
        let result = match chunk.stream {
            OutputStream::Stdout => stdout.write_all(&chunk.bytes),
            OutputStream::Stderr => stderr.write_all(&chunk.bytes),
        };
        if let Err(error) = result {
            return Err(ConsoleError::message(format!(
                "failed to write command output: {error}"
            )));
        }
    }

    // flush outputs to ensure they are visible
    if let Err(error) = stdout.flush() {
        return Err(ConsoleError::message(format!(
            "failed to flush stdout: {error}"
        )));
    }
    if let Err(error) = stderr.flush() {
        return Err(ConsoleError::message(format!(
            "failed to flush stderr: {error}"
        )));
    }

    Ok(())
}

/// Emit workspace message records to the console.
pub(crate) fn emit_workspace_messages(messages: &[Message]) {
    // print each message based on severity
    for message in messages {
        match message.kind {
            MessageKind::Info => console::info(&message.message),
            MessageKind::Warning => console::warn(&message.message),
            MessageKind::Error => console::error(&message.message),
        }
    }
}

/// Print a compact command completion summary.
/// Clean runs lead with a green check, failing runs with a red cross
/// and the error count; counts of one drop their noise words.
fn print_command_summary(
    summary: &CommandSummary<'_>,
    errors: usize,
    warnings: usize,
    is_warning_limit_exceeded: bool,
    line_writer: Option<&LineWriter>,
) {
    let status = if errors > 0 || is_warning_limit_exceeded {
        console::red("✗")
    } else if warnings > 0 {
        console::color_for_stream("✓", "33", console::Stream::Stderr)
    } else {
        console::green("✓")
    };

    let command = format!(
        "{} {}",
        summary.verb.to_lowercase(),
        pluralize(summary.modules, "module")
    );
    let command = console::style_for_stream(&command, &["1"], console::Stream::Stderr);
    let mut parts = vec![command];
    if summary.targets > 1 {
        parts.push(pluralize(summary.targets, "target"));
    }
    if errors > 0 {
        parts.push(console::red(&pluralize(errors, "error")));
    }
    if warnings > 0 {
        parts.push(console::color_for_stream(
            &pluralize(warnings, "warning"),
            "33",
            console::Stream::Stderr,
        ));
    }
    let duration = console::format_duration(summary.duration);
    let duration = console::style_for_stream(&duration, &["2"], console::Stream::Stderr);
    parts.push(duration);

    let line = format!("{status} {}", parts.join(" · "));
    if let Some(line_writer) = line_writer {
        line_writer(&line);
    } else {
        eprintln!("{line}");
    }
}

/// Pluralize a word for a display count.
fn pluralize(count: usize, word: &str) -> String {
    if count == 1 {
        return format!("{count} {word}");
    }

    format!("{count} {word}s")
}
