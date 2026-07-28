use destack_core::{FxIndexMap, FxIndexSet};
use std::iter;
use std::sync::Arc;

use destack_artifact::{
    ArtifactDependencySet, ArtifactKey, ArtifactPayload, ArtifactProjectionKey, ArtifactSidecar,
    ComponentGraph, ComponentGraphProjection, DirChecked, DirCheckedComponent,
    DirDeclaredComponent, GlobalEnvironment,
};
use destack_repository::{ArtifactReader, ProfileId, ProviderContext, ProviderError};
use destack_source::{ComponentId, Content, ModuleId};

use crate::check::{AnnotatedSource, CheckExternalComponent, CheckState};
use crate::{Compiler, CompilerError, CompilerResult};

impl Compiler {
    /// Collect inputs for one declared DIR reference component.
    pub(crate) fn collect_dir_declared_component(
        &self,
        component: ComponentId,
        profile: ProfileId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactDependencySet> {
        let mut dependencies = ArtifactDependencySet::default();
        let graph_key = ArtifactKey::component_graph(profile);
        dependencies.project(
            graph_key,
            ComponentGraphProjection::ReferenceMembers(component),
        );
        dependencies.project(
            graph_key,
            ComponentGraphProjection::ReferenceDependencies(component),
        );

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

        // observe package config for check options
        let entry = reference_entry(&graph, component)?;
        let entry_module = self.module(context.revision(), entry)?;
        self.observe_package_config(context, entry_module.package_id, &mut dependencies)?;

        // require DIR payloads read for owned members
        for module in graph.reference_members(component) {
            dependencies.require(ArtifactKey::dir_parsed(*module));
            dependencies.require(ArtifactKey::dir_bound(*module, profile));
            dependencies.require(ArtifactKey::dir_resolved(*module, profile));
            dependencies.require(ArtifactKey::dir_expanded(*module, profile));
        }

        // require and project the external components read by this check
        self.collect_external_components(
            &artifacts,
            &graph,
            component,
            profile,
            &mut dependencies,
        )?;

        Ok(dependencies)
    }

    /// Provide one declared DIR reference component.
    pub(crate) fn provide_dir_declared_component(
        &self,
        component: ComponentId,
        profile: ProfileId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactPayload> {
        let artifacts = self.artifact_reader(context.revision());
        let graph = artifacts
            .component_graph(profile)
            .map_err(CompilerError::from)?;

        // require the requested reference component
        let modules = graph.reference_members(component).to_vec();
        let entry = reference_entry(&graph, component)?;

        // load context shared across the declaration check
        let global = artifacts
            .global_environment(profile)
            .map_err(CompilerError::from)?;
        let external_components = self.external_components(&graph, component, &global)?;
        let environment = self.environment(context.revision())?;
        let entry_module = self.module(context.revision(), entry)?;
        let options = self.workspace_compiler_options(context, entry_module.as_ref())?;

        // check declarations and module state without walking callable bodies
        let mut check = CheckState::new(
            self,
            context,
            &artifacts,
            profile,
            global,
            environment,
            external_components.modules,
            external_components.inherent,
            FxIndexSet::default(),
            options.emit_events || context.emit_events(),
        );
        check.declare(modules.as_slice())?;

        // emit solver counters for the declaration pass
        let stats = check.stats();
        context.emit_counter("variables", stats.variables as u64);
        context.emit_counter("constraints", stats.constraints as u64);
        context.emit_counter("types", stats.types as u64);

        // seal every member into the declared component
        let modules = check.write_declared()?;

        Ok(ArtifactPayload::DirDeclaredComponent(Arc::new(
            DirDeclaredComponent { component, modules },
        )))
    }

    /// Collect inputs for checked DIR side tables of one inference component.
    pub(crate) fn collect_dir_checked_component(
        &self,
        component: ComponentId,
        profile: ProfileId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactDependencySet> {
        let mut dependencies = ArtifactDependencySet::default();
        let graph_key = ArtifactKey::component_graph(profile);
        dependencies.project(
            graph_key,
            ComponentGraphProjection::InferenceMembers(component),
        );
        dependencies.project(
            graph_key,
            ComponentGraphProjection::InferenceDependencies(component),
        );

        // resolve graph projections before collecting component inputs
        let artifacts = self.artifact_reader(context.revision());
        let graph = match artifacts.component_graph(profile) {
            Ok(graph) => graph,
            Err(ProviderError::Blocked { .. }) => {
                dependencies.mark_partial();

                return Ok(dependencies);
            }
            Err(error) => return Err(error.into()),
        };

        // resolve the enclosing reference component and package
        let entry = inference_entry(&graph, component)?;
        let reference =
            graph
                .reference_component(entry)
                .ok_or_else(|| CompilerError::Internal {
                    message: format!(
                        "inference component {component} entry {entry:?} has no reference component"
                    ),
                })?;
        dependencies.project(
            graph_key,
            ComponentGraphProjection::ReferenceMembers(reference),
        );
        let entry_module = self.module(context.revision(), entry)?;
        self.observe_package_config(context, entry_module.package_id, &mut dependencies)?;

        // require own members' DIR payloads
        for module in graph.inference_members(component) {
            dependencies.require(ArtifactKey::dir_parsed(*module));
            dependencies.require(ArtifactKey::dir_bound(*module, profile));
            dependencies.require(ArtifactKey::dir_resolved(*module, profile));
            dependencies.require(ArtifactKey::dir_expanded(*module, profile));
        }

        // require a shared declaration prefix only when inference splits the reference component
        let reference_members = graph.reference_members(reference);
        let inference_members = graph.inference_members(component);
        if inference_members.len() < reference_members.len() {
            let declared_key = ArtifactKey::dir_declared_component(reference, profile);
            let inference_members = inference_members.iter().copied().collect::<FxIndexSet<_>>();
            for module in reference_members {
                dependencies.project(
                    declared_key,
                    ArtifactProjectionKey::DirDeclaredModule(*module),
                );
                if !inference_members.contains(module) {
                    dependencies.require(ArtifactKey::dir_expanded(*module, profile));
                }
            }
        }

        // require checked tables for upstream inference components
        for upstream in graph.inference_dependencies(component) {
            dependencies.require(ArtifactKey::dir_checked_component(upstream, profile));
        }

        // require and project the external components read by this check
        self.collect_external_components(
            &artifacts,
            &graph,
            reference,
            profile,
            &mut dependencies,
        )?;

        Ok(dependencies)
    }

    /// Provide checked DIR side tables for one inference component.
    pub(crate) fn provide_dir_checked_component(
        &self,
        component: ComponentId,
        profile: ProfileId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactPayload> {
        let artifacts = self.artifact_reader(context.revision());
        let graph = artifacts
            .component_graph(profile)
            .map_err(CompilerError::from)?;

        // require the requested inference component
        let modules = graph.inference_members(component).to_vec();
        let entry = inference_entry(&graph, component)?;
        let reference =
            graph
                .reference_component(entry)
                .ok_or_else(|| CompilerError::Internal {
                    message: format!(
                        "inference component {component} entry {entry:?} has no reference component"
                    ),
                })?;
        let inference_modules = modules.iter().copied().collect::<FxIndexSet<_>>();

        // load the shared declaration prefix only when inference splits the reference component
        let declared = if modules.len() < graph.reference_members(reference).len() {
            Some(
                artifacts
                    .dir_declared_component(reference, profile)
                    .map_err(CompilerError::from)?,
            )
        } else {
            None
        };

        // map sibling members to their sealed artifacts
        let global = artifacts
            .global_environment(profile)
            .map_err(CompilerError::from)?;
        let external_components = self.external_components(&graph, reference, &global)?;
        let mut externals = external_components.modules;
        if declared.is_some() {
            let mut upstream_members = FxIndexMap::default();
            for upstream in graph.inference_dependencies(component) {
                for module in graph.inference_members(upstream) {
                    upstream_members.insert(*module, upstream);
                }
            }
            for module in graph.reference_members(reference) {
                if inference_modules.contains(module) {
                    continue;
                }
                let artifact = match upstream_members.get(module) {
                    Some(key) => CheckExternalComponent::Checked(*key),
                    None => CheckExternalComponent::Declared(reference),
                };
                externals.insert(*module, artifact);
            }
        }

        // load context shared across the component check
        let environment = self.environment(context.revision())?;
        let entry_module = self.module(context.revision(), entry)?;
        let options = self.workspace_compiler_options(context, entry_module.as_ref())?;

        // check the inference component over declared component state
        let emit_events = options.emit_events || context.emit_events();
        let mut check = CheckState::new(
            self,
            context,
            &artifacts,
            profile,
            global,
            environment,
            externals,
            external_components.inherent,
            inference_modules,
            emit_events,
        );
        check.check(modules.as_slice(), declared.as_deref())?;
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
        let (modules, diagnostics) = check.write_checked()?;
        context.emit_diagnostics(diagnostics);

        Ok(ArtifactPayload::DirCheckedComponent(Arc::new(
            DirCheckedComponent { component, modules },
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
            ComponentGraphProjection::InferenceComponent(module),
        );

        // resolve the graph projection before collecting the component input
        let artifacts = self.artifact_reader(context.revision());
        let graph = match artifacts.component_graph(profile) {
            Ok(graph) => graph,
            Err(ProviderError::Blocked { .. }) => {
                dependencies.mark_partial();

                return Ok(dependencies);
            }
            Err(error) => return Err(error.into()),
        };

        // resolve the owning inference component behind the checked facade
        let component =
            graph
                .inference_component(module)
                .ok_or_else(|| CompilerError::Internal {
                    message: format!("module {module:?} is absent from the component graph"),
                })?;

        dependencies.project(
            ArtifactKey::dir_checked_component(component, profile),
            ArtifactProjectionKey::DirCheckedModule(module),
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

        // resolve the owning inference component behind the checked facade
        let component =
            graph
                .inference_component(module)
                .ok_or_else(|| CompilerError::Internal {
                    message: format!("module {module:?} is absent from the component graph"),
                })?;

        Ok(ArtifactPayload::DirChecked(Arc::new(DirChecked {
            component,
        })))
    }

    /// Require and project the external components one component check reads.
    fn collect_external_components(
        &self,
        artifacts: &ArtifactReader<'_>,
        graph: &Arc<ComponentGraph>,
        component: ComponentId,
        profile: ProfileId,
        dependencies: &mut ArtifactDependencySet,
    ) -> CompilerResult<()> {
        let graph_key = ArtifactKey::component_graph(profile);
        dependencies.project(graph_key, ComponentGraphProjection::InherentExtensions);
        dependencies.require(ArtifactKey::global_environment(profile));

        // follow the global environment's implicit modules
        let global = match artifacts.global_environment(profile) {
            Ok(global) => global,
            Err(ProviderError::Blocked { .. }) => {
                dependencies.mark_partial();

                return Ok(());
            }
            Err(error) => return Err(error.into()),
        };

        // project the graph slices and require the tables behind each external
        let externals = self.external_components(graph, component, &global)?;
        for component in &externals.references {
            dependencies.project(
                graph_key,
                ComponentGraphProjection::ReferenceMembers(*component),
            );
            dependencies.project(
                graph_key,
                ComponentGraphProjection::ReferenceDependencies(*component),
            );
        }
        for component in &externals.inference {
            dependencies.project(
                graph_key,
                ComponentGraphProjection::InferenceMembers(*component),
            );
            dependencies.require(ArtifactKey::dir_checked_component(*component, profile));
        }
        for module in externals.modules.keys() {
            dependencies.require(ArtifactKey::dir_expanded(*module, profile));
        }

        Ok(())
    }
}

/// Return one reference component's entry module.
fn reference_entry(graph: &ComponentGraph, component: ComponentId) -> CompilerResult<ModuleId> {
    graph
        .reference_entry(component)
        .ok_or_else(|| CompilerError::Internal {
            message: format!("reference component {component} has no entry module"),
        })
}

/// Return one inference component's entry module.
fn inference_entry(graph: &ComponentGraph, component: ComponentId) -> CompilerResult<ModuleId> {
    graph
        .inference_entry(component)
        .ok_or_else(|| CompilerError::Internal {
            message: format!("inference component {component} has no entry module"),
        })
}

impl Compiler {
    /// Map every transitive external module to its checked component.
    fn external_components(
        &self,
        graph: &Arc<ComponentGraph>,
        component: ComponentId,
        global: &GlobalEnvironment,
    ) -> CompilerResult<ExternalComponents> {
        let mut inference = FxIndexSet::default();
        let mut modules = FxIndexMap::default();

        // resolve reference and Inherent Extension components
        let external = graph.external_reference_components(component, global.implicit_modules());
        let inherent = external
            .inherent
            .iter()
            .flat_map(|component| graph.reference_members(*component))
            .copied()
            .collect::<FxIndexSet<_>>();

        // bind each reachable member to its checked component
        for reference in external.components() {
            for module in graph.reference_members(reference) {
                let component =
                    graph
                        .inference_component(*module)
                        .ok_or_else(|| CompilerError::Internal {
                            message: format!("module {module:?} has no inference component"),
                        })?;
                inference.insert(component);
                modules.insert(*module, CheckExternalComponent::Checked(component));
            }
        }

        Ok(ExternalComponents {
            references: external.components().collect(),
            inference,
            modules,
            inherent,
        })
    }
}

/// External checked component inputs reached from one component.
struct ExternalComponents {
    /// The external reference components reached through the condensation.
    references: FxIndexSet<ComponentId>,
    /// The checked inference components containing external modules.
    inference: FxIndexSet<ComponentId>,
    /// The external modules keyed to their checked component.
    modules: FxIndexMap<ModuleId, CheckExternalComponent>,
    /// The inherent extension modules imported lazily on first extension lookup.
    inherent: FxIndexSet<ModuleId>,
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
