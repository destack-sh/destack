use std::ops::AsyncFnMut;
use std::path::Path;

use crate::common::{
    CommandOptionsBuilder, CommandResult, DiagnosticFormat, FormatOptions, InputArgs, InputSource,
    ProgramArgs, ReportArgs, RuntimeArgs, TargetArgs, WatchCompileContext, WatchCompileReason,
    WatchCycle, WatchReporter, WorkspaceWatch, command_error, command_inputs_from_sources,
    emit_watch_compile_report, emit_workspace_text_output, ensure_no_watch_or_dev,
    finish_run_command, report_error, run_workspace_command, target_overrides_from_args,
    watch_error,
};
use crate::console;
use crate::diagnostic::ConsoleResult;
use clap::Args;
use destack_workspace::{CommandRevision, RunInput, RunMode, WatchPolicy, Workspace};

/// Arguments for the run command.
#[derive(Args, Debug, Clone)]
pub struct RunArgs {
    /// Input arguments.
    #[command(flatten)]
    pub input: InputArgs,

    /// The program options.
    #[command(flatten)]
    pub program: ProgramArgs,

    /// Target configuration.
    #[command(flatten)]
    pub target: TargetArgs,

    /// Runtime configuration.
    #[command(flatten)]
    pub runtime: RuntimeArgs,

    /// Report output options.
    #[command(flatten)]
    pub report: ReportArgs,

    /// Entry function name (default: main).
    #[arg(long, default_value = "main")]
    pub entry: String,

    /// Arguments passed to the program.
    #[arg(last = true, value_name = "ARGS")]
    pub args: Vec<String>,
}

/// Mode for running entry execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RunSourceMode {
    /// Execute a source entry.
    Program,
    /// Evaluate inline code and optionally print the result.
    Eval { print: bool },
}

/// Shared run request for run-like commands.
#[derive(Debug, Clone)]
pub(crate) struct RunRequest {
    /// Command name for reporting.
    pub command_name: &'static str,
    /// Input arguments.
    pub input: InputArgs,
    /// Program options.
    pub program: ProgramArgs,
    /// Target configuration.
    pub target: TargetArgs,
    /// Runtime configuration.
    pub runtime: RuntimeArgs,
    /// Report output options.
    pub report: ReportArgs,
    /// Entry function name.
    pub entry: String,
    /// Arguments passed to the program.
    pub args: Vec<String>,
    /// Execution mode for the run.
    pub mode: RunSourceMode,
}

/// State for run watch mode.
struct RunWatchState {
    /// The resolved input sources.
    sources: Vec<InputSource>,
}

/// Prepared workspace execution for run like commands.
struct PreparedRunCommand {
    /// Workspace run request.
    request: RunInput,
}

/// Prepared watch execution state for run like commands.
struct PreparedRunWatch {
    /// Mutable watch state.
    state: RunWatchState,
}

/// Compile and run a source entry.
pub async fn run(args: &RunArgs) -> i32 {
    run_with_request(RunRequest {
        command_name: "run",
        input: args.input.clone(),
        program: args.program.clone(),
        target: args.target.clone(),
        runtime: args.runtime.clone(),
        report: args.report.clone(),
        entry: args.entry.clone(),
        args: args.args.clone(),
        mode: RunSourceMode::Program,
    })
    .await
}

/// Compile and run a source entry with shared execution logic.
pub(crate) async fn run_with_request(request: RunRequest) -> i32 {
    let command_name = request.command_name;

    // run watch mode when requested
    if request.program.watch {
        return run_watch(&request).await;
    }

    // reject unsupported watch or dev flags
    if let Some(code) = ensure_no_watch_or_dev(command_name, &request.program, &request.report) {
        return code;
    }

    run_program(&request).await
}

/// Run a single execution request.
async fn run_program(request: &RunRequest) -> i32 {
    // prepare workspace execution
    let prepared = match prepare_run_execution(request) {
        Ok(prepared) => prepared,
        Err(code) => return code,
    };

    // execute the workspace command
    let result = match execute_run_command(request, &prepared).await {
        Ok(result) => result,
        Err(code) => return code,
    };

    finish_run_command(request.command_name, &request.report, &result)
}

/// Compile and run a source file in watch mode.
async fn run_watch(request: &RunRequest) -> i32 {
    // run with default watch settings
    run_watch_with_options(request, WatchPolicy::default(), || {}, |_, _, _| {}, false).await
}

/// Compile and run a source file in watch mode with injected options.
pub(crate) async fn run_watch_with_options<StartFn, ObserveFn>(
    request: &RunRequest,
    watch_policy: WatchPolicy,
    on_start: StartFn,
    on_compile: ObserveFn,
    is_one_shot: bool,
) -> i32
where
    StartFn: FnOnce(),
    ObserveFn: FnMut(WatchCompileReason, bool, bool),
{
    run_watch_with_driver(
        request,
        watch_policy,
        on_start,
        on_compile,
        async |request, workspace, root, sources, reporter, cycle| {
            compile_and_run_workspace(request, workspace, root, sources, reporter, cycle).await
        },
        is_one_shot,
    )
    .await
}

/// Compile and run a source file in watch mode with an injected runner.
pub(crate) async fn run_watch_with_driver<StartFn, ObserveFn, CompileFn>(
    request: &RunRequest,
    watch_policy: WatchPolicy,
    on_start: StartFn,
    mut on_compile: ObserveFn,
    mut compile: CompileFn,
    is_one_shot: bool,
) -> i32
where
    StartFn: FnOnce(),
    ObserveFn: FnMut(WatchCompileReason, bool, bool),
    for<'a> CompileFn: AsyncFnMut(
        &'a RunRequest,
        &'a dyn Workspace,
        &'a Path,
        &'a [InputSource],
        &'a mut Option<WatchReporter>,
        WatchCycle,
    ) -> i32,
{
    let command_name = request.command_name;

    // prepare watch mode state
    let prepared = match prepare_run_watch(request) {
        Ok(prepared) => prepared,
        Err(code) => return code,
    };
    let PreparedRunWatch { mut state } = prepared;

    let mut watch = match WorkspaceWatch::start(
        command_name,
        &request.program,
        &request.report,
        watch_policy,
    ) {
        Ok(watch) => watch,
        Err(code) => return code,
    };

    on_start();

    let mut exit_code = watch
        .run_command(async |workspace, root, reporter| {
            compile(
                request,
                workspace,
                root,
                &state.sources,
                reporter,
                WatchCycle::startup(),
            )
            .await
        })
        .await;
    while let Some(cycle) = watch.next_cycle() {
        // refresh sources when the workspace requests a rescan
        if cycle.requires_rescan
            && let Err(error) = refresh_run_watch_state(request, &mut state)
        {
            watch.emit_warning(&error.to_string());
            continue;
        }

        exit_code = watch
            .run_command(async |workspace, root, reporter| {
                compile(request, workspace, root, &state.sources, reporter, cycle).await
            })
            .await;
        on_compile(cycle.reason, cycle.updated, cycle.requires_rescan);
        if is_one_shot {
            break;
        }
    }

    watch.stop();
    exit_code
}

/// Compile the entry module and run the program through the workspace.
#[allow(clippy::too_many_arguments)]
async fn compile_and_run_workspace(
    request: &RunRequest,
    workspace: &dyn Workspace,
    root: &Path,
    sources: &[InputSource],
    watch_reporter: &mut Option<WatchReporter>,
    cycle: WatchCycle,
) -> i32 {
    let run_request = match build_run_command(request, sources) {
        Ok(run_request) => run_request,
        Err(error) => {
            let message = watch_error(&error.to_string());
            if let Some(reporter) = watch_reporter.as_mut() {
                reporter.emit_warning(&message);
                return 1;
            }
            return report_error(request.command_name, &request.report, &message);
        }
    };

    // execute the workspace command
    let result = match workspace.run(root, run_request, None).await {
        Ok(result) => match CommandResult::from_output(result) {
            Ok(result) => result,
            Err(error) => {
                let message = watch_error(&error.to_string());
                if let Some(reporter) = watch_reporter.as_mut() {
                    reporter.emit_warning(&message);
                    return 1;
                }
                return report_error(request.command_name, &request.report, &message);
            }
        },
        Err(error) => {
            let message = watch_error(&error.to_string());
            if let Some(reporter) = watch_reporter.as_mut() {
                reporter.emit_warning(&message);
                return 1;
            }
            return report_error(request.command_name, &request.report, &message);
        }
    };

    // emit watch diagnostics
    let format_options = FormatOptions::default();
    let json_format_options = FormatOptions {
        format: DiagnosticFormat::Json,
        ..FormatOptions::default()
    };
    let diagnostic_exit = emit_watch_compile_report(
        watch_reporter,
        WatchCompileContext {
            files: &result.files,
            diagnostics: &result.diagnostics,
            format_options: &format_options,
            json_format_options: &json_format_options,
            module_count: result.response.module_count,
            line_writer: None,
        },
        cycle,
    );
    if diagnostic_exit != 0 {
        return diagnostic_exit;
    }

    // emit workspace output for text mode
    if watch_reporter.is_none() {
        emit_workspace_text_output(
            &request.report,
            &result.response.messages,
            &result.response.output,
        );
    }

    let exit_code = result.response.exit_code;
    if exit_code != 0 {
        if let Some(reporter) = watch_reporter.as_mut() {
            reporter.emit_warning(&format!("watch: process exited with code {exit_code}"));
        } else {
            console::warn(&format!("process exited with code {exit_code}"));
        }
    }

    exit_code
}

/// Prepare one shot execution for a run like command.
fn prepare_run_execution(request: &RunRequest) -> Result<PreparedRunCommand, i32> {
    // resolve a single entry source for the run
    let sources = resolve_run_sources_or_report(request)?;
    let run_request = build_run_command(request, &sources)
        .map_err(|error| report_error(request.command_name, &request.report, &error.to_string()))?;

    Ok(PreparedRunCommand {
        request: run_request,
    })
}

/// Execute a prepared run command through the workspace.
async fn execute_run_command(
    request: &RunRequest,
    prepared: &PreparedRunCommand,
) -> Result<CommandResult, i32> {
    run_workspace_command(
        &request.program,
        async |workspace, root, progress| {
            let result = workspace
                .run(root, prepared.request.clone(), progress)
                .await
                .map_err(command_error)?;

            CommandResult::from_output(result)
        },
        None,
    )
    .await
    .map_err(|error| report_error(request.command_name, &request.report, &error.to_string()))
}

/// Prepare watch state for a run like command.
fn prepare_run_watch(request: &RunRequest) -> Result<PreparedRunWatch, i32> {
    if matches!(request.mode, RunSourceMode::Eval { .. }) {
        return Err(report_error(
            request.command_name,
            &request.report,
            "--watch does not support --eval",
        ));
    }

    if request.input.stdin || !request.input.eval.is_empty() || !request.input.module.is_empty() {
        return Err(report_error(
            request.command_name,
            &request.report,
            "--watch requires file or directory inputs",
        ));
    }

    let sources = resolve_run_sources_or_report(request)?;

    Ok(PreparedRunWatch {
        state: RunWatchState { sources },
    })
}

/// Refresh watch state after a rescan.
fn refresh_run_watch_state(request: &RunRequest, state: &mut RunWatchState) -> ConsoleResult<()> {
    state.sources = resolve_run_sources_for_watch(request)?;

    Ok(())
}

/// Resolve entry sources for a one shot run command.
fn resolve_run_sources_or_report(request: &RunRequest) -> Result<Vec<InputSource>, i32> {
    let sources = request
        .input
        .explicit_sources()
        .map_err(|error| report_error(request.command_name, &request.report, &error.to_string()))?;

    if !sources.is_empty() && sources.len() != 1 {
        return Err(report_error(
            request.command_name,
            &request.report,
            "run expects a single entry module",
        ));
    }

    Ok(sources)
}

/// Resolve entry sources for watch mode rescans.
fn resolve_run_sources_for_watch(request: &RunRequest) -> ConsoleResult<Vec<InputSource>> {
    let sources = request.input.explicit_sources()?;

    if !sources.is_empty() && sources.len() != 1 {
        return Err(watch_error("expected a single entry module").into());
    }

    Ok(sources)
}

/// Build the workspace command options for a run command.
fn build_run_command(request: &RunRequest, sources: &[InputSource]) -> ConsoleResult<RunInput> {
    let inputs = command_inputs_from_sources(sources, request.input.file_type())?;
    let common = CommandOptionsBuilder::new(&request.program)?
        .inputs(inputs)
        .config_inputs(!request.input.has_input())
        .target(request.target.target_name().map(str::to_string))
        .target_overrides(target_overrides_from_args(&request.target))
        .manifest_override(request.runtime.to_manifest_override())
        .build();
    Ok(RunInput {
        entry: Some(request.entry.clone()),
        args: request.args.clone(),
        run_mode: resolve_run_mode(request),
        ..(CommandRevision::Current, common).into()
    })
}

/// Resolve the workspace run mode for a request.
fn resolve_run_mode(request: &RunRequest) -> RunMode {
    match request.mode {
        RunSourceMode::Program => RunMode::Program,
        RunSourceMode::Eval { print } => RunMode::Eval { print },
    }
}
