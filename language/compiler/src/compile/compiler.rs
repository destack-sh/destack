use std::sync::Arc;

use destack_artifact::{ArtifactStore, DiagnosticBuilder, DiagnosticLike};
use destack_core::StringPool;
use destack_repository::{ArtifactReader, ProviderContext, Repository, Target};

use crate::CompilerResult;

/// Compile files and sources into something (via DIR).
/// #Architecture: should Compiler be per-target? what about comptime though?
#[allow(clippy::type_complexity)]
pub struct Compiler {
    /// The repository being compiled.
    pub repository: Arc<Repository>,
    /// The live artifact store for the repository.
    pub artifacts: Arc<ArtifactStore>,
    /// The shared comptime target configuration.
    pub comptime_target: Target,
}

impl std::fmt::Debug for Compiler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Compiler")
            .field("repository", &"...")
            .finish()
    }
}

#[allow(clippy::too_many_arguments)]
#[allow(dead_code)]
impl Compiler {
    /// Create a new compiler.
    pub fn new(repository: Arc<Repository>) -> Self {
        let comptime_target = Target::comptime();
        let artifacts = repository.artifact_store().clone();

        Self {
            repository,
            artifacts,
            comptime_target,
        }
    }

    /// Return the shared string pool.
    pub(crate) fn strings(&self) -> &StringPool {
        self.repository.string_pool().as_ref()
    }

    /// Return a provider-scoped artifact reader.
    pub(crate) fn artifact_reader<'a>(
        &'a self,
        context: &'a dyn ProviderContext,
    ) -> ArtifactReader<'a> {
        ArtifactReader::new(context, Arc::clone(&self.artifacts))
    }

    /// Add one diagnostic produced during a provider attempt.
    pub(crate) fn emit_diagnostic<T>(
        &self,
        context: &dyn ProviderContext,
        diagnostic: impl Into<DiagnosticBuilder<T>>,
    ) -> CompilerResult<()>
    where
        T: DiagnosticLike,
    {
        let diagnostic = diagnostic.into();

        context.emit(&diagnostic)?;

        Ok(())
    }
}
