use destack_daemon::protocol::{CommandBuildOptions, CommandPayload, CommonCommandOptions};
use destack_source::DiagnosticOptions;

use crate::common::{
    CommandReport, DiagnosticArgs, DiagnosticFormat, FormatOptions, InputArgs, InputSource,
    ProgramArgs, ReportArgs, StatsSummary, TargetArgs, WatchCompileReason,
    collect_diagnostics_json, format_diagnostics, print_command_stats_summary, print_report,
    report_error,
};
use crate::error::CliResult;
use crate::pipeline::daemon::{
    CommandOptionsBuilder, ProtocolDaemonClient, command_inputs_from_sources,
    command_stats_from_protocol, emit_daemon_text_output, run_daemon_command,
    target_overrides_from_args,
};
use crate::pipeline::input::{ResolveSourcesError, resolve_sources};
use crate::pipeline::target::target_name_from_args;
use crate::pipeline::watch::{
    WatchCompileContext, WatchContext, WatchLoopAction, WatchLoopOptions, build_daemon_options,
    build_watch_context, build_watch_loop_options, emit_watch_compile_report, run_watch_loop,
    watch_error,
};
use crate::pipeline::workspace::{load_dsconfig_for_program, workspace_context};
use clap::Args;

/// State for build watch mode.
struct BuildWatchState {
    /// The resolved input sources.
    sources: Vec<InputSource>,
}

#[derive(Args, Debug, Clone)]
pub struct BuildArgs {
    /// Input arguments.
    #[command(flatten)]
    pub input: InputArgs,

    /// Target configuration.
    #[command(flatten)]
    pub target: TargetArgs,

    /// The program options.
    #[command(flatten)]
    pub program: ProgramArgs,

    /// The diagnostic options.
    #[command(flatten)]
    pub diagnostics: DiagnosticArgs,

    /// Report output options.
    #[command(flatten)]
    pub report: ReportArgs,

    /// Show what would be built without compiling.
    #[arg(long)]
    pub dry_run: bool,
}

/// Compile source files and produce output.
pub fn run(args: &BuildArgs) -> i32 {
    // resolve target name (prefer dsconfig default target for package builds)
    let target_name = if args.input.has_input() {
        target_name_from_args(&args.target, "default")
    } else {
        let context = match workspace_context(&args.program, None) {
            Ok(context) => context,
            Err(error) => return report_error("build", &args.report, &error.to_string()),
        };
        let dsconfig =
            match load_dsconfig_for_program(&args.program, &context.resolver, &context.session.cwd)
            {
                Ok(dsconfig) => dsconfig,
                Err(error) => return report_error("build", &args.report, &error.to_string()),
            };
        dsconfig
            .options
            .default_target
            .clone()
            .unwrap_or_else(|| "default".to_string())
    };

    if args.program.watch {
        return run_watch(args, &target_name);
    }

    run_build_via_daemon(args, &target_name)
}

/// Run a single build command through the daemon.
fn run_build_via_daemon(args: &BuildArgs, target_name: &str) -> i32 {
    // build diagnostic options for the daemon command
    let diagnostic_options: DiagnosticOptions = args.diagnostics.clone().into();

    // build command inputs when explicitly provided
    let inputs = if args.input.has_input() {
        let sources = match args.input.to_sources() {
            Ok(sources) => sources,
            Err(error) => {
                return report_error("build", &args.report, &error.to_string());
            }
        };
        match command_inputs_from_sources(&sources, args.input.file_type()) {
            Ok(inputs) => inputs,
            Err(error) => {
                return report_error("build", &args.report, &error.to_string());
            }
        }
    } else {
        Vec::new()
    };

    // build command options for daemon execution
    let options = CommandOptionsBuilder::new(&args.program, Some(diagnostic_options.clone()))
        .inputs(inputs)
        .allow_dsconfig_fallback(!args.input.has_input())
        .target(target_name.to_string())
        .target_overrides(target_overrides_from_args(&args.target))
        .dry_run(args.dry_run)
        .build();
    let payload = CommandPayload::Build(CommandBuildOptions::default());

    // execute the daemon command
    let result = match run_daemon_command(
        &args.program,
        Some(diagnostic_options),
        options,
        payload,
        None,
    ) {
        Ok(result) => result,
        Err(error) => return report_error("build", &args.report, &error.to_string()),
    };

    // emit daemon output and messages for text modes
    if !args.report.is_json() {
        emit_daemon_text_output(
            &args.report,
            &result.response.messages,
            &result.response.output,
        );
    }

    // report diagnostics in requested format
    if args.report.is_json() {
        let format_options = FormatOptions {
            format: DiagnosticFormat::Json,
            ..FormatOptions::default()
        };
        let (output, format_result) =
            collect_diagnostics_json(&result.files, &result.diagnostics, &format_options);
        let mut report = if format_result.exit_code() == 0 {
            CommandReport::success("build", 0)
        } else {
            CommandReport::failure("build", format_result.exit_code())
        };
        if let Some(stats) = result.response.stats.as_ref() {
            report.stats = Some(command_stats_from_protocol(stats));
        }
        if let Some(payload) = result.response.data.as_ref() {
            match payload.to_json_value() {
                Ok(value) => report.data = Some(value),
                Err(error) => {
                    return report_error(
                        "build",
                        &args.report,
                        &format!("invalid build payload: {error}"),
                    );
                }
            }
        }
        report.diagnostics = Some(output);
        print_report(&report, args.report.format());
        return format_result.exit_code();
    }

    let format_options = FormatOptions {
        format: DiagnosticFormat::Text,
        ..FormatOptions::default()
    };
    let format_result = format_diagnostics(
        &result.files,
        &result.diagnostics,
        &format_options,
        result.response.module_count,
    );

    // print stats summary in text mode
    if let Some(stats) = result.response.stats.as_ref() {
        let summary = StatsSummary {
            verb: "Built",
            modules: result.response.module_count,
            profiles: result.response.profile_count,
            targets: result.response.target_count,
            errors: format_result.error_count,
            warnings: format_result.warning_count,
        };
        let stats = command_stats_from_protocol(stats);
        print_command_stats_summary(&summary, &stats, None);
    }

    format_result.exit_code()
}

/// Compile source files and produce output in watch mode.
fn run_watch(args: &BuildArgs, target_name: &str) -> i32 {
    // run with default watch settings
    run_watch_with_options(
        args,
        target_name,
        build_watch_loop_options(),
        || {},
        |_, _, _| {},
        false,
    )
}

/// Compile source files and produce output in watch mode with injected options.
pub(crate) fn run_watch_with_options<StartFn, ObserveFn>(
    args: &BuildArgs,
    target_name: &str,
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
    if args.dry_run {
        return report_error("build", &args.report, "--watch does not support --dry-run");
    }
    if args.input.stdin || !args.input.eval.is_empty() || !args.input.module.is_empty() {
        return report_error(
            "build",
            &args.report,
            "--watch requires file or directory inputs",
        );
    }

    // resolve sources for the initial compile
    let sources = match resolve_sources(&args.input, Some(&args.program), Some(target_name)) {
        Ok(sources) => sources,
        Err(ResolveSourcesError::NoInput) => {
            return report_error("build", &args.report, "no input files provided");
        }
        Err(ResolveSourcesError::Message(message)) => {
            return report_error("build", &args.report, &message);
        }
    };

    // build diagnostic options for the daemon command
    let diagnostic_options: DiagnosticOptions = args.diagnostics.clone().into();

    // prepare watch mode output
    let json_format_options = FormatOptions {
        format: DiagnosticFormat::Json,
        ..FormatOptions::default()
    };
    let session = args.program.setup();
    let WatchContext {
        roots,
        root,
        mut reporter,
    } = build_watch_context("build", &args.program, &args.report, &session);

    // configure the daemon client for incremental updates
    let daemon_options = build_daemon_options(&args.program, diagnostic_options.clone(), None);
    let daemon = match ProtocolDaemonClient::new(session.clone(), daemon_options, roots.clone()) {
        Ok(daemon) => daemon,
        Err(error) => {
            let message = watch_error(&error.to_string());
            if let Some(reporter) = reporter.as_mut() {
                reporter.emit_warning(&message);
                reporter.emit_stop();
                return 1;
            }
            return report_error("build", &args.report, &message);
        }
    };

    // set up shared watch state
    let mut watch_state = BuildWatchState { sources };

    // prepare command options for the watch run
    let target_overrides = target_overrides_from_args(&args.target);
    let build_options = |sources: &[InputSource]| -> CliResult<CommonCommandOptions> {
        let inputs = command_inputs_from_sources(sources, args.input.file_type())?;
        Ok(
            CommandOptionsBuilder::new(&args.program, Some(diagnostic_options.clone()))
                .inputs(inputs)
                .allow_dsconfig_fallback(!args.input.has_input())
                .target(target_name.to_string())
                .target_overrides(target_overrides.clone())
                .build(),
        )
    };

    // compile the initial state
    let format_options = FormatOptions::default();
    let mut exit_code = match build_options(&watch_state.sources) {
        Ok(options) => match daemon.run_command(
            &root,
            options,
            CommandPayload::Build(CommandBuildOptions::default()),
        ) {
            Ok(result) => {
                if !args.report.is_json() {
                    emit_daemon_text_output(
                        &args.report,
                        &result.response.messages,
                        &result.response.output,
                    );
                }
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
                        format_options: &format_options,
                        json_format_options: &json_format_options,
                        module_count: result.response.module_count,
                        line_writer: None,
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
                    report_error("build", &args.report, &message)
                }
            }
        },
        Err(message) => {
            let message = watch_error(&message.to_string());
            if let Some(reporter) = reporter.as_mut() {
                reporter.emit_warning(&message);
                1
            } else {
                report_error("build", &args.report, &message)
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
            state.sources =
                match resolve_sources(&args.input, Some(&args.program), Some(target_name)) {
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
            let options = match build_options(&state.sources) {
                Ok(options) => options,
                Err(message) => {
                    let message = watch_error(&message.to_string());
                    if let Some(reporter) = reporter.as_mut() {
                        reporter.emit_warning(&message);
                        return WatchLoopAction::continue_with(Some(1));
                    }
                    let next_exit = report_error("build", &args.report, &message);
                    return WatchLoopAction::continue_with(Some(next_exit));
                }
            };

            // run the daemon build command
            let result = match daemon.run_command(
                &root,
                options,
                CommandPayload::Build(CommandBuildOptions::default()),
            ) {
                Ok(result) => result,
                Err(message) => {
                    let message = watch_error(&message.to_string());
                    if let Some(reporter) = reporter.as_mut() {
                        reporter.emit_warning(&message);
                        return WatchLoopAction::continue_with(Some(1));
                    }
                    let next_exit = report_error("build", &args.report, &message);
                    return WatchLoopAction::continue_with(Some(next_exit));
                }
            };

            // emit daemon output for text mode
            if !args.report.is_json() {
                emit_daemon_text_output(
                    &args.report,
                    &result.response.messages,
                    &result.response.output,
                );
            }

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
                    format_options: &format_options,
                    json_format_options: &json_format_options,
                    module_count: result.response.module_count,
                    line_writer: None,
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
