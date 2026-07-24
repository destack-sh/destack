use destack_artifact::{
    ArtifactDependencySet, ArtifactKey, ArtifactPayload, ArtifactVersion, ComponentGraph,
    InherentExtension,
};
use destack_dir as dir;
use destack_repository::{ArtifactReader, ModuleDelta, ProfileId, ProviderContext};
use destack_source::ModuleId;
use indexmap::{IndexMap, IndexSet};
use std::sync::Arc;

use crate::{Compiler, CompilerError, CompilerResult};

/// Predecessor component graph used to derive one new component graph.
struct ComponentGraphBase {
    /// The predecessor artifact version.
    version: ArtifactVersion,
    /// The predecessor component graph payload.
    graph: Arc<ComponentGraph>,
    /// Module changes since the predecessor revision.
    delta: ModuleDelta,
}

impl Compiler {
    /// Collect inputs for the component partition of one profile.
    pub(crate) fn collect_component_graph(
        &self,
        profile: ProfileId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactDependencySet> {
        let base = self.component_graph_base(context)?;

        let mut dependencies = ArtifactDependencySet::default();
        if let Some(base) = &base {
            dependencies.derive_from(base.version);
        }

        // inference edges read every module's resolution and exports
        let modules = self
            .repository
            .module_ids(context.revision())
            .map_err(|error| CompilerError::Internal {
                message: format!("failed to enumerate profile modules: {error}"),
            })?;
        for module in modules {
            dependencies.require(ArtifactKey::dir_resolved(module, profile));
            dependencies.require(ArtifactKey::dir_exported(module, profile));
        }

        Ok(dependencies)
    }

    /// Return the predecessor graph usable for one component graph build.
    fn component_graph_base(
        &self,
        context: &dyn ProviderContext,
    ) -> CompilerResult<Option<ComponentGraphBase>> {
        let Some(base) = context.artifact_base() else {
            return Ok(None);
        };

        let Some(graph) = self
            .repository
            .artifact_table()
            .component_graph(&base.version)
        else {
            return Ok(None);
        };

        let Some(delta) = self
            .repository
            .module_delta_between(context.revision(), base.revision)
            .map_err(|error| CompilerError::Internal {
                message: format!("failed to read changed modules: {error}"),
            })?
        else {
            return Ok(None);
        };

        Ok(Some(ComponentGraphBase {
            version: base.version,
            graph,
            delta,
        }))
    }

    /// Build the component partition for one profile.
    pub(crate) fn provide_component_graph(
        &self,
        profile: ProfileId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactPayload> {
        let artifacts = self.artifact_reader(context.revision());
        let started = self.repository.host().clock().now();
        let modules = self.repository.module_ids(context.revision())?;
        if let Some(started) = started {
            context.emit_span("modules", started);
        }
        context.emit_counter("modules", modules.len() as u64);

        // build or reuse the profile graph
        let base = self.component_graph_base(context)?;
        let graph = self.component_graph(&artifacts, profile, context, &modules, base)?;

        Ok(ArtifactPayload::ComponentGraph(graph))
    }

    /// Return the component graph for one profile.
    fn component_graph(
        &self,
        artifacts: &ArtifactReader<'_>,
        profile: ProfileId,
        context: &dyn ProviderContext,
        modules: &[ModuleId],
        base: Option<ComponentGraphBase>,
    ) -> CompilerResult<Arc<ComponentGraph>> {
        let started = self.repository.host().clock().now();
        let mut edge_count = 0u64;

        // collect inherent extensions before classifying their consumers
        let mut inherent = Vec::new();
        for module in modules.iter().copied() {
            self.collect_inherent_extensions(artifacts, profile, module, &mut inherent)?;
        }

        // classify each module's inference edge subset
        let mut inference_edges = IndexMap::with_capacity(modules.len());
        let mut is_inference_changed = false;
        for module in modules.iter().copied() {
            let edges =
                self.module_inference_edges(artifacts, profile, module, inherent.as_slice())?;
            if let Some(base) = &base {
                is_inference_changed |= !base.graph.inference_edges_equal(module, edges.as_ref());
            }
            inference_edges.insert(module, edges);
        }

        // build all edges when no predecessor graph is available
        let Some(base) = base else {
            let mut edges_by_module = IndexMap::with_capacity(modules.len());

            for module in modules.iter().copied() {
                let edges = self.module_edges(artifacts, profile, module)?;
                edge_count += edges.len() as u64;
                edges_by_module.insert(module, edges);
            }

            if let Some(started) = started {
                context.emit_span("edges", started);
            }
            context.emit_counter("edges", edge_count);

            validate_component_edges(&edges_by_module)?;
            let graph =
                ComponentGraph::from_edges(profile, edges_by_module, inference_edges, inherent);

            return Ok(Arc::new(graph));
        };

        let changed = base.delta.edge_modules().count();
        let reused = modules.len().saturating_sub(changed);

        context.emit_counter("changed_modules", changed as u64);
        context.emit_counter("added_modules", base.delta.added.len() as u64);
        context.emit_counter("removed_modules", base.delta.removed.len() as u64);
        context.emit_counter("reused_modules", reused as u64);

        let mut changed_edges = IndexMap::with_capacity(changed);
        let mut is_changed = base.delta.is_module_set_changed();

        // read only edges whose source module changed
        for module in base.delta.edge_modules() {
            let edges = self.module_edges(artifacts, profile, module)?;

            edge_count += edges.len() as u64;
            is_changed |= !base.graph.edges_equal(module, edges.as_ref());
            changed_edges.insert(module, edges);
        }

        if let Some(started) = started {
            context.emit_span("edges", started);
        }
        context.emit_counter("edges", edge_count);

        // return the predecessor graph when its inputs still match
        let is_inherent_changed = !base.graph.inherent_extensions_equal(&inherent);
        if !is_changed && !is_inference_changed && !is_inherent_changed {
            return Ok(base.graph);
        }

        let graph = base
            .graph
            .derive(changed_edges, base.delta.removed, inference_edges, inherent);
        validate_component_graph(&graph)?;

        Ok(Arc::new(graph))
    }

    /// Return the modules whose inference one module's checking consumes.
    fn module_inference_edges(
        &self,
        artifacts: &ArtifactReader<'_>,
        profile: ProfileId,
        module: ModuleId,
        inherent: &[InherentExtension],
    ) -> CompilerResult<Arc<[ModuleId]>> {
        let resolved = artifacts
            .dir_resolved(module, profile)
            .map_err(CompilerError::from)?;
        let mut edges = IndexSet::new();
        let mut referenced_symbols = IndexSet::new();

        // referenced symbols depend on inferred export forms
        for (_, reference) in &resolved.references.entries {
            match reference {
                dir::Reference::Bound(symbols) => {
                    for symbol in symbols {
                        referenced_symbols.insert(*symbol);
                        if symbol.module_id != module
                            && self.export_requires_inference(artifacts, profile, *symbol)?
                        {
                            edges.insert(symbol.module_id);
                        }
                    }
                }
                dir::Reference::Projected { base, .. } => {
                    referenced_symbols.insert(*base);
                    if base.module_id != module
                        && self.export_requires_inference(artifacts, profile, *base)?
                    {
                        edges.insert(base.module_id);
                    }
                }
                dir::Reference::Namespace(namespace) => {
                    if *namespace != module
                        && self.namespace_requires_inference(artifacts, profile, *namespace)?
                    {
                        edges.insert(*namespace);
                    }
                }
                dir::Reference::Ambiguous(_) | dir::Reference::Missing => {}
            }
        }

        // inferred inherent extensions add dependencies from target consumers
        for extension in inherent {
            if extension.symbol.module_id != module
                && referenced_symbols.contains(&extension.target)
                && self.export_requires_inference(artifacts, profile, extension.symbol)?
            {
                edges.insert(extension.symbol.module_id);
            }
        }

        let edges = edges.into_iter().collect::<Vec<_>>();

        Ok(Arc::from(edges))
    }

    /// Collect one module's exported extensions kept inherent by target ownership.
    fn collect_inherent_extensions(
        &self,
        artifacts: &ArtifactReader<'_>,
        profile: ProfileId,
        module: ModuleId,
        inherent: &mut Vec<InherentExtension>,
    ) -> CompilerResult<()> {
        let resolved = artifacts
            .dir_resolved(module, profile)
            .map_err(CompilerError::from)?;

        for (symbol, target) in resolved.extensions.targets() {
            // keep extensions of another package's target import-scoped
            if target.module_id.package_id != module.package_id {
                continue;
            }

            inherent.push(InherentExtension { symbol, target });
        }

        Ok(())
    }

    /// Return whether one exported symbol carries inference to its consumers.
    fn export_requires_inference(
        &self,
        artifacts: &ArtifactReader<'_>,
        profile: ProfileId,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<bool> {
        let exported = artifacts
            .dir_exported(symbol.module_id, profile)
            .map_err(CompilerError::from)?;

        // require every resolved cross-module symbol to carry one canonical form
        let form = exported
            .exports
            .symbol_form(symbol.local_id)
            .ok_or_else(|| CompilerError::Internal {
                message: format!(
                    "resolved cross-module symbol {symbol:?} has no local export form"
                ),
            })?;

        Ok(form.requires_inference())
    }

    /// Return whether one namespace object carries any inference.
    fn namespace_requires_inference(
        &self,
        artifacts: &ArtifactReader<'_>,
        profile: ProfileId,
        namespace: ModuleId,
    ) -> CompilerResult<bool> {
        let exported = artifacts
            .dir_exported(namespace, profile)
            .map_err(CompilerError::from)?;

        // any inferred local export requires namespace inference
        Ok(exported.exports.exports().any(|(_, export)| match export {
            dir::NamedExport::Local(local) => local.form.requires_inference(),
            dir::NamedExport::Indirect(_) => false,
        }))
    }

    /// Return the modules one module depends on, deduplicated in order.
    fn module_edges(
        &self,
        artifacts: &ArtifactReader<'_>,
        profile: ProfileId,
        module: ModuleId,
    ) -> CompilerResult<Arc<[ModuleId]>> {
        let resolved = artifacts
            .dir_resolved(module, profile)
            .map_err(CompilerError::from)?;

        // collect the defining modules of resolved targets
        let mut edges = IndexSet::new();
        for target in resolved.target_modules() {
            if target != module {
                edges.insert(target);
            }
        }

        let edges = edges.into_iter().collect::<Vec<_>>();

        Ok(Arc::from(edges))
    }
}
