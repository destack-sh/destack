use std::path::PathBuf;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::Arc;

#[cfg(not(target_arch = "wasm32"))]
use destack_artifact::ArtifactKey;
#[cfg(not(target_arch = "wasm32"))]
use destack_mir::Tree;
#[cfg(not(target_arch = "wasm32"))]
use destack_program::Program;
#[cfg(not(target_arch = "wasm32"))]
use destack_repository::{
    ArtifactReader, Environment, Profile, ProviderError, Repository, Revision,
};
#[cfg(not(target_arch = "wasm32"))]
use destack_runtime::runtime::World;
#[cfg(not(target_arch = "wasm32"))]
use destack_runtime::runtime::machine::{Entry, Execution, Value};
use destack_serde::Reflect;
#[cfg(target_arch = "wasm32")]
use destack_source::DiagnosticCollection;
#[cfg(not(target_arch = "wasm32"))]
use destack_source::{ModuleId, ProfileId, TargetId};
#[cfg(not(target_arch = "wasm32"))]
use destack_vm::MachineOptions;
use serde::{Deserialize, Serialize};

use super::CommandResult;
use super::common::{
    CommandEnvVar, CommandInput, CommandOptions, CommandRevision, CommandTargetOverrides,
    ManifestOverride, impl_command_input_options,
};
use super::context::CommandContext;
#[cfg(not(target_arch = "wasm32"))]
use super::context::SelectedTarget;
use super::outcome::CommandOutcome;
#[cfg(not(target_arch = "wasm32"))]
use super::output::OutputBuffer;

/// Run mode for the run command.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, Default)]
pub enum RunMode {
    /// Execute an entry module.
    #[default]
    Program,
    /// Evaluate inline input and optionally print the result.
    Eval { print: bool },
}

/// Options for the run command.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect, Default)]
pub struct RunOptions {
    /// Optional run entry function name.
    pub entry: Option<String>,
    /// Optional command arguments.
    pub args: Vec<String>,
    /// Optional run mode override.
    pub run_mode: RunMode,
}

/// Request to run a workspace target.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct RunInput {
    /// Revision selected for this run.
    pub revision: CommandRevision,
    /// Input sources for the command.
    pub inputs: Vec<CommandInput>,
    /// Whether destack.json should resolve inputs when none are provided.
    pub config_inputs: bool,
    /// Optional working directory for this command.
    pub cwd: Option<PathBuf>,
    /// Optional Destack manifest path override.
    pub manifest: Option<PathBuf>,
    /// Optional target name override.
    pub target: Option<String>,
    /// Optional target overrides.
    pub target_overrides: Option<CommandTargetOverrides>,
    /// Optional profile name override.
    pub profile: Option<String>,
    /// Optional environment overrides.
    pub env: Vec<CommandEnvVar>,
    /// Optional manifest overrides.
    pub overrides: Vec<ManifestOverride>,
    /// Whether the command should watch for changes.
    pub watch: bool,
    /// Whether the command should skip writes.
    pub dry_run: bool,
    /// Optional run entry function name.
    pub entry: Option<String>,
    /// Optional command arguments.
    pub args: Vec<String>,
    /// Optional run mode override.
    pub run_mode: RunMode,
}

impl_command_input_options!(RunInput {
    entry: None,
    args: Vec::new(),
    run_mode: RunMode::default(),
});

/// Payload for run command output.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RunPayload {
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

/// Result of executing the entry module.
#[cfg(not(target_arch = "wasm32"))]
struct RunResult {
    /// Process exit code derived from the return value.
    exit_code: i32,
    /// Serialized return value.
    payload: serde_json::Value,
}

#[cfg(not(target_arch = "wasm32"))]
impl CommandContext<'_> {
    /// Execute a run command.
    pub(crate) fn execute_run_command(
        &mut self,
        input: &RunInput,
    ) -> CommandResult<CommandOutcome<Option<RunPayload>>> {
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
        let diagnostics = self.command_diagnostics(revision, &artifact_keys)?;
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
            )
            .with_data(None));
        }

        // execute the entry module
        let entry_name = input.entry.clone().unwrap_or_else(|| "main".to_string());
        let run_result = match run_entry_module(
            &self.repository,
            revision,
            &inputs,
            entry_module,
            &target,
            &entry_name,
            &input.args,
            input.run_mode,
            self.output,
        ) {
            Ok(result) => result,
            Err(error) => {
                let payload = RunPayload::RuntimeError {
                    message: error.to_string(),
                };
                return Ok(
                    CommandOutcome::new(diagnostics, 1, modules.len(), profile_count, 1)
                        .with_data(Some(payload)),
                );
            }
        };

        let payload = RunPayload::Value {
            value: run_result.payload,
        };

        Ok(CommandOutcome::new(
            diagnostics,
            run_result.exit_code,
            modules.len(),
            profile_count,
            1,
        )
        .with_data(Some(payload)))
    }
}

#[cfg(target_arch = "wasm32")]
impl CommandContext<'_> {
    /// Execute a run command.
    pub(crate) fn execute_run_command(
        &mut self,
        _input: &RunInput,
    ) -> CommandResult<CommandOutcome<Option<RunPayload>>> {
        Ok(CommandOutcome::new(DiagnosticCollection::default(), 1, 0, 0, 0).with_data(None))
    }
}

/// Build the requested run roots for one target.
#[cfg(not(target_arch = "wasm32"))]
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
#[cfg(not(target_arch = "wasm32"))]
#[allow(clippy::too_many_arguments)]
fn run_entry_module(
    repository: &Arc<Repository>,
    revision: Revision,
    inputs: &[CommandInput],
    entry_module: ModuleId,
    target: &SelectedTarget,
    entry_name: &str,
    args: &[String],
    run_mode: RunMode,
    output: &mut OutputBuffer,
) -> CommandResult<RunResult> {
    // profile facts
    let target_id = target.id;
    let profile = target_profile(repository, revision, entry_module, target_id)?;
    let mut runtime_options = target.target.execution.clone();
    runtime_options.conditions = profile.conditions().clone();

    // program
    let program = create_program(
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
    let execution = Execution::vm(MachineOptions::default());
    let runtime_id = world
        .spawn_runtime(environment, &runtime_options, program, execution)
        .map_err(|error| format!("{error}"))?;

    let entry = Entry::new(entry_name);
    let result = world
        .run_entrypoint(runtime_id, &entry, &[])
        .map_err(|error| format!("{error}"))?;
    let exit_code = exit_status_from_value(&result);

    if matches!(run_mode, RunMode::Program) && !is_exit_code_value(&result) {
        output.push_stderr(b"non-integer return value, defaulting to exit code 0\n".to_vec());
    }

    if matches!(run_mode, RunMode::Eval { print: true }) {
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
#[cfg(not(target_arch = "wasm32"))]
fn environment_for_source(source: &CommandInput, args: &[String]) -> Environment {
    let mut arguments = Vec::with_capacity(args.len().saturating_add(1));
    arguments.push(command_input_display_name(source));
    arguments.extend(args.iter().cloned());
    let mut environment = Environment::capture_process();
    environment.args = arguments;

    environment
}

/// Get a display name for the entry source.
#[cfg(not(target_arch = "wasm32"))]
fn command_input_display_name(source: &CommandInput) -> String {
    match source {
        CommandInput::File { path } => path.to_string_lossy().into_owned(),
        CommandInput::Inline { name, .. } => name.clone(),
        CommandInput::Stdin { name, .. } => name.clone(),
    }
}

/// Format a VM value for eval output.
#[cfg(not(target_arch = "wasm32"))]
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
#[cfg(not(target_arch = "wasm32"))]
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
#[cfg(not(target_arch = "wasm32"))]
fn is_exit_code_value(value: &Value) -> bool {
    matches!(
        value,
        Value::Void | Value::Bool(_) | Value::Int { .. } | Value::UInt { .. }
    )
}

/// Convert a runtime value into payload data.
#[cfg(not(target_arch = "wasm32"))]
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

/// Convert one signed integer to command JSON.
#[cfg(not(target_arch = "wasm32"))]
fn int_payload(value: i128) -> serde_json::Value {
    if let Ok(value) = i64::try_from(value) {
        serde_json::Value::Number(value.into())
    } else {
        serde_json::Value::String(value.to_string())
    }
}

/// Convert one unsigned integer to command JSON.
#[cfg(not(target_arch = "wasm32"))]
fn uint_payload(value: u128) -> serde_json::Value {
    if let Ok(value) = u64::try_from(value) {
        serde_json::Value::Number(value.into())
    } else {
        serde_json::Value::String(value.to_string())
    }
}

/// Create a runtime program from the module MIR.
#[cfg(not(target_arch = "wasm32"))]
fn create_program(
    repository: &Repository,
    revision: Revision,
    module_id: ModuleId,
    target_id: &TargetId,
    options: MachineOptions,
) -> CommandResult<Arc<Program>> {
    let profile_id = target_profile_id(repository, revision, module_id, *target_id)?;
    let artifacts = repository.artifact_reader(revision);
    let tree = machine_mir_tree(&artifacts, module_id, profile_id, *target_id)?;
    let strings = repository.string_pool().as_ref().clone();
    let program = destack_vm::ProgramLowerer::new(tree, strings, options.heap, options.shared_heap)
        .build()
        .map_err(|error| error.to_string())?;

    Ok(Arc::new(program))
}

/// Return the best available MIR tree for VM execution.
#[cfg(not(target_arch = "wasm32"))]
fn machine_mir_tree(
    artifacts: &ArtifactReader<'_>,
    module_id: ModuleId,
    profile_id: ProfileId,
    target_id: TargetId,
) -> CommandResult<Tree> {
    // prefer optimized mir when the optimize stage has run
    match artifacts.mir_optimized(module_id, profile_id, target_id) {
        Ok(mir) => {
            let tree = mir
                .latest_patch_tree()
                .ok_or_else(|| "optimized MIR artifact has no patches".to_string())?;

            return Ok(tree.clone());
        }
        Err(ProviderError::Blocked { .. }) => {}
        Err(error) => return Err(error.to_string().into()),
    }

    // otherwise use lowered mir
    let mir = artifacts
        .mir_lowered(module_id, profile_id, target_id)
        .map_err(|error| error.to_string())?;

    Ok(mir.tree.clone())
}

/// Return the profile id selected for one module target.
#[cfg(not(target_arch = "wasm32"))]
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
#[cfg(not(target_arch = "wasm32"))]
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

/// Convert one float to command JSON.
#[cfg(not(target_arch = "wasm32"))]
fn float_payload(value: f64) -> serde_json::Value {
    if value.is_finite() {
        serde_json::Number::from_f64(value)
            .map(serde_json::Value::Number)
            .unwrap_or_else(|| unreachable!("finite float must produce a JSON number"))
    } else {
        serde_json::Value::String(value.to_string())
    }
}
