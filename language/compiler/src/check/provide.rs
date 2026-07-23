use destack_core::{FxIndexMap, FxIndexSet};
use std::iter;
use std::sync::Arc;

use destack_artifact::{
    ArtifactDependencySet, ArtifactKey, ArtifactPayload, ArtifactProjectionKey, ArtifactSidecar,
    ComponentGraph, ComponentGraphProjection, DirChecked, DirCheckedComponent, DirDeclared,
    GlobalEnvironment,
};
use destack_repository::{ArtifactReader, ProfileId, ProviderContext, ProviderError};
use destack_source::{ComponentId, Content, ModuleId};

use crate::check::{AnnotatedSource, CheckComponentKey, CheckExternalArtifact, CheckState};
use crate::{Compiler, CompilerError, CompilerResult};

impl Compiler {
    /// Collect inputs for the declared DIR environment of one component.
    pub(crate) fn collect_dir_declared(
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

        // require and project the external components read by this check
        self.collect_external_components(
            &artifacts,
            &graph,
            component_id,
            profile,
            &mut dependencies,
        )?;

        Ok(dependencies)
    }

    /// Provide the declared DIR environment for one reference component.
    pub(crate) fn provide_dir_declared(
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

        // validate the requested component entry against the graph
        let modules = graph.members(component_id).to_vec();
        if modules.is_empty() || graph.entry(component_id) != Some(entry) {
            return Err(CompilerError::Internal {
                message: format!(
                    "declared component key entry={entry:?} component={component_id} does not \
                     match the component graph"
                ),
            });
        }

        // load context shared across the environment check
        let global = artifacts
            .global_environment(profile)
            .map_err(CompilerError::from)?;
        let external_components = self.external_components(&graph, component_id, &global)?;
        let environment = self.environment(context.revision())?;
        let entry_module = self.module(context.revision(), entry)?;
        let options = self.workspace_compiler_options(context, entry_module.as_ref())?;

        // check declarations and module state; callable bodies stay unwalked
        let mut check = CheckState::new(
            self,
            context,
            &artifacts,
            profile,
            global,
            environment,
            checked_externals(external_components.modules),
            external_components.inherent,
            inherent_extension_symbols(&graph),
            FxIndexSet::default(),
            options.emit_events || context.emit_events(),
        );
        check.check(modules.as_slice(), None)?;

        // emit solver counters for the environment pass
        let stats = check.stats();
        context.emit_counter("variables", stats.variables as u64);
        context.emit_counter("constraints", stats.constraints as u64);
        context.emit_counter("types", stats.types as u64);

        // seal every member; diagnostics belong to the owning units
        let (modules, _) = check.write()?;

        Ok(ArtifactPayload::DirDeclared(Arc::new(DirDeclared {
            component: component_id,
            modules,
        })))
    }

    /// Collect inputs for checked DIR side tables of one inference component.
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
        let unit = graph
            .inference_component(entry)
            .ok_or_else(|| CompilerError::Internal {
                message: format!("inference entry {entry:?} is absent from the component graph"),
            })?;
        dependencies.project(graph_key, ComponentGraphProjection::InferenceMembers(unit));
        dependencies.project(
            graph_key,
            ComponentGraphProjection::InferenceDependencies(unit),
        );

        // require own members' DIR payloads
        for module in graph.inference_members(unit) {
            dependencies.require(ArtifactKey::dir_parsed(*module));
            dependencies.require(ArtifactKey::dir_bound(*module, profile));
            dependencies.require(ArtifactKey::dir_resolved(*module, profile));
            dependencies.require(ArtifactKey::dir_expanded(*module, profile));
        }

        // skip the declared environment when one inference component covers
        //  the whole reference component; per-module projections keep body
        //  edits from crossing inference components
        if graph.inference_members(unit).len() < graph.members(component_id).len() {
            let declared_entry = component_entry(&graph, component_id)?;
            let declared_key = ArtifactKey::dir_declared(declared_entry, component_id, profile);
            let members = graph
                .inference_members(unit)
                .iter()
                .copied()
                .collect::<FxIndexSet<_>>();
            for module in graph.members(component_id) {
                dependencies.project(declared_key, ArtifactProjectionKey::DirChecked(*module));
                if !members.contains(module) {
                    dependencies.require(ArtifactKey::dir_expanded(*module, profile));
                }
            }
        }

        // require checked tables for upstream inference components
        for upstream in graph.inference_dependencies(unit) {
            let upstream_entry = inference_entry(&graph, upstream)?;
            dependencies.require(ArtifactKey::dir_checked_component(
                upstream_entry,
                component_id,
                profile,
            ));
        }

        // require and project the external components read by this check
        self.collect_external_components(
            &artifacts,
            &graph,
            component_id,
            profile,
            &mut dependencies,
        )?;

        Ok(dependencies)
    }

    /// Provide checked DIR side tables for one inference component.
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

        // validate the requested inference entry against the graph
        if graph.members(component_id).is_empty()
            || graph.inference_component_entry(entry) != Some((component_id, entry))
        {
            return Err(CompilerError::Internal {
                message: format!(
                    "checked component key entry={entry:?} component={component_id} does not \
                     match the component graph"
                ),
            });
        }
        let unit = graph
            .inference_component(entry)
            .ok_or_else(|| CompilerError::Internal {
                message: format!("inference entry {entry:?} is absent from the component graph"),
            })?;
        let modules = graph.inference_members(unit).to_vec();
        let inference_modules = modules.iter().copied().collect::<FxIndexSet<_>>();

        // skip the declared environment when one inference component covers
        //  the whole reference component
        let declared_entry = component_entry(&graph, component_id)?;
        let declared = if modules.len() < graph.members(component_id).len() {
            Some(
                artifacts
                    .dir_declared(declared_entry, component_id, profile)
                    .map_err(CompilerError::from)?,
            )
        } else {
            None
        };

        // map sibling members to their sealed artifacts
        let global = artifacts
            .global_environment(profile)
            .map_err(CompilerError::from)?;
        let external_components = self.external_components(&graph, component_id, &global)?;
        let mut externals = checked_externals(external_components.modules);
        if declared.is_some() {
            let mut upstream_members = FxIndexMap::default();
            for upstream in graph.inference_dependencies(unit) {
                let upstream_entry = inference_entry(&graph, upstream)?;
                for module in graph.inference_members(upstream) {
                    upstream_members.insert(
                        *module,
                        CheckComponentKey {
                            entry: upstream_entry,
                            component: component_id,
                        },
                    );
                }
            }
            for module in graph.members(component_id) {
                if inference_modules.contains(module) {
                    continue;
                }
                let artifact = match upstream_members.get(module) {
                    Some(key) => CheckExternalArtifact::Checked(*key),
                    None => CheckExternalArtifact::Declared(CheckComponentKey {
                        entry: declared_entry,
                        component: component_id,
                    }),
                };
                externals.insert(*module, artifact);
            }
        }

        // load context shared across the component check
        let environment = self.environment(context.revision())?;
        let entry_module = self.module(context.revision(), entry)?;
        let options = self.workspace_compiler_options(context, entry_module.as_ref())?;

        // check the inference component over the declared environment
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
            inherent_extension_symbols(&graph),
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
        dependencies.project(graph_key, ComponentGraphProjection::InferenceEntry(module));

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

        // resolve the owning inference component behind the checked facade
        let (component, entry) =
            graph
                .inference_component_entry(module)
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

        // resolve the owning inference component behind the checked facade
        let (component, entry) =
            graph
                .inference_component_entry(module)
                .ok_or_else(|| CompilerError::Internal {
                    message: format!("module {module:?} is absent from the component graph"),
                })?;

        Ok(ArtifactPayload::DirChecked(Arc::new(DirChecked {
            component,
            entry,
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

        // the external set follows the global environment's implicit modules
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
        for key in &externals.components {
            dependencies.project(graph_key, ComponentGraphProjection::Members(key.component));
            dependencies.project(
                graph_key,
                ComponentGraphProjection::Dependencies(key.component),
            );
            dependencies.require(ArtifactKey::dir_checked_component(
                key.entry,
                key.component,
                profile,
            ));
        }
        for module in externals.modules.keys() {
            dependencies.require(ArtifactKey::dir_expanded(*module, profile));
        }

        Ok(())
    }
}

/// Return one component's entry module.
fn component_entry(graph: &ComponentGraph, component: ComponentId) -> CompilerResult<ModuleId> {
    graph
        .entry(component)
        .ok_or_else(|| CompilerError::Internal {
            message: format!("component {component} has no entry module"),
        })
}

/// Return one inference component's entry module.
fn inference_entry(graph: &ComponentGraph, unit: ComponentId) -> CompilerResult<ModuleId> {
    graph
        .inference_entry(unit)
        .ok_or_else(|| CompilerError::Internal {
            message: format!("inference component {unit} has no entry module"),
        })
}

/// Map external component modules to checked artifact loads.
fn checked_externals(
    modules: FxIndexMap<ModuleId, CheckComponentKey>,
) -> FxIndexMap<ModuleId, CheckExternalArtifact> {
    modules
        .into_iter()
        .map(|(module, key)| (module, CheckExternalArtifact::Checked(key)))
        .collect()
}

impl Compiler {
    /// Map every transitive external module to its checked component key.
    fn external_components(
        &self,
        graph: &Arc<ComponentGraph>,
        component: ComponentId,
        global: &GlobalEnvironment,
    ) -> CompilerResult<ExternalComponents> {
        let mut components = FxIndexSet::default();
        let mut modules = FxIndexMap::default();

        // reach inherent extensions whose target loads here; components the
        //  extensions build from keep plain imports, so the artifact graph
        //  stays acyclic
        let plain = graph.transitive_dependencies(component);
        let mut roots = plain.clone();
        if !graph.inherent_closure_contains(component) {
            // treat language items and globals as loaded everywhere
            let implicit = global.implicit_modules().collect::<FxIndexSet<_>>();
            for extension in graph.inherent_extensions() {
                // skip extensions whose target does not load here
                let loaded = implicit.contains(&extension.target)
                    || graph
                        .component(extension.target)
                        .is_some_and(|target| plain.contains(&target));
                let Some(source) = graph.component(extension.symbol.module_id) else {
                    continue;
                };
                if !loaded || source == component || roots.contains(&source) {
                    continue;
                }

                // reach the extension's component and its dependencies
                roots.push(source);
                for dependency in graph.transitive_dependencies(source) {
                    if !roots.contains(&dependency) {
                        roots.push(dependency);
                    }
                }
            }
        }

        // bind each reachable member to its checked component; inherent
        //  extension modules import lazily, on the first extension lookup
        let mut inherent = FxIndexSet::default();
        for dependency in roots {
            let is_inherent = !plain.contains(&dependency);
            for module in graph.members(dependency) {
                if is_inherent {
                    inherent.insert(*module);
                }
                let entry = graph
                    .inference_component_entry(*module)
                    .map(|(_, entry)| entry);
                let entry = entry.ok_or_else(|| CompilerError::Internal {
                    message: format!("module {module:?} has no inference entry"),
                })?;
                let key = CheckComponentKey {
                    entry,
                    component: dependency,
                };

                components.insert(key);
                modules.insert(*module, key);
            }
        }

        Ok(ExternalComponents {
            components,
            modules,
            inherent,
        })
    }
}

/// Return the graph's inherent extension symbols.
fn inherent_extension_symbols(graph: &ComponentGraph) -> Vec<destack_dir::GlobalSymbolId> {
    graph
        .inherent_extensions()
        .iter()
        .map(|extension| extension.symbol)
        .collect()
}

/// External checked component inputs reached from one component.
struct ExternalComponents {
    /// The checked component keys reached through the condensation.
    components: FxIndexSet<CheckComponentKey>,
    /// The external modules keyed to their checked component.
    modules: FxIndexMap<ModuleId, CheckComponentKey>,
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
