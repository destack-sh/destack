use std::sync::Arc;
use std::time::Instant;

use clap::Args;
use serde::Serialize;
use tspp_artifact::{ArtifactPayload, ConditionSet, Host, Platform, Runtime};
use tspp_program::Program;
use tspp_repository::{Environment, RuntimeOptions, WorldOptions};
use tspp_runtime::binding::BindingTable;
use tspp_runtime::diagnostic::RuntimeResult;
use tspp_runtime::machine::Engine;
use tspp_runtime::world::World;
use tspp_vm::MachineLimits;
use tspp_workspace::{BuildInput, BuildOutputs, CommandRevision, Output};

use crate::common::{
    CommandOptionsBuilder, CommandResult, CommandSummary, DiagnosticFormat, FormatOptions,
    InputArgs, ProgramArgs, ReportArgs, TargetArgs, command_data_json, command_error,
    finish_diagnostic_command, report_error, run_workspace_command_or_report,
    target_overrides_from_args,
};
use crate::console;
use crate::diagnostic::ConsoleError;

#[derive(Args, Debug, Clone)]
pub struct RunArgs {
    #[command(flatten)]
    pub input: InputArgs,

    #[command(flatten)]
    pub target: TargetArgs,

    #[command(flatten)]
    pub program: ProgramArgs,

    #[command(flatten)]
    pub report: ReportArgs,
}

/// What one run reports beside the build.
#[derive(Debug, Clone, Serialize)]
struct RunPayload {
    /// The module initializers the program ran.
    initializers: usize,
}

pub async fn run(args: &RunArgs) -> i32 {
    let started_at = Instant::now();
    let inputs = if args.input.has_input() {
        let sources = match args.input.to_sources() {
            Ok(sources) => sources,
            Err(error) => return report_error("run", &args.report, &error.to_string()),
        };
        match args.input.command_inputs(&sources) {
            Ok(inputs) => inputs,
            Err(error) => return report_error("run", &args.report, &error.to_string()),
        }
    } else {
        Vec::new()
    };

    // build the program target the run executes
    let target = args.target.target_name().map(str::to_string);
    let common = match CommandOptionsBuilder::new(&args.program) {
        Ok(common) => common,
        Err(error) => return report_error("run", &args.report, &error.to_string()),
    }
    .inputs(inputs)
    .config_inputs(!args.input.has_input())
    .target(target.clone())
    .target_overrides(target_overrides_from_args(&args.target))
    .build();
    let request = BuildInput {
        product: None,
        outputs: BuildOutputs {
            products: false,
            bundles: false,
            programs: true,
            assets: false,
        },
        ..(CommandRevision::Current, common).into()
    };

    // build, then run the linked program's initializers in a fresh world
    let result = match run_workspace_command_or_report(
        "run",
        &args.report,
        &args.program,
        async |workspace, progress| {
            let built = workspace
                .build(request, progress)
                .await
                .map_err(command_error)?;
            if !built.success {
                return CommandResult::from_output(built);
            }
            let Some(reference) = built.data.programs.first().cloned() else {
                return Err(ConsoleError::message(
                    "the target builds no program to run".to_string(),
                ));
            };
            let payload = workspace
                .artifact(reference)
                .map_err(|error| ConsoleError::message(error.to_string()))?;
            let ArtifactPayload::Program(program) = payload else {
                return Err(ConsoleError::message(
                    "the built artifact is not a program".to_string(),
                ));
            };
            console::status("Running", target.as_deref().unwrap_or("program"));
            let initializers = run_program(program, target.as_deref())
                .map_err(|error| ConsoleError::message(format!("run failed: {error}")))?;
            let output = Output {
                revision: built.revision,
                success: built.success,
                exit_code: built.exit_code,
                diagnostics: built.diagnostics,
                files: built.files,
                messages: built.messages,
                output: built.output,
                outputs: built.outputs,
                trace: built.trace,
                data: RunPayload { initializers },
                module_count: built.module_count,
                profile_count: built.profile_count,
                target_count: built.target_count,
            };

            CommandResult::from_output(output)
        },
    )
    .await
    {
        Ok(result) => result,
        Err(code) => return code,
    };
    let data = match command_data_json("run", &args.report, Some(&result.response.data)) {
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

    // name the last phase the command reached
    let verb = match result.response.success {
        true => "Ran",
        false => "Built",
    };
    let summary = CommandSummary {
        verb,
        modules: result.response.module_count,
        targets: result.response.target_count,
        duration: started_at.elapsed(),
    };

    finish_diagnostic_command(
        "run",
        &args.report,
        &result,
        &json_format_options,
        &text_format_options,
        None,
        Some(summary),
        data,
    )
}

/// Run one linked program's module initializers in a fresh world, returning how many ran.
fn run_program(program: Arc<Program>, target: Option<&str>) -> RuntimeResult<usize> {
    let environment = Arc::new(Environment::default());
    let mut world = World::new(&WorldOptions::default(), environment.clone())?;
    let engine = Engine::new(program.clone(), MachineLimits::default());
    let conditions = ConditionSet {
        modes: Default::default(),
        roles: Default::default(),
        features: Default::default(),
        tags: Default::default(),
        target: target.map(str::to_string),
        product: None,
        role: None,
        labels: Default::default(),
        platform: Platform::Unknown,
        host: Host::Native,
        runtime: Runtime::Tspp,
    };
    let bindings = Arc::new(BindingTable::new().with_fiber_bindings());
    let runtime_id = world.spawn_runtime(
        environment,
        &RuntimeOptions::default(),
        conditions,
        bindings,
        engine,
    )?;
    world.bootstrap_host()?;
    world.run_initializers(runtime_id)?;

    Ok(program.initializers().len())
}
