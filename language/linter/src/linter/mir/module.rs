use std::sync::Arc;

use destack_artifact::{DiagnosticAnchor, MirLowered, MirVerified};
use destack_core::StringPool;
use destack_mir as mir;
use destack_repository::{ArtifactReader, ProfileId, ProviderError};
use destack_source::{ModuleId, Span, TargetId};

/// Verified MIR and analyses for one module.
#[derive(Debug)]
pub struct MirModule {
    /// The module id.
    pub id: ModuleId,
    /// The MIR artifact.
    pub lowered: Arc<MirLowered>,
    /// Analyses of the verified MIR tree.
    pub analyses: mir::ModuleAnalyses,
    /// The repository string pool.
    pub strings: Arc<StringPool>,
}

impl MirModule {
    /// Create one verified MIR module.
    pub(crate) fn new(id: ModuleId, lowered: Arc<MirLowered>, strings: Arc<StringPool>) -> Self {
        let options = mir::AnalysisOptions::new(lowered.target);
        let analyses = mir::ModuleAnalyses::with_options(options);

        Self {
            id,
            lowered,
            analyses,
            strings,
        }
    }

    /// Load one module's verified MIR.
    pub(super) fn load(
        artifacts: &ArtifactReader<'_>,
        profile: ProfileId,
        target: TargetId,
        module: ModuleId,
        strings: Arc<StringPool>,
    ) -> Result<Self, ProviderError> {
        artifacts.read::<MirVerified>((module, profile, target))?;
        let lowered = artifacts.read::<MirLowered>((module, profile, target))?;

        Ok(Self::new(module, lowered, strings))
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
