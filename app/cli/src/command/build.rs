use destack_daemon::protocol::{CommandBuildOptions, CommandPayload, CommonCommandOptions};

use crate::common::{
    DiagnosticArgs, DiagnosticFormat, FormatOptions, InputArgs, InputSource, ProgramArgs,
    ReportArgs, TargetArgs, WatchCompileReason, report_error,
};
use crate::error::CliResult;
use crate::pipeline::daemon::{
    CommandOptionsBuilder, DiagnosticCommandSummary, command_inputs_from_sources,
    emit_daemon_text_output, finish_diagnostic_command, run_root_command_or_report,
    target_overrides_from_args,
};
use crate::pipeline::input::{ResolveSourcesError, resolve_sources};
use crate::pipeline::target::target_name_from_args;
use crate::pipeline::watch::{
    WatchCompileContext, WatchLoopOptions, build_watch_loop_options, emit_watch_compile_report,
    run_daemon_watch_command, watch_error,
};
use crate::pipeline::workspace::{load_destack_declaration_for_program, workspace_context};
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
    // resolve target name (prefer destack.json default target for package builds)
    let target_name = if args.input.has_input() {
        target_name_from_args(&args.target, "default")
    } else {
        let context = match workspace_context(&args.program, None) {
            Ok(context) => context,
            Err(error) => return report_error("build", &args.report, &error.to_string()),
        };
        let declaration = match load_destack_declaration_for_program(
            &args.program,
            &context.repository,
            context.revision,
            context.repository.workspace_root(),
        ) {
            Ok(declaration) => declaration,
            Err(error) => return report_error("build", &args.report, &error.to_string()),
        };
        declaration
            .package_options()
            .default_target
            .unwrap_or_else(|| "default".to_string())
    };

    if args.program.watch {
        return run_watch(args, &target_name);
    }

    run_build_via_daemon(args, &target_name)
}

/// Run a single build command through the daemon.
fn run_build_via_daemon(args: &BuildArgs, target_name: &str) -> i32 {
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
    let options = CommandOptionsBuilder::new(&args.program)
        .inputs(inputs)
        .allow_destack_config_fallback(!args.input.has_input())
        .target(target_name.to_string())
        .target_overrides(target_overrides_from_args(&args.target))
        .dry_run(args.dry_run)
        .build();
    let payload = CommandPayload::Build(CommandBuildOptions::default());

    // execute the daemon command
    let result =
        match run_root_command_or_report("build", &args.report, &args.program, options, payload) {
            Ok(result) => result,
            Err(code) => return code,
        };

    let data = match result.response.data.as_ref() {
        Some(payload) => match payload.to_json_value() {
            Ok(value) => Some(value),
            Err(error) => {
                return report_error(
                    "build",
                    &args.report,
                    &format!("invalid build payload: {error}"),
                );
            }
        },
        None => None,
    };

    let json_format_options = FormatOptions {
        format: DiagnosticFormat::Json,
        ..FormatOptions::default()
    };
    let text_format_options = FormatOptions {
        format: DiagnosticFormat::Text,
        ..FormatOptions::default()
    };
    finish_diagnostic_command(
        "build",
        &args.report,
        &result,
        &json_format_options,
        &text_format_options,
        None,
        Some(DiagnosticCommandSummary {
            verb: "Built",
            modules: result.response.module_count,
            profiles: result.response.profile_count,
            targets: result.response.target_count,
        }),
        data,
    )
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
    on_compile: ObserveFn,
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

    // prepare watch mode output
    let json_format_options = FormatOptions {
        format: DiagnosticFormat::Json,
        ..FormatOptions::default()
    };
    let session = args.program.setup();

    // set up shared watch state
    let mut watch_state = BuildWatchState { sources };

    // prepare command options for the watch run
    let target_overrides = target_overrides_from_args(&args.target);
    let build_options = |sources: &[InputSource]| -> CliResult<CommonCommandOptions> {
        let inputs = command_inputs_from_sources(sources, args.input.file_type())?;
        Ok(CommandOptionsBuilder::new(&args.program)
            .inputs(inputs)
            .allow_destack_config_fallback(!args.input.has_input())
            .target(target_name.to_string())
            .target_overrides(target_overrides.clone())
            .build())
    };

    let format_options = FormatOptions::default();

    run_daemon_watch_command(
        "build",
        session,
        &args.program,
        &args.report,
        None,
        watch_loop_options,
        &mut watch_state,
        move |_state| on_start(),
        |state, _session| {
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
        |daemon, root, reporter, state, reason, batch_id, updated, requires_rescan| {
            // build options for the updated sources
            let options = match build_options(&state.sources) {
                Ok(options) => options,
                Err(message) => {
                    let message = watch_error(&message.to_string());
                    if let Some(reporter) = reporter.as_mut() {
                        reporter.emit_warning(&message);
                        return 1;
                    }
                    let next_exit = report_error("build", &args.report, &message);
                    return next_exit;
                }
            };

            // run the daemon build command
            let result = match daemon.run_root_command(
                root,
                options,
                CommandPayload::Build(CommandBuildOptions::default()),
            ) {
                Ok(result) => result,
                Err(message) => {
                    let message = watch_error(&message.to_string());
                    if let Some(reporter) = reporter.as_mut() {
                        reporter.emit_warning(&message);
                        return 1;
                    }
                    let next_exit = report_error("build", &args.report, &message);
                    return next_exit;
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
            emit_watch_compile_report(
                reporter,
                WatchCompileContext {
                    files: &result.files,
                    diagnostics: &result.diagnostics,
                    format_options: &format_options,
                    json_format_options: &json_format_options,
                    module_count: result.response.module_count,
                    line_writer: None,
                },
                reason,
                updated,
                requires_rescan,
                batch_id,
            )
        },
        on_compile,
        is_one_shot,
    )
}
