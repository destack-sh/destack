use destack_artifact::{ArtifactKey, ArtifactPayload, DirChecked, DirCheckedComponent};
use destack_source::{ComponentId, ModuleId};
use destack_workspace::{ProfileId, ProviderContext};
use smallvec::SmallVec;

use crate::check::CheckComponentState;
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
        // discover checked component
        let graph = self.collect_check_component_graph(entry, profile, context)?;
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

        // reject stale component keys after import graph changes
        if entry != discovered_entry || component_id != discovered_component_id {
            return Err(CompilerError::Internal {
                message: format!(
                    "checked component key entry={entry:?} component={component_id} does not match discovered entry={discovered_entry:?} component={discovered_component_id}"
                ),
            });
        }

        // require pre-checked artifacts for component dependencies
        let dependencies = graph.dependencies(component_modules.as_slice());
        let artifacts = self.artifact_reader(context);
        let dependencies = dependencies
            .iter()
            .map(|dependency| ArtifactKey::dir_checked(*dependency, profile))
            .collect::<SmallVec<[_; 8]>>();
        artifacts
            .require_all(dependencies.as_slice())
            .map_err(CompilerError::from)?;
        let environment = artifacts
            .global_environment(profile)
            .map_err(CompilerError::from)?;

        // check component
        let mut check =
            CheckComponentState::new(self, context, profile, component_modules, environment);
        check.load()?;
        check.walk()?;
        check.solve()?;

        // commit checked DIR
        let (modules, diagnostics) = check.commit()?;
        context.emit_collection(diagnostics);
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
        // discover checked component
        let graph = self.collect_check_component_graph(module, profile, context)?;
        let component_modules = graph.component(module);
        let component_id = ComponentId::from_modules(profile, component_modules.iter().copied());
        let entry = component_modules
            .first()
            .copied()
            .ok_or_else(|| CompilerError::Internal {
                message: "checked component has no modules".to_string(),
            })?;

        // require checked component
        let artifacts = self.artifact_reader(context);
        let component_key = ArtifactKey::dir_checked_component(entry, component_id, profile);
        artifacts
            .require(component_key)
            .map_err(CompilerError::from)?;

        Ok(ArtifactPayload::DirChecked(DirChecked {
            component: component_id,
            entry,
        }))
    }
}
