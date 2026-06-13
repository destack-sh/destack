use std::collections::{HashSet, VecDeque};

use crate::generate::js::DependencyForm;
use destack_artifact::{ArtifactDependencySet, ArtifactKey, ModuleOutput};
use destack_repository::ProviderError;
use destack_source::ModuleId;
use indexmap::IndexSet;

use crate::{CompilerResult, LinkError, LinkResult};

use super::super::{JsLinker, dynamic_js_dependencies, static_js_dependencies};
use super::{ModuleSet, OutputGraph, Plan};
use crate::CompilerError;

/// Order the included modules after their bundled dependencies.
fn order_script_modules(
    linker: &JsLinker<'_>,
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
    linker: &JsLinker<'_>,
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

impl<'a> JsLinker<'a> {
    /// Build the JS module set for one target.
    pub(crate) fn build_js_module_set(
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

    /// Collect the retained external and dynamic JS targets.
    fn collect_retained_script_targets(&self, module_set: &mut ModuleSet) -> LinkResult<()> {
        for module_id in &module_set.modules {
            let module = self.module(*module_id)?;

            if !module.is_code() {
                continue;
            }

            let artifact = self.module_output(*module_id)?;

            let ModuleOutput::Js(script) = artifact.as_ref() else {
                continue;
            };

            // retained static externals
            for dependency in static_js_dependencies(&script.module) {
                if !self.should_bundle_js_dependency(
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
            for dependency in dynamic_js_dependencies(&script.module) {
                let Some(dependency_target) = &dependency.target else {
                    module_set.has_opaque_dynamic_imports = true;
                    continue;
                };

                let should_bundle = self.should_bundle_js_dependency(
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
                        target: *self.target_id,
                        message: format!(
                            "bundled dynamic import '{}' is not implemented yet",
                            dependency_target.specifier()
                        ),
                    });
                }

                module_set
                    .dynamic_targets
                    .insert(dependency_target.specifier().to_string());
            }
        }

        Ok(())
    }

    /// Declare every artifact needed to link one target's JS closure.
    ///
    /// The bundled traversal only widens through modules whose generated
    /// output is ready, so re-collecting reaches deeper bundles in turn.
    pub(crate) fn collect_modules(
        &self,
        root_modules: &[ModuleId],
        dependencies: &mut ArtifactDependencySet,
    ) -> CompilerResult<()> {
        self.plan_modules(root_modules, dependencies)?;

        Ok(())
    }

    /// Return the linked module closure, declaring each artifact it reads.
    fn plan_modules(
        &self,
        root_modules: &[ModuleId],
        dependencies: &mut ArtifactDependencySet,
    ) -> CompilerResult<Vec<ModuleId>> {
        let mut required_modules = Vec::new();
        let mut queued_modules = HashSet::new();
        let mut pending_modules = root_modules.iter().copied().collect::<VecDeque<_>>();

        // walk root modules and their bundled reachable closure
        while let Some(module_id) = pending_modules.pop_front() {
            if !queued_modules.insert(module_id) {
                continue;
            }

            self.ensure_profile_for_target(module_id)?;
            let module = self.module(module_id)?;
            let profile_id = self.profile_id_for_module(module_id)?;

            // resource modules link directly from patched module state
            if !module.is_code() {
                dependencies.require(ArtifactKey::dir_checked(module_id, profile_id));
                required_modules.push(module_id);
                continue;
            }

            // code modules link from generated output and the checked dir
            let output_key = ArtifactKey::module_output(module_id, *self.target_id);
            dependencies.require(output_key);
            dependencies.require(ArtifactKey::dir_checked(module_id, profile_id));
            required_modules.push(module_id);

            // the bundle closure is revealed once the generated output is built
            match self.artifacts.module_output(module_id, *self.target_id) {
                Ok(_) => {}
                Err(ProviderError::Blocked { .. }) => continue,
                Err(error) => return Err(CompilerError::from(error)),
            }

            for key in self.bundled_static_dependency_exports(module_id)? {
                dependencies.require(key);
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
        let ModuleOutput::Js(script) = artifact.as_ref() else {
            return Ok(Vec::new());
        };
        let mut requirements = IndexSet::new();

        // collect bundled static dependency export tables
        for dependency in static_js_dependencies(&script.module) {
            if dependency.form == DependencyForm::Type {
                continue;
            }

            let should_bundle = self
                .should_bundle_js_dependency(
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

    /// Return the bundled internal JS dependencies for one generated module.
    fn bundled_script_dependency_modules(&self, module_id: ModuleId) -> LinkResult<Vec<ModuleId>> {
        let module = self.module(module_id)?;

        if !module.is_code() {
            return Ok(Vec::new());
        }

        let mut dependency_modules = self.bundled_static_js_modules(
            module_id,
            self.target,
            self.target_id,
            self.package_id,
        )?;
        let dynamic_dependency_modules = self.bundled_dynamic_js_modules(
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
            let mut declared = ArtifactDependencySet::default();
            self.plan_modules(&script_root_modules, &mut declared)?
        };
        let module_set = if script_module_id_set.is_empty() {
            super::ModuleSet::default()
        } else {
            self.build_js_module_set(&script_root_modules, &script_module_id_set)
                .map_err(CompilerError::from)?
        };
        let output_graph = if module_set.modules().is_empty() {
            OutputGraph::default_empty(self.target.assembly)
        } else {
            self.build_js_output_graph(&module_set)
                .map_err(CompilerError::from)?
        };
        let asset_module_id_set =
            self.collect_asset_modules(&asset_root_modules, &script_module_id_set)?;
        let output_layout = self
            .build_output_layout(&output_graph)
            .map_err(CompilerError::from)?;
        let asset_reference_map = self
            .plan_asset_references(asset_module_id_set)
            .map_err(CompilerError::from)?;

        Ok(Plan::new(
            module_set,
            output_graph,
            output_layout,
            asset_reference_map,
        ))
    }
}
