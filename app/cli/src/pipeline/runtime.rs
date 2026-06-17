use destack_engine::{EngineId, Value};
use destack_mir::Tree;
use destack_repository::{ArtifactReader, Environment, ProviderError, Repository, Revision};
use destack_source::{ModuleId, ProfileId, TargetId};
use destack_vm::{Machine, MachineOptions};

use crate::common::InputSource;
use crate::error::{CliError, CliResult};

/// Create a VM machine from the module MIR.
pub fn create_machine(
    repository: &Repository,
    revision: Revision,
    module_id: ModuleId,
    target_id: &TargetId,
    options: MachineOptions,
) -> CliResult<Machine> {
    // resolve lowered mir for the target
    let profile_id = target_profile_id(repository, revision, module_id, *target_id)?;
    let artifacts = repository.artifact_reader(revision);
    let tree = machine_mir_tree(&artifacts, module_id, profile_id, *target_id)?;
    let strings = repository.string_pool().as_ref().clone();

    // construct the machine from mir state
    Machine::build_with_options(EngineId::new(1), tree, strings, options)
        .map_err(|error| CliError::message(error.to_string()))
}

/// Return the best available MIR tree for VM execution.
fn machine_mir_tree(
    artifacts: &ArtifactReader<'_>,
    module_id: ModuleId,
    profile_id: ProfileId,
    target_id: TargetId,
) -> CliResult<Tree> {
    // prefer optimized mir when the optimize stage has run
    match artifacts.mir_optimized(module_id, profile_id, target_id) {
        Ok(mir) => {
            let tree = mir.latest_patch_tree().ok_or_else(|| {
                CliError::message("optimized MIR artifact has no patches".to_string())
            })?;

            return Ok(tree.clone());
        }
        Err(ProviderError::Blocked { .. }) => {}
        Err(error) => return Err(CliError::message(error.to_string())),
    }

    // otherwise use lowered mir
    let mir = artifacts
        .mir_lowered(module_id, profile_id, target_id)
        .map_err(|error| CliError::message(error.to_string()))?;

    Ok(mir.tree.clone())
}

/// Build the launch environment for the entry source.
pub fn environment_for_source(source: &InputSource, args: &[String]) -> Environment {
    // include the entry display name as argv[0]
    let mut environment = Environment::capture_process();
    let mut launch_args = Vec::with_capacity(args.len().saturating_add(1));
    launch_args.push(entry_display_name(source));
    launch_args.extend(args.iter().cloned());
    environment.args = launch_args;

    environment
}

/// Get a display name for the entry source.
pub fn entry_display_name(source: &InputSource) -> String {
    // pick a displayable identifier for the entry source
    match source {
        InputSource::File(path) => path.to_string_lossy().into_owned(),
        InputSource::Inline { name, .. } => name.clone(),
        InputSource::Stdin { name } => name.clone(),
    }
}

/// Convert a VM return value into an exit status.
pub fn exit_status_from_value(value: Value) -> i32 {
    match value {
        Value::Void => 0,
        Value::Bool(value) => {
            if value {
                0
            } else {
                1
            }
        }
        Value::Int { value, .. } => {
            let min = i128::from(i32::MIN);
            let max = i128::from(i32::MAX);

            value.clamp(min, max) as i32
        }
        Value::UInt { value, .. } => value.min(i32::MAX as u128) as i32,
        _ => 0,
    }
}

/// Format a VM value for eval output.
pub fn format_value_for_eval(value: &Value) -> String {
    match value {
        Value::Void => "void".to_string(),
        Value::Bool(value) => value.to_string(),
        Value::Int { value, .. } => value.to_string(),
        Value::UInt { value, .. } => value.to_string(),
        Value::Float32 { bits } => f32::from_bits(*bits).to_string(),
        Value::Float64 { bits } => f64::from_bits(*bits).to_string(),
        Value::Char(value) => value.to_string(),
        Value::Float16 { bits } => bits.to_string(),
        Value::Bfloat16 { bits } => bits.to_string(),
        Value::HeapReference(_) | Value::SharedHeapReference(_) | Value::Address(_) => {
            format!("{value:?}")
        }
    }
}

/// Return the profile id selected for one module target.
fn target_profile_id(
    repository: &Repository,
    revision: Revision,
    module_id: ModuleId,
    target_id: TargetId,
) -> CliResult<ProfileId> {
    let profile = repository
        .profile_for_module_target(revision, module_id, target_id)
        .map_err(|error| CliError::message(error.to_string()))?;

    Ok(profile.id())
}
