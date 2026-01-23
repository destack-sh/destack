use std::path::Path;

use crate::common::{
    CommandError, CommandReport, DiagnosticArgs, DiagnosticFormat, FormatOptions, InputArgs,
    InputSource, ProgramArgs, ReportArgs, TargetArgs, WatchCompileReason, WatchReporter,
    collect_diagnostics_json, ensure_no_watch_or_dev, format_diagnostics, parse_command_payload,
    print_report, report_error, report_no_input,
};
use crate::console;
use crate::pipeline::daemon::{
    CommandOptionsBuilder, ProtocolDaemonClient, command_inputs_from_sources,
    command_stats_from_protocol, emit_daemon_text_output, run_daemon_command_with_session,
    target_overrides_from_args,
};
use crate::pipeline::input::{ResolveSourcesError, resolve_sources};
use crate::pipeline::script::{ScriptSource, resolve_script_command, shell_command};
use crate::pipeline::target::target_name_from_args;
use crate::pipeline::watch::{
    WatchCompileContext, WatchContext, WatchLoopAction, WatchLoopOptions, build_daemon_options,
    build_watch_context, build_watch_loop_options, emit_watch_compile_report, run_watch_loop,
    watch_error,
};
use crate::pipeline::workspace::default_target_for_session;
use clap::Args;
use destack_daemon::protocol::{
    CommandPayload, CommandRunMode, CommandRunOptions, CommandRunPayload,
};
use destack_source::{DiagnosticOptions, FileSystem};

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

/// Compile and run a source file or script.
pub fn run(args: &RunArgs) -> i32 {
    run_with_request(RunRequest {
        command_name: "run",
        input: args.input.clone(),
        program: args.program.clone(),
        target: args.target.clone(),
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
    let command_name = request.command_name;

    // prepare session context for script detection
    let session = request.program.setup();

    // check for script execution before compilation
    if matches!(request.mode, RunMode::Program)
        && let Some(exit_code) = try_run_script(request, session.fs.as_ref(), &session.cwd)
    {
        return exit_code;
    }

    // resolve input sources for the run
    let sources = match resolve_sources(&request.input, None, None) {
        Ok(sources) => sources,
        Err(ResolveSourcesError::NoInput) => {
            return report_no_input(command_name, &request.report);
        }
        Err(ResolveSourcesError::Message(message)) => {
            return report_error(command_name, &request.report, &message);
        }
    };
    if sources.len() > 1 {
        return report_error(
            command_name,
            &request.report,
            "run expects a single entry module",
        );
    }

    // build command inputs and options for the daemon
    let inputs = match command_inputs_from_sources(&sources, request.input.file_type()) {
        Ok(inputs) => inputs,
        Err(error) => return report_error(command_name, &request.report, &error.to_string()),
    };
    let run_mode = match request.mode {
        RunMode::Program => CommandRunMode::Program,
        RunMode::Eval { print } => CommandRunMode::Eval { print },
    };
    let diagnostic_options: DiagnosticOptions = request.diagnostics.clone().into();
    let target_name = match default_target_for_session(&request.program, &session) {
        Ok(default_target) => {
            let fallback = default_target.as_deref().unwrap_or("native");
            target_name_from_args(&request.target, fallback)
        }
        Err(error) => {
            return report_error(command_name, &request.report, &error.to_string());
        }
    };
    let common = CommandOptionsBuilder::new(&request.program, Some(diagnostic_options.clone()))
        .inputs(inputs)
        .target(target_name)
        .target_overrides(target_overrides_from_args(&request.target))
        .build();
    let payload = CommandPayload::Run(CommandRunOptions {
        entry: Some(request.entry.clone()),
        args: request.args.clone(),
        run_mode,
    });

    // execute the daemon command
    let result = match run_daemon_command_with_session(
        session.clone(),
        &request.program,
        diagnostic_options.clone(),
        common,
        payload,
        None,
    ) {
        Ok(result) => result,
        Err(error) => return report_error(command_name, &request.report, &error.to_string()),
    };

    // emit diagnostics in the requested format
    if request.report.is_json() {
        let format_options = FormatOptions {
            format: DiagnosticFormat::Json,
            ..FormatOptions::default()
        };
        let (output, format_result) =
            collect_diagnostics_json(&result.files, &result.diagnostics, &format_options);

        if format_result.exit_code() != 0 {
            let mut report = CommandReport::failure(command_name, format_result.exit_code());
            if let Some(stats) = result.response.stats.as_ref() {
                report.stats = Some(command_stats_from_protocol(stats));
            }
            report.diagnostics = Some(output);
            print_report(&report, request.report.format());
            return format_result.exit_code();
        }
    } else {
        let format_options = FormatOptions::default();
        let _ = format_diagnostics(
            &result.files,
            &result.diagnostics,
            &format_options,
            result.response.module_count,
        );
        if result.diagnostics.get_status_code() != 0 {
            return result.diagnostics.get_status_code();
        }
    }

    // emit daemon messages and output for non-json runs
    emit_daemon_text_output(
        &request.report,
        &result.response.messages,
        &result.response.output,
    );

    let exit_code = result.response.exit_code;
    if request.report.is_json() {
        // decode payload for structured output
        let payload = match parse_command_payload::<CommandRunPayload>(
            command_name,
            &request.report,
            result.response.data.as_ref(),
            "run",
            false,
        ) {
            Ok(payload) => payload,
            Err(code) => return code,
        };
        let mut report = if exit_code == 0 {
            CommandReport::success(command_name, exit_code)
        } else {
            CommandReport::failure(command_name, exit_code)
        };
        if let Some(stats) = result.response.stats.as_ref() {
            report.stats = Some(command_stats_from_protocol(stats));
        }
        if let Some((payload, value)) = payload {
            report.data = Some(value);
            if let CommandRunPayload::RuntimeError { message } = payload {
                report.summary = Some(message.clone());
                report.error = Some(CommandError::new("runtime_error", "run", &message));
            }
        }
        print_report(&report, request.report.format());
    } else if exit_code != 0 {
        console::warn(&format!("process exited with code {exit_code}"));
    }

    exit_code
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
    let format_options = FormatOptions::default();
    let json_format_options = FormatOptions {
        format: DiagnosticFormat::Json,
        ..FormatOptions::default()
    };

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
                &format_options,
                &json_format_options,
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
    mut on_compile: ObserveFn,
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

    // reject eval mode for watch
    if matches!(request.mode, RunMode::Eval { .. }) {
        return report_error(
            command_name,
            &request.report,
            "--watch does not support --eval",
        );
    }

    // reject watch mode for inline inputs
    if request.input.stdin || !request.input.eval.is_empty() || !request.input.module.is_empty() {
        return report_error(
            command_name,
            &request.report,
            "--watch requires file or directory inputs",
        );
    }

    // prepare session context for watch mode
    let session = request.program.setup();

    // disallow scripts in watch mode
    if matches!(request.mode, RunMode::Program)
        && request.input.files.len() == 1
        && request.input.eval.is_empty()
        && request.input.module.is_empty()
        && !request.input.stdin
    {
        let candidate = &request.input.files[0];
        let candidate_path = if candidate.is_absolute() {
            candidate.clone()
        } else {
            session.cwd.join(candidate)
        };
        if session.fs.metadata(&candidate_path).is_err() {
            return report_error(
                command_name,
                &request.report,
                "--watch does not support script commands",
            );
        }
    }

    // load sources and enforce a single entry module
    let sources = match resolve_sources(&request.input, None, None) {
        Ok(sources) => sources,
        Err(ResolveSourcesError::NoInput) => {
            return report_no_input(command_name, &request.report);
        }
        Err(ResolveSourcesError::Message(message)) => {
            return report_error(command_name, &request.report, &message);
        }
    };
    if sources.len() > 1 {
        return report_error(
            command_name,
            &request.report,
            "run expects a single entry module",
        );
    }

    // resolve the entry source
    let entry_source = sources[0].clone();

    // prepare watch mode output
    let WatchContext {
        roots,
        root,
        mut reporter,
    } = build_watch_context(command_name, &request.program, &request.report, &session);

    // configure the daemon client for incremental updates
    let diagnostic_options: DiagnosticOptions = request.diagnostics.clone().into();
    let daemon_options = build_daemon_options(&request.program, diagnostic_options.clone(), None);
    let daemon = match ProtocolDaemonClient::new(
        session.clone(),
        daemon_options,
        roots.clone(),
        &request.program,
    ) {
        Ok(daemon) => daemon,
        Err(error) => {
            let message = watch_error(&error.to_string());
            if let Some(reporter) = reporter.as_mut() {
                reporter.emit_warning(&message);
                reporter.emit_stop();
                return 1;
            }
            return report_error(command_name, &request.report, &message);
        }
    };

    // resolve the target name for watch executions
    let target_name = match default_target_for_session(&request.program, &session) {
        Ok(default_target) => {
            let fallback = default_target.as_deref().unwrap_or("native");
            target_name_from_args(&request.target, fallback)
        }
        Err(error) => {
            let message = watch_error(&error.to_string());
            if let Some(reporter) = reporter.as_mut() {
                reporter.emit_warning(&message);
                reporter.emit_stop();
                return 1;
            }
            return report_error(command_name, &request.report, &message);
        }
    };

    // set up shared watch state
    let mut watch_state = RunWatchState {
        sources,
        target_name,
    };

    // run the initial compile and execute
    let mut exit_code = compile(
        request,
        &daemon,
        &root,
        &entry_source,
        &watch_state.target_name,
        &mut reporter,
        WatchCompileReason::Startup,
        None,
        false,
        false,
    );

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
            state.sources = match resolve_sources(&request.input, None, None) {
                Ok(sources) => sources,
                Err(ResolveSourcesError::NoInput) => {
                    return Err(watch_error("no input files after rescan").into());
                }
                Err(ResolveSourcesError::Message(message)) => {
                    return Err(watch_error(&message).into());
                }
            };
            if state.sources.len() != 1 {
                return Err(watch_error("expected a single entry module").into());
            }

            // refresh target defaults after rescan
            state.target_name = match default_target_for_session(&request.program, &session) {
                Ok(default_target) => {
                    let fallback = default_target.as_deref().unwrap_or("native");
                    target_name_from_args(&request.target, fallback)
                }
                Err(error) => {
                    return Err(watch_error(&error.to_string()).into());
                }
            };

            Ok(())
        },
        |state, reporter, reason, batch_id, updated, requires_rescan| {
            // recompile and rerun when updates occur
            let next_exit_code = compile(
                request,
                &daemon,
                &root,
                &state.sources[0],
                &state.target_name,
                reporter,
                reason,
                Some(batch_id),
                updated,
                requires_rescan,
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

/// Compile the entry module and run the program through the daemon.
#[allow(clippy::too_many_arguments)]
fn compile_and_run_daemon(
    request: &RunRequest,
    daemon: &ProtocolDaemonClient,
    root: &Path,
    entry_source: &InputSource,
    target_name: &str,
    watch_reporter: &mut Option<WatchReporter>,
    format_options: &FormatOptions,
    json_format_options: &FormatOptions,
    compile_reason: WatchCompileReason,
    batch_id: Option<u64>,
    updated: bool,
    rescan: bool,
) -> i32 {
    // build the daemon command options
    let inputs = match command_inputs_from_sources(
        std::slice::from_ref(entry_source),
        request.input.file_type(),
    ) {
        Ok(inputs) => inputs,
        Err(error) => {
            let message = watch_error(&error.to_string());
            if let Some(reporter) = watch_reporter.as_mut() {
                reporter.emit_warning(&message);
                return 1;
            }
            return report_error(request.command_name, &request.report, &message);
        }
    };
    let run_mode = match request.mode {
        RunMode::Program => CommandRunMode::Program,
        RunMode::Eval { print } => CommandRunMode::Eval { print },
    };
    let diagnostic_options: DiagnosticOptions = request.diagnostics.clone().into();
    let common = CommandOptionsBuilder::new(&request.program, Some(diagnostic_options.clone()))
        .inputs(inputs)
        .target(target_name.to_string())
        .target_overrides(target_overrides_from_args(&request.target))
        .build();
    let payload = CommandPayload::Run(CommandRunOptions {
        entry: Some(request.entry.clone()),
        args: request.args.clone(),
        run_mode,
    });

    // execute the daemon command
    let result = match daemon.run_command(root, common, payload) {
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
    let stats = result
        .response
        .stats
        .as_ref()
        .map(command_stats_from_protocol);
    let diagnostic_exit = emit_watch_compile_report(
        watch_reporter,
        WatchCompileContext {
            files: &result.files,
            diagnostics: &result.diagnostics,
            format_options,
            json_format_options,
            module_count: result.response.module_count,
            line_writer: None,
        },
        stats,
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

/// Attempt to run a dsconfig or package.json script when input is not a file.
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

    // resolve script command from dsconfig or package.json
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
            ScriptSource::DsConfig => "dsconfig",
            ScriptSource::PackageJson => "package.json",
        };
        let mut report = CommandReport::success(command_name, exit_code);
        report.data = Some(serde_json::json!({
            "script": script.name,
            "command": command,
            "cwd": script.cwd,
            "source": source,
        }));
        print_report(&report, request.report.format());
    } else if exit_code != 0 {
        console::warn(&format!("process exited with code {exit_code}"));
    }

    Some(exit_code)
}
