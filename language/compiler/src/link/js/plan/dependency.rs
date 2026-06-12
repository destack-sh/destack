use std::collections::VecDeque;

use crate::generate::js::DependencyForm;
use destack_artifact::{JsOutput, ModuleOutput};
use destack_repository::Target;
use destack_source::{ModuleId, PackageId, Span, TargetId};
use indexmap::{IndexMap, IndexSet};

use crate::{CompilerError, LinkError, LinkResult};

use super::super::{JsDependencyTarget, JsLinker, dynamic_js_dependencies, static_js_dependencies};
use super::ModuleSet;

impl JsLinker<'_> {
    /// Return one linked JS output when the module participates in runtime linking.
    fn linked_js_output(
        &self,
        module_id: ModuleId,
        target_id: &TargetId,
        package_id: PackageId,
    ) -> LinkResult<Option<JsOutput>> {
        let artifact = self
            .artifacts
            .module_output(module_id, *target_id)
            .map_err(CompilerError::from)
            .map_err(|error| LinkError::Internal {
                anchor: (package_id).into(),
                package: package_id,
                message: format!(
                    "missing module output for module {:?} target '{}': {error:?}",
                    module_id, target_id
                ),
            })?;
        let ModuleOutput::Js(script) = artifact.as_ref() else {
            return Err(LinkError::Internal {
                anchor: (package_id).into(),
                package: package_id,
                message: format!(
                    "expected JS output for module {:?} target '{}'",
                    module_id, target_id
                ),
            });
        };

        Ok(Some(script.as_ref().clone()))
    }

    /// Return whether target policy explicitly externalizes one dependency specifier.
    fn js_dependency_is_external(&self, target: &Target, specifier: &str) -> bool {
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
    fn js_dependency_is_always_bundled(&self, target: &Target, specifier: &str) -> bool {
        target
            .bundle_dependencies
            .always_bundle
            .iter()
            .any(|candidate| candidate == specifier)
    }

    /// Return whether one dependency specifier names a package import.
    fn is_package_like_dependency_specifier(specifier: &str) -> bool {
        !specifier.starts_with('.')
            && !specifier.starts_with('/')
            && !specifier.contains(':')
            && !specifier.is_empty()
    }

    /// Return whether one dependency should remain bundled for this target.
    pub(in crate::link::js) fn should_bundle_js_dependency(
        &self,
        span: Span,
        package_id: PackageId,
        target_id: &TargetId,
        target: &Target,
        dependency_target: &JsDependencyTarget,
    ) -> LinkResult<bool> {
        let specifier = dependency_target.specifier();
        let has_resolved_module = dependency_target.module().is_some();
        let is_package_like = Self::is_package_like_dependency_specifier(specifier);
        let dependency = &target.bundle_dependencies;

        // explicit external policy
        if self.js_dependency_is_external(target, specifier) {
            return Ok(false);
        }

        // explicit inclusion policy
        if self.js_dependency_is_always_bundled(target, specifier) {
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
                target: target_id.clone(),
                message: format!(
                    "dependencies.onlyBundle does not allow bundled dependency '{specifier}'"
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

    /// Collect the bundled static dependency modules for one JS module.
    pub(super) fn bundled_static_js_modules(
        &self,
        module_id: ModuleId,
        target: &Target,
        target_id: &TargetId,
        package_id: PackageId,
    ) -> LinkResult<Vec<ModuleId>> {
        let Some(script) = self.linked_js_output(module_id, target_id, package_id)? else {
            return Ok(Vec::new());
        };
        let mut dependency_modules = Vec::new();

        // bundled static imports
        for dependency in static_js_dependencies(&script.module) {
            if dependency.form == DependencyForm::Type {
                continue;
            }

            let module = self.module(module_id)?;

            if !self.should_bundle_js_dependency(
                Span::empty(module.file_id),
                package_id,
                target_id,
                target,
                &dependency.target,
            )? {
                continue;
            }

            let Some(target_module) = dependency.target.module() else {
                continue;
            };

            dependency_modules.push(target_module);
        }

        Ok(dependency_modules)
    }

    /// Collect the bundled dynamic dependency modules for one JS module.
    pub(super) fn bundled_dynamic_js_modules(
        &self,
        module_id: ModuleId,
        target: &Target,
        target_id: &TargetId,
        package_id: PackageId,
    ) -> LinkResult<Vec<ModuleId>> {
        let Some(script) = self.linked_js_output(module_id, target_id, package_id)? else {
            return Ok(Vec::new());
        };
        let mut dependency_modules = Vec::new();

        // bundled dynamic imports
        for dependency in dynamic_js_dependencies(&script.module) {
            let Some(dependency_target) = &dependency.target else {
                continue;
            };

            let module = self.module(module_id)?;

            if !self.should_bundle_js_dependency(
                Span::empty(module.file_id),
                package_id,
                target_id,
                target,
                dependency_target,
            )? {
                continue;
            }

            let Some(target_module) = dependency_target.module() else {
                continue;
            };

            dependency_modules.push(target_module);
        }

        Ok(dependency_modules)
    }

    /// Collect the retained external static imports for one JS module.
    pub(super) fn retained_static_js_imports(
        &self,
        module_id: ModuleId,
        target: &Target,
        target_id: &TargetId,
        package_id: PackageId,
    ) -> LinkResult<Vec<String>> {
        let Some(script) = self.linked_js_output(module_id, target_id, package_id)? else {
            return Ok(Vec::new());
        };
        let mut import_specifiers = Vec::new();

        // retained external static imports
        for dependency in static_js_dependencies(&script.module) {
            if dependency.form == DependencyForm::Type {
                continue;
            }

            let module = self.module(module_id)?;

            if self.should_bundle_js_dependency(
                Span::empty(module.file_id),
                package_id,
                target_id,
                target,
                &dependency.target,
            )? {
                continue;
            }

            import_specifiers.push(dependency.target.specifier().to_string());
        }

        Ok(import_specifiers)
    }

    /// Collect the retained external dynamic imports for one JS module.
    pub(super) fn retained_dynamic_js_imports(
        &self,
        module_id: ModuleId,
        target: &Target,
        target_id: &TargetId,
        package_id: PackageId,
    ) -> LinkResult<Vec<String>> {
        let Some(script) = self.linked_js_output(module_id, target_id, package_id)? else {
            return Ok(Vec::new());
        };
        let mut import_specifiers = Vec::new();

        // retained external dynamic imports
        for dependency in dynamic_js_dependencies(&script.module) {
            let Some(dependency_target) = &dependency.target else {
                continue;
            };

            let module = self.module(module_id)?;

            if self.should_bundle_js_dependency(
                Span::empty(module.file_id),
                package_id,
                target_id,
                target,
                dependency_target,
            )? {
                continue;
            }

            import_specifiers.push(dependency_target.specifier().to_string());
        }

        Ok(import_specifiers)
    }

    /// Build the dependent static entry sets for the current linked modules.
    pub(super) fn collect_script_static_entry_sets(
        &self,
        target: &Target,
        target_id: &TargetId,
        package_id: PackageId,
        module_set: &ModuleSet,
    ) -> LinkResult<IndexMap<ModuleId, IndexSet<ModuleId>>> {
        let mut entry_sets: IndexMap<ModuleId, IndexSet<ModuleId>> = IndexMap::new();

        // walk bundled static edges from each entry root independently
        for entry_module in module_set.entry_modules() {
            let mut pending_modules = VecDeque::from([*entry_module]);
            let mut visited_modules = IndexSet::new();

            while let Some(module_id) = pending_modules.pop_front() {
                if !visited_modules.insert(module_id) {
                    continue;
                }

                entry_sets
                    .entry(module_id)
                    .or_default()
                    .insert(*entry_module);

                let dependency_modules =
                    self.bundled_static_js_modules(module_id, target, target_id, package_id)?;

                for dependency_module in dependency_modules {
                    pending_modules.push_back(dependency_module);
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
        module_set: &ModuleSet,
    ) -> LinkResult<IndexSet<ModuleId>> {
        let mut dynamic_target_modules = IndexSet::new();

        // bundled dynamic imports become internal lazy boundaries
        for module_id in module_set.modules() {
            let dependency_modules =
                self.bundled_dynamic_js_modules(*module_id, target, target_id, package_id)?;

            for target_module in dependency_modules {
                dynamic_target_modules.insert(target_module);
            }
        }

        Ok(dynamic_target_modules)
    }

    /// Collect the lazy only bundled modules reached through dynamic imports.
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
            let mut pending_modules = VecDeque::from([*dynamic_target_module]);
            let mut visited_modules = IndexSet::new();

            while let Some(module_id) = pending_modules.pop_front() {
                if !visited_modules.insert(module_id) {
                    continue;
                }

                entry_sets
                    .entry(module_id)
                    .or_default()
                    .insert(*dynamic_target_module);

                let dependency_modules =
                    self.bundled_static_js_modules(module_id, target, target_id, package_id)?;

                for dependency_module in dependency_modules {
                    pending_modules.push_back(dependency_module);
                }
            }
        }

        Ok(entry_sets)
    }
}
