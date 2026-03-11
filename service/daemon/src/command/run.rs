use std::sync::Arc;

use destack_compiler::{BuildKey, Compiler};
use destack_runtime::runtime::World;
use destack_runtime::runtime::engine::Entry;
use destack_source::ModuleId;
use destack_vm::{ExecutionMode, Isolate, IsolateOptions, TrustPolicy as VmTrustPolicy, Value};
use destack_workspace::{
    ArtifactKey, DebugMode, DsConfigRuntimeOptionsJson, Program, Target, TargetId, TrustPolicy,
};
use serde::{Deserialize, Serialize};

use super::common::CommandInput;
use super::context::CommandContext;
use super::dispatch::{CommandOutcome, CommandOutputBuffer};

/// Run mode for the run command.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum CommandRunMode {
    /// Execute an entry module.
    #[default]
    Program,
    /// Evaluate inline input and optionally print the result.
    Eval { print: bool },
}

/// Options for the run command.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct CommandRunOptions {
    /// Optional run entry function name.
    pub entry: Option<String>,
    /// Optional command arguments.
    pub args: Vec<String>,
    /// Optional run mode override.
    pub run_mode: CommandRunMode,
}

/// Payload for run command output.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CommandRunPayload {
    /// Run completed successfully with a return value.
    Value {
        /// Returned value from the entry function.
        value: serde_json::Value,
    },
    /// Run failed with a runtime error.
    RuntimeError {
        /// Runtime error string.
        message: String,
    },
}

impl CommandContext<'_> {
    /// Execute a run command.
    pub(super) fn execute_run_command(
        &mut self,
        options: &CommandRunOptions,
    ) -> super::CommandResult<CommandOutcome> {
        // resolve inputs for the command
        let inputs = self.resolve_command_inputs()?;
        let modules = self.resolve_modules(&inputs)?;
        let entry_module = modules
            .first()
            .copied()
            .ok_or_else(|| "run requires an entry module".to_string())?;

        // resolve the target configuration
        let target_overrides = self.common.target_overrides.as_ref();
        let target_id = self.resolve_target_for_module(entry_module, target_overrides)?;

        // enqueue lowering tasks
        self.reset_diagnostics();
        enqueue_lower_tasks(&self.program, &self.compiler, entry_module, &target_id);

        // optionally enqueue optimize tasks
        if let Some(target) = self.target_for_id(&target_id)
            && self.should_optimize(&target)
        {
            let profile = self
                .program
                .profile_id_for_target_or_default(entry_module, &target_id);
            self.compiler
                .enqueue(BuildKey::Artifact(ArtifactKey::MirOptimized {
                    module: entry_module,
                    profile,
                    target: target_id.clone(),
                }));
        }

        // compile and collect diagnostics
        self.compiler.compile();
        let raw_diagnostics = self.collect_raw_diagnostics();
        self.commit_diagnostics_for_modules(&modules, &raw_diagnostics)?;
        let diagnostics = raw_diagnostics.map(&self.diagnostic_options);
        let exit_code = diagnostics.get_status_code();
        if exit_code != 0 {
            return Ok(CommandOutcome::new(
                diagnostics,
                exit_code,
                modules.len(),
                self.program.profiles.len(),
                1,
                Some(
                    self.compiler
                        .stats
                        .snapshot_with_program(self.program.modules.len(), Some(&self.program)),
                ),
            ));
        }

        // execute the entry module
        let entry_name = options.entry.clone().unwrap_or_else(|| "main".to_string());
        let run_result = match run_entry_module(
            &self.program,
            &inputs,
            entry_module,
            &target_id,
            &entry_name,
            &options.args,
            options.run_mode,
            self.common.runtime_overrides.as_ref(),
            self.output,
        ) {
            Ok(result) => result,
            Err(error) => {
                let stats = self
                    .compiler
                    .stats
                    .snapshot_with_program(self.program.modules.len(), Some(&self.program));
                let payload = CommandRunPayload::RuntimeError {
                    message: error.to_string(),
                };
                let data = serde_json::to_value(payload)
                    .map_err(|error| format!("invalid run payload: {error}"))?;
                return Ok(CommandOutcome::new(
                    diagnostics,
                    1,
                    modules.len(),
                    self.program.profiles.len(),
                    1,
                    Some(stats),
                )
                .with_data(data));
            }
        };

        let stats = self
            .compiler
            .stats
            .snapshot_with_program(self.program.modules.len(), Some(&self.program));

        let payload = CommandRunPayload::Value {
            value: run_result.payload,
        };
        let data = serde_json::to_value(payload)
            .map_err(|error| format!("invalid run payload: {error}"))?;

        Ok(CommandOutcome::new(
            diagnostics,
            run_result.exit_code,
            modules.len(),
            self.program.profiles.len(),
            1,
            Some(stats),
        )
        .with_data(data))
    }
}

/// Result of executing the entry module.
struct RunResult {
    exit_code: i32,
    payload: serde_json::Value,
}

/// Enqueue lowering tasks for the entry module.
fn enqueue_lower_tasks(
    program: &Arc<Program>,
    compiler: &Arc<Compiler>,
    module_id: ModuleId,
    target_id: &TargetId,
) {
    let profile = program.profile_id_for_target_or_default(module_id, target_id);
    compiler.enqueue(BuildKey::Artifact(ArtifactKey::Mir {
        module: module_id,
        profile,
        target: target_id.clone(),
    }));
}

/// Execute the entry module in the VM.
#[allow(clippy::too_many_arguments)]
fn run_entry_module(
    program: &Arc<Program>,
    inputs: &[CommandInput],
    entry_module: ModuleId,
    target_id: &TargetId,
    entry_name: &str,
    args: &[String],
    run_mode: CommandRunMode,
    runtime_overrides: Option<&DsConfigRuntimeOptionsJson>,
    output: &mut CommandOutputBuffer,
) -> super::CommandResult<RunResult> {
    let mut target = target_for_id(program, target_id)
        .ok_or_else(|| format!("target '{target_id:?}' not found"))?;
    if let Some(runtime_overrides) = runtime_overrides {
        apply_runtime_overrides(&mut target, runtime_overrides);
    }

    let isolate = create_isolate(
        program,
        entry_module,
        target_id,
        isolate_options_for_target(&target),
    )?;

    let entry_source = inputs
        .first()
        .ok_or_else(|| "run requires an entry module".to_string())?;
    let process_args = process_args_for_source(entry_source, args);
    let world = World::from_options(&target.runtime_options).map_err(|error| format!("{error}"))?;
    let runtime_id = world
        .spawn_runtime(process_args, &target.runtime_options, isolate)
        .map_err(|error| format!("{error}"))?;

    let entry = Entry::vm(entry_name);
    let result = world
        .run_entrypoint(runtime_id, &entry, &[])
        .map_err(|error| format!("{error}"))?;
    let exit_code = exit_status_from_value(result.value);

    if matches!(run_mode, CommandRunMode::Program) && !is_exit_code_value(&result.value) {
        output.push_stderr(b"non-integer return value, defaulting to exit code 0\n".to_vec());
    }

    if matches!(run_mode, CommandRunMode::Eval { print: true }) {
        let formatted = format_value_for_eval(&result.value);
        output.push_stdout(format!("{formatted}\n").into_bytes());
    }

    if exit_code != 0 {
        output.push_stderr(format!("process exited with code {exit_code}\n").into_bytes());
    }

    Ok(RunResult {
        exit_code,
        payload: value_payload(&result.value),
    })
}

/// Build process arguments for the entry source.
fn process_args_for_source(source: &CommandInput, args: &[String]) -> Vec<String> {
    let mut process_args = Vec::with_capacity(args.len().saturating_add(1));
    process_args.push(command_input_display_name(source));
    process_args.extend(args.iter().cloned());
    process_args
}

/// Get a display name for the entry source.
fn command_input_display_name(source: &CommandInput) -> String {
    match source {
        CommandInput::File { path } => path.to_string_lossy().into_owned(),
        CommandInput::Inline { name, .. } => name.clone(),
        CommandInput::Stdin { name, .. } => name.clone(),
    }
}

/// Format a VM value for eval output.
fn format_value_for_eval(value: &Value) -> String {
    if value.is_void() {
        return "void".to_string();
    }
    if let Some(result) = value.as_bool() {
        return result.to_string();
    }
    if let Some(result) = value.as_int_with_width() {
        return result.0.to_string();
    }
    if let Some(result) = value.as_uint_with_width() {
        return result.0.to_string();
    }
    if let Some(result) = value.as_float64() {
        return result.to_string();
    }
    if let Some(result) = value.as_float32() {
        return result.to_string();
    }
    if let Some(result) = value.as_char() {
        return result.to_string();
    }
    format!("{value:?}")
}

/// Convert a VM return value into an exit status.
fn exit_status_from_value(value: Value) -> i32 {
    if value.is_void() {
        return 0;
    }
    if let Some(result) = value.as_bool() {
        return if result { 0 } else { 1 };
    }
    if let Some(result) = value.as_int() {
        return result.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32;
    }
    0
}

/// Check whether a return value is a valid exit code.
fn is_exit_code_value(value: &Value) -> bool {
    value.is_void() || value.as_bool().is_some() || value.as_int().is_some()
}

/// Convert a runtime value into payload data.
fn value_payload(value: &Value) -> serde_json::Value {
    if value.is_void() {
        return serde_json::Value::Null;
    }
    if let Some(result) = value.as_bool() {
        return serde_json::Value::Bool(result);
    }
    if let Some(result) = value.as_int_with_width() {
        return serde_json::Value::Number(result.0.into());
    }
    if let Some(result) = value.as_uint_with_width() {
        return serde_json::Value::Number(result.0.into());
    }
    if let Some(result) = value.as_float64() {
        if let Some(number) = serde_json::Number::from_f64(result) {
            return serde_json::Value::Number(number);
        }
        return serde_json::Value::Null;
    }
    if let Some(result) = value.as_float32() {
        if let Some(number) = serde_json::Number::from_f64(result.into()) {
            return serde_json::Value::Number(number);
        }
        return serde_json::Value::Null;
    }
    if let Some(result) = value.as_char() {
        return serde_json::Value::String(result.to_string());
    }
    serde_json::Value::String(format!("{value:?}"))
}

/// Resolve target by id.
fn target_for_id(program: &Arc<Program>, target_id: &TargetId) -> Option<Target> {
    let package = program.packages.get(target_id.package_id);
    let package = package.read();
    package.targets.get(target_id).cloned()
}

/// Create a VM isolate from the module MIR.
fn create_isolate(
    program: &Program,
    module_id: ModuleId,
    target_id: &TargetId,
    options: IsolateOptions,
) -> super::CommandResult<Isolate> {
    let module = program.modules.get(module_id);
    let module = module.read();
    let mir = module
        .mir_maybe(target_id)
        .ok_or_else(|| format!("missing MIR for target {target_id:?} (run requires lowering)"))?;
    let tree = mir.tree.read().clone();
    let strings = mir.strings.clone().into_immutable();

    Ok(Isolate::build_with_options(tree, strings, options).map_err(|error| error.to_string())?)
}

/// Create isolate options from target configuration.
fn isolate_options_for_target(target: &Target) -> IsolateOptions {
    let mut options = IsolateOptions::default();

    let trust_policy = match target.trust_policy {
        TrustPolicy::Untrusted => VmTrustPolicy::Untrusted,
        TrustPolicy::Trusted => VmTrustPolicy::Trusted,
        TrustPolicy::Internal => VmTrustPolicy::Internal,
    };
    options.apply_trust_policy(trust_policy);

    let execution_mode = match target.debug_mode {
        DebugMode::Vm => ExecutionMode::Debug,
        DebugMode::Auto if target.debug => ExecutionMode::Debug,
        _ => ExecutionMode::Runtime,
    };
    options.execution.mode = execution_mode;

    options
}

/// Apply runtime overrides to a target.
fn apply_runtime_overrides(target: &mut Target, overrides: &DsConfigRuntimeOptionsJson) {
    let mut runtime_options = target.runtime_options.clone();
    overrides.apply_to(&mut runtime_options);
    target.runtime_options = runtime_options;
}
