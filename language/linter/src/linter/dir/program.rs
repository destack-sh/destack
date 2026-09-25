use std::sync::Arc;

use tspp_artifact::{EnvironmentBound, IndexKind};
use tspp_repository::{ArtifactReader, ProviderError, Repository, Revision};
use tspp_source::ModuleId;

use super::super::LintProgram;
use super::{Dir, DirModule};

/// DIR for one target program.
#[derive(Debug)]
pub struct DirProgram<'a> {
    /// The program.
    pub program: Arc<LintProgram>,
    /// The DIR.
    pub dir: Dir<'a>,
}

impl<'a> DirProgram<'a> {
    /// Load DIR for one target program.
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

    /// Return DIR for one module.
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
