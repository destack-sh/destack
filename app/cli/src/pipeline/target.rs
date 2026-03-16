use destack_source::ModuleId;
use destack_workspace::{Program, Target, TargetId};

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
    program: &Program,
    module_id: ModuleId,
    target_name: &str,
    target_args: &TargetArgs,
) -> CliResult<ResolvedTarget> {
    // locate the entry module package
    let module = program.modules.get(module_id);
    let package_id = module.package_id;
    let target_id = TargetId::new(package_id, target_name);

    // look for an existing target entry
    let package = program.packages.get(package_id);
    let mut package = package.write();
    let existing_target = package.targets.get(&target_id).cloned();

    // reject overrides for named targets
    if existing_target.is_some() && target_args.has_adhoc_options() {
        return Err(CliError::message(
            "ad-hoc target options are not supported for named targets",
        ));
    }

    // insert implicit target when missing
    if existing_target.is_none() {
        let mut target = Target::implicit_for_name(target_name)
            .ok_or_else(|| CliError::message(format!("unknown target '{target_name}'")))?;
        target_args.apply_to_target(&mut target);
        package.targets.insert(target_id.clone(), target);
    }

    // fetch the resolved target after updates
    let target = package.targets.get(&target_id).cloned().ok_or_else(|| {
        CliError::message(format!("target '{target_id}' not found in package config"))
    })?;

    // return the resolved target info
    Ok(ResolvedTarget {
        id: target_id,
        target,
    })
}
