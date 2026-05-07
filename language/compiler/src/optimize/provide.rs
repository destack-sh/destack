use crate::{Compiler, CompilerResult, OptimizeError, OptimizeResult};
use destack_workspace::ProviderContext;
use std::mem;
use std::str::FromStr;

use destack_artifact::{ArtifactKey, ArtifactPayload, EmitFormat, MirOptimized, TargetArch};
use destack_source::{ModuleId, PackageId, TargetId};
use destack_workspace::{
    DebugMode, Module, OptimizeLevel as WorkspaceOptimizeLevel, ProfileId, Target,
};
use target_lexicon::Triple;

use super::{
    OptimizationLevel, Pipeline, PipelineContext, PipelineOptions, PipelineTarget, TypeContext,
    default_pipeline,
};
use crate::CompilerError;
use crate::optimize::OptimizeState;

impl Compiler {
    /// Build optimized MIR for one module and target.
    pub(crate) fn provide_mir_optimized(
        &self,
        module: ModuleId,
        profile: ProfileId,
        target: TargetId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactPayload> {
        let state = OptimizeState::new(module, profile, target, context);
        let artifact_key = ArtifactKey::mir_optimized(state.module, state.profile, state.target);

        // require MIR before deriving the optimized artifact version
        self.require_mir_lowered(state.context, state.module, state.profile, &state.target)
            .map_err(CompilerError::from)?;

        // optimize the module
        let payload =
            self.optimize_module(state.module, state.profile, &state.target, state.context)?;

        assert_eq!(
            artifact_key,
            state.context.artifact_key(),
            "compiler attempted to provide the wrong artifact"
        );

        Ok(ArtifactPayload::MirOptimized(payload))
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

        let resolved_profile = self
            .target_profile_id(context.revision(), module, target)
            .ok_or_else(|| OptimizeError::InvalidTarget {
                anchor: package_id.into(),
                package: package_id,
                target: *target,
                message: "target not found for profile resolution".to_string(),
            })?;
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
        let pipeline_target = if matches!(target_config.debug_mode, DebugMode::Vm) {
            PipelineTarget::Vm
        } else {
            PipelineTarget::Native
        };
        let pipeline = default_pipeline(level, pipeline_target);

        // read committed MIR artifact truth
        let source_mir = context
            .require(ArtifactKey::mir_lowered(module, profile, *target))
            .map_err(CompilerError::from)?;
        let mir = self
            .mir_lowered(context, module, profile, target)
            .map_err(CompilerError::from)?;
        let mut tree = mir.tree.clone();
        let strings = mir.strings.clone();

        // resolve pipeline options
        let module_ref = self.module(context.revision(), module);
        let options =
            self.pipeline_options_for_module(module_ref.as_ref(), &target_config, level, context);

        // run the pipeline
        let mut pipeline_context = PipelineContext::with_source_mir(
            &strings, options, module, profile, *target, source_mir, None,
        );
        pipeline.run(&mut tree, &mut pipeline_context);

        // collect accumulated diagnostics from verification passes
        for error in pipeline_context.take_errors() {
            self.emit_built_diagnostic(context, error)?;
        }
        for warning in pipeline_context.take_warnings() {
            self.emit_built_diagnostic(context, warning)?;
        }

        // freeze optimized MIR
        let payload = MirOptimized { tree, strings };

        Ok(payload)
    }

    /// Resolve the optimization level for a target configuration.
    fn optimization_level_for_target_config(&self, target: &Target) -> OptimizationLevel {
        // honor disabled optimization
        if !target.optimize {
            return OptimizationLevel::O0;
        }

        // map target optimize level to pipeline level
        match target.optimize_level {
            WorkspaceOptimizeLevel::O0 => OptimizationLevel::O0,
            WorkspaceOptimizeLevel::O1 => OptimizationLevel::O1,
            WorkspaceOptimizeLevel::O2 => OptimizationLevel::O2,
            WorkspaceOptimizeLevel::O3 => OptimizationLevel::O3,
            WorkspaceOptimizeLevel::O4 => OptimizationLevel::O4,
        }
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

        // resolve unroll threshold
        let unroll_threshold = target
            .unroll_threshold
            .unwrap_or_else(|| Self::unroll_threshold_for_level(level));
        let unroll_threshold = unroll_threshold.min(usize::MAX as u64) as usize;

        let inline_budget_scale_percent = target
            .inline_budget_scale_percent
            .unwrap_or_else(|| Self::inline_budget_scale_percent_for_level(level));

        let require_optimized_metadata = matches!(
            level,
            OptimizationLevel::O2 | OptimizationLevel::O3 | OptimizationLevel::O4
        );
        let pipeline_target = if matches!(target.debug_mode, DebugMode::Vm) {
            PipelineTarget::Vm
        } else {
            PipelineTarget::Native
        };

        PipelineOptions {
            strict_borrow_mode: true,
            target: pipeline_target,
            float_math: target.float_math,
            type_context: TypeContext { pointer_width_bits },
            unroll_threshold,
            inline_budget_scale_percent,
            require_optimized_metadata,
            ..Default::default()
        }
    }

    /// Resolve unroll threshold from an optimization level.
    fn unroll_threshold_for_level(level: OptimizationLevel) -> u64 {
        match level {
            OptimizationLevel::O0 => 0,
            OptimizationLevel::O1 => 0,
            OptimizationLevel::O2 => 200,
            OptimizationLevel::O3 => 300,
            OptimizationLevel::O4 => 400,
        }
    }

    /// Resolve inline budget scale percent from an optimization level.
    fn inline_budget_scale_percent_for_level(level: OptimizationLevel) -> u64 {
        match level {
            OptimizationLevel::O0 => 50,
            OptimizationLevel::O1 => 75,
            OptimizationLevel::O2 => 100,
            OptimizationLevel::O3 => 140,
            OptimizationLevel::O4 => 180,
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

        if let Some(target_arch) = target.target_arch.as_ref() {
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
        self.effective_target(context, *target)
            .ok_or_else(|| OptimizeError::InvalidTarget {
                anchor: PackageId::EPHEMERAL.into(),
                package: PackageId::EPHEMERAL,
                target: *target,
                message: "target not found".to_string(),
            })
    }
}
