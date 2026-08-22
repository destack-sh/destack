use std::sync::Arc;

use destack_artifact::{EnvironmentBound, IndexKind};
use destack_repository::{ArtifactReader, ProviderError, Repository, Revision};
use destack_source::ModuleId;

use super::super::LintProgram;
use super::{Dir, DirModule};

/// Checked DIR for one target program.
#[derive(Debug)]
pub struct DirProgram<'a> {
    /// The program.
    pub program: Arc<LintProgram>,
    /// The checked DIR.
    pub dir: Dir<'a>,
}

impl<'a> DirProgram<'a> {
    /// Load checked DIR for one target program.
    pub(crate) fn load(
        repository: &'a Repository,
        revision: Revision,
        artifacts: &ArtifactReader<'a>,
        program: Arc<LintProgram>,
        environment: Arc<EnvironmentBound>,
        modules: &[ModuleId],
        indexed_modules: &[ModuleId],
        indexes: &[IndexKind],
    ) -> Result<Self, ProviderError> {
        let dir = Dir::load(
            repository,
            revision,
            artifacts,
            program.profile.id(),
            environment,
            modules,
            indexed_modules,
            indexes,
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
