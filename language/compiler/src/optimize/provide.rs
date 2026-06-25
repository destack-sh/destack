use crate::{Compiler, CompilerResult, OptimizeError, OptimizeResult, OptimizeWarning};
use destack_repository::ProviderContext;
use std::mem;
use std::str::FromStr;
use std::sync::Arc;

use destack_artifact::{
    ArtifactDependencySet, ArtifactKey, ArtifactPayload, EmitFormat, MirOptimized, ProgramAnalysis,
    TargetArch,
};
use destack_mir as mir;
use destack_repository::{Module, ProfileId, Target};
use destack_source::{ModuleId, TargetId};
use target_lexicon::Triple;

use super::{
    OptimizationLevel, OptimizeState, Pipeline, PipelineContext, PipelineOptions, default_pipeline,
};
use crate::CompilerError;
use destack_mir::{AnalysisOptions, TargetLayout};

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
        dependencies.require(ArtifactKey::mir_verified(module, profile, target));

        // resolve the optimization program scope
        let target_config = self.target_for_module(module, &target, context)?;
        let level = self.optimization_level_for_target_config(&target_config);
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

        let resolved_profile = self.profile_id_for_target(context.revision(), module, target)?;
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
        let target_config = self.target_for_module(module, target, context)?;
        let level = self.optimization_level_for_target_config(&target_config);
        let pipeline = default_pipeline(level, target_config.uses_native_emit_pipeline());

        // load provider inputs
        let artifacts = self.artifact_reader(context.revision());
        let verified = artifacts
            .mir_verified(module, profile, *target)
            .map_err(CompilerError::from)?;
        let mut tree = verified.patch.tree.clone();

        // load analysis for the optimization program scope
        let program_analysis = if level.uses_program_analysis() {
            artifacts
                .program_analysis(profile, *target)
                .map_err(CompilerError::from)?
        } else {
            let analyzed = artifacts
                .mir_analyzed(module, profile, *target)
                .map_err(CompilerError::from)?;
            Arc::new(module_scoped_analysis(&analyzed.links))
        };
        let strings = self.repository.string_pool().clone();

        // resolve pipeline options
        let module_ref = self.module(context.revision(), module)?;
        let options =
            self.pipeline_options_for_module(module_ref.as_ref(), &target_config, level, context);

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
        pipeline.run(&mut tree, &mut pipeline_context);

        // collect accumulated diagnostics from verification passes
        for error in pipeline_context.take_errors() {
            self.emit_diagnostic::<OptimizeError>(context, error)?;
        }
        for warning in pipeline_context.take_warnings() {
            self.emit_diagnostic::<OptimizeWarning>(context, warning)?;
        }

        // freeze optimized MIR patch
        let payload = MirOptimized {
            patches: vec![mir::Patch::from_tree("optimize", tree)],
        };

        Ok(payload)
    }

    /// Resolve the optimization level for a target configuration.
    fn optimization_level_for_target_config(&self, target: &Target) -> OptimizationLevel {
        target.compiler.optimize.into()
    }

    /// Resolve pipeline options for a module and target.
    fn pipeline_options_for_module(
        &self,
        _module: &Module,
        target: &Target,
        level: OptimizationLevel,
        _context: &dyn ProviderContext,
    ) -> PipelineOptions {
        // resolve pointer width from target configuration
        let pointer_width_bits = self.pointer_width_bits_for_target(target);

        PipelineOptions {
            analysis: AnalysisOptions::new(TargetLayout { pointer_width_bits }),
            unroll_threshold: level.unroll_threshold(),
            inline_budget_scale_percent: level.inline_budget_scale_percent(),
            ..Default::default()
        }
    }

    /// Resolve pointer width in bits for a target configuration.
    fn pointer_width_bits_for_target(&self, target: &Target) -> u16 {
        // prefer explicit triple for pointer width
        if let Some(triple) = target.resolved_target_triple()
            && let Ok(triple) = Triple::from_str(&triple)
            && let Ok(pointer_width) = triple.pointer_width()
        {
            let bits = pointer_width.bits() as u16;
            if matches!(bits, 16 | 32 | 64) {
                return bits;
            }
        }

        if let Some(target_arch) = target.native.arch.as_ref() {
            let bits = match target_arch {
                TargetArch::X86_64
                | TargetArch::Aarch64
                | TargetArch::Riscv64
                | TargetArch::PowerPc64
                | TargetArch::PowerPc64le
                | TargetArch::S390x
                | TargetArch::Mips64
                | TargetArch::Mips64el
                | TargetArch::LoongArch64
                | TargetArch::Wasm64 => 64,
                TargetArch::X86
                | TargetArch::Armv7
                | TargetArch::Armv6
                | TargetArch::Riscv32
                | TargetArch::Wasm32 => 32,
                TargetArch::Other(_) => (mem::size_of::<usize>() * 8) as u16,
            };
            if matches!(bits, 16 | 32 | 64) {
                return bits;
            }
        }

        // fall back to output defaults
        let bits = match target.emit {
            EmitFormat::Wasm => 32,
            EmitFormat::Native => (mem::size_of::<usize>() * 8) as u16,
            _ => (mem::size_of::<usize>() * 8) as u16,
        };

        // ensure supported sizes
        if matches!(bits, 16 | 32 | 64) {
            bits
        } else {
            (mem::size_of::<usize>() * 8) as u16
        }
    }

    /// Resolve the target configuration for a module.
    fn target_for_module(
        &self,
        _module: ModuleId,
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
fn module_scoped_analysis(links: &mir::LinkGraph) -> ProgramAnalysis {
    let roots: Vec<mir::Symbol> = links
        .nodes()
        .filter(|(_, node)| node.linkage().is_exported())
        .map(|(symbol, _)| symbol)
        .collect();

    let supergraph = mir::LinkSupergraph::build([links]);
    ProgramAnalysis::analyze(&supergraph, &roots)
}
