use std::sync::Arc;

use destack_artifact::GlobalEnvironment;
use destack_core::FxIndexMap;
use destack_repository::{ArtifactReader, ProviderError, Repository, Revision};
use destack_source::ModuleId;

use super::super::LintProgram;
use super::DirModule;

/// Checked DIR for one target program.
#[derive(Debug)]
pub struct DirProgram {
    /// The program.
    pub program: Arc<LintProgram>,
    /// The global environment.
    pub environment: Arc<GlobalEnvironment>,
    /// Checked DIR keyed by module id.
    pub(crate) modules: FxIndexMap<ModuleId, DirModule>,
}

impl DirProgram {
    /// Load checked DIR for one target program.
    pub(crate) fn load(
        repository: &Repository,
        revision: Revision,
        artifacts: &ArtifactReader<'_>,
        program: Arc<LintProgram>,
        environment: Arc<GlobalEnvironment>,
        modules: &[ModuleId],
    ) -> Result<Self, ProviderError> {
        let profile = program.profile.id();
        let target = program.target;
        let mut loaded = FxIndexMap::default();
        loaded.reserve(modules.len());

        // load every code module in the DIR closure
        for module in modules.iter().copied() {
            let repository_module = repository
                .module(revision, module)
                .map_err(|error| ProviderError::internal(error.to_string()))?
                .ok_or_else(|| {
                    ProviderError::internal(format!("missing lint module {module:?}"))
                })?;
            if !repository_module.is_code() {
                continue;
            }

            let module = DirModule::load(
                repository,
                revision,
                profile,
                target,
                environment.clone(),
                repository_module,
                artifacts,
            )?;
            loaded.insert(module.id, module);
        }

        Ok(Self {
            program,
            environment,
            modules: loaded,
        })
    }

    /// Return checked DIR for one module.
    pub fn module(&self, module: ModuleId) -> Result<&DirModule, ProviderError> {
        self.modules
            .get(&module)
            .ok_or_else(|| ProviderError::Internal {
                message: format!("DIR module {module:?} is outside the lint program"),
            })
    }

    /// Iterate every loaded DIR module.
    pub fn modules(&self) -> impl Iterator<Item = &DirModule> {
        self.modules.values()
    }

    /// Iterate package-owned DIR modules.
    pub fn owned_modules(&self) -> impl Iterator<Item = &DirModule> {
        self.modules
            .values()
            .filter(|module| self.program.owns(module.id))
    }
}
