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
        let component =
            self.resolve_check_component_for_key(entry, component_id, profile, context)?;

        // require checked dependency facades
        let artifacts = self.artifact_reader(context);
        let dependencies = component
            .dependencies
            .iter()
            .map(|dependency| ArtifactKey::dir_checked(*dependency, profile))
            .collect::<SmallVec<[_; 8]>>();
        artifacts
            .require_all(dependencies.as_slice())
            .map_err(CompilerError::from)?;

        let mut check = CheckComponentState::new(self, context, profile);
        check.load(component.modules())?;
        check.walk()?;
        check.solve()?;
        let diagnostics = check.validate()?;
        context.emit_collection(diagnostics);
        let modules = check.finish()?;
        let checked = DirCheckedComponent {
            component: component.id(),
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
        let component = self.resolve_check_component_for_module(module, profile, context)?;
        let entry = component.entry()?;
        let artifacts = self.artifact_reader(context);
        let component_key = ArtifactKey::dir_checked_component(entry, component.id(), profile);
        artifacts
            .require(component_key)
            .map_err(CompilerError::from)?;

        Ok(ArtifactPayload::DirChecked(DirChecked {
            component: component.id(),
            entry,
        }))
    }
}
