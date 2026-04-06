use crate::{Compiler, CompilerContext, ResolveResult};
use destack_artifact::{ArtifactKey, ArtifactStamp, DirResolved, ModuleGraph};
use destack_dir::ModuleTarget;
use destack_source::ModuleId;
use destack_workspace::ProfileId;

impl Compiler {
    /// Build or load the module graph for one profile.
    pub(crate) fn process_module_graph(
        &self,
        profile_id: ProfileId,
        context: &CompilerContext<'_>,
    ) -> ResolveResult<()> {
        let revision = context.revision();
        let artifact_key = ArtifactKey::module_graph(profile_id);

        // reuse one persisted module graph image when available
        if context
            .restore_cached_artifact(
                artifact_key,
                |compiler| compiler.load_module_graph_image(revision, profile_id),
                |store, version, payload| store.publish_module_graph(version, payload),
            )
            .is_some()
        {
            return Ok(());
        }

        // start with an empty graph when nothing has been published yet
        if self.module_graph(profile_id).is_none() {
            let graph = ModuleGraph::new(profile_id);
            context.publish_artifact(artifact_key, graph.clone(), |store, version, payload| {
                store.publish_module_graph(version, payload)
            });
            context.store_artifact(&artifact_key, &graph, |compiler, _artifact_stamp, graph| {
                compiler.store_module_graph_image(revision, profile_id, graph)
            });
        }

        Ok(())
    }

    /// Update the module graph from one resolved DIR snapshot.
    pub(crate) fn update_module_graph_from_dir(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
        _artifact_stamp: ArtifactStamp,
        dir: &DirResolved,
        context: &CompilerContext<'_>,
    ) -> ResolveResult<()> {
        let revision = context.revision();
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
                    let bindings = self.module_bindings_for_specifier(
                        revision, module_id, profile_id, specifier,
                    )?;
                    let Some(bindings) = bindings else {
                        continue;
                    };
                    for binding in bindings {
                        dependencies.push(binding.module_id);
                    }
                }
                ModuleTarget::External(_) => {}
            }
        }

        // update the graph for this profile
        let artifact_key = ArtifactKey::module_graph(profile_id);
        let graph = self
            .module_graph(profile_id)
            .map(|graph| graph.as_ref().clone())
            .unwrap_or_else(|| ModuleGraph::new(profile_id));
        let mut graph = graph;
        graph.update_module(module_id, dependencies);
        context.publish_artifact(artifact_key, graph.clone(), |store, version, payload| {
            store.publish_module_graph(version, payload)
        });
        context.store_artifact(&artifact_key, &graph, |compiler, _artifact_stamp, graph| {
            compiler.store_module_graph_image(revision, profile_id, graph)
        });

        Ok(())
    }
}
