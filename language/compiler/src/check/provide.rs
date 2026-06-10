use std::iter;

use destack_artifact::{
    ArtifactKey, ArtifactPayload, ArtifactSidecar, DirChecked, DirCheckedComponent,
};
use destack_repository::{ArtifactReader, ProfileId, ProviderContext};
use destack_source::{ComponentId, FileContent, ModuleId};
use indexmap::{IndexMap, IndexSet};

use crate::check::{CheckComponentGraph, CheckState};
use crate::{Compiler, CompilerError, CompilerResult};

impl Compiler {
    /// Build checked DIR side tables for one resolved component.
    pub(crate) fn provide_dir_checked_component(
        &self,
        entry: ModuleId,
        component_id: ComponentId,
        profile: ProfileId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactPayload> {
        // create provider reader before graph discovery
        let artifacts = self.artifact_reader(context);

        // discover checked component
        let graph = self.collect_check_component_graph(entry, profile, &artifacts)?;
        let component_modules = graph.component(entry);
        let discovered_entry =
            component_modules
                .first()
                .copied()
                .ok_or_else(|| CompilerError::Internal {
                    message: "checked component has no modules".to_string(),
                })?;
        let discovered_component_id =
            ComponentId::from_modules(profile, component_modules.iter().copied());

        // validate the component entry
        if entry != discovered_entry {
            return Err(CompilerError::Internal {
                message: format!(
                    "checked component key entry={entry:?} does not match discovered entry={discovered_entry:?}"
                ),
            });
        }

        // validate the component identity
        if component_id != discovered_component_id {
            return Err(CompilerError::Internal {
                message: format!(
                    "checked component key component={component_id} does not match discovered component={discovered_component_id}"
                ),
            });
        }

        // require artifacts needed before component check work
        let external_components =
            graph.external_components(profile, component_modules.as_slice())?;
        let mut requirements = Vec::new();
        for module in external_components.keys() {
            requirements.push(ArtifactKey::dir_expanded(*module, profile));
        }
        requirements.extend(
            external_components
                .values()
                .copied()
                .collect::<IndexSet<_>>()
                .into_iter()
                .map(|dependency| {
                    ArtifactKey::dir_checked_component(
                        dependency.entry,
                        dependency.component,
                        profile,
                    )
                }),
        );
        requirements.push(ArtifactKey::global_environment(profile));
        artifacts
            .require_all(requirements.as_slice())
            .map_err(CompilerError::from)?;
        let environment = artifacts
            .global_environment(profile)
            .map_err(CompilerError::from)?;
        let entry_module = self.module(context.revision(), entry)?;
        let options = self.workspace_compiler_options(context, entry_module.as_ref())?;

        // check component
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
        check.load(component_modules.as_slice())?;
        check.walk()?;
        check.build()?;
        check.solve()?;

        // track stats
        if options.emit_stats {
            let stats = check.stats();
            context.emit_sidecar(ArtifactSidecar::new(
                "metadata",
                iter::once(("phase", "check")),
                FileContent::Text {
                    content: stats.render_metadata(),
                },
            ));
        }
        let events = emit_events.then(|| check.events());

        // commit output DIR tables
        let (modules, diagnostics) = check.commit()?;
        context.emit_diagnostics(diagnostics);

        // track events
        if let Some(events) = events {
            context.emit_sidecar(ArtifactSidecar::new(
                "events",
                iter::once(("phase", "check")),
                FileContent::Text {
                    content: events.render(),
                },
            ));
        }

        let checked = DirCheckedComponent {
            component: component_id,
            modules,
        };
        Ok(ArtifactPayload::DirCheckedComponent(checked))
    }

    /// Provide the checked DIR facade for one module.
    pub(crate) fn provide_dir_checked(
        &self,
        module: ModuleId,
        profile: ProfileId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactPayload> {
        // create provider reader before graph discovery
        let artifacts = self.artifact_reader(context);

        // discover checked component
        let graph = self.collect_check_component_graph(module, profile, &artifacts)?;
        let component_modules = graph.component(module);
        let component_id = ComponentId::from_modules(profile, component_modules.iter().copied());
        let entry = component_modules
            .first()
            .copied()
            .ok_or_else(|| CompilerError::Internal {
                message: "checked component has no modules".to_string(),
            })?;

        // require checked component
        let component_key = ArtifactKey::dir_checked_component(entry, component_id, profile);
        artifacts
            .require(component_key)
            .map_err(CompilerError::from)?;

        Ok(ArtifactPayload::DirChecked(DirChecked {
            component: component_id,
            entry,
        }))
    }

    /// Load the resolved dependency graph reachable from one module.
    fn collect_check_component_graph(
        &self,
        module: ModuleId,
        profile: ProfileId,
        artifacts: &ArtifactReader<'_>,
    ) -> CompilerResult<CheckComponentGraph> {
        let mut graph = IndexMap::new();
        let mut pending = IndexSet::new();
        pending.insert(module);

        // discover resolved imports one dependency frontier at a time
        while !pending.is_empty() {
            let frontier = pending.iter().copied().collect::<Vec<_>>();
            pending.clear();
            let frontier = frontier
                .into_iter()
                .filter(|module| !graph.contains_key(module))
                .collect::<Vec<_>>();
            if frontier.is_empty() {
                continue;
            }

            // require the whole unresolved frontier before reading it
            let requirements = frontier
                .iter()
                .map(|module| ArtifactKey::dir_resolved(*module, profile))
                .collect::<Vec<_>>();
            artifacts
                .require_all(requirements.as_slice())
                .map_err(CompilerError::from)?;

            // read resolved modules and collect the next frontier
            for module in frontier {
                let resolved = artifacts
                    .dir_resolved(module, profile)
                    .map_err(CompilerError::from)?;
                let mut dependencies = resolved.imports.modules().collect::<IndexSet<_>>();
                dependencies.shift_remove(&module);
                let dependencies = dependencies.into_iter().collect::<Vec<_>>();

                // enqueue only unresolved dependency modules
                for dependency in &dependencies {
                    if !graph.contains_key(dependency) {
                        pending.insert(*dependency);
                    }
                }

                graph.insert(module, dependencies);
            }
        }

        Ok(CheckComponentGraph::new(graph))
    }
}
