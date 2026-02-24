use crate::{Compiler, ResolveError, ResolveResult};
use destack_dir::ModuleTarget;
use destack_source::{ModuleId, ModuleVersion, ProfileVersion};
use destack_workspace::{ModuleGraph, ModuleGraphKey, ProfileId};

impl Compiler {
    /// Update the module graph for a resolved module.
    pub(super) fn update_module_graph(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
        module_version: ModuleVersion,
        profile_version: ProfileVersion,
    ) -> ResolveResult<()> {
        // skip stale tasks
        self.ensure_module_profile_matches::<ResolveError>(
            module_id,
            module_version,
            profile_id,
            profile_version,
        )?;

        // load module data for dependency discovery
        let module = self.program.modules.get(module_id);
        let module = module.read();
        let Some(dir) = module.dir_maybe(profile_id) else {
            return Ok(());
        };

        // collect module dependency targets from imports and namespace exports
        let mut targets = Vec::new();
        for targets_for_kind in dir.imported_modules.read().values() {
            if let Some(target) = targets_for_kind.value {
                targets.push(target);
            }
            if let Some(target) = targets_for_kind.ty {
                targets.push(target);
            }
        }
        for export in dir.namespace_exports.read().iter() {
            targets.push(export.module_id);
        }

        // drop module guard before resolving binding dependencies
        drop(module);

        // resolve module binding targets into module ids
        let mut dependencies = Vec::new();
        for target in targets {
            match target {
                ModuleTarget::Module(target_id) => {
                    dependencies.push(target_id);
                }
                ModuleTarget::Binding(specifier) => {
                    let bindings =
                        self.module_bindings_for_specifier(module_id, profile_id, specifier)?;
                    let Some(bindings) = bindings else {
                        continue;
                    };
                    for binding in bindings {
                        dependencies.push(binding.module_id);
                    }
                }
            }
        }

        // update the graph for this profile
        let key = ModuleGraphKey::new(profile_id);
        let mut entry = self
            .program
            .index
            .module_graphs
            .entry(key)
            .or_insert_with(|| ModuleGraph::new(profile_id));
        entry.update_module(module_id, module_version, dependencies);

        Ok(())
    }
}
