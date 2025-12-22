use crate::{Compiler, EmitError, EmitResult, TaskResultCollector};

use destack_source::PackageId;
use destack_workspace::TargetId;

impl Compiler {
    /// Emit all outputs for the entire program.
    pub(super) fn emit_program(&self, target_id: &TargetId) -> EmitResult<()> {
        let mut collector = TaskResultCollector::new();

        // collect all packages that have this target (by name)
        let target_name = &target_id.name;
        let packages_with_target: Vec<PackageId> = self
            .program
            .packages
            .iter()
            .filter(|pkg| {
                let pkg = pkg.read();
                pkg.targets.keys().any(|t| &t.name == target_name)
            })
            .map(|pkg| pkg.read().id)
            .collect();

        // emit each package
        for package_id in packages_with_target {
            let pkg_target_id = TargetId::new(package_id, target_name);
            self.collect(&mut collector, self.emit_package(package_id, &pkg_target_id));
        }

        // yield on any yield
        if let Some(dependency) = collector.try_into_yield_any() {
            return Err(EmitError::Yield { dependency });
        }

        Ok(())
    }
}
