use destack_runtime::runtime::bindings::BindingPolicy;
use destack_source::{ModuleId, TargetId};
use destack_vm::{
    ExecutionMode, Isolate, IsolateId, IsolateOptions, TrustPolicy as VmTrustPolicy, Value,
};
use destack_workspace::{DebugMode, Repository, Revision, Target, TrustPolicy};

use crate::common::InputSource;
use crate::error::{CliError, CliResult};

/// Create isolate options from target configuration.
pub fn isolate_options_for_target(target: &Target) -> IsolateOptions {
    // start from default isolate options
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

    // return resolved options
    options
}

/// Create binding policy from target configuration.
pub fn binding_policy_for_target(target: &Target) -> BindingPolicy {
    // map execution mode into runtime binding settings
    let mode = target.runtime_options.execution_mode();

    // build the policy object
    BindingPolicy::new(mode)
}

/// Create a VM isolate from the module MIR.
pub fn create_isolate(
    repository: &Repository,
    revision: Revision,
    module_id: ModuleId,
    target_id: &TargetId,
    options: IsolateOptions,
) -> CliResult<Isolate> {
    // resolve lowered mir for the target
    let profile_id = repository
        .default_profile_id_for_module(revision, module_id)
        .map_err(|error| CliError::message(error.to_string()))?;
    let (tree, strings) =
        if let Some(mir) = repository.mir_optimized(revision, module_id, profile_id, *target_id) {
            (mir.tree.clone(), mir.strings.clone().into_immutable())
        } else if let Some(mir) = repository.mir_base(revision, module_id, profile_id, *target_id) {
            (mir.tree.clone(), mir.strings.clone().into_immutable())
        } else {
            return Err(CliError::message(format!(
                "missing MIR for target {target_id:?} (run requires lowering)"
            )));
        };

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
    // treat void as successful exit
    if value.is_void() {
        return 0;
    }

    // map booleans to success or failure
    if let Some(result) = value.as_bool() {
        return if result { 0 } else { 1 };
    }

    // clamp integers into an exit code range
    if let Some(result) = value.as_int() {
        return result.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32;
    }

    // default to success for non numeric values
    0
}

/// Format a VM value for eval output.
pub fn format_value_for_eval(value: &Value) -> String {
    // format void values
    if value.is_void() {
        return "void".to_string();
    }

    // format boolean values
    if let Some(result) = value.as_bool() {
        return result.to_string();
    }

    // format signed integers
    if let Some(result) = value.as_int_with_width() {
        return result.0.to_string();
    }

    // format unsigned integers
    if let Some(result) = value.as_uint_with_width() {
        return result.0.to_string();
    }

    // format float64 values
    if let Some(result) = value.as_float64() {
        return result.to_string();
    }

    // format float32 values
    if let Some(result) = value.as_float32() {
        return result.to_string();
    }

    // format char values
    if let Some(result) = value.as_char() {
        return result.to_string();
    }

    // fallback to debug output
    format!("{value:?}")
}
