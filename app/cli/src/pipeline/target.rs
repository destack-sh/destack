use destack_source::{ModuleId, TargetId};
use destack_workspace::{Ref, Repository, Target};

use crate::common::TargetArgs;
use crate::error::{CliError, CliResult};

/// Resolved target configuration for a module.
#[derive(Debug)]
pub struct ResolvedTarget {
    /// Target id.
    pub id: TargetId,
    /// Target configuration.
    pub target: Target,
}

/// Pick the target name from CLI args or fall back to a default.
pub fn target_name_from_args(args: &TargetArgs, default_name: &str) -> String {
    // prefer a target name provided by cli args
    args.target_name()
        .map(String::from)
        .unwrap_or_else(|| default_name.to_string())
}

/// Ensure a target exists for a module and return its configuration.
pub fn resolve_target_for_module(
    repository: &Repository,
    module_id: ModuleId,
    target_name: &str,
    target_args: &TargetArgs,
) -> CliResult<ResolvedTarget> {
    let reference = Ref::for_workspace_root(repository.workspace_root());
    let revision = repository.current(&reference).map_err(|error| {
        CliError::message(format!("failed to resolve current revision: {error}"))
    })?;

    // locate the entry module package
    let module = repository
        .module(revision, module_id)
        .map_err(|error| CliError::message(format!("failed to read module snapshot: {error}")))?
        .ok_or_else(|| CliError::message(format!("missing module snapshot for {module_id:?}")))?;
    let package_id = module.package_id;
    let target_id = TargetId::new(package_id, target_name);

    // resolve target truth
    let target = repository
        .target(revision, target_id)
        .map_err(|error| CliError::message(format!("failed to read target snapshot: {error}")))?;
    let is_explicit_target = target.is_some();

    // reject overrides for named targets
    if is_explicit_target && target_args.has_adhoc_options() {
        return Err(CliError::message(
            "ad-hoc target options are not supported for named targets",
        ));
    }

    // synthesize one implicit target when missing
    let target = if let Some(target) = target {
        target
    } else {
        let mut target = Target::implicit_for_name(target_name)
            .ok_or_else(|| CliError::message(format!("unknown target '{target_name}'")))?;
        target_args.apply_to_target(&mut target);
        target
    };

    // return the resolved target info
    Ok(ResolvedTarget {
        id: target_id,
        target,
    })
}
