use crate::common::format::{DiagnosticFormat, FormatOptions};
use crate::common::{
    CommandOptionsBuilder, CommandResult, DiagnosticCommandSummary, InputArgs, InputSource,
    ProgramArgs, ProgressMode, ProgressReporter, ReportArgs, WatchCompileContext,
    WatchCompileReason, WatchCycle, WorkspaceWatch, command_data_json, command_error,
    command_inputs_from_sources, emit_watch_compile_report, emit_workspace_text_output,
    finish_diagnostic_command, is_tty, report_error, run_workspace_command, watch_error,
};
use crate::console;
use crate::console::{render_stage_summary, render_timeline};
use crate::diagnostic::ConsoleResult;
use clap::{Args, ValueEnum};
use destack_repository::{TraceSnapshot, TraceView};
use destack_workspace::{CheckInput, CommandRevision, WatchPolicy};

/// State for check watch mode.
struct CheckWatchState {
    /// The resolved input sources.
    sources: Vec<InputSource>,
}

/// Execution context shared across check command paths.
struct CheckExecutionContext {
    /// Output formatting options.
    format_options: FormatOptions,
    /// Progress reporter for interactive output.
    progress_reporter: Option<ProgressReporter>,
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

    /// Show statistics grouped by diagnostic id.
    #[arg(long)]
    pub statistics: bool,

    /// Show progress indicator (auto, on, off, detailed).
    #[arg(long, value_enum, default_value = "auto")]
    pub progress: Progress,

    /// Show a detailed per-worker timeline.
    #[arg(long)]
    pub timings: bool,
}

/// Check source files for type errors and lint issues.
pub fn run(args: &CheckArgs) -> i32 {
    run_with_command(args, "check")
}

/// Check source files with a custom command label.
pub fn run_with_command(args: &CheckArgs, command_name: &str) -> i32 {
    // a directory argument selects the package or workspace root
    let mut args = args.clone();
    match args.input.take_directory_root() {
        Ok(Some(root)) => match &args.program.workspace {
            Some(workspace) if *workspace != root => {
                let message =
                    format!("directory argument {root:?} conflicts with --workspace {workspace:?}");
                return report_error(command_name, &args.report, &message);
            }
            _ => args.program.workspace = Some(root),
        },
        Ok(None) => {}
        Err(error) => return report_error(command_name, &args.report, &error.to_string()),
    }
    let args = &args;

    if let Some(code) = validate_check_args(args, command_name) {
        return code;
    }

    let context = match CheckExecutionContext::new(args) {
        Ok(context) => context,
        Err(error) => return report_error(command_name, &args.report, &error.to_string()),
    };

    // run watch mode when requested
    if args.program.watch {
        let exit_code = run_watch(args, command_name, &context);
        context.finish();
        return exit_code;
    }

    let exit_code = run_check(args, command_name, &context);
    context.finish();
    exit_code
}

/// Run a single check command.
fn run_check(args: &CheckArgs, command_name: &str, context: &CheckExecutionContext) -> i32 {
    // build the workspace command
    let sources = match resolve_check_sources_or_report(args, command_name) {
        Ok(sources) => sources,
        Err(code) => return code,
    };
    let request = match build_check_command(args, &sources) {
        Ok(request) => request,
        Err(error) => return report_error(command_name, &args.report, &error.to_string()),
    };

    // execute the workspace command
    let result = match run_workspace_command(
        &args.program,
        |workspace, root, progress| {
            let result = workspace
                .check(root, request, progress)
                .map_err(command_error)?;

            CommandResult::from_output(result)
        },
        context.progress_reporter.as_ref(),
    ) {
        Ok(result) => result,
        Err(error) => return report_error(command_name, &args.report, &error.to_string()),
    };

    let data = match command_data_json(command_name, &args.report, Some(&result.response.data)) {
        Ok(data) => data,
        Err(code) => return code,
    };
    let exit_code = finish_diagnostic_command(
        command_name,
        &args.report,
        &result,
        &context.json_format_options(),
        &context.format_options,
        context.line_writer().as_ref(),
        context.summary(args, &result),
        data.clone(),
    );

    // show where the check spent its time in text mode
    if !args.report.is_json() {
        let trace = data
            .as_ref()
            .and_then(|value| value.get("trace"))
            .and_then(|value| serde_json::from_value::<TraceSnapshot>(value.clone()).ok());
        if let Some(report) = trace {
            println!("{}", render_stage_summary(&report));
            if args.timings {
                let timeline = render_timeline(&report);
                if !timeline.is_empty() {
                    println!("\n{timeline}");
                }
            }
        }
    }

    exit_code
}

/// Run check in watch mode with incremental updates.
fn run_watch(args: &CheckArgs, command_name: &str, context: &CheckExecutionContext) -> i32 {
    // run with default watch settings
    run_watch_with_options(
        args,
        command_name,
        &context.format_options,
        context.progress_reporter.as_ref(),
        WatchPolicy::default(),
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
    format_options: &FormatOptions,
    progress_reporter: Option<&ProgressReporter>,
    watch_policy: WatchPolicy,
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

    let sources = match resolve_check_watch_sources_or_report(args, command_name) {
        Ok(sources) => sources,
        Err(code) => return code,
    };

    // prepare watch mode output
    let json_format_options = FormatOptions {
        format: DiagnosticFormat::Json,
        quiet: format_options.quiet,
        max_warnings: format_options.max_warnings,
        statistics: format_options.statistics,
        suppress_diagnostics: false,
    };
    let line_writer = progress_reporter.map(|reporter| reporter.line_writer());

    // set up shared watch state
    let mut watch_state = CheckWatchState { sources };

    let mut watch =
        match WorkspaceWatch::start(command_name, &args.program, &args.report, watch_policy) {
            Ok(watch) => watch,
            Err(code) => return code,
        };

    on_start();

    let compile = |watch: &mut WorkspaceWatch, state: &CheckWatchState, cycle: WatchCycle| {
        watch.run_command(|workspace, root, reporter| {
            // build options for the updated sources
            let request = match build_check_command(args, &state.sources) {
                Ok(request) => request,
                Err(message) => {
                    let message = watch_error(&message.to_string());
                    if let Some(reporter) = reporter.as_mut() {
                        reporter.emit_warning(&message);
                        return 1;
                    }
                    let next_exit = report_error(command_name, &args.report, &message);
                    return next_exit;
                }
            };

            // run the workspace check command
            let result = match workspace.check(root, request, None) {
                Ok(result) => match CommandResult::from_output(result) {
                    Ok(result) => result,
                    Err(message) => {
                        let message = watch_error(&message.to_string());
                        if let Some(reporter) = reporter.as_mut() {
                            reporter.emit_warning(&message);
                            return 1;
                        }
                        let next_exit = report_error(command_name, &args.report, &message);
                        return next_exit;
                    }
                },
                Err(message) => {
                    let message = watch_error(&message.to_string());
                    if let Some(reporter) = reporter.as_mut() {
                        reporter.emit_warning(&message);
                        return 1;
                    }
                    let next_exit = report_error(command_name, &args.report, &message);
                    return next_exit;
                }
            };

            // emit workspace output for text mode
            emit_workspace_text_output(
                &args.report,
                &result.response.messages,
                &result.response.output,
            );

            // report diagnostics for the updated state
            emit_watch_compile_report(
                reporter,
                WatchCompileContext {
                    files: &result.files,
                    diagnostics: &result.diagnostics,
                    format_options,
                    json_format_options: &json_format_options,
                    module_count: result.response.module_count,
                    line_writer: line_writer.as_ref(),
                },
                cycle,
            )
        })
    };

    let mut exit_code = compile(&mut watch, &watch_state, WatchCycle::startup());
    while let Some(cycle) = watch.next_cycle() {
        // refresh sources when the workspace requests a rescan
        if cycle.requires_rescan
            && let Err(error) = refresh_check_watch_sources(args, &mut watch_state)
        {
            watch.emit_warning(&error.to_string());
            continue;
        }

        exit_code = compile(&mut watch, &watch_state, cycle);
        on_compile(cycle.reason, cycle.updated, cycle.requires_rescan);
        if is_one_shot {
            break;
        }
    }

    watch.stop();
    exit_code
}

impl CheckExecutionContext {
    /// Build the shared execution context for a check command.
    fn new(args: &CheckArgs) -> ConsoleResult<Self> {
        let progress_reporter = ProgressReporter::with_label(progress_mode(args), "check")?;
        let format_options = FormatOptions {
            format: args.format.into(),
            quiet: args.quiet,
            max_warnings: args.max_warnings,
            statistics: args.statistics,
            suppress_diagnostics: args.no_diagnostics,
        };

        Ok(Self {
            format_options,
            progress_reporter,
        })
    }

    /// Finish progress output for the command.
    fn finish(&self) {
        if let Some(reporter) = &self.progress_reporter {
            reporter.finish();
        }
    }

    /// Build a line writer for formatted output.
    fn line_writer(&self) -> Option<crate::common::LineWriter> {
        self.progress_reporter
            .as_ref()
            .map(|reporter| reporter.line_writer())
    }

    /// Build json format options derived from the text options.
    fn json_format_options(&self) -> FormatOptions {
        FormatOptions {
            format: DiagnosticFormat::Json,
            quiet: self.format_options.quiet,
            max_warnings: self.format_options.max_warnings,
            statistics: self.format_options.statistics,
            suppress_diagnostics: false,
        }
    }

    /// Build the summary metadata for text output.
    fn summary(
        &self,
        args: &CheckArgs,
        result: &CommandResult,
    ) -> Option<DiagnosticCommandSummary<'static>> {
        if !matches!(args.format, Format::Text) {
            return None;
        }

        Some(DiagnosticCommandSummary {
            verb: "Checked",
            modules: result.response.module_count,
            targets: 0,
        })
    }
}

/// Validate user facing check argument combinations.
fn validate_check_args(args: &CheckArgs, command_name: &str) -> Option<i32> {
    if !args.report.is_json() {
        if args.unsafe_fixes && !args.fix && !args.diff {
            console::warn("--unsafe-fixes has no effect without --fix or --diff");
        }

        if args.no_lint && (args.fix || args.diff) {
            console::warn("--fix and --diff have no effect with --no-lint");
        }
    }

    if args.report.is_json() && (args.fix || args.diff) {
        return Some(report_error(
            command_name,
            &args.report,
            "--output-format json is not supported with --fix or --diff",
        ));
    }

    None
}

/// Resolve the progress mode for a check command.
fn progress_mode(args: &CheckArgs) -> ProgressMode {
    match args.progress {
        Progress::Auto => {
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
    }
}

/// Resolve explicit command input sources for one shot execution.
fn resolve_check_sources_or_report(
    args: &CheckArgs,
    command_name: &str,
) -> Result<Vec<InputSource>, i32> {
    if !args.input.has_input() {
        return Ok(Vec::new());
    }

    args.input
        .to_sources()
        .map_err(|error| report_error(command_name, &args.report, &error.to_string()))
}

/// Resolve initial watch sources for the check command.
fn resolve_check_watch_sources_or_report(
    args: &CheckArgs,
    command_name: &str,
) -> Result<Vec<InputSource>, i32> {
    args.input
        .explicit_sources()
        .map_err(|error| report_error(command_name, &args.report, &error.to_string()))
}

/// Refresh watch sources after a rescan.
fn refresh_check_watch_sources(args: &CheckArgs, state: &mut CheckWatchState) -> ConsoleResult<()> {
    state.sources = args.input.explicit_sources()?;

    Ok(())
}

/// Build the workspace command for the check command.
fn build_check_command(args: &CheckArgs, sources: &[InputSource]) -> ConsoleResult<CheckInput> {
    let inputs = command_inputs_from_sources(sources, args.input.file_type())?;
    let common = CommandOptionsBuilder::new(&args.program)?
        .inputs(inputs)
        .config_inputs(!args.input.has_input())
        .build();
    let request = CheckInput {
        lint: !args.no_lint,
        fix: args.fix,
        unsafe_fixes: args.unsafe_fixes,
        diff: args.diff,
        trace: TraceView::detailed(args.timings),
        ..(CommandRevision::Current, common).into()
    };

    Ok(request)
}
