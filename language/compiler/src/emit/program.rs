use crate::{Compiler, EmitError, EmitOutput, EmitResult, TaskResultCollector};

use destack_source::PackageId;

impl Compiler {
    /// Emit all outputs for the entire program.
    pub(super) fn emit_program(&self, target_name: &str) -> EmitResult<EmitOutput> {
        let mut output = EmitOutput::default();
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
            let package_output =
                self.collect(&mut collector, self.emit_package(package_id, target_name));
            if let Some(package_output) = package_output {
                output.artifacts.extend(package_output.artifacts);
            }
        }

        // yield on any yield
        if let Some(dependency) = collector.try_into_yield_any() {
            return Err(EmitError::Yield { dependency });
        }

        Ok(output)
    }
}
