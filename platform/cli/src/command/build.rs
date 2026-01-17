use clap::Args;
use destack_daemon::Daemon;

use crate::common::{
    CommandReport, CommandStats, CompilerMode, DiagnosticArgs, DiagnosticFormat, FormatOptions,
    InputArgs, InputSource, ProgramArgs, ReportArgs, StatsSummary, TargetArgs, WatchCompileReason,
    WatchReporter, collect_diagnostics_json, format_diagnostics, print_report, print_stats_summary,
    report_error,
};
use crate::console;
use crate::pipeline::compile::{CompileRequest, prepare_compile};
use crate::pipeline::input::{ResolveSourcesError, resolve_sources};
use crate::pipeline::target::{resolve_target_for_module, target_name_from_args};
use crate::pipeline::watch::{
    WatchCompileContext, WatchLoopAction, WatchLoopOptions, build_daemon_options,
    build_watch_loop_options, emit_watch_compile_report, run_watch_loop, watch_error, watch_roots,
};
use crate::pipeline::workspace::{load_dsconfig_for_program, workspace_context};

/// State for build watch mode.
struct BuildWatchState {
    /// The resolved input sources.
    sources: Vec<InputSource>,
    /// The active module list.
    modules: Vec<destack_source::ModuleId>,
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
            Err(message) => return report_error("build", &args.report, &message),
        };
        let dsconfig =
            match load_dsconfig_for_program(&args.program, &context.resolver, &context.session.cwd)
            {
                Ok(dsconfig) => dsconfig,
                Err(message) => return report_error("build", &args.report, &message),
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

    let setup = match prepare_compile(CompileRequest {
        command: "build",
        input: &args.input,
        program: &args.program,
        diagnostics: &args.diagnostics,
        report: &args.report,
        mode: CompilerMode::Build {
            target: target_name.clone(),
        },
        target_name: Some(&target_name),
        allow_dsconfig_fallback: true,
        event_handler: None,
    }) {
        Ok(setup) => setup,
        Err(code) => return code,
    };

    if args.dry_run {
        return report_dry_run(args, &target_name, &setup.sources);
    }

    // enqueue sources for compilation
    let modules = setup.modules;

    // ensure targets exist for each module
    for module_id in &modules {
        if let Err(message) = resolve_target_for_module(
            &setup.context.program,
            *module_id,
            &target_name,
            &args.target,
        ) {
            return report_error("build", &args.report, &message);
        }
    }

    // compile and collect diagnostics
    let result = setup.context.compile();
    let diagnostics = result
        .program
        .diagnostics
        .collect()
        .map(&result.diagnostic_options);

    // report diagnostics in requested format
    if args.report.is_json() {
        let format_options = FormatOptions {
            format: DiagnosticFormat::Json,
            ..FormatOptions::default()
        };
        let (output, format_result) =
            collect_diagnostics_json(&result.program.files, &diagnostics, &format_options);
        let mut report = if format_result.exit_code() == 0 {
            CommandReport::success("build", 0)
        } else {
            CommandReport::failure("build", format_result.exit_code())
        };
        report.diagnostics = Some(output);
        report.stats = Some(CommandStats::from_snapshot(&result.stats));
        print_report(&report, args.report.format());
        return format_result.exit_code();
    }

    let format_options = FormatOptions::default();
    let format_result = format_diagnostics(
        &result.program.files,
        &diagnostics,
        &format_options,
        result.program.modules.len(),
    );

    // print stats summary in text mode
    let module_count = modules.len();
    let profile_count = result.program.profiles.len();
    let summary = StatsSummary {
        verb: "Built",
        modules: module_count,
        profiles: profile_count,
        targets: usize::from(!modules.is_empty()),
        errors: format_result.error_count,
        warnings: format_result.warning_count,
    };
    print_stats_summary(&summary, &result.stats, None);

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

    // create the compiler context
    let context = crate::common::CompilerContext::new(
        &args.program,
        &args.diagnostics,
        CompilerMode::Build {
            target: target_name.to_string(),
        },
        None,
    );

    // enqueue sources for compilation
    let modules = match context.enqueue(&sources) {
        Ok(modules) => modules,
        Err(code) => return code,
    };

    // prepare watch mode output
    let json_format_options = FormatOptions {
        format: DiagnosticFormat::Json,
        ..FormatOptions::default()
    };
    let mut reporter = if args.report.is_json() {
        Some(WatchReporter::new("build"))
    } else {
        None
    };
    let roots = watch_roots(&args.program, &context.session);
    if let Some(reporter) = reporter.as_mut() {
        reporter.emit_start(&roots);
    }

    // ensure targets exist for each module
    if let Err(message) = validate_targets(&context.program, &modules, target_name, &args.target) {
        if let Some(reporter) = reporter.as_mut() {
            reporter.emit_warning(&watch_error(&message));
            reporter.emit_stop();
            return 1;
        }
        return report_error("build", &args.report, &message);
    }

    // compile the initial state
    context.run_compile();

    // print diagnostics for the initial state
    let format_options = FormatOptions::default();
    let stats_snapshot = reporter.as_ref().map(|_| {
        context
            .compiler
            .stats
            .snapshot_with_program(context.program.modules.len(), Some(&context.program))
    });
    let mut exit_code = emit_watch_compile_report(
        &mut reporter,
        WatchCompileContext {
            program: &context.program,
            diagnostic_options: &context.diagnostic_options,
            format_options: &format_options,
            json_format_options: &json_format_options,
            module_count: modules.len(),
            line_writer: None,
        },
        stats_snapshot,
        WatchCompileReason::Startup,
        false,
        false,
        None,
    );

    // configure the daemon for incremental updates
    let daemon_options =
        build_daemon_options(&args.program, context.diagnostic_options.clone(), None);
    let daemon = Daemon::with_options(context.session.clone(), daemon_options);

    // set up shared watch state
    let mut watch_state = BuildWatchState { sources, modules };

    // run the watch loop for incremental updates
    exit_code = run_watch_loop(
        &daemon,
        &context.session,
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
                        return Err(watch_error("no input files after rescan"));
                    }
                    Err(ResolveSourcesError::Message(message)) => {
                        return Err(watch_error(&message));
                    }
                };

            Ok(())
        },
        |state, reporter, reason, batch_id, updated, requires_rescan| {
            // clear diagnostics before each compile
            let _ = context.program.diagnostics.drain();
            let Ok(next_modules) = context.enqueue(&state.sources) else {
                return WatchLoopAction::continue_with(None);
            };
            state.modules = next_modules;

            // ensure targets exist for each module
            if let Err(message) =
                validate_targets(&context.program, &state.modules, target_name, &args.target)
            {
                let next_exit_code = if let Some(reporter) = reporter.as_mut() {
                    reporter.emit_warning(&watch_error(&message));
                    1
                } else {
                    report_error("build", &args.report, &message)
                };

                return WatchLoopAction::continue_with(Some(next_exit_code));
            }

            // compile the updated state
            context.run_compile();

            // report diagnostics for the updated state
            let stats_snapshot = reporter.as_ref().map(|_| {
                context
                    .compiler
                    .stats
                    .snapshot_with_program(context.program.modules.len(), Some(&context.program))
            });
            let next_exit_code = emit_watch_compile_report(
                reporter,
                WatchCompileContext {
                    program: &context.program,
                    diagnostic_options: &context.diagnostic_options,
                    format_options: &format_options,
                    json_format_options: &json_format_options,
                    module_count: state.modules.len(),
                    line_writer: None,
                },
                stats_snapshot,
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

    exit_code
}

/// Ensure targets exist for each module in watch mode.
fn validate_targets(
    program: &destack_workspace::Program,
    modules: &[destack_source::ModuleId],
    target_name: &str,
    target_args: &TargetArgs,
) -> Result<(), String> {
    // check targets for each module
    for module_id in modules {
        resolve_target_for_module(program, *module_id, target_name, target_args)?;
    }

    Ok(())
}

/// Report build dry-run output.
fn report_dry_run(args: &BuildArgs, target_name: &str, sources: &[InputSource]) -> i32 {
    let source_names: Vec<String> = sources
        .iter()
        .map(|source| match source {
            InputSource::File(path) => path.display().to_string(),
            InputSource::Inline { name, .. } => name.clone(),
            InputSource::Stdin { name } => name.clone(),
        })
        .collect();

    if args.report.is_json() {
        let mut report = CommandReport::success("build", 0);
        report.data = Some(serde_json::json!({
            "dry_run": true,
            "target": target_name,
            "sources": source_names,
        }));
        print_report(&report, args.report.format());
        return 0;
    }

    console::info("build dry-run:");
    console::info(&format!("target: {target_name}"));
    for source in source_names {
        console::info(&format!("source: {source}"));
    }

    0
}
