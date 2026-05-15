use destack_source::{ModuleId, TargetId};
use destack_workspace::{Ref, Repository, Target};

use crate::common::TargetArgs;
use crate::error::{CliError, CliResult};

/// Selected target configuration for a module.
#[derive(Debug)]
pub struct SelectedTarget {
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
) -> CliResult<SelectedTarget> {
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
        .target_or_builtin(revision, target_id)
        .map_err(|error| CliError::message(format!("failed to read target snapshot: {error}")))?;

    // synthesize one built-in target when missing
    let mut target = if let Some(target) = target {
        target
    } else {
        Target::builtin_for_name(target_name)
            .ok_or_else(|| CliError::message(format!("unknown target '{target_name}'")))?
    };

    // apply command output redirection
    if let Some(out_dir) = target_args.out_dir.as_ref() {
        target.out_dir = out_dir.clone();
    }

    if let Some(out_file) = target_args.out_file.as_ref() {
        target.out_file = Some(out_file.clone());
    }

    // return the selected target info
    Ok(SelectedTarget {
        id: target_id,
        target,
    })
}
