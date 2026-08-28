use std::sync::Arc;

use destack_artifact::{DiagnosticAnchor, MirLowered, MirVerified};
use destack_core::StringPool;
use destack_dir as dir;
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
    pub analyses: mir::AnalysisCache,
    /// The repository string pool.
    pub strings: Arc<StringPool>,
}

impl MirModule {
    /// Create one verified MIR module.
    pub(crate) fn new(id: ModuleId, lowered: Arc<MirLowered>, strings: Arc<StringPool>) -> Self {
        let options = mir::AnalysisOptions::new(lowered.target);
        let analyses = mir::AnalysisCache::with_options(options);

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
        let provenance = self.lowered.tree.provenance(node.id);
        self.lowered
            .provenance
            .primary_span(provenance)
            .or_else(|| self.lowered.provenance.span(provenance))
            .ok_or_else(|| ProviderError::Internal {
                message: format!(
                    "MIR node {} in module {:?} has no authored span",
                    node.id, self.id
                ),
            })
    }

    /// Return whether one MIR node traces to authored source.
    pub fn is_authored(&self, node: mir::LocalNodeIdAny) -> bool {
        let provenance = self.lowered.tree.provenance(node.id);

        self.lowered.provenance.span(provenance).is_some()
    }

    /// Return a source anchor for one MIR node.
    pub fn anchor(&self, node: mir::LocalNodeIdAny) -> Result<DiagnosticAnchor, ProviderError> {
        let span = self.span(node)?;

        Ok(DiagnosticAnchor::Span(span))
    }

    /// Return one call operation when it invokes a canonical language member.
    pub fn language_call<'a>(
        &'a self,
        instruction: mir::LocalNodeId<mir::Instruction>,
        member: dir::LanguageMember,
        resolution: &mir::ResolutionTable,
    ) -> Option<&'a mir::Call> {
        let operation = self.lowered.tree.get(instruction);
        let mir::Instruction::Call { call, .. } = operation else {
            return None;
        };

        // resolve direct and statically dispatched calls
        let callsite = mir::CallSite::Instruction(instruction);
        let target = operation
            .call_direct_target()
            .or_else(|| resolution.target(callsite))?;

        self.implements_language_member(target, member)
            .then_some(call)
    }

    /// Return whether one MIR declaration implements a canonical language member.
    fn implements_language_member<T>(
        &self,
        node: mir::LocalNodeId<T>,
        member: dir::LanguageMember,
    ) -> bool
    where
        T: mir::Node,
    {
        let owner = destack_core::StringId::for_text(&member.owner.key());
        let key = match member.key {
            dir::StaticKey::Name(name) => mir::StaticKey::Name(name),
            dir::StaticKey::Index(index) => mir::StaticKey::Index(index as u64),
        };
        let member = mir::LanguageMember { owner, key };

        self.lowered.language.implements_member(node, member)
    }
}
