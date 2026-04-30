use std::sync::Arc;

use destack_artifact::ArtifactKey;
use destack_runtime::runtime::World;
use destack_runtime::runtime::engine::Entry;
use destack_source::{ModuleId, ProfileId, TargetId};
use destack_vm::{
    ExecutionMode, Isolate, IsolateId, IsolateOptions, TrustPolicy as VmTrustPolicy, Value,
};
use destack_workspace::{DebugMode, Repository, Revision, RuntimeOptionsJson, Target, TrustPolicy};
use serde::{Deserialize, Serialize};

use super::common::CommandInput;
use super::context::{CommandContext, ResolvedTarget};
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
        let revision = self.revision()?;
        let entry_module = modules
            .first()
            .copied()
            .ok_or_else(|| "run requires an entry module".to_string())?;

        // resolve the target configuration
        let target_overrides = self.common.target_overrides.as_ref();
        let target = self.resolve_target_for_module(entry_module, target_overrides)?;

        // collect run roots
        let artifact_keys = run_roots_for_target(
            &self.repository,
            revision,
            entry_module,
            &target.id,
            self.should_optimize(&target.target),
        )?;

        // provide the requested roots
        let revision = self.revision()?;
        self.session
            .provide(revision, &artifact_keys)
            .map_err(|error| error.to_string())?;
        let raw_diagnostics = self
            .repository
            .diagnostics(revision)
            .map_err(|error| error.to_string())?;
        self.commit_diagnostics_for_modules(&modules, &raw_diagnostics)?;
        let diagnostics = raw_diagnostics.map(&self.diagnostic_options);
        let exit_code = diagnostics.get_status_code();
        let profile_count = self
            .target_profile_id(revision, entry_module, target.id)
            .map(|_| 1)?;
        if exit_code != 0 {
            return Ok(CommandOutcome::new(
                diagnostics,
                exit_code,
                modules.len(),
                profile_count,
                1,
            ));
        }

        // execute the entry module
        let entry_name = options.entry.clone().unwrap_or_else(|| "main".to_string());
        let run_result = match run_entry_module(
            &self.repository,
            revision,
            &inputs,
            entry_module,
            &target,
            &entry_name,
            &options.args,
            options.run_mode,
            self.common.runtime_overrides.as_ref(),
            self.output,
        ) {
            Ok(result) => result,
            Err(error) => {
                let payload = CommandRunPayload::RuntimeError {
                    message: error.to_string(),
                };
                let data = serde_json::to_value(payload)
                    .map_err(|error| format!("invalid run payload: {error}"))?;
                return Ok(
                    CommandOutcome::new(diagnostics, 1, modules.len(), profile_count, 1)
                        .with_data(data),
                );
            }
        };

        let payload = CommandRunPayload::Value {
            value: run_result.payload,
        };
        let data = serde_json::to_value(payload)
            .map_err(|error| format!("invalid run payload: {error}"))?;

        Ok(CommandOutcome::new(
            diagnostics,
            run_result.exit_code,
            modules.len(),
            profile_count,
            1,
        )
        .with_data(data))
    }
}

/// Result of executing the entry module.
struct RunResult {
    exit_code: i32,
    payload: serde_json::Value,
}

/// Build the requested run roots for one target.
fn run_roots_for_target(
    repository: &Arc<Repository>,
    revision: Revision,
    module_id: ModuleId,
    target_id: &TargetId,
    is_optimized: bool,
) -> Result<Vec<ArtifactKey>, String> {
    let profile = target_profile_id(repository, revision, module_id, *target_id)?;
    let mut artifact_keys = vec![ArtifactKey::mir_lowered(module_id, profile, *target_id)];

    if is_optimized {
        artifact_keys.push(ArtifactKey::mir_optimized(module_id, profile, *target_id));
    }

    Ok(artifact_keys)
}

/// Execute the entry module in the VM.
#[allow(clippy::too_many_arguments)]
fn run_entry_module(
    repository: &Arc<Repository>,
    revision: Revision,
    inputs: &[CommandInput],
    entry_module: ModuleId,
    target: &ResolvedTarget,
    entry_name: &str,
    args: &[String],
    run_mode: CommandRunMode,
    runtime_overrides: Option<&RuntimeOptionsJson>,
    output: &mut CommandOutputBuffer,
) -> super::CommandResult<RunResult> {
    let target_id = target.id;
    let mut target = target.target.clone();
    if let Some(runtime_overrides) = runtime_overrides {
        apply_runtime_overrides(&mut target, runtime_overrides);
    }

    let isolate = create_isolate(
        repository,
        revision,
        entry_module,
        &target_id,
        isolate_options_for_target(&target),
    )?;

    let entry_source = inputs
        .first()
        .ok_or_else(|| "run requires an entry module".to_string())?;
    let process_args = process_args_for_source(entry_source, args);
    let mut world =
        World::from_options(&target.runtime_options).map_err(|error| format!("{error}"))?;
    let runtime_id = world
        .spawn_runtime(process_args, &target.runtime_options, isolate)
        .map_err(|error| format!("{error}"))?;

    let entry = Entry::new(entry_name);
    let result = world
        .run_entrypoint(runtime_id, &entry, &[])
        .map_err(|error| format!("{error}"))?;
    let exit_code = exit_status_from_value(&result.value);

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
    match value {
        Value::Void => "void".to_string(),
        Value::Bool(value) => value.to_string(),
        Value::Int { value, .. } => value.to_string(),
        Value::UInt { value, .. } => value.to_string(),
        Value::Float32 { bits } => f32::from_bits(*bits).to_string(),
        Value::Float64 { bits } => f64::from_bits(*bits).to_string(),
        Value::Char(value) => value.to_string(),
        Value::HeapReference(_)
        | Value::SharedHeapReference(_)
        | Value::RawPointer(_)
        | Value::SharedRawPointer(_) => format!("{value:?}"),
    }
}

/// Convert a VM return value into an exit status.
fn exit_status_from_value(value: &Value) -> i32 {
    match value {
        Value::Void => 0,
        Value::Bool(value) => {
            if *value {
                0
            } else {
                1
            }
        }
        Value::Int { value, .. } => (*value).clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32,
        _ => 0,
    }
}

/// Check whether a return value is a valid exit code.
fn is_exit_code_value(value: &Value) -> bool {
    matches!(
        value,
        Value::Void | Value::Bool(_) | Value::Int { .. } | Value::UInt { .. }
    )
}

/// Convert a runtime value into payload data.
fn value_payload(value: &Value) -> serde_json::Value {
    match value {
        Value::Void => serde_json::Value::Null,
        Value::Bool(value) => serde_json::Value::Bool(*value),
        Value::Int { value, .. } => serde_json::Value::Number((*value).into()),
        Value::UInt { value, .. } => serde_json::Value::Number((*value).into()),
        Value::Float32 { bits } => float_payload(f32::from_bits(*bits).into()),
        Value::Float64 { bits } => float_payload(f64::from_bits(*bits)),
        Value::Char(value) => serde_json::Value::String(value.to_string()),
        Value::HeapReference(_)
        | Value::SharedHeapReference(_)
        | Value::RawPointer(_)
        | Value::SharedRawPointer(_) => serde_json::Value::String(format!("{value:?}")),
    }
}

/// Create a VM isolate from the module MIR.
fn create_isolate(
    repository: &Repository,
    revision: Revision,
    module_id: ModuleId,
    target_id: &TargetId,
    options: IsolateOptions,
) -> super::CommandResult<Isolate> {
    let profile_id = target_profile_id(repository, revision, module_id, *target_id)?;
    let optimized_key = ArtifactKey::mir_optimized(module_id, profile_id, *target_id);
    let lowered_key = ArtifactKey::mir_lowered(module_id, profile_id, *target_id);

    let optimized_version = repository
        .artifact_version(revision, &optimized_key)
        .map_err(|error| error.to_string())?;
    let lowered_version = repository
        .artifact_version(revision, &lowered_key)
        .map_err(|error| error.to_string())?;

    let artifact_store = repository.artifact_store();
    let (tree, strings) = if let Some(version) = optimized_version
        && let Some(mir) = artifact_store.mir_optimized(&version)
    {
        (mir.tree.clone(), mir.strings.clone().into_immutable())
    } else if let Some(version) = lowered_version
        && let Some(mir) = artifact_store.mir_lowered(&version)
    {
        (mir.tree.clone(), mir.strings.clone().into_immutable())
    } else {
        return Err(format!("missing MIR for target {target_id:?} (run requires lowering)").into());
    };

    let isolate_id = IsolateId::new(1);

    Ok(
        Isolate::build_with_options(isolate_id, tree, strings, options)
            .map_err(|error| error.to_string())?,
    )
}

/// Return the profile id selected for one module target.
fn target_profile_id(
    repository: &Repository,
    revision: Revision,
    module_id: ModuleId,
    target_id: TargetId,
) -> Result<ProfileId, String> {
    let profile = repository
        .module_target_profile(revision, module_id, target_id)
        .map_err(|error| format!("failed to resolve target profile: {error}"))?;
    let profile = if let Some(profile) = profile {
        profile
    } else {
        repository
            .module_profile(revision, module_id)
            .map_err(|error| format!("failed to resolve module profile: {error}"))?
    };

    Ok(profile.id())
}

/// Convert one finite float to json.
fn float_payload(value: f64) -> serde_json::Value {
    serde_json::Number::from_f64(value)
        .map(serde_json::Value::Number)
        .unwrap_or(serde_json::Value::Null)
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
fn apply_runtime_overrides(target: &mut Target, overrides: &RuntimeOptionsJson) {
    let mut runtime_options = target.runtime_options.clone();
    overrides.apply_to(&mut runtime_options);
    target.runtime_options = runtime_options;
}
