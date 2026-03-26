use crate::{Compiler, CompilerContext, ResolveResult};
use destack_artifact::{
    ArtifactKey, ArtifactStamp, DirResolved, Loader, ModuleEdge, ModuleEdgeRelation, ModuleGraph,
    ModuleKind,
};
use destack_core::StringId;
use destack_dir::{DependencyItem, ModuleTarget};
use destack_source::{ModuleId, ModuleVersion};
use destack_workspace::{Module, ProfileId};

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
        artifact_stamp: ArtifactStamp,
        dir: &DirResolved,
        context: &CompilerContext<'_>,
    ) -> ResolveResult<()> {
        let revision = context.revision();
        let module = context.module(module_id);
        let file = context.file(module.file_id);
        let module_kind = ModuleKind::from_file_type_and_loader(file.ty, module.loader);
        let module_version = ModuleVersion::new(artifact_stamp.0);
        let dependencies =
            self.collect_module_graph_edges(revision, module.as_ref(), profile_id, dir, context)?;

        // update the graph for this profile
        let artifact_key = ArtifactKey::module_graph(profile_id);
        let graph = self
            .module_graph(profile_id)
            .map(|graph| graph.as_ref().clone())
            .unwrap_or_else(|| ModuleGraph::new(profile_id));
        let mut graph = graph;
        graph.update_module(module_id, module_kind, module_version, dependencies);
        context.publish_artifact(artifact_key, graph.clone(), |store, version, payload| {
            store.publish_module_graph(version, payload)
        });
        context.store_artifact(&artifact_key, &graph, |compiler, _artifact_stamp, graph| {
            compiler.store_module_graph_image(revision, profile_id, graph)
        });

        Ok(())
    }

    /// Collect graph edges for one resolved module snapshot.
    fn collect_module_graph_edges(
        &self,
        revision: destack_workspace::Revision,
        module: &Module,
        profile_id: ProfileId,
        dir: &DirResolved,
        _context: &CompilerContext<'_>,
    ) -> ResolveResult<Vec<ModuleEdge>> {
        let mut dependencies = Vec::new();

        // import edges
        for ((_, specifier, relation, loader_override), targets_for_kind) in
            dir.imported_modules.iter()
        {
            let specifier = Some(*specifier);
            let loader = *loader_override;

            if let Some(target) = targets_for_kind.value {
                self.collect_module_graph_target_edges(
                    &mut dependencies,
                    revision,
                    module.id,
                    profile_id,
                    *relation,
                    specifier,
                    loader,
                    target,
                )?;
            }

            if let Some(target) = targets_for_kind.ty {
                self.collect_module_graph_target_edges(
                    &mut dependencies,
                    revision,
                    module.id,
                    profile_id,
                    *relation,
                    specifier,
                    loader,
                    target,
                )?;
            }
        }

        // namespace exports
        for export in dir.namespace_exports.iter() {
            let specifier = self.namespace_export_module_edge_specifier(dir, *export);
            self.collect_module_graph_target_edges(
                &mut dependencies,
                revision,
                module.id,
                profile_id,
                ModuleEdgeRelation::NamespaceExport,
                specifier,
                None,
                export.module_id,
            )?;
        }

        Ok(dependencies)
    }

    /// Collect graph edges from one module target.
    fn collect_module_graph_target_edges(
        &self,
        dependencies: &mut Vec<ModuleEdge>,
        revision: destack_workspace::Revision,
        module_id: ModuleId,
        profile_id: ProfileId,
        relation: ModuleEdgeRelation,
        specifier: Option<StringId>,
        loader: Option<Loader>,
        target: ModuleTarget,
    ) -> ResolveResult<()> {
        match target {
            ModuleTarget::Module(target_id) => {
                dependencies.push(
                    ModuleEdge::new(target_id, relation)
                        .with_specifier(specifier)
                        .with_loader(loader),
                );
            }
            ModuleTarget::Binding(binding_specifier) => {
                let bindings = self.module_bindings_for_specifier(
                    revision,
                    module_id,
                    profile_id,
                    binding_specifier,
                )?;
                let Some(bindings) = bindings else {
                    return Ok(());
                };

                for binding in bindings {
                    dependencies.push(
                        ModuleEdge::new(binding.module_id, relation)
                            .with_specifier(specifier)
                            .with_loader(loader),
                    );
                }
            }
            ModuleTarget::External(_) => {}
        }

        Ok(())
    }

    /// Return the authored specifier for one namespace export.
    fn namespace_export_module_edge_specifier(
        &self,
        dir: &DirResolved,
        export: destack_dir::NamespaceExport,
    ) -> Option<StringId> {
        let item = dir.tree.get(export.item);
        match item {
            DependencyItem::Remote { target, .. }
            | DependencyItem::UnresolvedRemote { target, .. } => Some(*target),
            _ => None,
        }
    }
}
