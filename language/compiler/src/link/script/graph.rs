use std::collections::HashSet;

use crate::{ArtifactRequirementCollector, Compiler, LinkError, LinkResult};

use destack_artifact::ModuleArtifact;
use destack_source::{ModuleId, PackageId};
use destack_workspace::{Target, TargetId};

impl Compiler {
    /// Require all generated script artifacts needed to link one target.
    pub(crate) fn require_script_target_artifacts(
        &self,
        discovered_modules: &[ModuleId],
        target: &Target,
        target_id: &TargetId,
        package_id: PackageId,
    ) -> LinkResult<Vec<ModuleId>> {
        let mut collector = ArtifactRequirementCollector::new();
        let mut required_modules = Vec::new();
        let mut queued_modules = HashSet::new();
        let mut pending_modules = discovered_modules.to_vec();

        // require discovered modules and their bundled reachable closure
        while let Some(module_id) = pending_modules.pop() {
            if !queued_modules.insert(module_id) {
                continue;
            }

            let profile_id = self
                .program
                .profile_id_for_target(module_id, target_id)
                .ok_or_else(|| LinkError::Internal {
                    package: package_id,
                    message: format!("profile not found for target '{}'", target_id.name),
                })?;
            let result = self.require_module_artifact(module_id, profile_id, target_id);
            collector.try_collect(result);
            required_modules.push(module_id);

            let Some(artifact) = self.artifacts.module_artifact(module_id, target_id) else {
                continue;
            };
            let ModuleArtifact::Script(_) = artifact.as_ref() else {
                continue;
            };

            for dependency_id in
                self.script_bundled_module_dependencies(module_id, target_id, package_id, target)?
            {
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

    /// Return the bundled static script dependencies for one generated module.
    fn script_bundled_module_dependencies(
        &self,
        module_id: ModuleId,
        target_id: &TargetId,
        package_id: PackageId,
        target: &Target,
    ) -> LinkResult<Vec<ModuleId>> {
        let artifact = self
            .artifacts
            .module_artifact(module_id, target_id)
            .ok_or_else(|| LinkError::Internal {
                package: package_id,
                message: format!(
                    "missing module artifact for module {:?} target '{}'",
                    module_id, target_id.name
                ),
            })?;

        let ModuleArtifact::Script(script) = artifact.as_ref() else {
            return Ok(Vec::new());
        };

        let mut dependencies = Vec::new();

        for dependency in &script.linkage.static_dependencies {
            if !self.should_bundle_script_dependency(
                self.module_anchor_span(module_id),
                package_id,
                target_id,
                target,
                &dependency.target,
            )? {
                continue;
            }

            if let Some(module) = dependency.target.module() {
                dependencies.push(module);
            }
        }

        dependencies.sort_unstable();
        dependencies.dedup();

        Ok(dependencies)
    }

    /// Return the modules in stable dependency order for script linking.
    pub(crate) fn order_script_modules_for_link(
        &self,
        module_ids: &[ModuleId],
        target: &Target,
        target_id: &TargetId,
        package_id: PackageId,
    ) -> LinkResult<Vec<ModuleId>> {
        let mut sorted_modules = module_ids.to_vec();
        sorted_modules.sort_unstable();

        let included_modules: HashSet<_> = sorted_modules.iter().copied().collect();
        let mut active_modules = HashSet::new();
        let mut finished_modules = HashSet::new();
        let mut ordered_modules = Vec::new();

        // order each requested module after its included dependencies
        for module_id in sorted_modules {
            self.visit_script_module_for_link(
                module_id,
                target,
                target_id,
                package_id,
                &included_modules,
                &mut active_modules,
                &mut finished_modules,
                &mut ordered_modules,
            )?;
        }

        Ok(ordered_modules)
    }

    /// Return the full reachable script module set for one target.
    pub(crate) fn collect_script_modules_for_link(
        &self,
        entry_modules: &[ModuleId],
        target: &Target,
        target_id: &TargetId,
        package_id: PackageId,
    ) -> LinkResult<Vec<ModuleId>> {
        let mut discovered_modules = HashSet::new();
        let mut pending_modules = entry_modules.to_vec();

        // walk the static script dependency graph from all entry roots
        while let Some(module_id) = pending_modules.pop() {
            if !discovered_modules.insert(module_id) {
                continue;
            }

            for dependency_id in
                self.script_bundled_module_dependencies(module_id, target_id, package_id, target)?
            {
                pending_modules.push(dependency_id);
            }
        }

        let mut discovered_modules = discovered_modules.into_iter().collect::<Vec<_>>();
        discovered_modules.sort_unstable();

        Ok(discovered_modules)
    }

    /// Visit one module during script link ordering.
    fn visit_script_module_for_link(
        &self,
        module_id: ModuleId,
        target: &Target,
        target_id: &TargetId,
        package_id: PackageId,
        included_modules: &HashSet<ModuleId>,
        active_modules: &mut HashSet<ModuleId>,
        finished_modules: &mut HashSet<ModuleId>,
        ordered_modules: &mut Vec<ModuleId>,
    ) -> LinkResult<()> {
        // skip modules that are already fully ordered
        if finished_modules.contains(&module_id) {
            return Ok(());
        }

        // tolerate cycles for now and keep their existing relative order
        if !active_modules.insert(module_id) {
            return Ok(());
        }

        let dependencies =
            self.script_bundled_module_dependencies(module_id, target_id, package_id, target)?;

        // order included dependencies before the current module
        for dependency_id in dependencies {
            if included_modules.contains(&dependency_id) {
                self.visit_script_module_for_link(
                    dependency_id,
                    target,
                    target_id,
                    package_id,
                    included_modules,
                    active_modules,
                    finished_modules,
                    ordered_modules,
                )?;
            }
        }

        active_modules.remove(&module_id);
        finished_modules.insert(module_id);
        ordered_modules.push(module_id);

        Ok(())
    }
}
