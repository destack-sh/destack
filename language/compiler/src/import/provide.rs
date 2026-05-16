use std::path::Path;

use destack_artifact::{ArtifactKey, ArtifactPayload, GlobalEnvironment};
use destack_dir as dir;
use destack_source::ModuleId;
use destack_workspace::{ProfileId, ProviderContext};

use crate::import::state::ImportState;
use crate::{Compiler, CompilerError, CompilerResult};

impl Compiler {
    /// Build the global environment for one profile.
    pub(crate) fn provide_global_environment(
        &self,
        profile: ProfileId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactPayload> {
        // resolve configured global modules inside the sealed revision
        let profile = self.profile(context.revision(), profile);
        let mut modules = Vec::new();
        for path in &profile.key.globals {
            let path = Path::new(path);
            let module_id = self
                .module_id_for_path(context.revision(), path)?
                .ok_or_else(|| CompilerError::Internal {
                    message: format!("global module is not loaded: '{}'", path.display()),
                })?;

            modules.push(module_id);
        }

        let environment = GlobalEnvironment {
            language: Default::default(),
            modules,
        };

        Ok(ArtifactPayload::GlobalEnvironment(environment))
    }

    /// Build imported DIR for one module.
    pub(crate) fn provide_dir_imported(
        &self,
        module: ModuleId,
        profile: ProfileId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactPayload> {
        // require bound source tree and binding table
        context
            .require(ArtifactKey::dir_bound(module, profile))
            .map_err(CompilerError::from)?;

        // load provider inputs
        let parsed = self
            .dir_parsed(context, module)
            .map_err(CompilerError::from)?;
        let module = self.module(context.revision(), module);

        // build local dependency table
        let view = dir::View::new(&parsed.tree);
        let mut state = ImportState::new(context.revision(), module.as_ref(), self.strings(), view);
        self.collect_dependencies(&mut state, &parsed.roots)?;
        let (imported, diagnostics) = state.finish();
        for diagnostic in diagnostics {
            self.emit_diagnostic(context, diagnostic)?;
        }

        Ok(ArtifactPayload::DirImported(imported))
    }
}
