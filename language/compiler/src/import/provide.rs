use std::iter;
use std::path::Path;

use destack_artifact::{ArtifactPayload, ArtifactSidecar, GlobalEnvironment};
use destack_dir as dir;
use destack_source::{FileContent, ModuleId};
use destack_repository::{ProfileId, ProviderContext};

use crate::import::state::ImportState;
use crate::{Compiler, CompilerError, CompilerResult};

impl Compiler {
    /// Build the global environment for one profile.
    pub(crate) fn provide_global_environment(
        &self,
        profile: ProfileId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactPayload> {
        // build independent environment sections
        let globals = self.load_global_module_ids(profile, context)?;
        let language = self.build_language_environment(profile, context)?;
        let environment = GlobalEnvironment { language, globals };

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

    /// Load the global modules selected by one profile.
    pub(crate) fn load_global_module_ids(
        &self,
        profile: ProfileId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<Vec<ModuleId>> {
        let profile = self.profile(context.revision(), profile)?;
        let mut globals = Vec::new();

        // resolve configured global modules inside the sealed revision
        for path in &profile.key.globals {
            let path = Path::new(path);
            let module_id = self
                .module_id_for_path(context.revision(), path)?
                .ok_or_else(|| CompilerError::Internal {
                    message: format!("global module is not loaded: '{}'", path.display()),
                })?;

            globals.push(module_id);
        }

        Ok(globals)
    }

    /// Build imported DIR for one module.
    pub(crate) fn provide_dir_imported(
        &self,
        module: ModuleId,
        profile: ProfileId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactPayload> {
        // load provider inputs
        let profile_id = profile;
        let profile_state = self.profile(context.revision(), profile_id)?;
        let artifacts = self.artifact_reader(context);
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
