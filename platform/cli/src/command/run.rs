use clap::Args;
use destack_compiler::OptimizeTask;
use destack_runtime::platform::{
    BindingPolicy, BindingRegistry, DeterminismPolicy, HostContext, ReplayMode,
};
use destack_vm::{ExecutionMode, Isolate, IsolateOptions, TrustPolicy as VmTrustPolicy};
use destack_workspace::{
    DebugMode, DeterminismPolicy as TargetDeterminismPolicy, OptimizeLevel,
    ReplayMode as TargetReplayMode, Target, TargetId, TrustPolicy,
};

use crate::common::{
    CompilerContext, DiagnosticArgs, InputArgs, InputSource, ProgramArgs, TargetArgs,
    print_diagnostics,
};
use crate::console;

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

    /// Entry function name (default: main).
    #[arg(long, default_value = "main")]
    pub entry: String,

    /// Arguments passed to the program.
    #[arg(last = true, value_name = "ARGS")]
    pub args: Vec<String>,
}

/// Compile and run a source file.
pub fn run(args: &RunArgs) -> i32 {
    // resolve the target name
    let target_name = args
        .target
        .target_name()
        .map(String::from)
        .unwrap_or_else(|| "native".to_string());

    // set up the compiler context
    let context = CompilerContext::for_run(&args.program, &args.diagnostics, target_name.clone());

    // load sources and enforce a single entry module
    let sources = match context.load_sources_for(&args.input, "run") {
        Ok(sources) => sources,
        Err(code) => return code,
    };
    if sources.is_empty() {
        console::error("error: no input provided");
        return 1;
    }
    if sources.len() > 1 {
        console::error("error: run expects a single entry module");
        return 1;
    }

    // resolve the entry module and source display name
    let entry_source = sources[0].clone();
    let entry_module = match context.resolve_source(&entry_source) {
        Ok(module_id) => module_id,
        Err(message) => {
            console::error(&format!("error: {message}"));
            return 1;
        }
    };

    // ensure the target exists for lowering
    let target_id = match ensure_target_for_module(
        &context.program,
        entry_module,
        &target_name,
        &args.target,
    ) {
        Ok(target_id) => target_id,
        Err(message) => {
            console::error(&format!("error: {message}"));
            return 1;
        }
    };

    // enqueue lowering and optional optimization
    context.enqueue_module(entry_module);
    let target = match target_for_module(&context.program, entry_module, &target_id) {
        Ok(target) => target,
        Err(message) => {
            console::error(&format!("error: {message}"));
            return 1;
        }
    };
    if should_optimize(&target) {
        context.compiler.enqueue(OptimizeTask::OptimizeModule {
            module: entry_module,
            target: target_id.clone(),
        });
    }

    // run the compiler and surface diagnostics
    context.run_compile();
    let result = context.into_result();
    let diagnostics = result
        .program
        .diagnostics
        .collect()
        .map(&result.diagnostic_options);
    print_diagnostics(&result.program, &diagnostics);
    if diagnostics.get_status_code() != 0 {
        return diagnostics.get_status_code();
    }

    // build the VM isolate for the lowered MIR
    let mut isolate = match mir_isolate(
        &result.program,
        entry_module,
        &target_id,
        isolate_options_for_target(&target),
    ) {
        Ok(isolate) => isolate,
        Err(message) => {
            console::error(&format!("error: {message}"));
            return 1;
        }
    };

    // install default platform bindings
    let process_args = process_args_for_source(&entry_source, &args.args);
    let host = HostContext::new(process_args);
    let mut bindings = BindingRegistry::new();
    bindings.set_policy(binding_policy_for_target(&target));
    bindings.install_defaults(&mut isolate, &host);

    // execute the entry function
    match isolate.run_function_by_name(&args.entry, &[]) {
        Ok(output) => exit_status_from_value(output.value),
        Err(error) => {
            console::error(&format!("runtime error: {error}"));
            1
        }
    }
}

/// Ensure a target exists for the entry module.
fn ensure_target_for_module(
    program: &destack_workspace::Program,
    module_id: destack_source::ModuleId,
    target_name: &str,
    target_args: &TargetArgs,
) -> Result<TargetId, String> {
    // locate the entry module package
    let module = program.modules.get(module_id);
    let package_id = module.read().package_id;
    let target_id = TargetId::new(package_id, target_name);

    // look for an existing target entry
    let package = program.packages.get(package_id);
    let mut package = package.write();
    let existing_target = package.targets.get(&target_id).cloned();

    // reject overrides for named targets
    if existing_target.is_some() && target_args.has_adhoc_options() {
        return Err(String::from(
            "ad-hoc target options are not supported for named targets",
        ));
    }

    // insert implicit target when missing
    if existing_target.is_none() {
        let mut target = Target::implicit_for_name(target_name)
            .ok_or_else(|| format!("unknown target '{target_name}'"))?;
        target_args.apply_to_target(&mut target);
        package.targets.insert(target_id.clone(), target);
    }

    Ok(target_id)
}

/// Fetch the configured target for a module.
fn target_for_module(
    program: &destack_workspace::Program,
    module_id: destack_source::ModuleId,
    target_id: &TargetId,
) -> Result<Target, String> {
    // resolve the package for this module
    let module = program.modules.get(module_id);
    let package_id = module.read().package_id;

    // read the target config from the package
    let package = program.packages.get(package_id);
    let package = package.read();
    package
        .targets
        .get(target_id)
        .cloned()
        .ok_or_else(|| format!("target '{target_id}' not found in package config"))
}

/// Decide whether optimization should run for a target.
fn should_optimize(target: &Target) -> bool {
    // check explicit optimize flags
    if target.optimize {
        return true;
    }

    // check nonzero optimize level
    !matches!(target.optimize_level, OptimizeLevel::O0)
}

/// Create isolate options from target configuration.
fn isolate_options_for_target(target: &Target) -> IsolateOptions {
    // initialize default options
    let mut options = IsolateOptions::default();

    // apply trust policy defaults
    let trust_policy = match target.trust_policy {
        TrustPolicy::Untrusted => VmTrustPolicy::Untrusted,
        TrustPolicy::Trusted => VmTrustPolicy::Trusted,
        TrustPolicy::Internal => VmTrustPolicy::Internal,
    };
    options.apply_trust_policy(trust_policy);

    // enable debug execution when requested
    let execution_mode = match target.debug_mode {
        DebugMode::Vm => ExecutionMode::Debug,
        DebugMode::Auto if target.debug => ExecutionMode::Debug,
        _ => ExecutionMode::Runtime,
    };
    options.execution.mode = execution_mode;

    options
}

/// Create binding policy from target configuration.
fn binding_policy_for_target(target: &Target) -> BindingPolicy {
    let determinism = match target.determinism {
        TargetDeterminismPolicy::BestEffort => DeterminismPolicy::BestEffort,
        TargetDeterminismPolicy::Deterministic => DeterminismPolicy::Deterministic,
    };

    let replay = match target.replay {
        TargetReplayMode::Off => ReplayMode::Off,
        TargetReplayMode::Record => ReplayMode::Record,
        TargetReplayMode::Replay => ReplayMode::Replay,
    };

    BindingPolicy {
        determinism,
        replay,
    }
}

/// Build a VM isolate from the module MIR.
fn mir_isolate(
    program: &destack_workspace::Program,
    module_id: destack_source::ModuleId,
    target_id: &TargetId,
    options: IsolateOptions,
) -> Result<Isolate, String> {
    // pull the lowered MIR from the module
    let module = program.modules.get(module_id);
    let module = module.read();
    let mir = module.mir(target_id);
    let tree = mir.tree.read().clone();
    let strings = mir.strings.clone().into_immutable();

    Ok(Isolate::with_options(tree, strings, options))
}

/// Build process arguments for the entry source.
fn process_args_for_source(source: &InputSource, args: &[String]) -> Vec<String> {
    // prefix args with the entry module display name
    let mut process_args = Vec::with_capacity(args.len().saturating_add(1));
    process_args.push(entry_display_name(source));
    process_args.extend(args.iter().cloned());

    process_args
}

/// Get a display name for the entry source.
fn entry_display_name(source: &InputSource) -> String {
    // pick the display name for the source kind
    match source {
        InputSource::File(path) => path.to_string_lossy().into_owned(),
        InputSource::Inline { name, .. } => name.clone(),
        InputSource::Stdin { name } => name.clone(),
    }
}

/// Convert a VM return value into an exit status.
fn exit_status_from_value(value: destack_vm::Value) -> i32 {
    if value.is_void() {
        return 0;
    }

    if let Some(result) = value.as_bool() {
        return if result { 0 } else { 1 };
    }

    if let Some(result) = value.as_int() {
        return result.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32;
    }

    console::warn("non-integer return value, defaulting to exit code 0");
    0
}
