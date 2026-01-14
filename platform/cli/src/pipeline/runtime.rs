use destack_runtime::platform::{BindingPolicy, DeterminismPolicy, ReplayMode};
use destack_source::ModuleId;
use destack_vm::{ExecutionMode, Isolate, IsolateOptions, TrustPolicy as VmTrustPolicy, Value};
use destack_workspace::{
    DebugMode, DeterminismPolicy as TargetDeterminismPolicy, Program,
    ReplayMode as TargetReplayMode, Target, TargetId, TrustPolicy,
};

use crate::common::InputSource;

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
    // map determinism policy into runtime binding settings
    let determinism = match target.determinism {
        TargetDeterminismPolicy::BestEffort => DeterminismPolicy::BestEffort,
        TargetDeterminismPolicy::Deterministic => DeterminismPolicy::Deterministic,
    };

    // map replay policy into runtime binding settings
    let replay = match target.replay {
        TargetReplayMode::Off => ReplayMode::Off,
        TargetReplayMode::Record => ReplayMode::Record,
        TargetReplayMode::Replay => ReplayMode::Replay,
    };

    // build the policy object
    BindingPolicy {
        determinism,
        replay,
    }
}

/// Build a VM isolate from the module MIR.
pub fn mir_isolate(
    program: &Program,
    module_id: ModuleId,
    target_id: &TargetId,
    options: IsolateOptions,
) -> Result<Isolate, String> {
    // pull the lowered mir from the module
    let module = program.modules.get(module_id);
    let module = module.read();
    let mir = module.mir(target_id);
    let tree = mir.tree.read().clone();
    let strings = mir.strings.clone().into_immutable();

    // construct the isolate from mir state
    Ok(Isolate::with_options(tree, strings, options))
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
