use destack_source::ModuleId;
use indexmap::IndexSet;

use crate::{LinkError, LinkResult};

use super::super::{JsDependencyTarget, JsLinker, static_js_dependencies};

impl JsLinker<'_> {
    /// Return whether target policy explicitly externalizes one dependency specifier.
    fn js_dependency_is_external(&self, specifier: &str) -> bool {
        let dependency = &self.target.js.dependencies;

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
    fn js_dependency_is_always_bundled(&self, specifier: &str) -> bool {
        self.target
            .js
            .dependencies
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
        module: ModuleId,
        dependency_target: &JsDependencyTarget,
    ) -> LinkResult<bool> {
        let specifier = dependency_target.specifier();
        let has_resolved_module = dependency_target.module().is_some();
        let is_package_like = Self::is_package_like_dependency_specifier(specifier);
        let dependency = &self.target.js.dependencies;

        // explicit external policy
        if self.js_dependency_is_external(specifier) {
            return Ok(false);
        }

        // explicit inclusion policy
        if self.js_dependency_is_always_bundled(specifier) {
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
                anchor: self.module_anchor_span(module)?.into(),
                package: self.package_id,
                target: *self.target_id,
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
    ) -> LinkResult<Vec<ModuleId>> {
        let script = self.script(module_id)?;
        let mut dependency_modules = IndexSet::new();

        // bundled static imports
        for dependency in static_js_dependencies(&script) {
            if !self.should_bundle_js_dependency(module_id, &dependency)? {
                continue;
            }

            let Some(target_module) = dependency.module() else {
                continue;
            };

            dependency_modules.insert(target_module);
        }

        Ok(dependency_modules.into_iter().collect())
    }

    /// Collect the retained external static imports for one JS module.
    pub(super) fn retained_static_js_imports(
        &self,
        module_id: ModuleId,
    ) -> LinkResult<Vec<String>> {
        let script = self.script(module_id)?;
        let mut import_specifiers = IndexSet::new();

        // retained external static imports
        for dependency in static_js_dependencies(&script) {
            if self.should_bundle_js_dependency(module_id, &dependency)? {
                continue;
            }

            import_specifiers.insert(dependency.specifier().to_string());
        }

        Ok(import_specifiers.into_iter().collect())
    }
}
