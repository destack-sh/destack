use std::sync::Arc;

use destack_dir::{GlobalSymbolId, StaticKey, SymbolSpace};
use destack_source::{ModuleId, PackageId, TargetId};
use destack_workspace::{ProfileId, Target, TargetDiscovery, TargetSelection};

use super::globals::GlobalSymbolGroupKey;
use crate::{Compiler, ResolveError, ResolveResult, TargetDiscoveryError};

#[allow(dead_code)]
impl Compiler {
    /// Resolve a global symbol group by key and space.
    pub(crate) fn get_global_symbol_group(
        &self,
        revision: destack_workspace::Revision,
        module_id: ModuleId,
        profile_id: ProfileId,
        key: StaticKey,
        space: SymbolSpace,
    ) -> Option<Vec<GlobalSymbolId>> {
        let cache = self
            .global_symbol_table_for_module(revision, module_id, profile_id)
            .ok()?;
        cache
            .sources_by_space
            .get(&GlobalSymbolGroupKey { key, space })
            .cloned()
    }

    /// Select the global symbol table roots for a module.
    pub(crate) fn select_global_symbol_table(
        &self,
        revision: destack_workspace::Revision,
        module_id: ModuleId,
        profile_id: ProfileId,
    ) -> ResolveResult<Arc<[ModuleId]>> {
        // active execution cache
        if let Some(roots) = self.current_global_symbol_table_roots(revision, module_id, profile_id)
        {
            return Ok(roots);
        }

        // load module and package metadata
        let module = self
            .repository
            .module(revision, module_id)
            .map_err(|error| ResolveError::Internal {
                message: error.to_string(),
            })?
            .ok_or_else(|| ResolveError::Internal {
                message: format!("missing module for {module_id:?}"),
            })?;
        let module = module.as_ref();
        let package_id = module.package_id;
        let package = self
            .repository
            .package(revision, package_id)
            .map_err(|error| ResolveError::Internal {
                message: error.to_string(),
            })?
            .ok_or_else(|| ResolveError::Internal {
                message: format!("missing package for {package_id:?}"),
            })?;
        let package_path = package.path.clone();
        let has_targets = !package.targets.is_empty();

        // fall back to the current module when no targets exist
        if !has_targets {
            let roots = Arc::<[ModuleId]>::from([module_id]);
            self.store_current_global_symbol_table_roots(
                revision,
                module_id,
                profile_id,
                Arc::clone(&roots),
            );

            return Ok(roots);
        }

        // prefer roots based on the package target discovery rules
        let (target_id, target) = self
            .select_target_for_global_symbol_table(revision, module_id, profile_id)?
            .expect("checked has_targets");
        let roots = match target.discovery {
            TargetDiscovery::Entry => self
                .discover_entry_modules(revision, package_id, &package_path, &target, &target_id)
                .map_err(|issue| self.map_target_discovery_issue(issue))?,
            TargetDiscovery::Include => self
                .discover_include_modules(revision, package_id, &package_path, &target, &target_id)
                .map_err(|issue| self.map_target_discovery_issue(issue))?,
        };
        let roots = Arc::<[ModuleId]>::from(roots);

        self.store_current_global_symbol_table_roots(
            revision,
            module_id,
            profile_id,
            Arc::clone(&roots),
        );

        Ok(roots)
    }

    /// Select the target policy used for global symbol table selection.
    fn select_target_for_global_symbol_table(
        &self,
        revision: destack_workspace::Revision,
        module_id: ModuleId,
        profile_id: ProfileId,
    ) -> ResolveResult<Option<(TargetId, Target)>> {
        self.target_policy_for_module_profile(revision, module_id, profile_id)
    }

    /// Select the target policy for one module/profile pair.
    pub(crate) fn target_policy_for_module_profile(
        &self,
        revision: destack_workspace::Revision,
        module_id: ModuleId,
        profile_id: ProfileId,
    ) -> ResolveResult<Option<(TargetId, Target)>> {
        let module = self
            .repository
            .module(revision, module_id)
            .map_err(|error| ResolveError::Internal {
                message: error.to_string(),
            })?
            .ok_or_else(|| ResolveError::Internal {
                message: format!("missing module for {module_id:?}"),
            })?;
        let module = module.as_ref();
        let package_id = module.package_id;
        let package = self
            .repository
            .package(revision, package_id)
            .map_err(|error| ResolveError::Internal {
                message: error.to_string(),
            })?
            .ok_or_else(|| ResolveError::Internal {
                message: format!("missing package for {package_id:?}"),
            })?;

        if package.targets.is_empty() {
            return Ok(None);
        }

        if let Some(target) =
            self.select_target_for_module_profile(revision, module_id, profile_id)?
        {
            return Ok(Some(target));
        }

        self.select_default_target_for_package(revision, package_id)
            .map(Some)
    }

    /// Select a target for one module/profile pair when one profile mapping can be chosen.
    fn select_target_for_module_profile(
        &self,
        revision: destack_workspace::Revision,
        module_id: ModuleId,
        profile_id: ProfileId,
    ) -> ResolveResult<Option<(TargetId, Target)>> {
        // load package metadata for target inspection
        let module = self
            .repository
            .module(revision, module_id)
            .map_err(|error| ResolveError::Internal {
                message: error.to_string(),
            })?
            .ok_or_else(|| ResolveError::Internal {
                message: format!("missing module for {module_id:?}"),
            })?;
        let module = module.as_ref();
        let package_id = module.package_id;
        let package = self
            .repository
            .package(revision, package_id)
            .map_err(|error| ResolveError::Internal {
                message: error.to_string(),
            })?
            .ok_or_else(|| ResolveError::Internal {
                message: format!("missing package for {package_id:?}"),
            })?;

        // skip packages without configured targets
        if package.targets.is_empty() {
            return Ok(None);
        }

        // collect targets that resolve to the requested profile for this module
        let mut matching_targets: Vec<(TargetId, Target)> = package
            .targets
            .iter()
            .filter_map(|(target_id, target)| {
                let candidate_profile = self
                    .repository
                    .profile_for_target(revision, module_id, target_id)
                    .ok()
                    .flatten()
                    .map(|profile| profile.id())?;
                if candidate_profile == profile_id {
                    Some((*target_id, target.clone()))
                } else {
                    None
                }
            })
            .collect();

        // no profile match: caller must fall back to default target selection
        if matching_targets.is_empty() {
            return Ok(None);
        }

        // one profile match: use it directly
        if matching_targets.len() == 1 {
            return Ok(Some(matching_targets.remove(0)));
        }

        // multiple profile matches: prefer explicit default target when available
        let mut target_names: Vec<String> = matching_targets
            .iter()
            .map(|(target_id, _)| self.target_name_for_revision(revision, target_id))
            .collect();
        target_names.sort();
        let available = target_names.join(", ");

        // use the selected package default target when it disambiguates this profile set
        match self
            .repository
            .default_target(revision, package_id)
            .map_err(|error| ResolveError::Internal {
                message: format!("failed to load target snapshot for {package_id:?}: {error}"),
            })? {
            TargetSelection::Selected { target_id, target } => {
                if matching_targets
                    .iter()
                    .any(|(candidate_target_id, _)| *candidate_target_id == target_id)
                {
                    return Ok(Some((target_id, target)));
                }

                Err(ResolveError::InvalidTargetConfig {
                    package: package_id,
                    target: target_id,
                    message: format!(
                        "default target does not match profile target set; profile matches: {available}"
                    ),
                })
            }
            TargetSelection::MissingConfigured { target_id } => {
                Err(ResolveError::InvalidTargetConfig {
                    package: package_id,
                    target: target_id,
                    message: "default target not found".to_string(),
                })
            }
            TargetSelection::None | TargetSelection::Ambiguous { .. } => {
                Err(ResolveError::InvalidTargetConfig {
                    package: package_id,
                    target: self.repository.intern_target_id(package_id, "default"),
                    message: format!(
                        "multiple targets map to the same profile; set default target to disambiguate: {available}"
                    ),
                })
            }
        }
    }

    /// Select the default target for a package.
    fn select_default_target_for_package(
        &self,
        revision: destack_workspace::Revision,
        package_id: PackageId,
    ) -> ResolveResult<(TargetId, Target)> {
        // map repository target selection into resolve semantics
        let default_target = self
            .repository
            .default_target(revision, package_id)
            .map_err(|error| ResolveError::Internal {
                message: format!("failed to load target snapshot for {package_id:?}: {error}"),
            })?;

        match default_target {
            TargetSelection::Selected { target_id, target } => Ok((target_id, target)),
            TargetSelection::MissingConfigured { target_id } => {
                Err(ResolveError::InvalidTargetConfig {
                    package: package_id,
                    target: target_id,
                    message: "default target not found".to_string(),
                })
            }
            TargetSelection::None => Err(ResolveError::InvalidTargetConfig {
                package: package_id,
                target: self.repository.intern_target_id(package_id, "default"),
                message: "package has no targets".to_string(),
            }),
            TargetSelection::Ambiguous { target_ids } => {
                let mut target_names: Vec<String> = target_ids
                    .iter()
                    .map(|target_id| self.target_name_for_revision(revision, target_id))
                    .collect();
                target_names.sort();
                let available = target_names.join(", ");

                Err(ResolveError::InvalidTargetConfig {
                    package: package_id,
                    target: self.repository.intern_target_id(package_id, "default"),
                    message: format!(
                        "default target not specified; available targets: {available}"
                    ),
                })
            }
        }
    }

    /// Map a target discovery issue into a resolve error.
    fn map_target_discovery_issue(&self, issue: TargetDiscoveryError) -> ResolveError {
        // normalize issues into invalid target configuration errors
        match issue {
            TargetDiscoveryError::Repository {
                package,
                target,
                message,
            } => ResolveError::InvalidTargetConfig {
                package,
                target,
                message,
            },
            TargetDiscoveryError::MissingPackagePath { package, target } => {
                ResolveError::InvalidTargetConfig {
                    package,
                    target,
                    message: "entry based discovery requires package path".to_string(),
                }
            }
            TargetDiscoveryError::MissingEntry {
                package,
                target,
                path,
            } => ResolveError::InvalidTargetConfig {
                package,
                target,
                message: format!("entry point not found: {}", path.display()),
            },
        }
    }
}
