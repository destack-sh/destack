use crate::{Compiler, LinkError, LinkResult, TaskResultCollector};

use destack_source::{ModuleId, PackageId};
use destack_workspace::{TargetDiscovery, TargetId};

impl Compiler {
    /// Link all modules for a target.
    pub(super) fn link_target(&self, package_id: PackageId, target_id: &TargetId) -> LinkResult<()> {
        // get package and target configuration
        let package = self.program.packages.get(package_id);
        let package = package.read();
        let package_path = package.path.clone();
        let target =
            package
                .targets
                .get(target_id)
                .cloned()
                .ok_or_else(|| LinkError::MissingTarget {
                    package: package_id,
                    target: target_id.clone(),
                })?;

        // find modules to generate based on discovery mode
        let modules: Vec<ModuleId> = match target.discovery {
            // resolve entry points to module IDs
            TargetDiscovery::Entry => {
                self.discover_entry_modules(package_id, &package_path, &target, target_id)?
            }
            // match all modules in package against include/exclude patterns
            TargetDiscovery::Include => {
                self.discover_include_modules(package_id, &package_path, &target)?
            }
        };

        // generate all discovered modules
        let mut collector = TaskResultCollector::new();
        for module_id in modules {
            let result = self.require_generate_module(module_id, target_id);
            collector.try_collect(result);
        }

        // yield if any dependencies are pending
        if let Some(dependency) = collector.try_into_yield_all() {
            return Err(LinkError::Yield { dependency });
        }

        // NOTE #Incomplete: single-file targets require combining artifacts
        if target.is_single_file() {
            return Err(LinkError::Internal {
                package: package_id,
                message: "single-file target linking not yet implemented".to_string(),
            });
        }

        Ok(())
    }
}
