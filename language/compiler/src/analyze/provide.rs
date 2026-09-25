use std::collections::HashSet;
use std::sync::Arc;

use tspp_artifact::{
    ArtifactDependencySet, ArtifactKey, ArtifactPayload, MirAnalyzed, MirElaborated, ModuleGraph,
    ProgramAnalysis,
};
use tspp_mir as mir;
use tspp_repository::{ArtifactReader, ProfileId, ProviderContext, ProviderError};
use tspp_source::{ModuleId, PackageId, TargetId};

use crate::{Compiler, CompilerError, CompilerResult};

use super::{module, program};

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

    /// Analyse symbol links and extract function effects for one module and target.
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

        // analyse the module and publish its extracted results
        let analyzed = module::analyse(&elaborated)?;

        Ok(ArtifactPayload::MirAnalyzed(Arc::new(analyzed)))
    }

    /// Collect inputs for the whole-program analysis of one profile and target.
    pub(crate) fn collect_program_analysis(
        &self,
        profile: ProfileId,
        target: TargetId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactDependencySet> {
        // track the module graphs the target package sees and the target configuration
        let mut dependencies = ArtifactDependencySet::default();
        let artifacts = self.artifact_reader(context);
        let packages = artifacts
            .package_closure(target.package_id())
            .map_err(CompilerError::from)?;
        for package in &packages {
            dependencies.require_payload(ArtifactKey::module_graph(*package, profile));
        }
        self.observe_package_config(context, target.package_id(), &mut dependencies)?;

        // read the module graphs or request another collection attempt
        let graphs = match self.module_graphs(&artifacts, &packages, profile) {
            Ok(graphs) => graphs,
            Err(ProviderError::Blocked { .. }) => {
                dependencies.mark_partial();

                return Ok(dependencies);
            }
            Err(error) => return Err(error.into()),
        };

        // require symbol analysis for every module imported from the target roots
        let (_, modules) = self.analysis_modules(target, &graphs, context)?;
        for module in modules {
            dependencies.require_payload(ArtifactKey::mir_analyzed(module, profile, target));
        }

        Ok(dependencies)
    }

    /// Return one target's root modules and every module they import across the package graphs.
    fn analysis_modules(
        &self,
        target: TargetId,
        graphs: &[Arc<ModuleGraph>],
        context: &dyn ProviderContext,
    ) -> CompilerResult<(Vec<ModuleId>, Vec<ModuleId>)> {
        // select the graph of the target's package
        let package = target.package_id();
        let graph = graphs
            .iter()
            .find(|graph| graph.package() == package)
            .ok_or_else(|| {
                ProviderError::internal(format!(
                    "no module graph for the target package '{package}'"
                ))
            })?;

        // walk the imports of the target roots across every graph
        let roots = self.analysis_roots(target, graph, context)?;
        let modules = ModuleGraph::reachable_across(graphs, &roots).map_err(|module| {
            ProviderError::internal(format!("no module graph holds module '{module}'"))
        })?;

        Ok((roots, modules))
    }

    /// Read the module graphs of one package list under one profile.
    fn module_graphs(
        &self,
        artifacts: &ArtifactReader<'_>,
        packages: &[PackageId],
        profile: ProfileId,
    ) -> Result<Vec<Arc<ModuleGraph>>, ProviderError> {
        packages
            .iter()
            .map(|package| artifacts.read::<ModuleGraph>((*package, profile)))
            .collect()
    }

    /// Build the whole-program analysis for one profile and target.
    pub(crate) fn provide_program_analysis(
        &self,
        profile: ProfileId,
        target: TargetId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactPayload> {
        // read the module graphs the target package sees
        let artifacts = self.artifact_reader(context);
        let packages = artifacts
            .package_closure(target.package_id())
            .map_err(CompilerError::from)?;
        let graphs = self
            .module_graphs(&artifacts, &packages, profile)
            .map_err(CompilerError::from)?;

        // select the target roots and their imported modules
        let (root_modules, modules) = self.analysis_modules(target, &graphs, context)?;
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

        // reuse recursive components against the retained program artifact
        let previous = context
            .artifact_base()
            .map(|base| {
                base.artifact::<ProgramAnalysis>()
                    .ok_or_else(|| CompilerError::Internal {
                        message: "program analysis base has the wrong artifact type".to_string(),
                    })
            })
            .transpose()?;

        // analyse the program and publish its derived results
        let analysis = program::analyse(&analyzed_modules, &roots, previous.as_deref())?;

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
