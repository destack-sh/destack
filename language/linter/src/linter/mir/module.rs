use std::sync::Arc;

use destack_artifact::{DiagnosticAnchor, MirLowered};
use destack_mir as mir;
use destack_repository::{ArtifactReader, ProfileId, ProviderError};
use destack_source::{ModuleId, Span, TargetId};

use super::Mir;

/// A borrowed verified MIR module.
#[derive(Debug, Clone, Copy)]
pub struct MirModule<'a> {
    /// The indexed verified MIR.
    pub mir: &'a Mir,
    /// The module id.
    pub id: ModuleId,
    /// The MIR artifact.
    pub lowered: &'a MirLowered,
    /// Analyses of the verified MIR tree.
    pub analyses: &'a mir::TreeAnalysisCache,
}

/// Owned verified MIR storage for one module.
#[derive(Debug)]
pub(super) struct MirModuleStorage {
    /// The module id.
    id: ModuleId,
    /// The MIR artifact.
    lowered: Arc<MirLowered>,
    /// Analyses of the verified MIR tree.
    analyses: mir::TreeAnalysisCache,
}

impl<'a> MirModule<'a> {
    /// Create a borrowed verified MIR module.
    pub(super) fn new(mir: &'a Mir, storage: &'a MirModuleStorage) -> Self {
        Self {
            mir,
            id: storage.id,
            lowered: &storage.lowered,
            analyses: &storage.analyses,
        }
    }

    /// Return the required source span for one MIR node.
    pub fn span(&self, node: mir::LocalNodeIdAny) -> Result<Span, ProviderError> {
        self.lowered
            .tree
            .source_span_by_id(node.id)
            .ok_or_else(|| ProviderError::Internal {
                message: format!(
                    "MIR node {} in module {:?} has no source span",
                    node.id, self.id
                ),
            })
    }

    /// Return a source anchor for one MIR node.
    pub fn anchor(&self, node: mir::LocalNodeIdAny) -> Result<DiagnosticAnchor, ProviderError> {
        let span = self.span(node)?;

        Ok(DiagnosticAnchor::Span(span))
    }
}

impl MirModuleStorage {
    /// Load one module's verified MIR.
    pub(super) fn load(
        artifacts: &ArtifactReader<'_>,
        profile: ProfileId,
        target: TargetId,
        module: ModuleId,
    ) -> Result<Self, ProviderError> {
        artifacts.mir_verified(module, profile, target)?;
        let lowered = artifacts.mir_lowered(module, profile, target)?;
        let options = mir::AnalysisOptions::new(lowered.target);
        let analyses = mir::TreeAnalysisCache::with_options(
            options,
            &lowered.dispatch,
            &lowered.memory,
            &lowered.effects,
        );

        Ok(Self {
            id: module,
            lowered,
            analyses,
        })
    }
}
