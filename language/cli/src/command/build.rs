use std::time::Instant;

use destack_workspace::{BuildInput, BuildOutputs, CommandRevision};

use crate::common::{
    CommandOptionsBuilder, CommandResult, CommandSummary, DiagnosticFormat, FormatOptions,
    InputArgs, InputSource, ProgramArgs, ReportArgs, TargetArgs, WatchCompileContext,
    WatchCompileReason, WatchCycle, WorkspaceWatch, command_data_json, command_error,
    command_inputs_from_sources, emit_watch_compile_report, emit_workspace_text_output,
    finish_diagnostic_command, report_error, run_workspace_command_or_report,
    target_overrides_from_args, watch_error,
};
use crate::diagnostic::ConsoleResult;
use clap::Args;
use destack_workspace::WatchPolicy;

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

    /// Report output options.
    #[command(flatten)]
    pub report: ReportArgs,

    /// Show what would be built without compiling.
    #[arg(long)]
    pub dry_run: bool,
}

/// Compile source files and produce output.
pub fn run(args: &BuildArgs) -> i32 {
    if args.program.watch {
        return run_watch(args);
    }

    run_build(args)
}

/// Run a single build command.
fn run_build(args: &BuildArgs) -> i32 {
    let started_at = Instant::now();

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

    // build command options for workspace execution
    let common = match CommandOptionsBuilder::new(&args.program) {
        Ok(common) => common,
        Err(error) => return report_error("build", &args.report, &error.to_string()),
    }
    .inputs(inputs)
    .config_inputs(!args.input.has_input())
    .target(args.target.target_name().map(str::to_string))
    .target_overrides(target_overrides_from_args(&args.target))
    .dry_run(args.dry_run)
    .build();
    let request = BuildInput {
        product: None,
        outputs: build_outputs(),
        ..(CommandRevision::Current, common).into()
    };

    // execute the workspace command
    let result = match run_workspace_command_or_report(
        "build",
        &args.report,
        &args.program,
        |workspace, root, progress| {
            let result = workspace
                .build(root, request, progress)
                .map_err(command_error)?;

            CommandResult::from_output(result)
        },
    ) {
        Ok(result) => result,
        Err(code) => return code,
    };

    let data = match command_data_json("build", &args.report, Some(&result.response.data)) {
        Ok(data) => data,
        Err(code) => return code,
    };

    let json_format_options = FormatOptions {
        format: DiagnosticFormat::Json,
        ..FormatOptions::default()
    };
    let text_format_options = FormatOptions {
        format: DiagnosticFormat::Text,
        ..FormatOptions::default()
    };
    let summary = CommandSummary {
        verb: "Built",
        modules: result.response.module_count,
        targets: result.response.target_count,
        duration: started_at.elapsed(),
    };
    finish_diagnostic_command(
        "build",
        &args.report,
        &result,
        &json_format_options,
        &text_format_options,
        None,
        Some(summary),
        data,
    )
}

/// Compile source files and produce output in watch mode.
fn run_watch(args: &BuildArgs) -> i32 {
    // run with default watch settings
    run_watch_with_options(args, WatchPolicy::default(), || {}, |_, _, _| {}, false)
}

/// Compile source files and produce output in watch mode with injected options.
pub(crate) fn run_watch_with_options<StartFn, ObserveFn>(
    args: &BuildArgs,
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

    let sources = match resolve_watch_sources(&args.input) {
        Ok(sources) => sources,
        Err(error) => return report_error("build", &args.report, &error.to_string()),
    };

    // prepare watch mode output
    let json_format_options = FormatOptions {
        format: DiagnosticFormat::Json,
        ..FormatOptions::default()
    };
    // set up shared watch state
    let mut watch_state = BuildWatchState { sources };

    // prepare command options for the watch run
    let target_overrides = target_overrides_from_args(&args.target);
    let build_request = |sources: &[InputSource]| -> ConsoleResult<BuildInput> {
        let inputs = command_inputs_from_sources(sources, args.input.file_type())?;
        let common = CommandOptionsBuilder::new(&args.program)?
            .inputs(inputs)
            .config_inputs(!args.input.has_input())
            .target(args.target.target_name().map(str::to_string))
            .target_overrides(target_overrides.clone())
            .build();

        Ok(BuildInput {
            product: None,
            outputs: build_outputs(),
            ..(CommandRevision::Current, common).into()
        })
    };

    let format_options = FormatOptions::default();

    let mut watch = match WorkspaceWatch::start("build", &args.program, &args.report, watch_policy)
    {
        Ok(watch) => watch,
        Err(code) => return code,
    };

    on_start();

    let compile = |watch: &mut WorkspaceWatch, state: &BuildWatchState, cycle: WatchCycle| {
        watch.run_command(|workspace, root, reporter| {
            // build options for the updated sources
            let request = match build_request(&state.sources) {
                Ok(request) => request,
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

            // run the workspace build command
            let result = match workspace.build(root, request, None) {
                Ok(result) => match CommandResult::from_output(result) {
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
                },
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

            // emit workspace output for text mode
            if !args.report.is_json() {
                emit_workspace_text_output(
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
                cycle,
            )
        })
    };

    let mut exit_code = compile(&mut watch, &watch_state, WatchCycle::startup());
    while let Some(cycle) = watch.next_cycle() {
        // refresh sources when the workspace requests a rescan
        if cycle.requires_rescan {
            watch_state.sources = match resolve_watch_sources(&args.input) {
                Ok(sources) => sources,
                Err(error) => {
                    watch.emit_warning(&watch_error(&error.to_string()));
                    continue;
                }
            };
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

/// Resolve explicit sources for build watch mode.
fn resolve_watch_sources(input: &InputArgs) -> ConsoleResult<Vec<InputSource>> {
    input.explicit_sources()
}

/// Return default build output families for the CLI.
fn build_outputs() -> BuildOutputs {
    BuildOutputs {
        products: true,
        bundles: true,
        programs: true,
        assets: false,
    }
}
