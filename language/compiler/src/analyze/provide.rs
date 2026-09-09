use std::collections::HashSet;
use std::sync::Arc;

use destack_artifact::{
    ArtifactDependencySet, ArtifactKey, ArtifactPayload, MirAnalyzed, MirElaborated, ModuleGraph,
    ProgramAnalysis,
};
use destack_mir as mir;
use destack_repository::{ProfileId, ProviderContext, ProviderError};
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
        // require elaborated MIR for symbol analysis
        let mut dependencies = ArtifactDependencySet::default();
        dependencies.require_payload(ArtifactKey::mir_elaborated(module, profile, target));

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
        // read the elaborated module
        let artifacts = self.artifact_reader(context);
        let elaborated = artifacts
            .read::<MirElaborated>((module, profile, target))
            .map_err(CompilerError::from)?;

        // analyze the elaborated tree's symbol links
        let mut analyses = mir::AnalysisCache::new();
        let links = analyses.link(
            &elaborated.tree,
            &elaborated.effects,
            &elaborated.dispatch,
            &elaborated.drops,
        );

        // record the module initializer symbol
        let initializer = elaborated
            .initializer
            .map(|function| elaborated.tree.get(function).symbol);

        Ok(ArtifactPayload::MirAnalyzed(Arc::new(MirAnalyzed {
            links: (*links).clone(),
            initializer,
        })))
    }

    /// Collect inputs for the whole-program analysis of one profile and target.
    pub(crate) fn collect_program_analysis(
        &self,
        profile: ProfileId,
        target: TargetId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactDependencySet> {
        // track the module graph and target configuration
        let mut dependencies = ArtifactDependencySet::default();
        dependencies.require_payload(ArtifactKey::module_graph(profile));
        self.observe_package_config(context, target.package_id(), &mut dependencies)?;

        // read the module graph or request another collection attempt
        let artifacts = self.artifact_reader(context);
        let graph = match artifacts.read::<ModuleGraph>(profile) {
            Ok(graph) => graph,
            Err(ProviderError::Blocked { .. }) => {
                dependencies.mark_partial();

                return Ok(dependencies);
            }
            Err(error) => return Err(error.into()),
        };

        // require symbol analysis for every module imported from the target roots
        let roots = self.analysis_roots(target, &graph, context)?;
        for module in graph.reachable(&roots) {
            dependencies.require_payload(ArtifactKey::mir_analyzed(module, profile, target));
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
        // read the module graph
        let artifacts = self.artifact_reader(context);
        let graph = artifacts
            .read::<ModuleGraph>(profile)
            .map_err(CompilerError::from)?;

        // select the target roots and their imported modules
        let root_modules = self.analysis_roots(target, &graph, context)?;
        let modules = graph.reachable(&root_modules);
        let root_modules: HashSet<ModuleId> = root_modules.into_iter().collect();

        // collect symbol links and the runtime entry points
        let mut analyzed_modules: Vec<Arc<MirAnalyzed>> = Vec::new();
        let mut roots: Vec<mir::Symbol> = Vec::new();
        for module in modules {
            let analyzed = artifacts
                .read::<MirAnalyzed>((module, profile, target))
                .map_err(CompilerError::from)?;

            // add the module initializer to the program roots
            if let Some(initializer) = analyzed.initializer {
                roots.push(initializer);
            }

            // add exports from target root modules to the program roots
            if root_modules.contains(&module) {
                for (symbol, node) in analyzed.links.nodes() {
                    if node.linkage().is_exported() {
                        roots.push(symbol);
                    }
                }
            }

            // retain the module links for the program graph
            analyzed_modules.push(analyzed);
        }

        // build the supergraph and derive whole-program reachability
        let supergraph =
            mir::LinkSupergraph::build(analyzed_modules.iter().map(|analyzed| &analyzed.links));
        let analysis = ProgramAnalysis::analyze(&supergraph, &roots);

        Ok(ArtifactPayload::ProgramAnalysis(Arc::new(analysis)))
    }

    /// Resolve target root modules and require each root to occur in the module graph.
    fn analysis_roots(
        &self,
        target: TargetId,
        graph: &ModuleGraph,
        context: &dyn ProviderContext,
    ) -> CompilerResult<Vec<ModuleId>> {
        // resolve the modules selected by the target configuration
        let roots = self
            .repository
            .modules_for_target(context.revision(), target)
            .map_err(|error| CompilerError::Internal {
                message: format!("failed to resolve target root modules: {error}"),
            })?;

        // require every target root to occur in the module graph
        for root in &roots {
            if !graph.contains(*root) {
                return Err(CompilerError::Internal {
                    message: format!(
                        "program analysis root {root:?} is absent from its module graph"
                    ),
                });
            }
        }

        Ok(roots)
    }
}
