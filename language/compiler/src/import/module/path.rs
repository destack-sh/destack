use std::path::Path;

use tspp_repository::ModulePathResolution;
use tspp_source::Loader;

use crate::import::ImportState;
use crate::{Compiler, CompilerError, CompilerResult};

impl Compiler {
    /// Resolve one logical path, observing every candidate probe.
    pub(in crate::import) fn resolve_module_path(
        &self,
        state: &mut ImportState<'_>,
        path: &Path,
        loader: Option<Loader>,
    ) -> CompilerResult<ModulePathResolution> {
        let mut resolution = self
            .repository
            .resolve_module_path(state.revision, path, loader)
            .map_err(|error| CompilerError::Internal {
                message: format!(
                    "failed to resolve module path '{}': {error}",
                    path.display()
                ),
            })?;

        // record every candidate probe on this attempt
        for probe in resolution.probes.drain(..) {
            state.observe(probe);
        }

        Ok(resolution)
    }
}
