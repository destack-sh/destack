use std::sync::Arc;

use destack_artifact::{DiagnosticBuilder, DiagnosticLike};
use destack_core::StringPool;
use destack_repository::{ArtifactReader, ProviderContext, Repository, Target};
use destack_source::DiagnosticRegistry;

use crate::CompilerResult;

/// Compile source modules into DIR and MIR artifacts.
/// #Architecture: should Compiler be per-target? what about comptime though?
#[allow(clippy::type_complexity)]
pub struct Compiler {
    /// The repository being compiled.
    pub repository: Arc<Repository>,
    /// The diagnostics accepted by source controls.
    pub(crate) diagnostics: Arc<DiagnosticRegistry>,
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

impl Compiler {
    /// Create a new compiler.
    pub fn new(repository: Arc<Repository>, diagnostics: Arc<DiagnosticRegistry>) -> Self {
        let comptime_target = Target::comptime();

        Self {
            repository,
            diagnostics,
            comptime_target,
        }
    }

    /// Return the shared string pool.
    pub(crate) fn strings(&self) -> &StringPool {
        self.repository.string_pool().as_ref()
    }

    /// Return the artifact reader for one provider attempt.
    pub(crate) fn artifact_reader<'a>(
        &'a self,
        context: &'a dyn ProviderContext,
    ) -> ArtifactReader<'a> {
        let artifacts = self
            .repository
            .artifact_reader(context.revision())
            .with_context(context);
        let Some(dependencies) = context.artifact_dependencies() else {
            return artifacts;
        };

        artifacts.restrict(dependencies)
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
