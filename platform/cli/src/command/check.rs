use crate::common::format::{DiagnosticFormat, FormatOptions, format_diagnostics_with_writer};
use crate::common::{
    CommandReport, DiagnosticArgs, InputArgs, InputSource, ProgramArgs, ProgressMode,
    ProgressReporter, ReportArgs, StatsSummary, WatchCompileReason, collect_diagnostics_json,
    is_tty, print_command_stats_summary, print_report, report_error,
};
use crate::console;
use crate::error::CliResult;
use crate::pipeline::daemon::{
    CommandOptionsBuilder, ProtocolDaemonClient, command_inputs_from_sources,
    command_stats_from_protocol, emit_daemon_text_output, run_daemon_command,
};
use crate::pipeline::input::{ResolveSourcesError, resolve_sources};
use crate::pipeline::watch::{
    WatchCompileContext, WatchContext, WatchLoopAction, WatchLoopOptions, build_daemon_options,
    build_watch_context, build_watch_loop_options, emit_watch_compile_report, run_watch_loop,
    watch_error,
};
use clap::{Args, ValueEnum};
use destack_compiler::CompilerEventHandler;
use destack_daemon::protocol::{
    CommandCheckOptions, CommandLintOptions, CommandPayload, CommonCommandOptions,
};
use destack_source::DiagnosticOptions;

/// State for check watch mode.
struct CheckWatchState {
    /// The resolved input sources.
    sources: Vec<InputSource>,
}

/// Output format for diagnostics.
#[derive(Debug, Clone, Copy, Default, ValueEnum)]
pub enum Format {
    /// Human-readable text output (default).
    #[default]
    Text,
    /// JSON output for tooling integration.
    Json,
    /// GitHub Actions annotations format.
    Github,
}

impl From<Format> for DiagnosticFormat {
    /// Convert from CLI format to diagnostic format.
    fn from(format: Format) -> Self {
        match format {
            Format::Text => DiagnosticFormat::Text,
            Format::Json => DiagnosticFormat::Json,
            Format::Github => DiagnosticFormat::Github,
        }
    }
}

/// Progress display mode.
#[derive(Debug, Clone, Copy, Default, ValueEnum)]
pub enum Progress {
    /// Auto-detect: show progress on TTY, hide otherwise.
    #[default]
    Auto,
    /// Always show progress spinner.
    On,
    /// Never show progress.
    Off,
    /// Show detailed progress with task counts.
    Detailed,
}

#[derive(Args, Debug, Clone)]
pub struct CheckArgs {
    /// Input arguments.
    #[command(flatten)]
    pub input: InputArgs,

    /// The program options.
    #[command(flatten)]
    pub program: ProgramArgs,

    /// The diagnostic options.
    #[command(flatten)]
    pub diagnostics: DiagnosticArgs,

    /// Report output options.
    #[command(flatten)]
    pub report: ReportArgs,

    /// Automatically fix problems.
    #[arg(long)]
    pub fix: bool,

    /// Apply unsafe fixes in addition to safe fixes (requires --fix).
    #[arg(long = "unsafe-fixes")]
    pub unsafe_fixes: bool,

    /// Show what --fix would change without applying.
    #[arg(long)]
    pub diff: bool,

    /// Only type-check, skip linting.
    #[arg(long = "no-lint")]
    pub no_lint: bool,

    /// Output format (text, json, github).
    #[arg(long, short = 'f', value_enum, default_value = "text")]
    pub format: Format,

    /// Only show errors, suppress warnings.
    #[arg(long, short = 'q')]
    pub quiet: bool,

    /// Suppress diagnostics output, still print the summary line in text mode.
    #[arg(long = "no-diagnostics")]
    pub no_diagnostics: bool,

    /// Exit with error if warning count exceeds this threshold.
    #[arg(long = "max-warnings", value_name = "N")]
    pub max_warnings: Option<usize>,

    /// Show statistics grouped by rule.
    #[arg(long)]
    pub statistics: bool,

    /// Show progress indicator (auto, on, off, detailed).
    #[arg(long, value_enum, default_value = "auto")]
    pub progress: Progress,
}

/// Check source files for type errors and lint issues.
pub fn run(args: &CheckArgs) -> i32 {
    run_with_command(args, "check")
}

/// Check source files with a custom command label.
pub fn run_with_command(args: &CheckArgs, command_name: &str) -> i32 {
    // validate flag combinations
    if !args.report.is_json() {
        if args.unsafe_fixes && !args.fix && !args.diff {
            console::warn("--unsafe-fixes has no effect without --fix or --diff");
        }

        if args.no_lint && (args.fix || args.diff) {
            console::warn("--fix and --diff have no effect with --no-lint");
        }
    }

    // reject json output with fix modes
    if args.report.is_json() && (args.fix || args.diff) {
        return report_error(
            command_name,
            &args.report,
            "--output-format json is not supported with --fix or --diff",
        );
    }

    // determine progress mode
    let progress_mode = match args.progress {
        Progress::Auto => {
            // show progress on tty unless quiet mode, json output, or stdin input
            if is_tty()
                && !args.quiet
                && !args.input.stdin
                && !matches!(args.format, Format::Json | Format::Github)
                && !args.report.is_json()
            {
                ProgressMode::Spinner
            } else {
                ProgressMode::None
            }
        }
        Progress::On => ProgressMode::Spinner,
        Progress::Off => ProgressMode::None,
        Progress::Detailed => ProgressMode::Detailed,
    };

    // create progress reporter
    let progress_reporter = ProgressReporter::with_label(progress_mode, "Checking");
    let finish_progress = || {
        if let Some(reporter) = &progress_reporter {
            reporter.finish();
        }
    };
    let event_handler = progress_reporter.as_ref().map(|p| p.handler());

    // build format options
    let format_options = FormatOptions {
        format: args.format.into(),
        quiet: args.quiet,
        max_warnings: args.max_warnings,
        statistics: args.statistics,
        suppress_diagnostics: args.no_diagnostics,
    };

    // run watch mode when requested
    if args.program.watch {
        let exit_code = run_watch(
            args,
            command_name,
            event_handler,
            &format_options,
            progress_reporter.as_ref(),
        );
        finish_progress();
        return exit_code;
    }

    let exit_code = run_check_via_daemon(
        args,
        command_name,
        &format_options,
        progress_reporter.as_ref(),
        event_handler,
    );
    finish_progress();
    exit_code
}

/// Run a single check command through the daemon.
fn run_check_via_daemon(
    args: &CheckArgs,
    command_name: &str,
    format_options: &FormatOptions,
    progress_reporter: Option<&ProgressReporter>,
    event_handler: Option<CompilerEventHandler>,
) -> i32 {
    // build diagnostic options for the daemon command
    let diagnostic_options: DiagnosticOptions = args.diagnostics.clone().into();

    // build command inputs when explicitly provided
    let inputs = if args.input.has_input() {
        let sources = match args.input.to_sources() {
            Ok(sources) => sources,
            Err(error) => {
                return report_error(command_name, &args.report, &error.to_string());
            }
        };
        match command_inputs_from_sources(&sources, args.input.file_type()) {
            Ok(inputs) => inputs,
            Err(error) => {
                return report_error(command_name, &args.report, &error.to_string());
            }
        }
    } else {
        Vec::new()
    };

    // build command payload for daemon execution
    let lint_options = CommandLintOptions {
        fix: args.fix,
        unsafe_fixes: args.unsafe_fixes,
        diff: args.diff,
    };
    let lint_enabled = !args.no_lint && !args.fix && !args.diff;
    let common = CommandOptionsBuilder::new(&args.program, Some(diagnostic_options.clone()))
        .inputs(inputs)
        .allow_dsconfig_fallback(!args.input.has_input())
        .build();
    let payload = CommandPayload::Check(CommandCheckOptions {
        lint: lint_enabled,
        lint_options,
    });

    // execute the daemon command
    let result = match run_daemon_command(
        &args.program,
        Some(diagnostic_options),
        common,
        payload,
        event_handler,
    ) {
        Ok(result) => result,
        Err(error) => return report_error(command_name, &args.report, &error.to_string()),
    };

    // emit daemon output and messages for text modes
    if !args.report.is_json() {
        emit_daemon_text_output(
            &args.report,
            &result.response.messages,
            &result.response.output,
        );
    }

    // emit json report when requested
    if args.report.is_json() {
        let json_options = FormatOptions {
            format: DiagnosticFormat::Json,
            quiet: args.quiet,
            max_warnings: args.max_warnings,
            statistics: args.statistics,
            suppress_diagnostics: false,
        };
        let (output, format_result) =
            collect_diagnostics_json(&result.files, &result.diagnostics, &json_options);
        let mut report = if format_result.exit_code() == 0 {
            CommandReport::success(command_name, 0)
        } else {
            CommandReport::failure(command_name, format_result.exit_code())
        };
        if let Some(stats) = result.response.stats.as_ref() {
            report.stats = Some(command_stats_from_protocol(stats));
        }
        report.diagnostics = Some(output);
        print_report(&report, args.report.format());
        return format_result.exit_code();
    }

    // emit formatted diagnostics
    let line_writer = progress_reporter
        .as_ref()
        .map(|reporter| reporter.line_writer());
    let module_count = result.response.module_count;
    let output_result = format_diagnostics_with_writer(
        &result.files,
        &result.diagnostics,
        format_options,
        module_count,
        line_writer.as_ref(),
    );

    // print stats summary in text mode
    if matches!(args.format, Format::Text)
        && let Some(stats) = result.response.stats.as_ref()
    {
        let summary = StatsSummary {
            verb: "Checked",
            modules: module_count,
            profiles: result.response.profile_count,
            targets: 0,
            errors: output_result.error_count,
            warnings: output_result.warning_count,
        };
        let stats = command_stats_from_protocol(stats);
        print_command_stats_summary(&summary, &stats, line_writer.as_ref());
    }

    // enforce max warning threshold when configured
    if output_result.max_warnings_exceeded {
        console::warn(&format!(
            "warning count ({}) exceeds --max-warnings ({})",
            output_result.warning_count,
            args.max_warnings.unwrap_or(0)
        ));
        return 1;
    }

    output_result.exit_code()
}

/// Run check in watch mode with incremental updates.
fn run_watch(
    args: &CheckArgs,
    command_name: &str,
    event_handler: Option<CompilerEventHandler>,
    format_options: &FormatOptions,
    progress_reporter: Option<&ProgressReporter>,
) -> i32 {
    // run with default watch settings
    run_watch_with_options(
        args,
        command_name,
        event_handler,
        format_options,
        progress_reporter,
        build_watch_loop_options(),
        || {},
        |_, _, _| {},
        false,
    )
}

/// Run check in watch mode with injected options.
#[allow(clippy::too_many_arguments)]
pub(crate) fn run_watch_with_options<StartFn, ObserveFn>(
    args: &CheckArgs,
    command_name: &str,
    event_handler: Option<CompilerEventHandler>,
    format_options: &FormatOptions,
    progress_reporter: Option<&ProgressReporter>,
    watch_loop_options: WatchLoopOptions,
    on_start: StartFn,
    mut on_compile: ObserveFn,
    is_one_shot: bool,
) -> i32
where
    StartFn: FnOnce(),
    ObserveFn: FnMut(WatchCompileReason, bool, bool),
{
    // reject unsupported combinations
    if args.fix || args.diff {
        return report_error(
            command_name,
            &args.report,
            "--watch does not support --fix or --diff yet",
        );
    }

    // reject watch mode for inline inputs
    if args.input.stdin || !args.input.eval.is_empty() || !args.input.module.is_empty() {
        return report_error(
            command_name,
            &args.report,
            "--watch requires file or directory inputs",
        );
    }

    // resolve sources for the initial compile
    let sources = match resolve_sources(&args.input, Some(&args.program), None) {
        Ok(sources) => sources,
        Err(ResolveSourcesError::NoInput) => {
            return report_error(command_name, &args.report, "no input files provided");
        }
        Err(ResolveSourcesError::Message(message)) => {
            return report_error(command_name, &args.report, &message);
        }
    };

    // build diagnostic options for the daemon command
    let diagnostic_options: DiagnosticOptions = args.diagnostics.clone().into();

    // prepare watch mode output
    let json_format_options = FormatOptions {
        format: DiagnosticFormat::Json,
        ..format_options.clone()
    };
    let session = args.program.setup();
    let WatchContext {
        roots,
        root,
        mut reporter,
    } = build_watch_context(command_name, &args.program, &args.report, &session);
    let line_writer = progress_reporter.map(|reporter| reporter.line_writer());

    // configure the daemon client for incremental updates
    let daemon_options =
        build_daemon_options(&args.program, diagnostic_options.clone(), event_handler);
    let daemon = match ProtocolDaemonClient::new(
        session.clone(),
        daemon_options,
        roots.clone(),
        &args.program,
    ) {
        Ok(daemon) => daemon,
        Err(error) => {
            let message = watch_error(&error.to_string());
            if let Some(reporter) = reporter.as_mut() {
                reporter.emit_warning(&message);
                reporter.emit_stop();
                return 1;
            }
            return report_error("check", &args.report, &message);
        }
    };

    // set up shared watch state
    let mut watch_state = CheckWatchState { sources };

    // prepare lint options for watch runs
    let lint_options = CommandLintOptions {
        fix: args.fix,
        unsafe_fixes: args.unsafe_fixes,
        diff: args.diff,
    };
    let lint_enabled = !args.no_lint && !args.fix && !args.diff;

    // helper to build command options
    let build_options =
        |sources: &[InputSource]| -> CliResult<(CommonCommandOptions, CommandPayload)> {
            let inputs = command_inputs_from_sources(sources, args.input.file_type())?;
            let common =
                CommandOptionsBuilder::new(&args.program, Some(diagnostic_options.clone()))
                    .inputs(inputs)
                    .allow_dsconfig_fallback(!args.input.has_input())
                    .build();
            let payload = CommandPayload::Check(CommandCheckOptions {
                lint: lint_enabled,
                lint_options: lint_options.clone(),
            });
            Ok((common, payload))
        };

    // compile the initial state
    let mut exit_code = match build_options(&watch_state.sources) {
        Ok((common, payload)) => match daemon.run_command(&root, common, payload) {
            Ok(result) => {
                emit_daemon_text_output(
                    &args.report,
                    &result.response.messages,
                    &result.response.output,
                );
                let stats = result
                    .response
                    .stats
                    .as_ref()
                    .map(command_stats_from_protocol);
                emit_watch_compile_report(
                    &mut reporter,
                    WatchCompileContext {
                        files: &result.files,
                        diagnostics: &result.diagnostics,
                        format_options,
                        json_format_options: &json_format_options,
                        module_count: result.response.module_count,
                        line_writer: line_writer.as_ref(),
                    },
                    stats,
                    WatchCompileReason::Startup,
                    false,
                    false,
                    None,
                )
            }
            Err(message) => {
                let message = watch_error(&message.to_string());
                if let Some(reporter) = reporter.as_mut() {
                    reporter.emit_warning(&message);
                    1
                } else {
                    report_error(command_name, &args.report, &message)
                }
            }
        },
        Err(message) => {
            let message = watch_error(&message.to_string());
            if let Some(reporter) = reporter.as_mut() {
                reporter.emit_warning(&message);
                1
            } else {
                report_error(command_name, &args.report, &message)
            }
        }
    };

    // run the watch loop for incremental updates
    exit_code = run_watch_loop(
        &daemon,
        roots,
        &mut reporter,
        watch_loop_options,
        &mut watch_state,
        move |_| on_start(),
        |state| {
            // refresh sources when a rescan is requested
            state.sources = match resolve_sources(&args.input, Some(&args.program), None) {
                Ok(sources) => sources,
                Err(ResolveSourcesError::NoInput) => {
                    return Err(watch_error("no input files after rescan").into());
                }
                Err(ResolveSourcesError::Message(message)) => {
                    return Err(watch_error(&message).into());
                }
            };

            Ok(())
        },
        |state, reporter, reason, batch_id, updated, requires_rescan| {
            // build options for the updated sources
            let (common, payload) = match build_options(&state.sources) {
                Ok(options) => options,
                Err(message) => {
                    let message = watch_error(&message.to_string());
                    if let Some(reporter) = reporter.as_mut() {
                        reporter.emit_warning(&message);
                        return WatchLoopAction::continue_with(Some(1));
                    }
                    let next_exit = report_error(command_name, &args.report, &message);
                    return WatchLoopAction::continue_with(Some(next_exit));
                }
            };

            // run the daemon check command
            let result = match daemon.run_command(&root, common, payload) {
                Ok(result) => result,
                Err(message) => {
                    let message = watch_error(&message.to_string());
                    if let Some(reporter) = reporter.as_mut() {
                        reporter.emit_warning(&message);
                        return WatchLoopAction::continue_with(Some(1));
                    }
                    let next_exit = report_error(command_name, &args.report, &message);
                    return WatchLoopAction::continue_with(Some(next_exit));
                }
            };

            // emit daemon output for text mode
            emit_daemon_text_output(
                &args.report,
                &result.response.messages,
                &result.response.output,
            );

            // report diagnostics for the updated state
            let stats = result
                .response
                .stats
                .as_ref()
                .map(command_stats_from_protocol);
            let next_exit_code = emit_watch_compile_report(
                reporter,
                WatchCompileContext {
                    files: &result.files,
                    diagnostics: &result.diagnostics,
                    format_options,
                    json_format_options: &json_format_options,
                    module_count: result.response.module_count,
                    line_writer: line_writer.as_ref(),
                },
                stats,
                reason,
                updated,
                requires_rescan,
                Some(batch_id),
            );

            // record compile observation
            on_compile(reason, updated, requires_rescan);

            if is_one_shot {
                return WatchLoopAction::stop_with(Some(next_exit_code));
            }

            WatchLoopAction::continue_with(Some(next_exit_code))
        },
        exit_code,
    );

    if let Some(reporter) = reporter.as_mut() {
        reporter.emit_stop();
    }

    daemon.shutdown();

    exit_code
}
