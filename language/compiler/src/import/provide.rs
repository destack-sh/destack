use std::iter;

use destack_artifact::{ArtifactKey, ArtifactPayload, ArtifactSidecar, GlobalEnvironment};
use destack_dir as dir;
use destack_repository::{ProfileId, ProviderContext};
use destack_source::{FileContent, ModuleId};

use crate::import::state::ImportState;
use crate::{Compiler, CompilerError, CompilerResult};

impl Compiler {
    /// Build the global environment for one profile.
    pub(crate) fn provide_global_environment(
        &self,
        profile: ProfileId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactPayload> {
        // discover environment inputs
        let globals = self.load_global_module_ids(profile, context)?;
        let language_modules = self
            .repository
            .builtin_package()
            .module_ids()
            .collect::<Vec<_>>();

        // require language item declaration artifacts
        let artifacts = self.artifact_reader(context);
        let mut requirements = Vec::with_capacity(language_modules.len());
        for module in &language_modules {
            requirements.push(ArtifactKey::dir_bound(*module, profile));
        }
        artifacts
            .require_all(requirements.as_slice())
            .map_err(CompilerError::from)?;

        // build language environment for profile
        let language = self.build_language_environment(profile, &artifacts, &language_modules)?;
        let global_targets = self.build_global_targets(profile, &artifacts, &globals)?;
        let environment = GlobalEnvironment {
            language,
            globals,
            global_targets_by_key: global_targets,
        };

        Ok(ArtifactPayload::GlobalEnvironment(environment))
    }

    /// Build the active dependency index for one profile.
    pub(crate) fn provide_dependency_index(
        &self,
        profile: ProfileId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactPayload> {
        let profile_id = profile;
        let profile = self.profile(context.revision(), profile_id)?;
        let index =
            self.build_dependency_index(context.revision(), profile_id, profile.conditions())?;

        Ok(ArtifactPayload::DependencyIndex(index))
    }

    /// Build imported DIR for one module.
    pub(crate) fn provide_dir_imported(
        &self,
        module: ModuleId,
        profile: ProfileId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactPayload> {
        // require provider inputs
        let profile_id = profile;
        let profile_state = self.profile(context.revision(), profile_id)?;
        let artifacts = self.artifact_reader(context);
        let requirements = [
            ArtifactKey::dir_bound(module, profile_id),
            ArtifactKey::dependency_index(profile_id),
        ];
        artifacts
            .require_all(&requirements)
            .map_err(CompilerError::from)?;

        // load provider inputs
        let parsed = artifacts.dir_parsed(module).map_err(CompilerError::from)?;
        let bound = artifacts
            .dir_bound(module, profile_id)
            .map_err(CompilerError::from)?;
        let dependency_index = artifacts
            .dependency_index(profile_id)
            .map_err(CompilerError::from)?;
        let module = self.module(context.revision(), module)?;

        // build local module table
        let view = dir::View::new(&parsed.tree);
        let mut state = ImportState::new(
            context.revision(),
            module.as_ref(),
            dependency_index.as_ref(),
            &profile_state.key,
            profile_state.conditions(),
            self.strings(),
            view,
        );
        self.collect_modules(&mut state, &bound.roots)?;
        let stats = state.stats;
        let (imported, diagnostics) = state.finish();
        context.emit_sidecar(ArtifactSidecar::new(
            "metadata",
            iter::once(("phase", "import")),
            FileContent::Text {
                content: stats.render_metadata(),
            },
        ));
        for diagnostic in diagnostics {
            self.emit_diagnostic(context, diagnostic)?;
        }

        Ok(ArtifactPayload::DirImported(imported))
    }
}
