use std::io::Write;
use std::path::Path;

use destack_workspace::{
    CommandMessagePayload, CommandOutputChunk, CommandProgress, Message, MessageKind, OutputStream,
    ProgressEvent, RunPayload, Workspace,
};
use serde::de::DeserializeOwned;
use serde_json::Value;

use super::result::CommandResult;
use crate::common::{
    CommandError, CommandReport, DiagnosticFormat, FormatOptions, LineWriter, ProgramArgs,
    ProgressReporter, ReportArgs, collect_diagnostics_json, format_diagnostics_with_writer,
    parse_command_payload, parse_required_command_payload, print_report, report_error,
    report_from_message_payload, report_from_payload,
};
use crate::console;
use crate::diagnostic::{ConsoleError, ConsoleResult};

/// Run a workspace command.
pub(crate) fn run_workspace_command(
    program: &ProgramArgs,
    run: impl for<'a> FnOnce(
        &dyn Workspace,
        &Path,
        Option<CommandProgress<'a>>,
    ) -> ConsoleResult<CommandResult>,
    progress: Option<&ProgressReporter>,
) -> ConsoleResult<CommandResult> {
    let (workspace, roots) = program.workspace(None)?;
    let Some(root) = roots.first().cloned() else {
        return Err(ConsoleError::message("roots are empty"));
    };

    let result = run_with_progress(workspace.as_ref(), &root, progress, run)?;

    Ok(result)
}

/// Execute a workspace command or emit a CLI error report.
pub(crate) fn run_workspace_command_or_report(
    command: &str,
    report_args: &ReportArgs,
    program: &ProgramArgs,
    run: impl for<'a> FnOnce(
        &dyn Workspace,
        &Path,
        Option<CommandProgress<'a>>,
    ) -> ConsoleResult<CommandResult>,
) -> Result<CommandResult, i32> {
    run_workspace_command(program, run, None)
        .map_err(|error| report_error(command, report_args, &error.to_string()))
}

/// Shared summary metadata for diagnostic commands.
#[derive(Debug, Clone, Copy)]
pub(crate) struct DiagnosticCommandSummary<'a> {
    /// Verb used for the final summary.
    pub(crate) verb: &'a str,
    /// Number of modules processed.
    pub(crate) modules: usize,
    /// Number of targets processed.
    pub(crate) targets: usize,
}

/// Run a workspace command that returns a required typed payload.
pub(crate) fn run_workspace_payload_command_or_report<T, JsonFn, TextFn>(
    command: &str,
    report_args: &ReportArgs,
    program: &ProgramArgs,
    run: impl for<'a> FnOnce(
        &dyn Workspace,
        &Path,
        Option<CommandProgress<'a>>,
    ) -> ConsoleResult<CommandResult>,
    payload_label: &str,
    json_report: JsonFn,
    text_report: TextFn,
) -> i32
where
    T: DeserializeOwned,
    JsonFn: FnOnce(i32, T, Value) -> CommandReport,
    TextFn: FnOnce(i32, T),
{
    // execute the command and decode the payload
    let result = match run_workspace_command_or_report(command, report_args, program, run) {
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
    result.emit_timings();
    exit_code
}

/// Run one command with optional workspace progress reporting.
fn run_with_progress(
    workspace: &dyn Workspace,
    root: &Path,
    progress: Option<&ProgressReporter>,
    run: impl for<'a> FnOnce(
        &dyn Workspace,
        &Path,
        Option<CommandProgress<'a>>,
    ) -> ConsoleResult<CommandResult>,
) -> ConsoleResult<CommandResult> {
    // connect workspace progress to the CLI progress reporter
    if let Some(reporter) = progress {
        let notify = |event: ProgressEvent| {
            reporter.update_workspace(&event.task, event.message.as_deref(), event.done);
        };
        let progress = CommandProgress::new(&notify);

        return run(workspace, root, Some(progress));
    }

    run(workspace, root, None)
}

/// Finish a workspace command that primarily reports diagnostics.
pub(crate) fn finish_diagnostic_command(
    command: &str,
    report_args: &ReportArgs,
    result: &CommandResult,
    json_format_options: &FormatOptions,
    text_format_options: &FormatOptions,
    line_writer: Option<&LineWriter>,
    summary: Option<DiagnosticCommandSummary<'_>>,
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

    // emit a compact command summary after text diagnostics
    if matches!(text_format_options.format, DiagnosticFormat::Text)
        && let Some(summary) = summary
    {
        print_diagnostic_command_summary(
            &summary,
            format_result.error_count,
            format_result.warning_count,
            line_writer,
        );
    }

    // keep warning threshold failures loud in text mode
    if format_result.max_warnings_exceeded {
        console::warn(&format!(
            "warning count ({}) exceeds --max-warnings ({})",
            format_result.warning_count,
            text_format_options.max_warnings.unwrap_or(0)
        ));
        return 1;
    }

    let diagnostic_exit_code = format_result.exit_code();
    if diagnostic_exit_code == 0 {
        result.response.exit_code
    } else {
        diagnostic_exit_code
    }
}

/// Finish a run command with diagnostic and payload rendering.
pub(crate) fn finish_run_command(
    command: &str,
    report_args: &ReportArgs,
    result: &CommandResult,
) -> i32 {
    // render diagnostics before payload output
    if report_args.is_json() {
        let json_options = FormatOptions {
            format: DiagnosticFormat::Json,
            ..FormatOptions::default()
        };
        let (output, format_result) = collect_diagnostics_json(
            &|file_id| result.files.get(&file_id).cloned(),
            &result.diagnostics,
            &json_options,
        );

        if format_result.exit_code() != 0 {
            let mut report = CommandReport::failure(command, format_result.exit_code());
            report.trace = result.response.trace.clone();
            report.diagnostics = Some(output);
            print_report(&report, report_args.format());
            return format_result.exit_code();
        }
    } else {
        let text_options = FormatOptions::default();
        let format_result = format_diagnostics_with_writer(
            &|file_id| result.files.get(&file_id).cloned(),
            &result.diagnostics,
            &text_options,
            result.response.module_count,
            None,
        );
        if format_result.exit_code() != 0 {
            result.emit_timings();
            return format_result.exit_code();
        }
    }

    // emit workspace text output before the final run status
    emit_workspace_text_output(
        report_args,
        &result.response.messages,
        &result.response.output,
    );

    let exit_code = result.response.exit_code;

    // print the structured payload for json output
    if report_args.is_json() {
        let payload = match parse_command_payload::<RunPayload>(
            command,
            report_args,
            Some(&result.response.data),
            "run",
            false,
        ) {
            Ok(payload) => payload,
            Err(code) => return code,
        };

        let (summary, error, data) = match payload {
            Some((RunPayload::RuntimeError { message }, value)) => {
                let error = Some(CommandError::new("runtime_error", "run", message.clone()));
                (Some(message), error, Some(value))
            }
            Some((RunPayload::Value { .. }, value)) => (None, None, Some(value)),
            None => (None, None, None),
        };
        let mut report = report_from_payload(command, exit_code, data, summary, error);
        report.trace = result.response.trace.clone();
        print_report(&report, report_args.format());
    } else if exit_code != 0 {
        console::warn(&format!("process exited with code {exit_code}"));
    }
    if !report_args.is_json() {
        result.emit_timings();
    }

    exit_code
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
        result.emit_timings();
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

/// Print a compact diagnostic command summary.
/// Clean runs lead with a green check, failing runs with a red cross
/// and the error count; counts of one drop their noise words.
fn print_diagnostic_command_summary(
    summary: &DiagnosticCommandSummary<'_>,
    errors: usize,
    warnings: usize,
    line_writer: Option<&LineWriter>,
) {
    let status = if errors > 0 {
        console::red("✗")
    } else if warnings > 0 {
        console::color_for_stream("✓", "33", console::Stream::Stderr)
    } else {
        console::green("✓")
    };

    let mut parts = vec![console::bold(&format!(
        "{} {}",
        summary.verb.to_lowercase(),
        pluralize(summary.modules, "module")
    ))];
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
