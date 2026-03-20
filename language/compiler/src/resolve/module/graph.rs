use crate::{Compiler, ResolveResult};
use destack_dir::ModuleTarget;
use destack_source::{ModuleId, ModuleVersion};
use destack_workspace::{ArtifactKey, DirResolved, ModuleGraph, ProfileId};

impl Compiler {
    /// Build or load the module graph for one profile.
    pub(crate) fn process_module_graph(&self, profile_id: ProfileId) -> ResolveResult<()> {
        let artifact_key = ArtifactKey::module_graph(profile_id);

        // reuse one persisted module graph image when available
        if self
            .load_published_artifact(artifact_key.clone(), |compiler| {
                compiler.load_module_graph_image(profile_id)
            })
            .is_some()
        {
            return Ok(());
        }

        // start with an empty graph when nothing has been published yet
        if self.program.artifacts.module_graph(profile_id).is_none() {
            let graph = ModuleGraph::new(profile_id);
            self.program
                .artifacts
                .publish(artifact_key.clone(), graph.clone());
            self.store_artifact(&artifact_key, &graph, |compiler, graph| {
                compiler.store_module_graph_image(profile_id, graph)
            });
        }

        Ok(())
    }

    /// Update the module graph from one resolved DIR snapshot.
    pub(crate) fn update_module_graph_from_dir(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
        module_version: ModuleVersion,
        dir: &DirResolved,
    ) -> ResolveResult<()> {
        // collect module dependency targets from imports and namespace exports
        let mut targets = Vec::new();
        for targets_for_kind in dir.imported_modules.values() {
            if let Some(target) = targets_for_kind.value {
                targets.push(target);
            }
            if let Some(target) = targets_for_kind.ty {
                targets.push(target);
            }
        }
        for export in dir.namespace_exports.iter() {
            targets.push(export.module_id);
        }

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
        let artifact_key = ArtifactKey::module_graph(profile_id);
        let graph = self
            .program
            .artifacts
            .module_graph(profile_id)
            .map(|graph| graph.as_ref().clone())
            .unwrap_or_else(|| ModuleGraph::new(profile_id));
        let mut graph = graph;
        graph.update_module(module_id, module_version, dependencies);
        self.program
            .artifacts
            .publish(artifact_key.clone(), graph.clone());
        self.store_artifact(&artifact_key, &graph, |compiler, graph| {
            compiler.store_module_graph_image(profile_id, graph)
        });

        Ok(())
    }
}
