use std::sync::Arc;

use destack_artifact::ProgramAnalysis;
use destack_repository::{ArtifactReader, ProviderError, Repository, Revision};
use destack_source::ModuleId;

use super::super::LintProgram;
use super::{Mir, MirModule};

/// Verified MIR for one target program.
#[derive(Debug)]
pub struct MirProgram {
    /// The program.
    pub program: Arc<LintProgram>,
    /// The verified MIR.
    pub mir: Mir,
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
        let mut modules = Vec::new();

        // select every reachable code module
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

            modules.push(module);
        }

        let strings = repository.string_pool().clone();
        let mir = Mir::load(artifacts, profile, target, &modules, strings)?;
        let analysis = artifacts.read::<ProgramAnalysis>((profile, target))?;

        Ok(Self {
            program,
            mir,
            analysis,
        })
    }

    /// Return verified MIR for one module.
    pub fn module(&self, module: ModuleId) -> Result<&MirModule, ProviderError> {
        self.mir.module(module)
    }

    /// Return verified MIR for one module for analysis.
    pub fn module_mut(&mut self, module: ModuleId) -> Result<&mut MirModule, ProviderError> {
        self.mir.module_mut(module)
    }

    /// Iterate every loaded MIR module.
    pub fn modules(&self) -> impl Iterator<Item = &MirModule> {
        self.mir.modules()
    }

    /// Iterate every loaded MIR module for analysis.
    pub fn modules_mut(&mut self) -> impl Iterator<Item = &mut MirModule> {
        self.mir.modules_mut()
    }

    /// Iterate package-owned MIR modules.
    pub fn owned_modules(&self) -> impl Iterator<Item = &MirModule> {
        self.mir
            .modules()
            .filter(|module| self.program.owns(module.id))
    }

    /// Iterate package-owned MIR modules for analysis.
    pub fn owned_modules_mut(&mut self) -> impl Iterator<Item = &mut MirModule> {
        let program = &self.program;

        self.mir
            .modules_mut()
            .filter(move |module| program.owns(module.id))
    }
}
