use std::sync::Arc;

use destack_artifact::ProgramAnalysis;
use destack_core::FxIndexMap;
use destack_repository::{ArtifactReader, ProviderError, Repository, Revision};
use destack_source::ModuleId;

use super::super::LintProgram;
use super::MirModule;

/// Verified MIR for one target program.
#[derive(Debug)]
pub struct MirProgram {
    /// The program.
    pub program: Arc<LintProgram>,
    /// Verified MIR keyed by module id.
    pub(crate) modules: FxIndexMap<ModuleId, MirModule>,
    /// The program analysis.
    pub analysis: Arc<ProgramAnalysis>,
}

impl MirProgram {
    /// Load verified MIR for one target program.
    pub(crate) fn load(
        repository: &Repository,
        revision: Revision,
        artifacts: &ArtifactReader<'_>,
        program: Arc<LintProgram>,
    ) -> Result<Self, ProviderError> {
        let profile = program.profile.id();
        let target = program.target;
        let mut loaded = FxIndexMap::default();
        loaded.reserve(program.modules.len());

        // load every code module in the MIR closure
        for module in program.modules.iter().copied() {
            let repository_module = repository
                .module(revision, module)
                .map_err(|error| ProviderError::internal(error.to_string()))?
                .ok_or_else(|| {
                    ProviderError::internal(format!("missing lint module {module:?}"))
                })?;
            if !repository_module.is_code() {
                continue;
            }

            let module = MirModule::load(artifacts, profile, target, module)?;
            loaded.insert(module.id, module);
        }

        let analysis = artifacts.program_analysis(profile, target)?;

        Ok(Self {
            program,
            modules: loaded,
            analysis,
        })
    }

    /// Return verified MIR for one module.
    pub fn module(&self, module: ModuleId) -> Result<&MirModule, ProviderError> {
        self.modules
            .get(&module)
            .ok_or_else(|| ProviderError::Internal {
                message: format!("MIR module {module:?} is outside the lint program"),
            })
    }

    /// Iterate every loaded MIR module.
    pub fn modules(&self) -> impl Iterator<Item = &MirModule> {
        self.modules.values()
    }

    /// Iterate package-owned MIR modules.
    pub fn owned_modules(&self) -> impl Iterator<Item = &MirModule> {
        self.modules
            .values()
            .filter(|module| self.program.owns(module.id))
    }
}
