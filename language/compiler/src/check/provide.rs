use destack_core::{FxIndexMap, FxIndexSet};
use std::iter;
use std::sync::Arc;

use destack_artifact::{
    ArtifactDependencySet, ArtifactKey, ArtifactPayload, ArtifactProjectionKey, ArtifactSidecar,
    ComponentGraph, ComponentGraphProjection, DirChecked, DirCheckedComponent,
};
use destack_repository::{ProfileId, ProviderContext, ProviderError};
use destack_source::{ComponentId, Content, ModuleId};

use crate::check::{AnnotatedSource, CheckComponentKey, CheckState};
use crate::{Compiler, CompilerError, CompilerResult};

impl Compiler {
    /// Collect inputs for checked DIR side tables of one component.
    pub(crate) fn collect_dir_checked_component(
        &self,
        entry: ModuleId,
        component_id: ComponentId,
        profile: ProfileId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactDependencySet> {
        let mut dependencies = ArtifactDependencySet::default();
        let graph_key = ArtifactKey::component_graph(profile);
        dependencies.project(graph_key, ComponentGraphProjection::Members(component_id));
        dependencies.project(
            graph_key,
            ComponentGraphProjection::Dependencies(component_id),
        );

        // observe package config for check options
        let entry_module = self.module(context.revision(), entry)?;
        self.observe_package_config(context, entry_module.package_id, &mut dependencies)?;

        // resolve graph projections before declaring component inputs
        let artifacts = self.artifact_reader(context.revision());
        let graph = match artifacts.component_graph(profile) {
            Ok(graph) => graph,
            Err(ProviderError::Blocked { .. }) => {
                dependencies.mark_partial();

                return Ok(dependencies);
            }
            Err(error) => return Err(error.into()),
        };

        // require DIR payloads read for owned members
        for module in graph.members(component_id) {
            dependencies.require(ArtifactKey::dir_parsed(*module));
            dependencies.require(ArtifactKey::dir_bound(*module, profile));
            dependencies.require(ArtifactKey::dir_resolved(*module, profile));
            dependencies.require(ArtifactKey::dir_expanded(*module, profile));
        }

        // project external components reached by this component
        let external_components = external_components(&graph, component_id)?;
        for component in external_components.components.keys() {
            dependencies.project(graph_key, ComponentGraphProjection::Members(*component));
            dependencies.project(
                graph_key,
                ComponentGraphProjection::Dependencies(*component),
            );
        }
        self.require_component_check_dependencies(profile, &external_components, &mut dependencies);

        Ok(dependencies)
    }

    /// Provide checked DIR side tables for one resolved component.
    pub(crate) fn provide_dir_checked_component(
        &self,
        entry: ModuleId,
        component_id: ComponentId,
        profile: ProfileId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactPayload> {
        let artifacts = self.artifact_reader(context.revision());
        let graph = artifacts
            .component_graph(profile)
            .map_err(CompilerError::from)?;

        // validate the requested component key against the graph
        let modules = graph.members(component_id).to_vec();
        if modules.is_empty() || graph.entry(component_id) != Some(entry) {
            return Err(CompilerError::Internal {
                message: format!(
                    "checked component key entry={entry:?} component={component_id} does not \
                     match the component graph"
                ),
            });
        }

        // load context shared across the component check
        let external_components = external_components(&graph, component_id)?;
        let global = artifacts
            .global_environment(profile)
            .map_err(CompilerError::from)?;
        let environment = self.environment(context.revision())?;
        let entry_module = self.module(context.revision(), entry)?;
        let options = self.workspace_compiler_options(context, entry_module.as_ref())?;

        // check the component
        let emit_events = options.emit_events || context.emit_events();
        let mut check = CheckState::new(
            self,
            context,
            &artifacts,
            profile,
            global,
            environment,
            external_components.modules,
            emit_events,
        );
        check.load(modules.as_slice())?;
        check.walk()?;
        check.propagate_induced_parameters()?;
        check.check_decorators()?;
        check.check_bodies()?;
        check.settle()?;
        check.report_constant_conditions()?;

        // emit solver counters and optional trace sidecars
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

        // render checked type annotations when requested
        if options.emit_checked_types {
            for source in check.render_annotated_sources()? {
                context.emit_sidecar(annotated_sidecar(source));
            }
        }
        if let Some(events) = events {
            context.emit_sidecar(check_sidecar("events", events.render()));
        }

        // write checked DIR tables and diagnostics
        let (modules, diagnostics) = check.write()?;
        context.emit_diagnostics(diagnostics);

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
        let graph_key = ArtifactKey::component_graph(profile);
        dependencies.project(
            graph_key,
            ComponentGraphProjection::ComponentEntryOf(module),
        );

        // resolve the graph projection before declaring the component input
        let artifacts = self.artifact_reader(context.revision());
        let graph = match artifacts.component_graph(profile) {
            Ok(graph) => graph,
            Err(ProviderError::Blocked { .. }) => {
                dependencies.mark_partial();

                return Ok(dependencies);
            }
            Err(error) => return Err(error.into()),
        };

        // resolve the owning component behind the checked facade
        let (component, entry) =
            graph
                .component_entry(module)
                .ok_or_else(|| CompilerError::Internal {
                    message: format!("module {module:?} is absent from the component graph"),
                })?;

        dependencies.project(
            ArtifactKey::dir_checked_component(entry, component, profile),
            ArtifactProjectionKey::DirChecked(module),
        );

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
        let graph = artifacts
            .component_graph(profile)
            .map_err(CompilerError::from)?;

        // resolve the owning component behind the checked facade
        let (component, entry) =
            graph
                .component_entry(module)
                .ok_or_else(|| CompilerError::Internal {
                    message: format!("module {module:?} is absent from the component graph"),
                })?;

        Ok(ArtifactPayload::DirChecked(Arc::new(DirChecked {
            component,
            entry,
        })))
    }

    /// Require dependencies read by one component check.
    fn require_component_check_dependencies(
        &self,
        profile: ProfileId,
        external_components: &ExternalComponents,
        dependencies: &mut ArtifactDependencySet,
    ) {
        // require global tables shared by component checks
        dependencies.require(ArtifactKey::global_environment(profile));

        // require expanded DIR for external modules
        for module in external_components.modules.keys() {
            dependencies.require(ArtifactKey::dir_expanded(*module, profile));
        }

        // require checked tables for external components
        for (component, entry) in &external_components.components {
            dependencies.require(ArtifactKey::dir_checked_component(
                *entry, *component, profile,
            ));
        }
    }
}

/// Map every transitive external module to its checked component key.
fn external_components(
    graph: &ComponentGraph,
    component: ComponentId,
) -> CompilerResult<ExternalComponents> {
    let mut components = FxIndexMap::default();
    let mut modules = FxIndexMap::default();
    let mut seen = FxIndexSet::default();
    let mut pending = graph.dependencies(component).to_vec();

    // walk component dependencies transitively
    while let Some(dependency) = pending.pop() {
        if !seen.insert(dependency) {
            continue;
        }

        let entry = graph
            .entry(dependency)
            .ok_or_else(|| CompilerError::Internal {
                message: format!("component {dependency} has no entry module"),
            })?;
        components.insert(dependency, entry);

        // bind dependency members to their component check
        let key = CheckComponentKey {
            entry,
            component: dependency,
        };
        for module in graph.members(dependency) {
            modules.insert(*module, key);
        }
        pending.extend(graph.dependencies(dependency).iter().copied());
    }

    Ok(ExternalComponents {
        components,
        modules,
    })
}

/// External checked component inputs reached from one component.
struct ExternalComponents {
    /// The external component entries reached through the condensation.
    components: FxIndexMap<ComponentId, ModuleId>,
    /// The external modules keyed to their checked component.
    modules: FxIndexMap<ModuleId, CheckComponentKey>,
}

/// Return one check-phase sidecar.
fn check_sidecar(name: &str, content: String) -> ArtifactSidecar {
    ArtifactSidecar::new(
        name,
        iter::once(("phase", "check")),
        Content::Text { content },
    )
}

/// Return one annotated source sidecar for one member module.
fn annotated_sidecar(source: AnnotatedSource) -> ArtifactSidecar {
    ArtifactSidecar::new(
        "annotated",
        [
            ("phase", "check".to_string()),
            ("module", source.module.uri.to_string()),
        ],
        Content::Text {
            content: source.content,
        },
    )
}
