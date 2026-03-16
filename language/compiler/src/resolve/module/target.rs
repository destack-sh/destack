use destack_dir::{GlobalSymbolId, StaticKey, SymbolSpace};
use destack_source::{ModuleId, PackageId};
use destack_workspace::{
    GlobalSymbolGroupKey, GlobalSymbolTableKey, ProfileId, Target, TargetDiscovery, TargetId,
};

use crate::{Compiler, ResolveError, ResolveResult, TargetDiscoveryIssue};

impl Compiler {
    /// Build the cache key for a module and profile.
    pub(crate) fn build_global_symbol_table_key(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
    ) -> ResolveResult<GlobalSymbolTableKey> {
        // load module and package metadata
        let module = self.program.modules.get(module_id);
        let module = module.as_ref();
        let package_id = module.package_id;
        let package = self.program.packages.get(package_id);
        let package = package.read();
        let has_targets = !package.targets.is_empty();
        drop(package);

        // select the entry module and target id for the key
        let entry_module = (!has_targets).then_some(module_id);
        let target_id = if let Some((target_id, _)) =
            self.select_target_for_global_symbol_table(module_id, profile_id)?
        {
            target_id
        } else {
            TargetId::new(package_id, "default")
        };
        Ok(GlobalSymbolTableKey {
            target_id,
            profile_id,
            entry_module,
        })
    }

    /// Resolve a global symbol group by key and space.
    pub(crate) fn get_global_symbol_group(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
        key: StaticKey,
        space: SymbolSpace,
    ) -> Option<Vec<GlobalSymbolId>> {
        let cache_key = self
            .build_global_symbol_table_key(module_id, profile_id)
            .ok()?;
        let cache = self.program.index.global_symbol_tables.get(&cache_key)?;
        cache
            .sources_by_space
            .get(&GlobalSymbolGroupKey { key, space })
            .cloned()
    }

    /// Select the global symbol table roots for a module.
    /// Returns the cache key and root module list.
    pub(crate) fn select_global_symbol_table(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
    ) -> ResolveResult<(GlobalSymbolTableKey, Vec<ModuleId>)> {
        // load module and package metadata
        let module = self.program.modules.get(module_id);
        let module = module.as_ref();
        let package_id = module.package_id;
        let package = self.program.packages.get(package_id);
        let package = package.read();
        let package_path = package.path.clone();
        let has_targets = !package.targets.is_empty();
        drop(package);

        // fall back to the current module when no targets exist
        if !has_targets {
            let key = GlobalSymbolTableKey {
                target_id: TargetId::new(package_id, "default"),
                profile_id,
                entry_module: Some(module_id),
            };
            return Ok((key, vec![module_id]));
        }

        // prefer roots based on the package target discovery rules
        let (target_id, target) = self
            .select_target_for_global_symbol_table(module_id, profile_id)?
            .expect("checked has_targets");
        let roots = match target.discovery {
            TargetDiscovery::Entry => self
                .discover_entry_modules(package_id, &package_path, &target, &target_id)
                .map_err(|issue| self.map_target_discovery_issue(issue))?,
            TargetDiscovery::Include => self
                .discover_include_modules(package_id, &package_path, &target)
                .map_err(|issue| self.map_target_discovery_issue(issue))?,
        };
        let key = GlobalSymbolTableKey {
            target_id,
            profile_id,
            entry_module: None,
        };
        Ok((key, roots))
    }

    /// Select the target policy used for global symbol table selection.
    fn select_target_for_global_symbol_table(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
    ) -> ResolveResult<Option<(TargetId, Target)>> {
        let module = self.program.modules.get(module_id);
        let module = module.as_ref();
        let package_id = module.package_id;
        let package = self.program.packages.get(package_id);
        let package = package.read();

        if package.targets.is_empty() {
            return Ok(None);
        }
        drop(package);

        if let Some(target) = self.select_target_for_module_profile(module_id, profile_id)? {
            return Ok(Some(target));
        }

        self.select_default_target_for_package(package_id).map(Some)
    }

    /// Select a target for one module/profile pair when one profile mapping can be chosen.
    fn select_target_for_module_profile(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
    ) -> ResolveResult<Option<(TargetId, Target)>> {
        // load package metadata for target inspection
        let module = self.program.modules.get(module_id);
        let module = module.as_ref();
        let package_id = module.package_id;
        let package = self.program.packages.get(package_id);
        let package = package.read();

        // skip packages without configured targets
        if package.targets.is_empty() {
            return Ok(None);
        }

        // collect targets that resolve to the requested profile for this module
        let mut matching_targets: Vec<(TargetId, Target)> = package
            .targets
            .iter()
            .filter_map(|(target_id, target)| {
                let candidate_profile = self.program.profile_id_for_target(module_id, target_id)?;
                if candidate_profile == profile_id {
                    Some((target_id.clone(), target.clone()))
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
        if let Some(config) = package.config.as_ref()
            && let Some(default_target) = config.options.default_target.as_ref()
        {
            let default_target_id = TargetId::new(package_id, default_target);
            if let Some((target_id, target)) = matching_targets
                .iter()
                .find(|(target_id, _)| *target_id == default_target_id)
            {
                return Ok(Some((target_id.clone(), target.clone())));
            }

            let mut target_names: Vec<String> = matching_targets
                .iter()
                .map(|(target_id, _)| target_id.name.clone())
                .collect();
            target_names.sort();
            let available = target_names.join(", ");
            return Err(ResolveError::InvalidTargetConfig {
                package: package_id,
                target: default_target_id,
                message: format!(
                    "default target does not match profile target set; profile matches: {available}"
                ),
            });
        }

        let mut target_names: Vec<String> = matching_targets
            .iter()
            .map(|(target_id, _)| target_id.name.clone())
            .collect();
        target_names.sort();
        let available = target_names.join(", ");
        Err(ResolveError::InvalidTargetConfig {
            package: package_id,
            target: TargetId::new(package_id, "default"),
            message: format!(
                "multiple targets map to the same profile; set default target to disambiguate: {available}"
            ),
        })
    }

    /// Select the default target for a package.
    fn select_default_target_for_package(
        &self,
        package_id: PackageId,
    ) -> ResolveResult<(TargetId, Target)> {
        // load package metadata
        let package = self.program.packages.get(package_id);
        let package = package.read();

        // honor an explicit default target from config
        if let Some(config) = package.config.as_ref()
            && let Some(default_target) = config.options.default_target.as_ref()
        {
            let target_id = TargetId::new(package_id, default_target);
            let Some(target) = package.targets.get(&target_id) else {
                return Err(ResolveError::InvalidTargetConfig {
                    package: package_id,
                    target: target_id.clone(),
                    message: "default target not found".to_string(),
                });
            };
            return Ok((target_id, target.clone()));
        }

        // reject packages with no targets configured
        if package.targets.is_empty() {
            return Err(ResolveError::InvalidTargetConfig {
                package: package_id,
                target: TargetId::new(package_id, "default"),
                message: "package has no targets".to_string(),
            });
        }

        // select the only configured target when there is exactly one
        if package.targets.len() == 1 {
            let (target_id, target) = package.targets.iter().next().expect("checked len");
            return Ok((target_id.clone(), target.clone()));
        }

        // require an explicit default target when multiple targets exist
        let mut target_names: Vec<String> = package
            .targets
            .keys()
            .map(|target_id| target_id.name.clone())
            .collect();
        target_names.sort();
        let available = target_names.join(", ");
        Err(ResolveError::InvalidTargetConfig {
            package: package_id,
            target: TargetId::new(package_id, "default"),
            message: format!("default target not specified; available targets: {available}"),
        })
    }

    /// Map a target discovery issue into a resolve error.
    fn map_target_discovery_issue(&self, issue: TargetDiscoveryIssue) -> ResolveError {
        // normalize issues into invalid target configuration errors
        match issue {
            TargetDiscoveryIssue::MissingPackagePath { package, target } => {
                ResolveError::InvalidTargetConfig {
                    package,
                    target,
                    message: "entry based discovery requires package path".to_string(),
                }
            }
            TargetDiscoveryIssue::MissingEntry {
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
