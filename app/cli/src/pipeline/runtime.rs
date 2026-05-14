use destack_artifact::ArtifactKey;
use destack_source::{ModuleId, ProfileId, TargetId};
use destack_vm::{Isolate, IsolateId, IsolateOptions, Value};
use destack_workspace::{Repository, Revision};

use crate::common::InputSource;
use crate::error::{CliError, CliResult};

/// Create a VM isolate from the module MIR.
pub fn create_isolate(
    repository: &Repository,
    revision: Revision,
    module_id: ModuleId,
    target_id: &TargetId,
    options: IsolateOptions,
) -> CliResult<Isolate> {
    // resolve lowered mir for the target
    let profile_id = target_profile_id(repository, revision, module_id, *target_id)?;
    let optimized_key = ArtifactKey::mir_optimized(module_id, profile_id, *target_id);
    let lowered_key = ArtifactKey::mir_lowered(module_id, profile_id, *target_id);
    let optimized_version = repository
        .artifact_version(revision, &optimized_key)
        .map_err(|error| CliError::message(error.to_string()))?;
    let lowered_version = repository
        .artifact_version(revision, &lowered_key)
        .map_err(|error| CliError::message(error.to_string()))?;

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
        return Err(CliError::message(format!(
            "missing MIR for target {target_id:?} (run requires lowering)"
        )));
    };
    let strings = repository.string_pool().as_ref().clone();

    // construct the isolate from mir state
    Isolate::build_with_options(IsolateId::new(1), tree, strings, options)
        .map_err(|error| CliError::message(error.to_string()))
}

/// Build process arguments for the entry source.
pub fn process_args_for_source(source: &InputSource, args: &[String]) -> Vec<String> {
    // include the entry display name as argv[0]
    let mut process_args = Vec::with_capacity(args.len().saturating_add(1));
    process_args.push(entry_display_name(source));
    process_args.extend(args.iter().cloned());

    // return the final argv list
    process_args
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
        Value::HeapReference(_)
        | Value::SharedHeapReference(_)
        | Value::RawPointer(_)
        | Value::SharedRawPointer(_) => format!("{value:?}"),
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
        .module_target_profile(revision, module_id, target_id)
        .map_err(|error| CliError::message(error.to_string()))?;
    let profile = if let Some(profile) = profile {
        profile
    } else {
        repository
            .module_profile(revision, module_id)
            .map_err(|error| CliError::message(error.to_string()))?
    };

    Ok(profile.id())
}
