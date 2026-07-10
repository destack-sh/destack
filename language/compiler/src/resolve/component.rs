use destack_artifact::{
    ArtifactDependencySet, ArtifactKey, ArtifactPayload, ArtifactVersion, ComponentGraph,
};
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

        // read only edges whose source module changed
        if let Some(base) = base {
            for module in base.delta.edge_modules() {
                dependencies.require(ArtifactKey::dir_resolved(module, profile));
            }
        } else {
            // collect every edge when no predecessor graph is usable
            let modules = self
                .repository
                .module_ids(context.revision())
                .map_err(|error| CompilerError::Internal {
                    message: format!("failed to enumerate profile modules: {error}"),
                })?;

            for module in modules {
                dependencies.require(ArtifactKey::dir_resolved(module, profile));
            }
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
        let modules = self
            .repository
            .module_ids(context.revision())
            .map_err(|error| CompilerError::Internal {
                message: format!("failed to enumerate profile modules: {error}"),
            })?;
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
            let graph = ComponentGraph::from_edges(profile, edges_by_module);

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

        // return the predecessor graph when changed edges still match
        if !is_changed {
            return Ok(base.graph);
        }

        let graph = base.graph.derive(changed_edges, base.delta.removed);
        validate_component_graph(&graph)?;

        Ok(Arc::new(graph))
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

        // collect the defining modules of resolved imports and references
        let mut edges = IndexSet::new();
        let targets = resolved
            .imports
            .target_modules()
            .chain(resolved.references.target_modules());
        for target in targets {
            if target != module {
                edges.insert(target);
            }
        }

        let edges = edges.into_iter().collect::<Vec<_>>();

        Ok(Arc::from(edges))
    }
}

/// Validate that component graph edges stay inside the graph module set.
fn validate_component_edges(edges: &IndexMap<ModuleId, Arc<[ModuleId]>>) -> CompilerResult<()> {
    for (module, targets) in edges {
        // reject edges outside the declared module universe
        for target in targets.iter() {
            if edges.contains_key(target) {
                continue;
            }

            return Err(CompilerError::Internal {
                message: format!(
                    "component graph edge from {module:?} points outside the graph to {target:?}"
                ),
            });
        }
    }

    Ok(())
}

/// Validate that component graph edges stay inside the graph module set.
fn validate_component_graph(graph: &ComponentGraph) -> CompilerResult<()> {
    for (module, targets) in graph.iter_edges() {
        // reject edges outside the declared module universe
        for target in targets.iter().copied() {
            if graph.contains_module(target) {
                continue;
            }

            return Err(CompilerError::Internal {
                message: format!(
                    "component graph edge from {module:?} points outside the graph to {target:?}"
                ),
            });
        }
    }

    Ok(())
}
