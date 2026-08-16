use std::sync::Arc;

use destack_artifact::{
    ArtifactDependencySet, ArtifactKey, ArtifactPayload, MirAnalyzed, MirElaborated, MirOptimized,
    ProgramAnalysis,
};
use destack_mir as mir;
use destack_mir::AnalysisOptions;
use destack_repository::{ProfileId, ProviderContext, Target};
use destack_source::{ModuleId, TargetId};

use crate::{
    Compiler, CompilerError, CompilerResult, OptimizeError, OptimizeResult, OptimizeWarning,
};

use super::{
    OptimizationLevel, OptimizeState, Pipeline, PipelineContext, PipelineOptions, default_pipeline,
};

impl Compiler {
    /// Collect inputs for optimized MIR of one module and target.
    pub(crate) fn collect_mir_optimized(
        &self,
        module: ModuleId,
        profile: ProfileId,
        target: TargetId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactDependencySet> {
        let mut dependencies = ArtifactDependencySet::default();
        dependencies.require(ArtifactKey::mir_elaborated(module, profile, target));

        // resolve the optimization program scope
        let target_config = self.optimization_target(&target, context)?;
        let level: OptimizationLevel = target_config.compiler.optimize.into();
        if level.uses_program_analysis() {
            dependencies.require(ArtifactKey::program_analysis(profile, target));
        } else {
            dependencies.require(ArtifactKey::mir_analyzed(module, profile, target));
        }

        self.observe_package_config(context, target.package_id(), &mut dependencies)?;

        Ok(dependencies)
    }

    /// Build optimized MIR for one module and target.
    pub(crate) fn provide_mir_optimized(
        &self,
        module: ModuleId,
        profile: ProfileId,
        target: TargetId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactPayload> {
        let state = OptimizeState::new(module, profile, target, context);

        // optimize the module
        let payload =
            self.optimize_module(state.module, state.profile, &state.target, state.context)?;

        Ok(ArtifactPayload::MirOptimized(Arc::new(payload)))
    }

    /// Optimize a module's MIR.
    fn optimize_module(
        &self,
        module: ModuleId,
        profile: ProfileId,
        target: &TargetId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<MirOptimized> {
        let package_id = target.package_id();

        let resolved_profile = self.profile_id_for_target(context.revision(), target)?;
        if resolved_profile != profile {
            return Err(OptimizeError::InvalidTarget {
                anchor: package_id.into(),
                package: package_id,
                target: *target,
                message: format!("resolved to profile '{resolved_profile:?}', not '{profile:?}'"),
            }
            .into());
        }

        // resolve pipeline for this target
        let target_config = self.optimization_target(target, context)?;
        let level = target_config.compiler.optimize.into();
        let pipeline = default_pipeline(level);

        // load provider inputs
        let artifacts = self.artifact_reader(context);
        let elaborated = artifacts
            .read::<MirElaborated>((module, profile, *target))
            .map_err(CompilerError::from)?;
        let mut optimized = MirOptimized {
            tree: elaborated.tree.clone(),
            target: elaborated.target,
            layouts: elaborated.layouts.clone(),
            dispatch: elaborated.dispatch.clone(),
            drops: elaborated.drops.clone(),
            accesses: elaborated.accesses.clone(),
            effects: elaborated.effects.clone(),
            profile: elaborated.profile.clone(),
        };

        // load analysis for the optimization program scope
        let program_analysis = if level.uses_program_analysis() {
            artifacts
                .read::<ProgramAnalysis>((profile, *target))
                .map_err(CompilerError::from)?
        } else {
            let analyzed = artifacts
                .read::<MirAnalyzed>((module, profile, *target))
                .map_err(CompilerError::from)?;
            Arc::new(module_scoped_analysis(&analyzed.links))
        };
        let strings = self.repository.string_pool().clone();

        // resolve pipeline options
        let options = Self::pipeline_options(elaborated.target, level);

        // run the pipeline
        let mut pipeline_context = PipelineContext::new(
            &strings,
            options,
            module,
            profile,
            *target,
            None,
            program_analysis,
        );
        pipeline.run(&mut optimized, &mut pipeline_context);

        // collect accumulated diagnostics from verification passes
        for error in pipeline_context.take_errors() {
            self.emit_diagnostic::<OptimizeError>(context, error)?;
        }
        for warning in pipeline_context.take_warnings() {
            self.emit_diagnostic::<OptimizeWarning>(context, warning)?;
        }

        Ok(optimized)
    }

    /// Resolve pipeline options for one target.
    fn pipeline_options(
        target_layout: mir::TargetLayout,
        level: OptimizationLevel,
    ) -> PipelineOptions {
        PipelineOptions {
            analysis: AnalysisOptions::new(target_layout),
            unroll_threshold: level.unroll_threshold(),
            inline_budget_scale_percent: level.inline_budget_scale_percent(),
            ..Default::default()
        }
    }

    /// Resolve the target configuration for optimization.
    fn optimization_target(
        &self,
        target: &TargetId,
        context: &dyn ProviderContext,
    ) -> OptimizeResult<Target> {
        self.target_or_builtin(context, *target)
            .map_err(|error| OptimizeError::InvalidTarget {
                anchor: target.package_id().into(),
                package: target.package_id(),
                target: *target,
                message: format!("{error:?}"),
            })?
            .ok_or_else(|| OptimizeError::InvalidTarget {
                anchor: target.package_id().into(),
                package: target.package_id(),
                target: *target,
                message: "target not found".to_string(),
            })
    }
}

/// Analyze a single module as a standalone program over its own link graph.
///
/// Compiled alone, a module is an open world: anything it exports may be called from
/// outside, so every exported symbol is a root and reachability stays within the module.
fn module_scoped_analysis(links: &mir::LinkTable) -> ProgramAnalysis {
    let roots: Vec<mir::Symbol> = links
        .nodes()
        .filter(|(_, node)| node.linkage().is_exported())
        .map(|(symbol, _)| symbol)
        .collect();

    let supergraph = mir::LinkSupergraph::build([links]);
    ProgramAnalysis::analyze(&supergraph, &roots)
}
