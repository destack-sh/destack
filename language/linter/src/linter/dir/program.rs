use std::sync::Arc;

use destack_artifact::EnvironmentBound;
use destack_repository::{ArtifactReader, ProviderError, Repository, Revision};
use destack_source::ModuleId;

use super::super::LintProgram;
use super::{Dir, DirModule};

/// Checked DIR for one target program.
#[derive(Debug)]
pub struct DirProgram {
    /// The program.
    pub program: Arc<LintProgram>,
    /// The checked DIR.
    pub dir: Dir,
}

impl DirProgram {
    /// Load checked DIR for one target program.
    pub(crate) fn load(
        repository: &Repository,
        revision: Revision,
        artifacts: &ArtifactReader<'_>,
        program: Arc<LintProgram>,
        environment: Arc<EnvironmentBound>,
        modules: &[ModuleId],
    ) -> Result<Self, ProviderError> {
        let dir = Dir::load(
            repository,
            revision,
            artifacts,
            program.profile.id(),
            environment,
            modules,
        )?;

        Ok(Self { program, dir })
    }

    /// Return checked DIR for one module.
    pub fn module(&self, module: ModuleId) -> Result<DirModule<'_>, ProviderError> {
        self.dir.module(module)
    }

    /// Iterate every loaded DIR module.
    pub fn modules(&self) -> impl Iterator<Item = DirModule<'_>> {
        self.dir.modules()
    }

    /// Iterate package-owned DIR modules.
    pub fn owned_modules(&self) -> impl Iterator<Item = DirModule<'_>> {
        self.dir
            .modules()
            .filter(|module| self.program.owns(module.id))
    }
}
