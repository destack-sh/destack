use std::iter;
use std::sync::Arc;

use destack_artifact::{
    ArtifactDependencySet, ArtifactKey, ArtifactPayload, ArtifactSidecar, ComponentGraph,
    DirChecked, DirCheckedComponent, GlobalEnvironment,
};
use destack_repository::{ArtifactReader, ProfileId, ProviderContext, ProviderError};
use destack_source::{ComponentId, FileContent, ModuleId};
use indexmap::{IndexMap, IndexSet};

use crate::check::{AnnotatedSource, CheckComponentKey, CheckState};
use crate::{Compiler, CompilerError, CompilerResult};

impl Compiler {
    /// Collect inputs for checked DIR side tables of one component.
    ///
    /// The member and external inputs are derived from the component graph, so
    /// until it is built this returns the partial closure naming just the graph.
    pub(crate) fn collect_dir_checked_component(
        &self,
        entry: ModuleId,
        component_id: ComponentId,
        profile: ProfileId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactDependencySet> {
        let mut dependencies = ArtifactDependencySet::default();
        dependencies.require(ArtifactKey::component_graph(profile));

        // emit options come from the entry package config
        let entry_module = self.module(context.revision(), entry)?;
        self.observe_package_config(context, entry_module.package_id, &mut dependencies)?;

        // the component graph reveals the member and external inputs once built
        let artifacts = self.artifact_reader(context.revision());
        let graph = match artifacts.component_graph(profile) {
            Ok(graph) => graph,
            Err(ProviderError::Blocked { .. }) => {
                dependencies.mark_partial();

                return Ok(dependencies);
            }
            Err(error) => return Err(error.into()),
        };

        // the component's own resolved members
        for module in graph.members(component_id) {
            dependencies.require(ArtifactKey::dir_resolved(*module, profile));
        }

        // every transitive external component's checked inputs
        let external_components = external_components(&graph, component_id);
        self.collect_check_inputs(profile, &external_components, &mut dependencies);

        Ok(dependencies)
    }

    /// Build checked DIR side tables for one resolved component.
    pub(crate) fn provide_dir_checked_component(
        &self,
        entry: ModuleId,
        component_id: ComponentId,
        profile: ProfileId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactPayload> {
        let artifacts = self.artifact_reader(context.revision());
        let graph = self.read_component_graph(&artifacts, profile)?;

        // read the component's members and validate the key
        let modules = graph.members(component_id).to_vec();
        if modules.is_empty() || graph.entry(component_id) != Some(entry) {
            return Err(CompilerError::Internal {
                message: format!(
                    "checked component key entry={entry:?} component={component_id} does not \
                     match the component graph"
                ),
            });
        }

        // read the component's externals and global environment
        let external_components = external_components(&graph, component_id);
        let environment = self.read_check_environment(&artifacts, profile)?;
        let entry_module = self.module(context.revision(), entry)?;
        let options = self.workspace_compiler_options(context, entry_module.as_ref())?;

        // check the component
        let emit_events = options.emit_events || context.emit_events();
        let mut check = CheckState::new(
            self,
            context,
            &artifacts,
            profile,
            environment,
            external_components,
            emit_events,
        );
        check.load(modules.as_slice())?;
        check.walk()?;
        check.propagate()?;
        check.solve()?;

        // record state
        let stats = check.stats();
        context.emit_counter("variables", stats.variables as u64);
        context.emit_counter("constraints", stats.constraints as u64);
        context.emit_counter("types", stats.types as u64);
        context.emit_counter("bounds", stats.bounds as u64);
        context.emit_counter("decisions", stats.decisions as u64);
        if options.emit_stats {
            let content = stats.render_metadata();
            context.emit_sidecar(check_sidecar("metadata", content));
        }
        let events = emit_events.then(|| check.events());

        // reify solved annotations into rendered source sidecars
        if options.emit_checked_types {
            for source in check.render_annotated_sources()? {
                context.emit_sidecar(annotated_sidecar(source));
            }
        }

        // commit output DIR tables
        let (modules, diagnostics) = check.commit()?;
        context.emit_diagnostics(diagnostics);
        if let Some(events) = events {
            context.emit_sidecar(check_sidecar("events", events.render()));
        }

        Ok(ArtifactPayload::DirCheckedComponent(Arc::new(
            DirCheckedComponent {
                component: component_id,
                modules,
            },
        )))
    }

    /// Collect inputs for the checked DIR facade of one module.
    pub(crate) fn collect_dir_checked(
        &self,
        module: ModuleId,
        profile: ProfileId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactDependencySet> {
        let mut dependencies = ArtifactDependencySet::default();
        dependencies.require(ArtifactKey::component_graph(profile));

        // the owning component is revealed by the graph once it is built
        let artifacts = self.artifact_reader(context.revision());
        let graph = match artifacts.component_graph(profile) {
            Ok(graph) => graph,
            Err(ProviderError::Blocked { .. }) => {
                dependencies.mark_partial();

                return Ok(dependencies);
            }
            Err(error) => return Err(error.into()),
        };

        if let Some(component) = graph.component(module)
            && let Some(entry) = graph.entry(component)
        {
            dependencies.require(ArtifactKey::dir_checked_component(
                entry, component, profile,
            ));
        }

        Ok(dependencies)
    }

    /// Provide the checked DIR facade for one module.
    pub(crate) fn provide_dir_checked(
        &self,
        module: ModuleId,
        profile: ProfileId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactPayload> {
        let artifacts = self.artifact_reader(context.revision());
        let graph = self.read_component_graph(&artifacts, profile)?;

        // resolve the owning component behind the checked facade
        let component = graph
            .component(module)
            .ok_or_else(|| CompilerError::Internal {
                message: format!("module {module:?} is absent from the component graph"),
            })?;
        let entry = graph
            .entry(component)
            .ok_or_else(|| CompilerError::Internal {
                message: format!("component {component} has no entry module"),
            })?;

        Ok(ArtifactPayload::DirChecked(Arc::new(DirChecked {
            component,
            entry,
        })))
    }

    /// Read the component graph for one profile.
    fn read_component_graph(
        &self,
        artifacts: &ArtifactReader<'_>,
        profile: ProfileId,
    ) -> CompilerResult<Arc<ComponentGraph>> {
        artifacts
            .component_graph(profile)
            .map_err(CompilerError::from)
    }

    /// Declare every input artifact one component check reads.
    fn collect_check_inputs(
        &self,
        profile: ProfileId,
        external_components: &IndexMap<ModuleId, CheckComponentKey>,
        dependencies: &mut ArtifactDependencySet,
    ) {
        // each external module's expanded DIR
        for module in external_components.keys() {
            dependencies.require(ArtifactKey::dir_expanded(*module, profile));
        }

        // each external component's checked tables
        for dependency in external_components
            .values()
            .copied()
            .collect::<IndexSet<_>>()
        {
            dependencies.require(ArtifactKey::dir_checked_component(
                dependency.entry,
                dependency.component,
                profile,
            ));
        }

        dependencies.require(ArtifactKey::global_environment(profile));
    }

    /// Read the global environment one component check reads.
    fn read_check_environment(
        &self,
        artifacts: &ArtifactReader<'_>,
        profile: ProfileId,
    ) -> CompilerResult<Arc<GlobalEnvironment>> {
        artifacts
            .global_environment(profile)
            .map_err(CompilerError::from)
    }
}

/// Map every transitive external module to its checked component key.
/// Committed tables reference types across the whole dependency closure,
/// so every component reachable through the condensation is external.
fn external_components(
    graph: &ComponentGraph,
    component: ComponentId,
) -> IndexMap<ModuleId, CheckComponentKey> {
    let mut externals = IndexMap::new();
    let mut seen = IndexSet::new();
    let mut pending = graph.dependencies(component).to_vec();

    // walk the condensation forward from the checked component
    while let Some(dependency) = pending.pop() {
        if !seen.insert(dependency) {
            continue;
        }
        let Some(entry) = graph.entry(dependency) else {
            continue;
        };

        // bind every member of the external component to its key
        let key = CheckComponentKey {
            entry,
            component: dependency,
        };
        for module in graph.members(dependency) {
            externals.insert(*module, key);
        }
        pending.extend(graph.dependencies(dependency).iter().copied());
    }

    externals
}

/// Build one check-phase sidecar.
fn check_sidecar(name: &str, content: String) -> ArtifactSidecar {
    ArtifactSidecar::new(
        name,
        iter::once(("phase", "check")),
        FileContent::Text { content },
    )
}

/// Build one annotated source sidecar for one member module.
fn annotated_sidecar(source: AnnotatedSource) -> ArtifactSidecar {
    ArtifactSidecar::new(
        "annotated",
        [
            ("phase", "check".to_string()),
            ("module", source.module.uri.to_string()),
        ],
        FileContent::Text {
            content: source.content,
        },
    )
}
