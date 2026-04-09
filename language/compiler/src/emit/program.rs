use crate::{Compiler, CompilerContext, RequirementCollector};

use destack_source::{PackageId, TargetId};

use super::EmitError;

impl Compiler {
    /// Emit all outputs for the entire program.
    pub fn emit_program(
        &self,
        target_id: &TargetId,
        context: &CompilerContext<'_>,
    ) -> Result<(), EmitError> {
        let mut collector = RequirementCollector::new();

        // collect all packages that have this target name
        let target_name = self
            .repository
            .effective_target(context.revision(), *target_id)
            .ok()
            .flatten()
            .map(|target| target.name)
            .unwrap_or_else(|| target_id.to_string());
        let mut packages_with_target = Vec::<PackageId>::new();

        for package_id in context.workspace_package_ids() {
            let package = context.package(package_id);

            let has_target_name = package
                .targets
                .values()
                .any(|target| target.name == target_name);
            if has_target_name {
                packages_with_target.push(package_id);
            }
        }

        // emit each package
        for package_id in packages_with_target {
            let pkg_target_id = self.repository.intern_target_id(package_id, &target_name);
            self.collect(
                &mut collector,
                self.emit_package(package_id, &pkg_target_id, context),
            );
        }

        // yield on any yield
        if let Some(requirement) = collector.try_into_requirement() {
            return Err(EmitError::Yield { requirement });
        }

        Ok(())
    }
}
