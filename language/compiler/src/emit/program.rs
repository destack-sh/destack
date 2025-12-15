use crate::{Compiler, EmitError, EmitResult, TaskResultCollector};

use destack_source::PackageId;

impl Compiler {
    /// Emit all outputs for the entire program.
    pub(super) fn emit_program(&self, target_name: &str) -> EmitResult<()> {
        let mut collector = TaskResultCollector::new();

        // collect all packages that have this target
        let packages_with_target: Vec<PackageId> = self
            .program
            .packages
            .iter()
            .filter(|pkg| {
                let pkg = pkg.read();
                pkg.targets.contains_key(target_name)
            })
            .map(|pkg| pkg.read().id)
            .collect();

        // emit each package
        for package_id in packages_with_target {
            self.collect(&mut collector, self.emit_package(package_id, target_name));
        }

        // yield on any yield
        if let Some(dependency) = collector.try_into_yield_any() {
            return Err(EmitError::Yield { dependency });
        }

        Ok(())
    }
}
