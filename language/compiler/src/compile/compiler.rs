use std::sync::Arc;

use destack_artifact::{ArtifactStore, DiagnosticBuilder, DiagnosticLike};
use destack_source::{ProfileId, TargetId};
use destack_workspace::{Profile, ProviderContext, Repository, Revision, Target};

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
        let comptime_target = Target::comptime("comptime");
        let artifacts = repository.artifact_store().clone();

        Self {
            repository,
            artifacts,
            comptime_target,
        }
    }

    /// Return one derived semantic profile by id for one explicit revision.
    pub(crate) fn profile(&self, revision: Revision, profile_id: ProfileId) -> Profile {
        self.repository
            .profile(revision, profile_id)
            .unwrap_or_else(|error| panic!("failed to load profile {profile_id:?}: {error}"))
            .unwrap_or_else(|| panic!("missing compiler profile for {profile_id:?}"))
            .as_ref()
            .clone()
    }

    /// Return the display name for one target id at one pinned revision.
    pub(crate) fn target_name(&self, revision: Revision, target_id: &TargetId) -> String {
        self.repository
            .effective_target(revision, *target_id)
            .ok()
            .flatten()
            .map(|target| target.name)
            .unwrap_or_else(|| target_id.to_string())
    }

    /// Add one diagnostic produced during a provider attempt.
    pub(crate) fn emit_diagnostic<T>(
        &self,
        context: &dyn ProviderContext,
        diagnostic: T,
    ) -> CompilerResult<()>
    where
        T: DiagnosticLike,
    {
        self.emit_built_diagnostic(context, DiagnosticBuilder::new(diagnostic))
    }

    /// Add one decorated diagnostic produced during a provider attempt.
    pub(crate) fn emit_built_diagnostic<T>(
        &self,
        context: &dyn ProviderContext,
        diagnostic: DiagnosticBuilder<T>,
    ) -> CompilerResult<()>
    where
        T: DiagnosticLike,
    {
        context.emit(&diagnostic)?;

        Ok(())
    }
}
