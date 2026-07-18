use std::sync::Arc;

use destack_artifact::{DiagnosticAnchor, MirLowered};
use destack_mir as mir;
use destack_repository::{ArtifactReader, ProfileId, ProviderError};
use destack_source::{ModuleId, Span, TargetId};

/// One module's verified MIR.
#[derive(Debug)]
pub struct MirModule {
    /// The module id.
    pub id: ModuleId,
    /// The active profile.
    pub profile: ProfileId,
    /// The active target.
    pub target: TargetId,
    /// The MIR artifact.
    pub mir: Arc<MirLowered>,
    /// Analyses of the verified MIR tree.
    pub analyses: mir::TreeAnalysisCache,
}

impl MirModule {
    /// Load one module's verified MIR.
    pub(crate) fn load(
        artifacts: &ArtifactReader<'_>,
        profile: ProfileId,
        target: TargetId,
        module: ModuleId,
    ) -> Result<Self, ProviderError> {
        artifacts.mir_verified(module, profile, target)?;
        let mir = artifacts.mir_lowered(module, profile, target)?;
        let options = mir::AnalysisOptions::new(mir.target);
        let analyses =
            mir::TreeAnalysisCache::with_options(options, &mir.dispatch, &mir.memory, &mir.effects);

        Ok(Self {
            id: module,
            profile,
            target,
            mir,
            analyses,
        })
    }

    /// Return the required source span for one MIR node.
    pub fn span(&self, node: mir::LocalNodeIdAny) -> Result<Span, ProviderError> {
        self.mir
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
