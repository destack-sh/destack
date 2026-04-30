use std::path::Path;
use std::sync::Arc;

use crate::common::{
    DiagnosticArgs, DiagnosticFormat, FormatOptions, InputArgs, InputSource, ProgramArgs,
    ReportArgs, RuntimeArgs, TargetArgs, WatchCompileReason, WatchReporter, ensure_no_watch_or_dev,
    print_report, report_error, report_from_payload, report_no_input,
};
use crate::console;
use crate::error::CliResult;
use crate::pipeline::daemon::{
    CommandOptionsBuilder, DaemonCommandResult, ProtocolDaemonClient, command_inputs_from_sources,
    emit_daemon_text_output, finish_run_command, run_root_command_with_repository_or_report,
    target_overrides_from_args,
};
use crate::pipeline::input::{ResolveSourcesError, resolve_sources};
use crate::pipeline::script::{ScriptSource, resolve_script_command, shell_command};
use crate::pipeline::target::target_name_from_args;
use crate::pipeline::watch::{
    WatchCompileContext, WatchLoopOptions, build_watch_loop_options, emit_watch_compile_report,
    run_daemon_watch_command, watch_error,
};
use crate::pipeline::workspace::default_target_for_repository;
use clap::Args;
use destack_daemon::protocol::{
    CommandPayload, CommandRunMode, CommandRunOptions, CommonCommandOptions,
};
use destack_source::{DiagnosticOptions, FileSystem};
use destack_workspace::Repository;

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

    /// The diagnostic options.
    #[command(flatten)]
    pub diagnostics: DiagnosticArgs,

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
pub(crate) enum RunMode {
    /// Execute a source file or script.
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
    /// Diagnostic options.
    pub diagnostics: DiagnosticArgs,
    /// Report output options.
    pub report: ReportArgs,
    /// Entry function name.
    pub entry: String,
    /// Arguments passed to the program.
    pub args: Vec<String>,
    /// Execution mode for the run.
    pub mode: RunMode,
}

/// State for run watch mode.
struct RunWatchState {
    /// The resolved input sources.
    sources: Vec<InputSource>,
    /// Resolved target name for the run.
    target_name: String,
}

/// Prepared daemon execution for run like commands.
struct PreparedRunCommand {
    /// Repository used for daemon execution.
    repository: Arc<Repository>,
    /// Diagnostic options passed to the daemon.
    diagnostic_options: DiagnosticOptions,
    /// Common daemon command options.
    common: CommonCommandOptions,
    /// Command specific daemon payload.
    payload: CommandPayload,
}

/// Prepared watch execution state for run like commands.
struct PreparedRunWatch {
    /// Repository used for watch mode.
    repository: Arc<Repository>,
    /// Diagnostic options passed to the daemon.
    diagnostic_options: DiagnosticOptions,
    /// Mutable watch state.
    state: RunWatchState,
}

/// One shot execution plan for run like commands.
enum RunExecutionPlan {
    /// Script execution completed without using the daemon.
    Script(i32),
    /// Daemon execution is ready to run.
    Daemon(Box<PreparedRunCommand>),
}

/// Compile and run a source file or script.
pub fn run(args: &RunArgs) -> i32 {
    run_with_request(RunRequest {
        command_name: "run",
        input: args.input.clone(),
        program: args.program.clone(),
        target: args.target.clone(),
        runtime: args.runtime.clone(),
        diagnostics: args.diagnostics.clone(),
        report: args.report.clone(),
        entry: args.entry.clone(),
        args: args.args.clone(),
        mode: RunMode::Program,
    })
}

/// Compile and run a source file or script with shared execution logic.
pub(crate) fn run_with_request(request: RunRequest) -> i32 {
    let command_name = request.command_name;

    // run watch mode when requested
    if request.program.watch {
        return run_watch(&request);
    }

    // reject unsupported watch or dev flags
    if let Some(code) = ensure_no_watch_or_dev(command_name, &request.program, &request.report) {
        return code;
    }

    run_via_daemon(&request)
}

/// Run a single execution request through the daemon.
fn run_via_daemon(request: &RunRequest) -> i32 {
    // prepare the execution plan
    let plan = match prepare_run_execution(request) {
        Ok(plan) => plan,
        Err(code) => return code,
    };

    // return early when a script handled the command
    let prepared = match plan {
        RunExecutionPlan::Script(exit_code) => return exit_code,
        RunExecutionPlan::Daemon(prepared) => prepared,
    };

    // execute the daemon command
    let result = match execute_run_command(request, &prepared) {
        Ok(result) => result,
        Err(code) => return code,
    };

    finish_run_command(request.command_name, &request.report, &result)
}

/// Compile and run a source file in watch mode.
fn run_watch(request: &RunRequest) -> i32 {
    // run with default watch settings
    run_watch_with_options(
        request,
        build_watch_loop_options(),
        || {},
        |_, _, _| {},
        false,
    )
}

/// Compile and run a source file in watch mode with injected options.
pub(crate) fn run_watch_with_options<StartFn, ObserveFn>(
    request: &RunRequest,
    watch_loop_options: WatchLoopOptions,
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
        watch_loop_options,
        on_start,
        on_compile,
        |request,
         daemon,
         root,
         entry_source,
         target_name,
         reporter,
         reason,
         batch_id,
         updated,
         rescan| {
            compile_and_run_daemon(
                request,
                daemon,
                root,
                entry_source,
                target_name,
                reporter,
                reason,
                batch_id,
                updated,
                rescan,
            )
        },
        is_one_shot,
    )
}

/// Compile and run a source file in watch mode with an injected runner.
pub(crate) fn run_watch_with_driver<StartFn, ObserveFn, CompileFn>(
    request: &RunRequest,
    watch_loop_options: WatchLoopOptions,
    on_start: StartFn,
    on_compile: ObserveFn,
    mut compile: CompileFn,
    is_one_shot: bool,
) -> i32
where
    StartFn: FnOnce(),
    ObserveFn: FnMut(WatchCompileReason, bool, bool),
    CompileFn: FnMut(
        &RunRequest,
        &ProtocolDaemonClient,
        &Path,
        &InputSource,
        &str,
        &mut Option<WatchReporter>,
        WatchCompileReason,
        Option<u64>,
        bool,
        bool,
    ) -> i32,
{
    let command_name = request.command_name;

    // prepare watch mode state
    let prepared = match prepare_run_watch(request) {
        Ok(prepared) => prepared,
        Err(code) => return code,
    };
    let PreparedRunWatch {
        repository,
        diagnostic_options,
        mut state,
    } = prepared;

    run_daemon_watch_command(
        command_name,
        repository,
        &request.program,
        &request.report,
        diagnostic_options,
        None,
        watch_loop_options,
        &mut state,
        move |_state| on_start(),
        |state, session| refresh_run_watch_state(request, state, session),
        move |daemon, root, reporter, state, reason, batch_id, updated, requires_rescan| {
            compile(
                request,
                daemon,
                root,
                &state.sources[0],
                &state.target_name,
                reporter,
                reason,
                batch_id,
                updated,
                requires_rescan,
            )
        },
        on_compile,
        is_one_shot,
    )
}

/// Compile the entry module and run the program through the daemon.
#[allow(clippy::too_many_arguments)]
fn compile_and_run_daemon(
    request: &RunRequest,
    daemon: &ProtocolDaemonClient,
    root: &Path,
    entry_source: &InputSource,
    target_name: &str,
    watch_reporter: &mut Option<WatchReporter>,
    compile_reason: WatchCompileReason,
    batch_id: Option<u64>,
    updated: bool,
    rescan: bool,
) -> i32 {
    let diagnostic_options: DiagnosticOptions = request.diagnostics.clone().into();
    let (common, payload) = match build_run_command(
        request,
        std::slice::from_ref(entry_source),
        target_name,
        &diagnostic_options,
    ) {
        Ok(command) => command,
        Err(error) => {
            let message = watch_error(&error.to_string());
            if let Some(reporter) = watch_reporter.as_mut() {
                reporter.emit_warning(&message);
                return 1;
            }
            return report_error(request.command_name, &request.report, &message);
        }
    };

    // execute the daemon command
    let result = match daemon.run_root_command(root, common, payload) {
        Ok(result) => result,
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
        compile_reason,
        updated,
        rescan,
        batch_id,
    );
    if diagnostic_exit != 0 {
        return diagnostic_exit;
    }

    // emit daemon output for text mode
    if watch_reporter.is_none() {
        emit_daemon_text_output(
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

/// Attempt to run a destack.json or package.json script when input is not a file.
fn try_run_script(request: &RunRequest, fs: &dyn FileSystem, cwd: &Path) -> Option<i32> {
    let command_name = request.command_name;

    // skip script resolution when explicit input is provided
    if !request.input.eval.is_empty()
        || !request.input.module.is_empty()
        || request.input.stdin
        || request.input.files.len() != 1
    {
        return None;
    }

    // resolve the candidate path for the file
    let candidate = &request.input.files[0];
    let candidate_path = if candidate.is_absolute() {
        candidate.clone()
    } else {
        cwd.join(candidate)
    };

    // only treat the argument as a script if the file does not exist
    if fs.metadata(&candidate_path).is_ok() {
        return None;
    }

    // resolve script command from destack.json or package.json
    let script_name = candidate.to_string_lossy().to_string();
    let script = match resolve_script_command(&request.program, &script_name) {
        Ok(Some(script)) => script,
        Ok(None) => {
            let message = format!("no such file or script: {script_name}");
            return Some(report_error(command_name, &request.report, &message));
        }
        Err(error) => {
            return Some(report_error(
                command_name,
                &request.report,
                &error.to_string(),
            ));
        }
    };

    // build shell command with args appended
    let mut command = script.command.clone();
    if !request.args.is_empty() {
        command.push(' ');
        command.push_str(&request.args.join(" "));
    }

    // run via shell
    let mut shell = shell_command(&command);
    shell.current_dir(&script.cwd);
    let status = match shell.status() {
        Ok(status) => status,
        Err(error) => {
            let message = format!("script failed: {error}");
            return Some(report_error(command_name, &request.report, &message));
        }
    };
    let exit_code = status.code().unwrap_or(1);

    // report structured output when requested
    if request.report.is_json() {
        let source = match script.source {
            ScriptSource::Destack => "destack",
            ScriptSource::PackageJson => "package.json",
        };
        let data = serde_json::json!({
            "script": script.name,
            "command": command,
            "cwd": script.cwd,
            "source": source,
        });
        let report = report_from_payload(command_name, exit_code, Some(data), None, None);
        print_report(&report, request.report.format());
    } else if exit_code != 0 {
        console::warn(&format!("process exited with code {exit_code}"));
    }

    Some(exit_code)
}

/// Prepare one shot execution for a run like command.
fn prepare_run_execution(request: &RunRequest) -> Result<RunExecutionPlan, i32> {
    let repository = request.program.setup();

    // allow script execution before daemon setup
    if matches!(request.mode, RunMode::Program)
        && let Some(exit_code) = try_run_script(
            request,
            repository.file_system().as_ref(),
            &request.program.effective_cwd(),
        )
    {
        return Ok(RunExecutionPlan::Script(exit_code));
    }

    // resolve a single entry source for the run
    let sources = resolve_run_sources_or_report(request)?;
    let target_name = resolve_run_target_name_or_report(request, &repository)?;
    let diagnostic_options: DiagnosticOptions = request.diagnostics.clone().into();
    let (common, payload) = build_run_command(request, &sources, &target_name, &diagnostic_options)
        .map_err(|error| report_error(request.command_name, &request.report, &error.to_string()))?;

    Ok(RunExecutionPlan::Daemon(Box::new(PreparedRunCommand {
        repository,
        diagnostic_options,
        common,
        payload,
    })))
}

/// Execute a prepared run command through the daemon.
fn execute_run_command(
    request: &RunRequest,
    prepared: &PreparedRunCommand,
) -> Result<DaemonCommandResult, i32> {
    run_root_command_with_repository_or_report(
        request.command_name,
        &request.report,
        prepared.repository.clone(),
        &request.program,
        prepared.diagnostic_options.clone(),
        prepared.common.clone(),
        prepared.payload.clone(),
    )
}

/// Prepare watch state for a run like command.
fn prepare_run_watch(request: &RunRequest) -> Result<PreparedRunWatch, i32> {
    if matches!(request.mode, RunMode::Eval { .. }) {
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

    let repository = request.program.setup();

    if is_script_name_input(request)
        && let Some(candidate_path) = script_name_input_path(request)
        && repository.file_system().metadata(&candidate_path).is_err()
    {
        return Err(report_error(
            request.command_name,
            &request.report,
            "--watch does not support script commands",
        ));
    }

    let sources = resolve_run_sources_or_report(request)?;
    let target_name = resolve_run_target_name_for_watch(request, &repository)?;
    let diagnostic_options: DiagnosticOptions = request.diagnostics.clone().into();

    Ok(PreparedRunWatch {
        repository,
        diagnostic_options,
        state: RunWatchState {
            sources,
            target_name,
        },
    })
}

/// Refresh watch state after a rescan.
fn refresh_run_watch_state(
    request: &RunRequest,
    state: &mut RunWatchState,
    repository: &Repository,
) -> CliResult<()> {
    state.sources = resolve_run_sources_for_watch(request)?;
    let target_name = resolve_run_target_name(request, repository).map_err(|message| {
        let message = watch_error(&message);
        crate::error::CliError::message(message)
    })?;
    state.target_name = target_name;

    Ok(())
}

/// Resolve entry sources for a one shot run command.
fn resolve_run_sources_or_report(request: &RunRequest) -> Result<Vec<InputSource>, i32> {
    let sources = match resolve_sources(&request.input, None, None) {
        Ok(sources) => sources,
        Err(ResolveSourcesError::NoInput) => {
            return Err(report_no_input(request.command_name, &request.report));
        }
        Err(ResolveSourcesError::Message(message)) => {
            return Err(report_error(
                request.command_name,
                &request.report,
                &message,
            ));
        }
    };

    if sources.len() != 1 {
        return Err(report_error(
            request.command_name,
            &request.report,
            "run expects a single entry module",
        ));
    }

    Ok(sources)
}

/// Resolve entry sources for watch mode rescans.
fn resolve_run_sources_for_watch(request: &RunRequest) -> CliResult<Vec<InputSource>> {
    let sources = match resolve_sources(&request.input, None, None) {
        Ok(sources) => sources,
        Err(ResolveSourcesError::NoInput) => {
            return Err(watch_error("no input files after rescan").into());
        }
        Err(ResolveSourcesError::Message(message)) => {
            return Err(watch_error(&message).into());
        }
    };

    if sources.len() != 1 {
        return Err(watch_error("expected a single entry module").into());
    }

    Ok(sources)
}

/// Resolve the target name for a run command.
fn resolve_run_target_name(
    request: &RunRequest,
    repository: &Repository,
) -> Result<String, String> {
    let default_target = default_target_for_repository(&request.program, repository)
        .map_err(|error| error.to_string())?;
    let fallback = default_target.as_deref().unwrap_or("native");

    Ok(target_name_from_args(&request.target, fallback))
}

/// Resolve the target name for one shot execution.
fn resolve_run_target_name_or_report(
    request: &RunRequest,
    repository: &Repository,
) -> Result<String, i32> {
    resolve_run_target_name(request, repository)
        .map_err(|message| report_error(request.command_name, &request.report, &message))
}

/// Resolve the target name for watch execution.
fn resolve_run_target_name_for_watch(
    request: &RunRequest,
    repository: &Repository,
) -> Result<String, i32> {
    resolve_run_target_name(request, repository).map_err(|message| {
        let message = watch_error(&message);
        report_error(request.command_name, &request.report, &message)
    })
}

/// Build the daemon command options for a run command.
fn build_run_command(
    request: &RunRequest,
    sources: &[InputSource],
    target_name: &str,
    diagnostic_options: &DiagnosticOptions,
) -> CliResult<(CommonCommandOptions, CommandPayload)> {
    let inputs = command_inputs_from_sources(sources, request.input.file_type())?;
    let common = CommandOptionsBuilder::new(&request.program, Some(diagnostic_options.clone()))
        .inputs(inputs)
        .target(target_name.to_string())
        .target_overrides(target_overrides_from_args(&request.target))
        .runtime_overrides(request.runtime.to_runtime_overrides())
        .build();

    Ok((common, build_run_payload(request)))
}

/// Build the daemon payload for a run command.
fn build_run_payload(request: &RunRequest) -> CommandPayload {
    CommandPayload::Run(CommandRunOptions {
        entry: Some(request.entry.clone()),
        args: request.args.clone(),
        run_mode: resolve_run_mode(request),
    })
}

/// Resolve the daemon run mode for a request.
fn resolve_run_mode(request: &RunRequest) -> CommandRunMode {
    match request.mode {
        RunMode::Program => CommandRunMode::Program,
        RunMode::Eval { print } => CommandRunMode::Eval { print },
    }
}

/// Check whether the request could refer to a script name.
fn is_script_name_input(request: &RunRequest) -> bool {
    matches!(request.mode, RunMode::Program)
        && request.input.files.len() == 1
        && request.input.eval.is_empty()
        && request.input.module.is_empty()
        && !request.input.stdin
}

/// Resolve the candidate path for a potential script name.
fn script_name_input_path(request: &RunRequest) -> Option<std::path::PathBuf> {
    if !is_script_name_input(request) {
        return None;
    }

    let candidate = &request.input.files[0];
    let candidate_path = if candidate.is_absolute() {
        candidate.clone()
    } else {
        request.program.effective_cwd().join(candidate)
    };

    Some(candidate_path)
}
