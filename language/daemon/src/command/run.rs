use std::sync::Arc;

use destack_artifact::ArtifactKey;
use destack_runtime::runtime::World;
use destack_runtime::runtime::engine::{EngineId, Entry, Value};
use destack_source::{ModuleId, ProfileId, TargetId};
use destack_vm::{Machine, MachineOptions};
use destack_workspace::{Environment, Profile, Repository, Revision};
use serde::{Deserialize, Serialize};

use super::CommandResult;
use super::common::CommandInput;
use super::context::{CommandContext, SelectedTarget};
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
    ) -> CommandResult<CommandOutcome> {
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
        let target = self.resolve_target_for_module(revision, entry_module, target_overrides)?;

        // collect run roots
        let artifact_keys = run_roots_for_target(
            &self.repository,
            revision,
            entry_module,
            &target.id,
            self.should_optimize(&target.target),
        )?;

        // provide the requested roots
        self.session
            .provide(revision, &artifact_keys)
            .map_err(|error| error.to_string())?;
        let diagnostics = self
            .repository
            .diagnostics(revision, None)
            .map_err(|error| error.to_string())?;
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
    target: &SelectedTarget,
    entry_name: &str,
    args: &[String],
    run_mode: CommandRunMode,
    output: &mut CommandOutputBuffer,
) -> CommandResult<RunResult> {
    // profile facts
    let target_id = target.id;
    let profile = target_profile(repository, revision, entry_module, target_id)?;
    let mut runtime_options = target.target.runtime_options.clone();
    runtime_options.conditions = profile.conditions().clone();

    // vm machine
    let machine = create_machine(
        repository,
        revision,
        entry_module,
        &target_id,
        MachineOptions::default(),
    )?;

    // runtime launch
    let entry_source = inputs
        .first()
        .ok_or_else(|| "run requires an entry module".to_string())?;
    let environment = environment_for_source(entry_source, args);
    let mut world =
        World::new(&runtime_options, environment.clone()).map_err(|error| format!("{error}"))?;
    let runtime_id = world
        .spawn_runtime(environment, &runtime_options, machine)
        .map_err(|error| format!("{error}"))?;

    let entry = Entry::new(entry_name);
    let result = world
        .run_entrypoint(runtime_id, &entry, &[])
        .map_err(|error| format!("{error}"))?;
    let exit_code = exit_status_from_value(&result);

    if matches!(run_mode, CommandRunMode::Program) && !is_exit_code_value(&result) {
        output.push_stderr(b"non-integer return value, defaulting to exit code 0\n".to_vec());
    }

    if matches!(run_mode, CommandRunMode::Eval { print: true }) {
        let formatted = format_value_for_eval(&result);
        output.push_stdout(format!("{formatted}\n").into_bytes());
    }

    if exit_code != 0 {
        output.push_stderr(format!("process exited with code {exit_code}\n").into_bytes());
    }

    Ok(RunResult {
        exit_code,
        payload: value_payload(&result),
    })
}

/// Build the launch environment for the entry source.
fn environment_for_source(source: &CommandInput, args: &[String]) -> Environment {
    let mut arguments = Vec::with_capacity(args.len().saturating_add(1));
    arguments.push(command_input_display_name(source));
    arguments.extend(args.iter().cloned());
    let mut environment = Environment::capture_process();
    environment.args = arguments;

    environment
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
        Value::Float16 { bits } => format!("0x{bits:04x}"),
        Value::Bfloat16 { bits } => format!("0x{bits:04x}"),
        Value::Float32 { bits } => f32::from_bits(*bits).to_string(),
        Value::Float64 { bits } => f64::from_bits(*bits).to_string(),
        Value::Char(value) => value.to_string(),
        Value::HeapReference(_) | Value::SharedHeapReference(_) | Value::Address(_) => {
            format!("{value:?}")
        }
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
        Value::Int { value, .. } => {
            let min = i128::from(i32::MIN);
            let max = i128::from(i32::MAX);

            (*value).clamp(min, max) as i32
        }
        Value::UInt { value, .. } => (*value).min(i32::MAX as u128) as i32,
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
        Value::Int { value, .. } => int_payload(*value),
        Value::UInt { value, .. } => uint_payload(*value),
        Value::Float16 { bits } => serde_json::Value::String(format!("0x{bits:04x}")),
        Value::Bfloat16 { bits } => serde_json::Value::String(format!("0x{bits:04x}")),
        Value::Float32 { bits } => float_payload(f32::from_bits(*bits).into()),
        Value::Float64 { bits } => float_payload(f64::from_bits(*bits)),
        Value::Char(value) => serde_json::Value::String(value.to_string()),
        Value::HeapReference(_) | Value::SharedHeapReference(_) | Value::Address(_) => {
            serde_json::Value::String(format!("{value:?}"))
        }
    }
}

/// Convert one signed integer to json.
fn int_payload(value: i128) -> serde_json::Value {
    if let Ok(value) = i64::try_from(value) {
        serde_json::Value::Number(value.into())
    } else {
        serde_json::Value::String(value.to_string())
    }
}

/// Convert one unsigned integer to json.
fn uint_payload(value: u128) -> serde_json::Value {
    if let Ok(value) = u64::try_from(value) {
        serde_json::Value::Number(value.into())
    } else {
        serde_json::Value::String(value.to_string())
    }
}

/// Create a VM machine from the module MIR.
fn create_machine(
    repository: &Repository,
    revision: Revision,
    module_id: ModuleId,
    target_id: &TargetId,
    options: MachineOptions,
) -> CommandResult<Machine> {
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
    let tree = if let Some(version) = optimized_version
        && let Some(mir) = artifact_store.mir_optimized(&version)
        && let Some(tree) = mir.latest_patch_tree()
    {
        tree.clone()
    } else if let Some(version) = lowered_version
        && let Some(mir) = artifact_store.mir_lowered(&version)
    {
        mir.tree.clone()
    } else {
        return Err(format!("missing MIR for target {target_id:?} (run requires lowering)").into());
    };
    let strings = repository.string_pool().as_ref().clone();

    let machine_id = EngineId::new(1);

    Ok(
        Machine::build_with_options(machine_id, tree, strings, options)
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
    let profile = target_profile(repository, revision, module_id, target_id)?;

    Ok(profile.id())
}

/// Return the profile selected for one module target.
fn target_profile(
    repository: &Repository,
    revision: Revision,
    module_id: ModuleId,
    target_id: TargetId,
) -> Result<Arc<Profile>, String> {
    let profile = repository
        .profile_for_module_target(revision, module_id, target_id)
        .map_err(|error| format!("failed to resolve target profile: {error}"))?;

    Ok(profile)
}

/// Convert one finite float to json.
fn float_payload(value: f64) -> serde_json::Value {
    serde_json::Number::from_f64(value)
        .map(serde_json::Value::Number)
        .unwrap_or(serde_json::Value::Null)
}
