use crate::timing::tags;
use crate::{Compiler, CompilerContext, OptimizeError, OptimizeResult, RequirementError};
use std::mem;
use std::str::FromStr;

use destack_artifact::{ArtifactKey, EmitFormat, MirOptimized, TargetArch};
use destack_source::{ModuleId, PackageId, TargetId};
use destack_workspace::{
    DebugMode, OptimizeLevel as WorkspaceOptimizeLevel, ProfileId, Repository, Target,
};
use target_lexicon::Triple;

use super::{
    OptimizationLevel, Pipeline, PipelineContext, PipelineOptions, PipelineTarget, TypeContext,
    count_mir_size, default_pipeline,
};

/// Load one module snapshot for the current optimize pass.
impl Compiler {
    /// Return the package id for one target.
    fn optimize_target_package_id(&self, target: &TargetId) -> OptimizeResult<PackageId> {
        Ok(target.package_id())
    }

    /// Build optimized MIR for one module and target.
    pub fn process_mir_optimized(
        &self,
        module: ModuleId,
        profile: ProfileId,
        target: TargetId,
        context: &CompilerContext<'_>,
    ) -> OptimizeResult<()> {
        let revision = context.revision();
        let _timing = self.timing_scope(tags::OPTIMIZE_MODULE);
        let artifact_key = ArtifactKey::mir_optimized(module, profile, target);
        let artifact_stamp = context.artifact_stamp(&artifact_key);

        // reuse one persisted optimized mir image when available
        if context
            .restore_cached_artifact(
                artifact_key,
                |compiler| {
                    compiler.load_mir_optimized_image(
                        revision,
                        module,
                        artifact_stamp,
                        profile,
                        &target,
                    )
                },
                |store, version, payload| store.publish_mir_optimized(version, payload),
            )
            .is_some()
        {
            return Ok(());
        }

        // require MIR for this module and target
        self.require_mir(context.revision(), module, profile, &target)?;

        // optimize the module
        let payload = self.optimize_module(module, profile, &target, context)?;
        context.publish_artifact(artifact_key, payload.clone(), |store, version, payload| {
            store.publish_mir_optimized(version, payload)
        });
        context.store_artifact(&artifact_key, &payload, |compiler, artifact_stamp, mir| {
            compiler.store_mir_optimized_image(
                revision,
                module,
                profile,
                &target,
                artifact_stamp,
                mir,
            )
        });

        Ok(())
    }

    /// Ensure optimized MIR exists for one module and target.
    pub fn require_mir_optimized(
        &self,
        module: ModuleId,
        profile: ProfileId,
        target: &TargetId,
        context: &CompilerContext<'_>,
    ) -> Result<(), RequirementError> {
        let package_id = target.package_id();

        let resolved_profile = context.profile_id_for_target(module, target);
        if resolved_profile != Some(profile) {
            self.error(OptimizeError::InvalidTarget {
                package: package_id,
                target: *target,
                message: "target not found for profile resolution".to_string(),
            });
            return Ok(());
        }

        self.require_artifact(
            context.revision(),
            ArtifactKey::mir_optimized(module, profile, *target),
        )
    }

    /// Optimize a module's MIR.
    fn optimize_module(
        &self,
        module: ModuleId,
        profile: ProfileId,
        target: &TargetId,
        context: &CompilerContext<'_>,
    ) -> OptimizeResult<MirOptimized> {
        let package_id = self.optimize_target_package_id(target)?;

        let resolved_profile = context
            .profile_id_for_target(module, target)
            .ok_or_else(|| OptimizeError::InvalidTarget {
                package: package_id,
                target: *target,
                message: "target not found for profile resolution".to_string(),
            })?;
        if resolved_profile != profile {
            return Err(OptimizeError::InvalidTarget {
                package: package_id,
                target: *target,
                message: format!("resolved to profile '{resolved_profile:?}', not '{profile:?}'"),
            });
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
        let mir = self
            .mir_base(module, profile, target)
            .unwrap_or_else(|| unreachable!("missing MIR artifact for target '{target}'"));
        let mut tree = mir.tree.clone();
        let strings = mir.strings.clone();
        let profile_data = mir.profile.clone();

        // resolve pipeline options
        let module_ref = context.module(module);
        let options =
            self.pipeline_options_for_module(module_ref.as_ref(), &target_config, level, context);

        // count mir size before optimization
        let before = count_mir_size(&tree);

        // run the pipeline
        let mut context =
            PipelineContext::new(&strings, options, module, *target, profile_data.clone());
        pipeline.run(&mut tree, &mut context);

        // collect accumulated diagnostics from verification passes
        for error in context.take_errors() {
            self.error(error);
        }
        for warning in context.take_warnings() {
            self.warning(warning);
        }

        // count mir size after optimization
        let after = count_mir_size(&tree);

        // freeze optimized MIR
        let payload = MirOptimized {
            id: module,
            target: *target,
            tree,
            strings,
            profile: profile_data,
        };

        // record metrics
        self.stats.record_optimize();
        self.stats.record_optimize_mir(
            before.functions,
            before.instructions,
            after.instructions,
            before.blocks,
            after.blocks,
        );

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
        module: &destack_workspace::Module,
        target: &Target,
        level: OptimizationLevel,
        context: &CompilerContext<'_>,
    ) -> PipelineOptions {
        // resolve compiler options and derived restrictions for this target
        let compiler_options = context
            .compiler_options_for_module(module)
            .unwrap_or_default();
        let compiler_options = Repository::compiler_options_for_target(target, &compiler_options);

        // resolve strict borrow mode from effective options
        let strict_borrow_mode = compiler_options.borrow_mode.is_strict();

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
            strict_borrow_mode,
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
        context: &CompilerContext<'_>,
    ) -> OptimizeResult<Target> {
        self.repository
            .target(context.revision(), *target)
            .map_err(|error| OptimizeError::InvalidTarget {
                package: PackageId::EPHEMERAL,
                target: *target,
                message: format!("failed to load target config: {error}"),
            })?
            .ok_or_else(|| OptimizeError::InvalidTarget {
                package: PackageId::EPHEMERAL,
                target: *target,
                message: "target not found".to_string(),
            })
    }
}
