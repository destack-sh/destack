use crate::{Compiler, LinkError, LinkResult};

use destack_artifact::{DynamicScriptDependencyTarget, ModuleArtifact, ScriptDependencyTarget};
use destack_source::{ModuleId, PackageId, Span};
use destack_workspace::{Target, TargetId};
use indexmap::IndexSet;

/// The link plan for one script target.
#[derive(Debug, Clone, Default)]
pub(crate) struct ScriptLinkPlan {
    /// The discovered entry modules for this target.
    pub(super) entry_modules: Vec<ModuleId>,
    /// The ordered modules included in the target.
    pub(super) modules: Vec<ModuleId>,
    /// The external static dependency targets.
    pub(super) external_targets: IndexSet<String>,
    /// The statically known dynamic import targets.
    pub(super) dynamic_targets: IndexSet<String>,
    /// Whether the plan contains dynamic imports without static targets.
    pub(super) has_opaque_dynamic_imports: bool,
}

impl ScriptLinkPlan {
    /// Return the discovered entry modules for this plan.
    pub(crate) fn entry_modules(&self) -> &[ModuleId] {
        &self.entry_modules
    }

    /// Return the first discovered entry module when one exists.
    pub(crate) fn first_entry_module(&self) -> Option<ModuleId> {
        self.entry_modules.first().copied()
    }

    /// Return the ordered internal modules for this plan.
    pub(crate) fn modules(&self) -> &[ModuleId] {
        &self.modules
    }

    /// Return the retained external targets for this plan.
    pub(crate) fn external_targets(&self) -> indexmap::set::Iter<'_, String> {
        self.external_targets.iter()
    }

    /// Return the retained dynamic targets for this plan.
    pub(crate) fn dynamic_targets(&self) -> indexmap::set::Iter<'_, String> {
        self.dynamic_targets.iter()
    }
}

impl Compiler {
    /// Return whether one dependency should remain bundled for this target.
    pub(super) fn should_bundle_script_dependency(
        &self,
        span: Span,
        package_id: PackageId,
        target_id: &TargetId,
        target: &Target,
        dependency_target: &ScriptDependencyTarget,
    ) -> LinkResult<bool> {
        let dependency = &target.bundle.dependencies;
        let specifier = dependency_target.specifier();
        let has_resolved_module = dependency_target.module().is_some();

        // explicit external policy
        if dependency
            .external
            .iter()
            .any(|candidate| candidate == specifier)
            || dependency
                .never_bundle
                .iter()
                .any(|candidate| candidate == specifier)
        {
            return Ok(false);
        }

        // explicit inclusion policy
        if dependency
            .always_bundle
            .iter()
            .any(|candidate| candidate == specifier)
        {
            return Ok(has_resolved_module);
        }

        if !has_resolved_module {
            return Ok(false);
        }

        // restrictive allow list
        if !dependency.only_bundle.is_empty()
            && Self::is_package_dependency_specifier(specifier)
            && !dependency
                .only_bundle
                .iter()
                .any(|candidate| candidate == specifier)
        {
            return Err(LinkError::InvalidTarget {
                span,
                package: package_id,
                target: target_id.clone(),
                message: format!(
                    "bundle.dependencies.onlyBundle does not allow bundled dependency '{specifier}'"
                ),
            });
        }

        if !dependency.only_bundle.is_empty() && Self::is_package_dependency_specifier(specifier) {
            return Ok(dependency
                .only_bundle
                .iter()
                .any(|candidate| candidate == specifier));
        }

        Ok(true)
    }

    /// Return whether one script dependency is package-like instead of local.
    pub(super) fn is_package_dependency_specifier(specifier: &str) -> bool {
        !specifier.starts_with('.') && !specifier.starts_with('/') && !specifier.contains(':')
    }

    /// Build the link plan for one script target.
    pub(crate) fn plan_script_link(
        &self,
        entry_modules: &[ModuleId],
        target: &Target,
        target_id: &TargetId,
        package_id: PackageId,
    ) -> LinkResult<ScriptLinkPlan> {
        let entry_modules = entry_modules.to_vec();
        let discovered_modules =
            self.collect_script_modules_for_link(&entry_modules, target, target_id, package_id)?;
        let modules =
            self.order_script_modules_for_link(&discovered_modules, target, target_id, package_id)?;
        let mut plan = ScriptLinkPlan {
            entry_modules,
            modules,
            external_targets: IndexSet::new(),
            dynamic_targets: IndexSet::new(),
            has_opaque_dynamic_imports: false,
        };

        // collect external and dynamic edges from the generated script artifacts
        for module_id in &plan.modules {
            let artifact = self
                .artifacts
                .module_artifact(*module_id, target_id)
                .ok_or_else(|| LinkError::Internal {
                    package: package_id,
                    message: format!(
                        "missing module artifact for module {:?} target '{}'",
                        module_id, target_id.name
                    ),
                })?;

            let ModuleArtifact::Script(script) = artifact.as_ref() else {
                continue;
            };

            for dependency in &script.linkage.static_dependencies {
                if !self.should_bundle_script_dependency(
                    self.module_anchor_span(*module_id),
                    package_id,
                    target_id,
                    target,
                    &dependency.target,
                )? {
                    plan.external_targets
                        .insert(dependency.target.specifier().to_string());
                }
            }

            for dependency in &script.linkage.dynamic_dependencies {
                match &dependency.target {
                    DynamicScriptDependencyTarget::Resolved(dependency_target) => {
                        if !Self::is_package_dependency_specifier(dependency_target.specifier()) {
                            return Err(LinkError::InvalidTarget {
                                span: self.module_anchor_span(*module_id),
                                package: package_id,
                                target: target_id.clone(),
                                message: format!(
                                    "bundled dynamic import '{}' is not implemented yet",
                                    dependency_target.specifier()
                                ),
                            });
                        }

                        if self.should_bundle_script_dependency(
                            self.module_anchor_span(*module_id),
                            package_id,
                            target_id,
                            target,
                            dependency_target,
                        )? {
                            return Err(LinkError::InvalidTarget {
                                span: self.module_anchor_span(*module_id),
                                package: package_id,
                                target: target_id.clone(),
                                message: format!(
                                    "bundled dynamic import '{}' is not implemented yet",
                                    dependency_target.specifier()
                                ),
                            });
                        }

                        plan.dynamic_targets
                            .insert(dependency_target.specifier().to_string());
                    }
                    DynamicScriptDependencyTarget::Opaque => {
                        plan.has_opaque_dynamic_imports = true;
                    }
                }
            }
        }

        Ok(plan)
    }
}
