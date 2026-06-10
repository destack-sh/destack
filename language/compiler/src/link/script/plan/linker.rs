use std::collections::{HashSet, VecDeque};

use destack_artifact::{ArtifactKey, ModuleOutput};
use destack_codegen_js::DependencyForm;
use destack_repository::ProviderError;
use destack_source::ModuleId;
use indexmap::IndexSet;

use crate::{CompilerResult, LinkError, LinkResult};

use super::super::{ScriptLinker, dynamic_script_dependencies, static_script_dependencies};
use super::{ModuleSet, OutputGraph, Plan};
use crate::CompilerError;

/// Order the included modules after their bundled dependencies.
fn order_script_modules(
    linker: &ScriptLinker<'_>,
    module_ids: &[ModuleId],
) -> LinkResult<Vec<ModuleId>> {
    let included_modules = module_ids.iter().copied().collect::<HashSet<_>>();
    let mut active_modules = HashSet::new();
    let mut finished_modules = HashSet::new();
    let mut ordered_modules = Vec::with_capacity(module_ids.len());

    // walk each requested module
    for module_id in module_ids {
        visit_ordered_module(
            linker,
            &included_modules,
            &mut active_modules,
            &mut finished_modules,
            &mut ordered_modules,
            *module_id,
        )?;
    }

    Ok(ordered_modules)
}

/// Visit one module and append it after its included dependencies.
fn visit_ordered_module(
    linker: &ScriptLinker<'_>,
    included_modules: &HashSet<ModuleId>,
    active_modules: &mut HashSet<ModuleId>,
    finished_modules: &mut HashSet<ModuleId>,
    ordered_modules: &mut Vec<ModuleId>,
    module_id: ModuleId,
) -> LinkResult<()> {
    // skip modules that are already fully ordered
    if finished_modules.contains(&module_id) {
        return Ok(());
    }

    // keep dependency cycles in their discovered relative order
    if !active_modules.insert(module_id) {
        return Ok(());
    }

    // order included dependencies before the current module
    let dependency_modules = linker.bundled_script_dependency_modules(module_id)?;
    for dependency_module in dependency_modules {
        if !included_modules.contains(&dependency_module) {
            continue;
        }

        visit_ordered_module(
            linker,
            included_modules,
            active_modules,
            finished_modules,
            ordered_modules,
            dependency_module,
        )?;
    }

    active_modules.remove(&module_id);
    finished_modules.insert(module_id);
    ordered_modules.push(module_id);

    Ok(())
}

impl<'a> ScriptLinker<'a> {
    /// Build the script module set for one target.
    pub(crate) fn build_script_module_set(
        &self,
        entry_modules: &[ModuleId],
        module_ids: &[ModuleId],
    ) -> LinkResult<ModuleSet> {
        let entry_modules = entry_modules.to_vec();
        let included_modules = module_ids.to_vec();
        let modules = order_script_modules(self, &included_modules)?;
        let mut module_set = ModuleSet {
            entry_modules,
            modules,
            external_targets: IndexSet::new(),
            dynamic_targets: IndexSet::new(),
            has_opaque_dynamic_imports: false,
        };

        // collect retained external and dynamic edges
        self.collect_retained_script_targets(&mut module_set)?;

        Ok(module_set)
    }

    /// Collect the retained external and dynamic script targets.
    fn collect_retained_script_targets(&self, module_set: &mut ModuleSet) -> LinkResult<()> {
        for module_id in &module_set.modules {
            let module = self.module(*module_id)?;

            if !module.is_code() {
                continue;
            }

            let artifact = self.module_output(*module_id)?;

            let ModuleOutput::Script(script) = artifact.as_ref() else {
                continue;
            };

            // retained static externals
            for dependency in static_script_dependencies(&script.module) {
                if !self.should_bundle_script_dependency(
                    self.module_anchor_span(*module_id)?,
                    self.package_id,
                    self.target_id,
                    self.target,
                    &dependency.target,
                )? {
                    module_set
                        .external_targets
                        .insert(dependency.target.specifier().to_string());
                }
            }

            // retained and bundled dynamic edges
            for dependency in dynamic_script_dependencies(&script.module) {
                let Some(dependency_target) = &dependency.target else {
                    module_set.has_opaque_dynamic_imports = true;
                    continue;
                };

                let should_bundle = self.should_bundle_script_dependency(
                    self.module_anchor_span(*module_id)?,
                    self.package_id,
                    self.target_id,
                    self.target,
                    dependency_target,
                )?;

                // chunked outputs can retain internal dynamic edges as output links
                if should_bundle {
                    if self.target.assembly == destack_repository::BundleMode::Chunked {
                        continue;
                    }

                    return Err(LinkError::InvalidTarget {
                        anchor: (*module_id).into(),
                        package: self.package_id,
                        target: self.target_id.clone(),
                        message: format!(
                            "bundled dynamic import '{}' is not implemented yet",
                            dependency_target.specifier()
                        ),
                    }
                    .into());
                }

                module_set
                    .dynamic_targets
                    .insert(dependency_target.specifier().to_string());
            }
        }

        Ok(())
    }

    /// Require all generated script outputs needed to link one target.
    pub(crate) fn require_module_artifacts(
        &self,
        root_modules: &[ModuleId],
    ) -> CompilerResult<Vec<ModuleId>> {
        let mut blocked = Vec::new();
        let mut required_modules = Vec::new();
        let mut queued_modules = HashSet::new();
        let mut pending_modules = root_modules.iter().copied().collect::<VecDeque<_>>();

        // require root modules and their bundled reachable closure
        while let Some(module_id) = pending_modules.pop_front() {
            if !queued_modules.insert(module_id) {
                continue;
            }

            self.ensure_profile_for_target(module_id)?;
            let module = self.module(module_id)?;
            let profile_id = self.profile_id_for_module(module_id)?;

            // resource modules are linked directly from patched module state
            if !module.is_code() {
                match self
                    .artifacts
                    .require(ArtifactKey::dir_checked(module_id, profile_id))
                {
                    Ok(_) => {}
                    Err(ProviderError::Blocked { keys }) => blocked.extend(keys),
                    Err(error) => return Err(CompilerError::from(error)),
                }
                required_modules.push(module_id);
                continue;
            }

            let is_output_ready = match self
                .artifacts
                .require(ArtifactKey::module_output(module_id, *self.target_id))
            {
                Ok(_) => true,
                Err(ProviderError::Blocked { keys }) => {
                    blocked.extend(keys);

                    false
                }
                Err(error) => return Err(CompilerError::from(error)),
            };

            // linked output rewriting and identifier minification still consult
            // the patched dir for source backed code modules
            match self
                .artifacts
                .require(ArtifactKey::dir_checked(module_id, profile_id))
            {
                Ok(_) => {}
                Err(ProviderError::Blocked { keys }) => blocked.extend(keys),
                Err(error) => return Err(CompilerError::from(error)),
            }

            required_modules.push(module_id);

            // only traverse bundled dependencies after the generated output exists
            if !is_output_ready {
                continue;
            }

            let requirements = self.bundled_static_dependency_exports(module_id)?;
            if !requirements.is_empty() {
                match self.artifacts.require_all(&requirements) {
                    Ok(_) => {}
                    Err(ProviderError::Blocked { keys }) => blocked.extend(keys),
                    Err(error) => return Err(CompilerError::from(error)),
                }
            }

            for dependency_id in self
                .bundled_script_dependency_modules(module_id)
                .map_err(CompilerError::from)?
            {
                if !queued_modules.contains(&dependency_id) {
                    pending_modules.push_back(dependency_id);
                }
            }
        }

        // block while required generated outputs are still pending
        if !blocked.is_empty() {
            return Err(CompilerError::Blocked { keys: blocked });
        }

        Ok(required_modules)
    }

    /// Return exported DIR keys needed by bundled static dependency rewrites.
    fn bundled_static_dependency_exports(
        &self,
        module_id: ModuleId,
    ) -> CompilerResult<Vec<ArtifactKey>> {
        let module = self.module(module_id)?;
        if !module.is_code() {
            return Ok(Vec::new());
        }

        let profile_id = self.profile_id_for_module(module_id)?;
        let artifact = self.module_output(module_id)?;
        let ModuleOutput::Script(script) = artifact.as_ref() else {
            return Ok(Vec::new());
        };
        let mut requirements = IndexSet::new();

        // collect bundled static dependency export tables
        for dependency in static_script_dependencies(&script.module) {
            if dependency.form == DependencyForm::Type {
                continue;
            }

            let should_bundle = self
                .should_bundle_script_dependency(
                    self.module_anchor_span(module_id)?,
                    self.package_id,
                    self.target_id,
                    self.target,
                    &dependency.target,
                )
                .map_err(CompilerError::from)?;
            if !should_bundle {
                continue;
            }

            let Some(target_module) = dependency.target.module() else {
                continue;
            };

            requirements.insert(ArtifactKey::dir_exported(target_module, profile_id));
        }

        Ok(requirements.into_iter().collect())
    }

    /// Return the bundled internal script dependencies for one generated module.
    fn bundled_script_dependency_modules(&self, module_id: ModuleId) -> LinkResult<Vec<ModuleId>> {
        let module = self.module(module_id)?;

        if !module.is_code() {
            return Ok(Vec::new());
        }

        let mut dependency_modules = self.bundled_static_script_modules(
            module_id,
            self.target,
            self.target_id,
            self.package_id,
        )?;
        let dynamic_dependency_modules = self.bundled_dynamic_script_modules(
            module_id,
            self.target,
            self.target_id,
            self.package_id,
        )?;

        dependency_modules.extend(dynamic_dependency_modules);
        let mut seen_dependency_modules = HashSet::new();
        dependency_modules.retain(|module_id| seen_dependency_modules.insert(*module_id));

        Ok(dependency_modules)
    }

    /// Build the output plan for this target.
    pub(in super::super) fn plan(&self, root_modules: &[ModuleId]) -> CompilerResult<Plan> {
        let script_root_modules = root_modules.to_vec();
        let asset_root_modules = Vec::new();

        let script_module_id_set = if script_root_modules.is_empty() {
            Vec::new()
        } else {
            self.require_module_artifacts(&script_root_modules)?
        };
        let module_set = if script_module_id_set.is_empty() {
            super::ModuleSet::default()
        } else {
            self.build_script_module_set(&script_root_modules, &script_module_id_set)
                .map_err(CompilerError::from)?
        };
        let output_graph = if module_set.modules().is_empty() {
            OutputGraph::default_empty(self.target.assembly)
        } else {
            self.build_script_output_graph(&module_set)
                .map_err(CompilerError::from)?
        };
        let asset_module_id_set =
            self.collect_asset_modules(&asset_root_modules, &script_module_id_set)?;
        let output_layout = self
            .build_output_layout(&output_graph)
            .map_err(CompilerError::from)?;
        let asset_reference_map = self
            .plan_asset_references(asset_module_id_set.into_iter())
            .map_err(CompilerError::from)?;

        Ok(Plan::new(
            module_set,
            output_graph,
            output_layout,
            asset_reference_map,
        ))
    }
}
