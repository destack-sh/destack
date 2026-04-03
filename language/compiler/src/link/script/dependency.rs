use crate::{Compiler, LinkError, LinkResult};

use destack_artifact::{
    DynamicScriptDependencyTarget, ModuleOutput, ScriptArtifact, ScriptDependencyKind,
    ScriptDependencyTarget,
};
use destack_source::{ModuleId, PackageId, Span, TargetId};
use destack_workspace::Target;
use indexmap::{IndexMap, IndexSet};

use super::{ScriptLinker, ScriptModuleSet, ScriptOutputGraph, ScriptOutputId, ScriptOutputNode};

/// The finalized dependencies for one output.
#[derive(Debug)]
pub(super) struct ScriptOutputDependencies {
    /// The outgoing static output dependencies.
    pub(super) static_output_dependencies: Vec<ScriptOutputId>,
    /// The outgoing dynamic output dependencies.
    pub(super) dynamic_output_dependencies: Vec<ScriptOutputId>,
    /// The retained external static imports.
    pub(super) external_imports: Vec<String>,
    /// The retained external dynamic imports.
    pub(super) external_dynamic_imports: Vec<String>,
}

/// The classified dependencies for one script module.
#[derive(Debug, Default)]
pub(super) struct ScriptModuleDependencies {
    /// The bundled static dependency modules.
    pub(super) static_modules: Vec<ModuleId>,
    /// The bundled dynamic dependency modules.
    pub(super) dynamic_modules: Vec<ModuleId>,
    /// The retained external static dependency specifiers.
    pub(super) external_imports: Vec<String>,
    /// The retained external dynamic dependency specifiers.
    pub(super) external_dynamic_imports: Vec<String>,
}

/// The output dependency collector for one output.
#[derive(Debug, Default)]
struct ScriptOutputDependencyCollector {
    /// The outgoing static output dependencies collected so far.
    static_output_dependencies: IndexSet<ScriptOutputId>,
    /// The outgoing dynamic output dependencies collected so far.
    dynamic_output_dependencies: IndexSet<ScriptOutputId>,
    /// The retained external static imports collected so far.
    external_imports: IndexSet<String>,
    /// The retained external dynamic imports collected so far.
    external_dynamic_imports: IndexSet<String>,
}

impl ScriptOutputDependencyCollector {
    /// Collect one set of internal module dependencies into output edges.
    fn collect_internal_dependencies(
        &mut self,
        dependency_modules: Vec<ModuleId>,
        output_id: ScriptOutputId,
        output_graph: &ScriptOutputGraph,
        is_static: bool,
    ) {
        for dependency_module in dependency_modules {
            let Some(dependency_output_id) = output_graph.output_id_for_module(dependency_module)
            else {
                continue;
            };

            if dependency_output_id == output_id {
                continue;
            }

            if is_static {
                self.static_output_dependencies.insert(dependency_output_id);
            } else {
                self.dynamic_output_dependencies
                    .insert(dependency_output_id);
            }
        }
    }

    /// Finish dependency collection and return stable vectors.
    fn finish(self) -> ScriptOutputDependencies {
        ScriptOutputDependencies {
            static_output_dependencies: self.static_output_dependencies.into_iter().collect(),
            dynamic_output_dependencies: self.dynamic_output_dependencies.into_iter().collect(),
            external_imports: self.external_imports.into_iter().collect(),
            external_dynamic_imports: self.external_dynamic_imports.into_iter().collect(),
        }
    }
}

impl<'a> ScriptLinker<'a> {
    /// Return whether target policy explicitly externalizes one dependency specifier.
    fn script_dependency_is_external(&self, target: &Target, specifier: &str) -> bool {
        let dependency = &target.bundle_dependencies;

        dependency
            .external
            .iter()
            .any(|candidate| candidate == specifier)
            || dependency
                .never_bundle
                .iter()
                .any(|candidate| candidate == specifier)
    }

    /// Return whether target policy explicitly bundles one dependency specifier.
    fn script_dependency_is_always_bundled(&self, target: &Target, specifier: &str) -> bool {
        target
            .bundle_dependencies
            .always_bundle
            .iter()
            .any(|candidate| candidate == specifier)
    }

    /// Return whether one dependency should remain bundled for this target.
    pub(super) fn should_bundle_script_dependency(
        &self,
        span: Span,
        package_id: PackageId,
        target_id: &TargetId,
        target: &Target,
        dependency_target: &ScriptDependencyTarget,
    ) -> LinkResult<bool> {
        let specifier = dependency_target.specifier();
        let has_resolved_module = dependency_target.module().is_some();
        let is_package_like = Compiler::is_package_like_dependency_specifier(specifier);
        let dependency = &target.bundle_dependencies;

        // explicit external policy
        if self.script_dependency_is_external(target, specifier) {
            return Ok(false);
        }

        // explicit inclusion policy
        if self.script_dependency_is_always_bundled(target, specifier) {
            return Ok(has_resolved_module);
        }

        // unresolved targets cannot be bundled
        if !has_resolved_module {
            return Ok(false);
        }

        // restrictive package allow list
        if !dependency.only_bundle.is_empty()
            && is_package_like
            && !dependency
                .only_bundle
                .iter()
                .any(|candidate| candidate == specifier)
        {
            return Err(LinkError::InvalidTarget {
                anchor: span.into(),
                package: package_id,
                target: *target_id,
                message: format!(
                    "bundle.dependencies.onlyBundle does not allow bundled dependency '{specifier}'"
                ),
            });
        }

        // local resolved modules still bundle by default
        if !is_package_like {
            return Ok(true);
        }

        // package allow list, when present, is authoritative
        if !dependency.only_bundle.is_empty() {
            return Ok(dependency
                .only_bundle
                .iter()
                .any(|candidate| candidate == specifier));
        }

        Ok(true)
    }

    /// Classify the dependencies for one script module.
    pub(super) fn classify_script_module_dependencies(
        &self,
        module_id: ModuleId,
        script: &ScriptArtifact,
        target: &Target,
        target_id: &TargetId,
        package_id: PackageId,
    ) -> LinkResult<ScriptModuleDependencies> {
        let mut dependencies = ScriptModuleDependencies::default();

        // only value edges produce runtime output imports
        for dependency in &script.linkage.static_dependencies {
            if dependency.kind == ScriptDependencyKind::Type {
                continue;
            }

            if self.should_bundle_script_dependency(
                self.module_anchor_span(module_id),
                package_id,
                target_id,
                target,
                &dependency.target,
            )? {
                let Some(target_module) = dependency.target.module() else {
                    continue;
                };

                dependencies.static_modules.push(target_module);
                continue;
            }
            dependencies
                .external_imports
                .push(dependency.target.specifier().to_string());
        }

        // bundled and external dynamic imports
        for dependency in &script.linkage.dynamic_dependencies {
            let DynamicScriptDependencyTarget::Resolved(dependency_target) = &dependency.target
            else {
                continue;
            };

            if self.should_bundle_script_dependency(
                self.module_anchor_span(module_id),
                package_id,
                target_id,
                target,
                dependency_target,
            )? {
                let Some(target_module) = dependency_target.module() else {
                    continue;
                };

                dependencies.dynamic_modules.push(target_module);
                continue;
            }

            dependencies
                .external_dynamic_imports
                .push(dependency_target.specifier().to_string());
        }

        Ok(dependencies)
    }

    /// Collect all outgoing dependencies and retained externals for one output.
    pub(super) fn collect_output_dependencies(
        &self,
        output_id: ScriptOutputId,
        output: &ScriptOutputNode,
        output_graph: &ScriptOutputGraph,
        target: &Target,
        target_id: &TargetId,
        package_id: PackageId,
    ) -> LinkResult<ScriptOutputDependencies> {
        let mut dependencies = ScriptOutputDependencyCollector::default();

        // aggregate every member module into one output-level dependency set
        for module_id in output.modules() {
            let artifact = self
                .compiler
                .module_output(*module_id, target_id)
                .ok_or_else(|| LinkError::Internal {
                    package: package_id,
                    message: format!(
                        "missing module artifact for module {:?} target '{}'",
                        module_id,
                        self.target_name()
                    ),
                })?;

            let ModuleOutput::Script(script) = artifact.as_ref() else {
                return Err(LinkError::Internal {
                    package: package_id,
                    message: format!(
                        "expected script artifact for module {:?} target '{}'",
                        module_id,
                        self.target_name()
                    ),
                });
            };
            let module_dependencies = self.classify_script_module_dependencies(
                *module_id,
                script.as_ref(),
                target,
                target_id,
                package_id,
            )?;

            dependencies.collect_internal_dependencies(
                module_dependencies.static_modules,
                output_id,
                output_graph,
                true,
            );
            dependencies.collect_internal_dependencies(
                module_dependencies.dynamic_modules,
                output_id,
                output_graph,
                false,
            );
            dependencies
                .external_imports
                .extend(module_dependencies.external_imports);
            dependencies
                .external_dynamic_imports
                .extend(module_dependencies.external_dynamic_imports);
        }

        Ok(dependencies.finish())
    }

    /// Build the dependent static entry sets for the current linked modules.
    pub(super) fn collect_script_static_entry_sets(
        &self,
        target: &Target,
        target_id: &TargetId,
        package_id: PackageId,
        module_set: &ScriptModuleSet,
    ) -> LinkResult<IndexMap<ModuleId, IndexSet<ModuleId>>> {
        let mut entry_sets: IndexMap<ModuleId, IndexSet<ModuleId>> = IndexMap::new();

        // walk bundled static edges from each entry root independently
        for entry_module in module_set.entry_modules() {
            let mut pending_modules = vec![*entry_module];
            let mut visited_modules = IndexSet::new();

            while let Some(module_id) = pending_modules.pop() {
                if !visited_modules.insert(module_id) {
                    continue;
                }

                entry_sets
                    .entry(module_id)
                    .or_default()
                    .insert(*entry_module);

                let artifact = self
                    .compiler
                    .module_output(module_id, target_id)
                    .ok_or_else(|| LinkError::Internal {
                        package: package_id,
                        message: format!(
                            "missing module artifact for module {:?} target '{}'",
                            module_id,
                            self.target_name()
                        ),
                    })?;

                let ModuleOutput::Script(script) = artifact.as_ref() else {
                    continue;
                };
                let dependencies = self.classify_script_module_dependencies(
                    module_id,
                    script.as_ref(),
                    target,
                    target_id,
                    package_id,
                )?;

                for dependency_module in dependencies.static_modules {
                    pending_modules.push(dependency_module);
                }
            }
        }

        Ok(entry_sets)
    }

    /// Collect the direct bundled modules reached through dynamic imports.
    pub(super) fn collect_script_dynamic_target_modules(
        &self,
        target: &Target,
        target_id: &TargetId,
        package_id: PackageId,
        module_set: &ScriptModuleSet,
    ) -> LinkResult<IndexSet<ModuleId>> {
        let mut dynamic_target_modules = IndexSet::new();

        // bundled dynamic imports become internal lazy boundaries
        for module_id in module_set.modules() {
            let artifact = self
                .compiler
                .module_output(*module_id, target_id)
                .ok_or_else(|| LinkError::Internal {
                    package: package_id,
                    message: format!(
                        "missing module artifact for module {:?} target '{}'",
                        module_id,
                        self.target_name()
                    ),
                })?;

            let ModuleOutput::Script(script) = artifact.as_ref() else {
                continue;
            };
            let dependencies = self.classify_script_module_dependencies(
                *module_id,
                script.as_ref(),
                target,
                target_id,
                package_id,
            )?;

            for target_module in dependencies.dynamic_modules {
                dynamic_target_modules.insert(target_module);
            }
        }

        Ok(dynamic_target_modules)
    }

    /// Collect the lazy-only bundled modules reached through dynamic imports.
    pub(super) fn collect_script_dynamic_entry_modules(
        &self,
        dynamic_target_modules: &IndexSet<ModuleId>,
        static_reachable_modules: &IndexSet<ModuleId>,
    ) -> IndexSet<ModuleId> {
        dynamic_target_modules
            .iter()
            .copied()
            .filter(|module_id| !static_reachable_modules.contains(module_id))
            .collect()
    }

    /// Build the dependent bundled dynamic target sets for linked modules.
    pub(super) fn collect_script_dynamic_target_sets(
        &self,
        target: &Target,
        target_id: &TargetId,
        package_id: PackageId,
        dynamic_target_modules: &IndexSet<ModuleId>,
    ) -> LinkResult<IndexMap<ModuleId, IndexSet<ModuleId>>> {
        let mut entry_sets: IndexMap<ModuleId, IndexSet<ModuleId>> = IndexMap::new();

        // walk bundled static edges from each dynamic target independently
        for dynamic_target_module in dynamic_target_modules {
            let mut pending_modules = vec![*dynamic_target_module];
            let mut visited_modules = IndexSet::new();

            while let Some(module_id) = pending_modules.pop() {
                if !visited_modules.insert(module_id) {
                    continue;
                }

                entry_sets
                    .entry(module_id)
                    .or_default()
                    .insert(*dynamic_target_module);

                let artifact = self
                    .compiler
                    .module_output(module_id, target_id)
                    .ok_or_else(|| LinkError::Internal {
                        package: package_id,
                        message: format!(
                            "missing module artifact for module {:?} target '{}'",
                            module_id,
                            self.target_name()
                        ),
                    })?;

                let ModuleOutput::Script(script) = artifact.as_ref() else {
                    continue;
                };
                let dependencies = self.classify_script_module_dependencies(
                    module_id,
                    script.as_ref(),
                    target,
                    target_id,
                    package_id,
                )?;

                for dependency_module in dependencies.static_modules {
                    pending_modules.push(dependency_module);
                }
            }
        }

        Ok(entry_sets)
    }
}
