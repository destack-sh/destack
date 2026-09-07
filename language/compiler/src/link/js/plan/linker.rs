use std::collections::{HashSet, VecDeque};

use destack_artifact::{ArtifactDependencySet, ArtifactKey, Script};
use destack_repository::ProviderError;
use destack_source::ModuleId;
use indexmap::IndexSet;

use crate::{CompilerError, CompilerResult, LinkResult};

use super::super::{JsLinker, static_js_dependencies};
use super::{ModuleSet, OutputGraph, Plan};

impl<'a> JsLinker<'a> {
    /// Build the JS module set for one target.
    pub(crate) fn build_js_module_set(
        &self,
        entry_modules: &[ModuleId],
        module_ids: &[ModuleId],
    ) -> LinkResult<ModuleSet> {
        let entry_modules = entry_modules.to_vec();
        let included_modules = module_ids.to_vec();
        let modules = self.order_script_modules(&included_modules)?;
        let mut module_set = ModuleSet {
            entry_modules,
            modules,
            external_targets: IndexSet::new(),
        };

        // collect retained external edges
        self.collect_retained_script_targets(&mut module_set)?;

        Ok(module_set)
    }

    /// Order the included modules after their bundled dependencies.
    fn order_script_modules(&self, module_ids: &[ModuleId]) -> LinkResult<Vec<ModuleId>> {
        let included_modules = module_ids.iter().copied().collect::<HashSet<_>>();
        let mut active_modules = HashSet::new();
        let mut finished_modules = HashSet::new();
        let mut ordered_modules = Vec::with_capacity(module_ids.len());

        // walk each requested module
        for module_id in module_ids {
            self.visit_ordered_module(
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
        &self,
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
        let dependency_modules = self.bundled_script_dependency_modules(module_id)?;
        for dependency_module in dependency_modules {
            if !included_modules.contains(&dependency_module) {
                continue;
            }

            self.visit_ordered_module(
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

    /// Collect the retained external JS targets.
    fn collect_retained_script_targets(&self, module_set: &mut ModuleSet) -> LinkResult<()> {
        for module_id in &module_set.modules {
            let module = self.module(*module_id)?;

            if !module.is_code() {
                continue;
            }

            let script = self.script(*module_id)?;

            // retained static externals
            for dependency in static_js_dependencies(&script) {
                if !self.should_bundle_js_dependency(*module_id, &dependency)? {
                    module_set
                        .external_targets
                        .insert(dependency.specifier().to_string());
                }
            }
        }

        Ok(())
    }

    /// Declare every artifact needed to link one target's JS closure.
    ///
    /// The bundled traversal only widens through modules whose emitted
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

            let module = self.module(module_id)?;
            let profile_id = self.profile_id()?;

            // resource modules link from materialized DIR and parsed values
            if !module.is_code() {
                dependencies.require(ArtifactKey::dir_parsed(module_id));
                dependencies.require(ArtifactKey::dir_bound(module_id, profile_id));
                dependencies.require(ArtifactKey::dir_expanded(module_id, profile_id));
                dependencies.require(ArtifactKey::dir_materialized(module_id, profile_id));
                if module.loader.is_data() {
                    dependencies.require(ArtifactKey::data(module_id));
                }
                required_modules.push(module_id);
                continue;
            }

            // code modules link from emitted output and resolved DIR
            let output_key = ArtifactKey::script(module_id, *self.target_id);
            dependencies.require(output_key);
            dependencies.require(ArtifactKey::dir_bound(module_id, profile_id));
            dependencies.require(ArtifactKey::dir_expanded(module_id, profile_id));
            dependencies.require(ArtifactKey::dir_resolved(module_id, profile_id));
            required_modules.push(module_id);

            // the bundle closure is revealed once the emitted output is built
            match self.artifacts.read::<Script>((module_id, *self.target_id)) {
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

        let profile_id = self.profile_id()?;
        let script = self.script(module_id)?;
        let mut requirements = IndexSet::new();

        // collect bundled static dependency export tables
        for dependency in static_js_dependencies(&script) {
            let should_bundle = self
                .should_bundle_js_dependency(module_id, &dependency)
                .map_err(CompilerError::from)?;
            if !should_bundle {
                continue;
            }

            let Some(target_module) = dependency.module() else {
                continue;
            };

            requirements.insert(ArtifactKey::dir_bound(target_module, profile_id));
            requirements.insert(ArtifactKey::dir_expanded(target_module, profile_id));
            requirements.insert(ArtifactKey::dir_exported(target_module, profile_id));
        }

        Ok(requirements.into_iter().collect())
    }

    /// Return the bundled internal JS dependencies for one emitted module.
    fn bundled_script_dependency_modules(&self, module_id: ModuleId) -> LinkResult<Vec<ModuleId>> {
        let module = self.module(module_id)?;

        if !module.is_code() {
            return Ok(Vec::new());
        }

        self.bundled_static_js_modules(module_id)
    }

    /// Build the output plan for this target.
    pub(in super::super) fn plan(&self, root_modules: &[ModuleId]) -> CompilerResult<Plan> {
        let script_root_modules = root_modules.to_vec();

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
            OutputGraph::default_empty(self.target.js.mode)
        } else {
            self.build_js_output_graph(&module_set)
                .map_err(CompilerError::from)?
        };
        let asset_module_id_set = self.collect_file_modules(&script_module_id_set)?;
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
