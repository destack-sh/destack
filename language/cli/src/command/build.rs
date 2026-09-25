use std::time::Instant;

use tspp_workspace::{BuildInput, BuildOutputs, BuildRequest, CommandRevision};

use crate::common::{
    CommandOptionsBuilder, CommandResult, CommandSummary, DiagnosticFormat, FormatOptions,
    InputArgs, InputSource, ProgramArgs, ReportArgs, TargetArgs, WatchCompileContext, WatchCycle,
    WorkspaceWatch, command_data_json, command_error, emit_workspace_text_output,
    finish_diagnostic_command, report_error, run_workspace_command_or_report,
    target_overrides_from_args,
};
use crate::diagnostic::ConsoleResult;
use clap::Args;

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
pub async fn run(args: &BuildArgs) -> i32 {
    if args.program.watch {
        return run_watch(args);
    }

    run_build(args).await
}

/// Run a single build command.
async fn run_build(args: &BuildArgs) -> i32 {
    let started_at = Instant::now();

    // build command inputs when explicitly provided
    let inputs = if args.input.has_input() {
        let sources = match args.input.to_sources() {
            Ok(sources) => sources,
            Err(error) => {
                return report_error("build", &args.report, &error.to_string());
            }
        };
        match args.input.command_inputs(&sources) {
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
        async |workspace, progress| {
            let result = workspace
                .build(request, progress)
                .await
                .map_err(command_error)?;

            CommandResult::from_output(result)
        },
    )
    .await
    {
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
    let mut sources = sources;

    // prepare command options for the watch run
    let target_overrides = target_overrides_from_args(&args.target);
    let build_request =
        |sources: &[InputSource], revision: CommandRevision| -> ConsoleResult<BuildInput> {
            let inputs = args.input.command_inputs(sources)?;
            let common = CommandOptionsBuilder::new(&args.program)?
                .inputs(inputs)
                .config_inputs(!args.input.has_input())
                .target(args.target.target_name().map(str::to_string))
                .target_overrides(target_overrides.clone())
                .build();

            Ok(BuildInput {
                product: None,
                outputs: build_outputs(),
                ..(revision, common).into()
            })
        };

    let format_options = FormatOptions::default();

    let mut watch = match WorkspaceWatch::start("build", &args.program, &args.report) {
        Ok(watch) => watch,
        Err(code) => return code,
    };

    let compile = |watch: &mut WorkspaceWatch<'_>, sources: &[InputSource], cycle: WatchCycle| {
        // build options for the updated sources
        let revision = CommandRevision::Exact(cycle.revision);
        let request = match build_request(sources, revision) {
            Ok(request) => request,
            Err(error) => return watch.report_error(&error.to_string()),
        };

        // run the workspace build command
        let output = match watch.command(|workspace, root| {
            workspace.build(BuildRequest {
                root: root.to_path_buf(),
                input: request,
            })
        }) {
            Ok(output) => output,
            Err(code) => return code,
        };
        let result = match CommandResult::from_output(output) {
            Ok(result) => result,
            Err(error) => return watch.report_error(&error.to_string()),
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
        WatchCompileContext {
            files: &result.files,
            diagnostics: &result.diagnostics,
            format_options: &format_options,
            json_format_options: &json_format_options,
            module_count: result.response.module_count,
            line_writer: None,
        }
        .emit(watch, cycle)
    };

    let startup = watch.startup();
    compile(&mut watch, &sources, startup);

    loop {
        // wait for the next exact semantic revision
        let cycle = match watch.next_cycle() {
            Ok(cycle) => cycle,
            Err(code) => return watch.finish(code),
        };

        // refresh explicit paths when the workspace manifest changed
        if cycle.requires_source_refresh {
            sources = match resolve_watch_sources(&args.input) {
                Ok(sources) => sources,
                Err(error) => {
                    watch.emit_warning(&error.to_string());
                    continue;
                }
            };
        }

        compile(&mut watch, &sources, cycle);
    }
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
