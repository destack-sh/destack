use std::sync::Arc;

use destack_artifact::{DiagnosticAnchor, EnvironmentBound, MirLowered, MirVerified};
use destack_core::StringPool;
use destack_dir as dir;
use destack_mir as mir;
use destack_repository::{ArtifactReader, ProfileId, ProviderError, Repository, Revision};
use destack_source::{ModuleId, Span, TargetId};

use crate::Dir;

/// Verified MIR and analyses for one module.
#[derive(Debug)]
pub struct MirModule<'a> {
    /// The module id.
    pub id: ModuleId,
    /// The MIR artifact.
    pub lowered: Arc<MirLowered>,
    /// Analyses of the verified MIR tree.
    pub analyses: mir::ModuleCache,
    /// The repository string pool.
    pub strings: Arc<StringPool>,
    /// The DIR the MIR lowered from, absent for parsed MIR.
    dir: Option<Dir<'a>>,
}

impl<'a> MirModule<'a> {
    /// Create one verified MIR module.
    pub(crate) fn new(
        id: ModuleId,
        lowered: Arc<MirLowered>,
        strings: Arc<StringPool>,
        dir: Option<Dir<'a>>,
    ) -> Self {
        let target_layout = lowered.target;
        let analyses = mir::ModuleCache::with_target_layout(target_layout);

        Self {
            id,
            lowered,
            analyses,
            strings,
            dir,
        }
    }

    /// Load one module's verified MIR beside its DIR.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn load(
        repository: &'a Repository,
        revision: Revision,
        artifacts: &ArtifactReader<'a>,
        profile: ProfileId,
        target: TargetId,
        module: ModuleId,
        strings: Arc<StringPool>,
        environment: Arc<EnvironmentBound>,
    ) -> Result<Self, ProviderError> {
        artifacts.read::<MirVerified>((module, profile, target))?;
        let lowered = artifacts.read::<MirLowered>((module, profile, target))?;
        let dir = Dir::load(
            repository,
            revision,
            artifacts,
            profile,
            environment,
            &[module],
            &[],
            &[],
        )?;

        Ok(Self::new(module, lowered, strings, Some(dir)))
    }

    /// Return one call operation whose expression selects a canonical language member.
    pub fn language_call(
        &self,
        instruction: mir::LocalNodeId<mir::Instruction>,
        member: dir::LanguageMember,
    ) -> Result<Option<&mir::Call>, ProviderError> {
        let mir::Instruction::Call { call, .. } = self.lowered.tree.get(instruction) else {
            return Ok(None);
        };
        let Some(dir) = &self.dir else {
            return Ok(None);
        };
        let Some(source) = self.lowered.tree.get_source(instruction.id) else {
            return Ok(None);
        };

        // read the member the lowered expression selects
        let module = dir.module(self.id)?;
        let node = dir::LocalNodeIdAny::new(source, module.view().get_node_type(source));
        let Ok(expression) = node.try_into_typed::<dir::Expression>() else {
            return Ok(None);
        };
        let selected = module.implemented_language_member(expression)?;

        Ok((selected == Some(member)).then_some(call))
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
