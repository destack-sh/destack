use std::collections::HashSet;
use std::sync::Arc;

use destack_artifact::{
    ArtifactDependencySet, ArtifactKey, ArtifactPayload, MirAnalyzed, MirElaborated, MirLowered,
    ProgramAnalysis,
};
use destack_mir as mir;
use destack_repository::{ProfileId, ProviderContext};
use destack_source::{ModuleId, TargetId};

use crate::{Compiler, CompilerError, CompilerResult};

impl Compiler {
    /// Collect inputs for one module's MIR analysis.
    pub(crate) fn collect_mir_analyzed(
        &self,
        module: ModuleId,
        profile: ProfileId,
        target: TargetId,
        _context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactDependencySet> {
        let mut dependencies = ArtifactDependencySet::default();
        dependencies.require(ArtifactKey::mir_lowered(module, profile, target));
        dependencies.require(ArtifactKey::mir_elaborated(module, profile, target));

        Ok(dependencies)
    }

    /// Analyze symbol links for one module and target.
    pub(crate) fn provide_mir_analyzed(
        &self,
        module: ModuleId,
        profile: ProfileId,
        target: TargetId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactPayload> {
        let artifacts = self.artifact_reader(context);
        let lowered = artifacts
            .read::<MirLowered>((module, profile, target))
            .map_err(CompilerError::from)?;
        let elaborated = artifacts
            .read::<MirElaborated>((module, profile, target))
            .map_err(CompilerError::from)?;

        // analyze the elaborated tree's symbol links
        let mut analyses = mir::AnalysisCache::new();
        let links = analyses.link(
            &elaborated.tree,
            &elaborated.effects,
            &lowered.dispatch,
            &elaborated.drops,
        );

        Ok(ArtifactPayload::MirAnalyzed(Arc::new(MirAnalyzed::new(
            (*links).clone(),
        ))))
    }

    /// Collect inputs for the whole-program analysis of one profile and target.
    pub(crate) fn collect_program_analysis(
        &self,
        profile: ProfileId,
        target: TargetId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactDependencySet> {
        let modules = self
            .repository
            .module_ids(context.revision())
            .map_err(|error| CompilerError::Internal {
                message: format!("failed to enumerate profile modules: {error}"),
            })?;

        // depend on every module's symbol links
        let mut dependencies = ArtifactDependencySet::default();
        for module in modules {
            dependencies.require(ArtifactKey::mir_analyzed(module, profile, target));
        }

        Ok(dependencies)
    }

    /// Build the whole-program analysis for one profile and target.
    pub(crate) fn provide_program_analysis(
        &self,
        profile: ProfileId,
        target: TargetId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactPayload> {
        let revision = context.revision();
        let modules =
            self.repository
                .module_ids(revision)
                .map_err(|error| CompilerError::Internal {
                    message: format!("failed to enumerate profile modules: {error}"),
                })?;

        // the target's root modules anchor whole-program reachability
        let root_modules: HashSet<ModuleId> = self
            .repository
            .modules_for_target(revision, target)
            .map_err(|error| CompilerError::Internal {
                message: format!("failed to resolve target root modules: {error}"),
            })?
            .into_iter()
            .collect();
        let artifacts = self.artifact_reader(context);

        // load every module's links, seeding roots from each root module's exports
        let mut analyzed_modules: Vec<Arc<MirAnalyzed>> = Vec::new();
        let mut roots: Vec<mir::Symbol> = Vec::new();
        for module in modules {
            let analyzed = artifacts
                .read::<MirAnalyzed>((module, profile, target))
                .map_err(CompilerError::from)?;
            if root_modules.contains(&module) {
                for (symbol, node) in analyzed.links.nodes() {
                    if node.linkage().is_exported() {
                        roots.push(symbol);
                    }
                }
            }
            analyzed_modules.push(analyzed);
        }

        // build the supergraph and derive whole-program reachability
        let supergraph =
            mir::LinkSupergraph::build(analyzed_modules.iter().map(|analyzed| &analyzed.links));
        let analysis = ProgramAnalysis::analyze(&supergraph, &roots);

        Ok(ArtifactPayload::ProgramAnalysis(Arc::new(analysis)))
    }
}
