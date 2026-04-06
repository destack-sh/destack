use std::collections::HashSet;

use crate::{LinkError, LinkResult, RequirementCollector};

use destack_artifact::{DynamicScriptDependencyTarget, ModuleOutput};
use destack_source::ModuleId;
use destack_workspace::BundleMode;
use indexmap::IndexSet;

use super::ScriptLinker;

/// The linked script module set for one target.
#[derive(Debug, Clone, Default)]
pub(crate) struct ScriptModuleSet {
    /// The discovered entry modules for this target.
    pub(super) entry_modules: Vec<ModuleId>,
    /// The ordered bundled modules included in the target.
    pub(super) modules: Vec<ModuleId>,
    /// The retained external static dependency targets.
    pub(super) external_targets: IndexSet<String>,
    /// The retained dynamic import targets.
    pub(super) dynamic_targets: IndexSet<String>,
    /// Whether the set contains dynamic imports without static targets.
    pub(super) has_opaque_dynamic_imports: bool,
}

impl ScriptModuleSet {
    /// Return the discovered entry modules for this module set.
    pub(crate) fn entry_modules(&self) -> &[ModuleId] {
        &self.entry_modules
    }

    /// Return the first discovered entry module when one exists.
    pub(crate) fn first_entry_module(&self) -> Option<ModuleId> {
        self.entry_modules.first().copied()
    }

    /// Return the ordered internal modules for this module set.
    pub(crate) fn modules(&self) -> &[ModuleId] {
        &self.modules
    }

    /// Return the retained external targets for this module set.
    pub(crate) fn external_targets(&self) -> indexmap::set::Iter<'_, String> {
        self.external_targets.iter()
    }

    /// Return the retained dynamic targets for this module set.
    pub(crate) fn dynamic_targets(&self) -> indexmap::set::Iter<'_, String> {
        self.dynamic_targets.iter()
    }
}

/// The dependency-order traversal state for one script module set.
#[derive(Debug)]
struct ModuleOrderBuilder {
    /// The modules included in the current ordering pass.
    included_modules: HashSet<ModuleId>,
    /// The modules on the active recursion stack.
    active_modules: HashSet<ModuleId>,
    /// The modules already ordered completely.
    finished_modules: HashSet<ModuleId>,
    /// The ordered result built so far.
    ordered_modules: Vec<ModuleId>,
}

impl ModuleOrderBuilder {
    /// Create one order builder for one module slice.
    fn new(module_ids: &[ModuleId]) -> Self {
        Self {
            included_modules: module_ids.iter().copied().collect(),
            active_modules: HashSet::new(),
            finished_modules: HashSet::new(),
            ordered_modules: Vec::with_capacity(module_ids.len()),
        }
    }

    /// Visit one module and append it after its included dependencies.
    fn visit(&mut self, linker: &ScriptLinker<'_>, module_id: ModuleId) -> LinkResult<()> {
        // skip modules that are already fully ordered
        if self.finished_modules.contains(&module_id) {
            return Ok(());
        }

        // tolerate cycles for now and keep their existing relative order
        if !self.active_modules.insert(module_id) {
            return Ok(());
        }

        let dependency_modules = linker.script_bundled_dependency_modules(module_id)?;

        // order included dependencies before the current module
        for dependency_module in dependency_modules {
            if self.included_modules.contains(&dependency_module) {
                self.visit(linker, dependency_module)?;
            }
        }

        self.active_modules.remove(&module_id);
        self.finished_modules.insert(module_id);
        self.ordered_modules.push(module_id);

        Ok(())
    }

    /// Finish the traversal and return the ordered modules.
    fn finish(self) -> Vec<ModuleId> {
        self.ordered_modules
    }
}

impl<'a> ScriptLinker<'a> {
    /// Build the script module set for one target.
    pub(crate) fn build_script_module_set(
        &self,
        entry_modules: &[ModuleId],
        module_ids: &[ModuleId],
    ) -> LinkResult<ScriptModuleSet> {
        let entry_modules = entry_modules.to_vec();
        let modules = self.order_script_module_set(module_ids)?;
        let mut module_set = ScriptModuleSet {
            entry_modules,
            modules,
            external_targets: IndexSet::new(),
            dynamic_targets: IndexSet::new(),
            has_opaque_dynamic_imports: false,
        };

        // collect retained external and dynamic edges
        for module_id in &module_set.modules {
            let artifact = self
                .compiler
                .module_output(*module_id, self.target_id)
                .ok_or_else(|| LinkError::Internal {
                    package: self.package_id,
                    message: format!(
                        "missing module artifact for module {:?} target '{}'",
                        module_id,
                        self.target_name()
                    ),
                })?;

            let ModuleOutput::Script(script) = artifact.as_ref() else {
                continue;
            };

            // retained static externals
            for dependency in &script.linkage.static_dependencies {
                if !self.should_bundle_script_dependency(
                    self.module_anchor_span(*module_id),
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
            for dependency in &script.linkage.dynamic_dependencies {
                match &dependency.target {
                    DynamicScriptDependencyTarget::Resolved(dependency_target) => {
                        let should_bundle = self.should_bundle_script_dependency(
                            self.module_anchor_span(*module_id),
                            self.package_id,
                            self.target_id,
                            self.target,
                            dependency_target,
                        )?;

                        // chunked outputs can retain internal dynamic edges as output links
                        if should_bundle {
                            if self.target.bundle.mode == BundleMode::Chunked {
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

                    // opaque dynamic imports are tracked for later validation
                    DynamicScriptDependencyTarget::Opaque => {
                        module_set.has_opaque_dynamic_imports = true;
                    }
                }
            }
        }

        Ok(module_set)
    }

    /// Require all generated script artifacts needed to link one target.
    pub(crate) fn require_module_outputs(
        &self,
        discovered_modules: &[ModuleId],
    ) -> LinkResult<Vec<ModuleId>> {
        let mut collector = RequirementCollector::new();
        let mut required_modules = Vec::new();
        let mut queued_modules = HashSet::new();
        let mut pending_modules = discovered_modules.to_vec();

        // require discovered modules and their bundled reachable closure
        while let Some(module_id) = pending_modules.pop() {
            if !queued_modules.insert(module_id) {
                continue;
            }

            let profile_id = self
                .context
                .profile_id_for_target(module_id, self.target_id)
                .ok_or_else(|| LinkError::Internal {
                    package: self.package_id,
                    message: format!("profile not found for target '{}'", self.target_name()),
                })?;
            let result = self.compiler.require_module_output(
                self.context.revision(),
                module_id,
                profile_id,
                self.target_id,
            );
            collector.try_collect(result);
            required_modules.push(module_id);

            // only traverse bundled dependencies after the generated artifact exists
            if self
                .compiler
                .module_output(module_id, self.target_id)
                .is_none()
            {
                continue;
            }

            for dependency_id in self.script_bundled_dependency_modules(module_id)? {
                if !queued_modules.contains(&dependency_id) {
                    pending_modules.push(dependency_id);
                }
            }
        }

        // yield while required generated artifacts are still pending
        if let Some(requirement) = collector.try_into_requirement() {
            return Err(LinkError::Yield { requirement });
        }

        required_modules.sort_unstable();
        required_modules.dedup();

        Ok(required_modules)
    }

    /// Return the modules in stable dependency order for one script module set.
    fn order_script_module_set(&self, module_ids: &[ModuleId]) -> LinkResult<Vec<ModuleId>> {
        let mut sorted_modules = module_ids.to_vec();
        sorted_modules.sort_unstable();
        let mut order_builder = ModuleOrderBuilder::new(&sorted_modules);

        // order each requested module after its included dependencies
        for module_id in sorted_modules {
            order_builder.visit(self, module_id)?;
        }

        Ok(order_builder.finish())
    }

    /// Return the bundled internal script dependencies for one generated module.
    fn script_bundled_dependency_modules(&self, module_id: ModuleId) -> LinkResult<Vec<ModuleId>> {
        let artifact = self
            .compiler
            .module_output(module_id, self.target_id)
            .ok_or_else(|| LinkError::Internal {
                package: self.package_id,
                message: format!(
                    "missing module artifact for module {:?} target '{}'",
                    module_id,
                    self.target_name()
                ),
            })?;
        let ModuleOutput::Script(script) = artifact.as_ref() else {
            return Ok(Vec::new());
        };

        // static imports and reexports
        let mut dependencies = Vec::new();
        for dependency in &script.linkage.static_dependencies {
            if !self.should_bundle_script_dependency(
                self.module_anchor_span(module_id),
                self.package_id,
                self.target_id,
                self.target,
                &dependency.target,
            )? {
                continue;
            }

            if let Some(module) = dependency.target.module() {
                dependencies.push(module);
            }
        }

        // dynamic imports
        for dependency in &script.linkage.dynamic_dependencies {
            let DynamicScriptDependencyTarget::Resolved(dependency_target) = &dependency.target
            else {
                continue;
            };

            if !self.should_bundle_script_dependency(
                self.module_anchor_span(module_id),
                self.package_id,
                self.target_id,
                self.target,
                dependency_target,
            )? {
                continue;
            }

            if let Some(module) = dependency_target.module() {
                dependencies.push(module);
            }
        }

        dependencies.sort_unstable();
        dependencies.dedup();

        Ok(dependencies)
    }
}
