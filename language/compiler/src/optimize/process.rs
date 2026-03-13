use std::mem;
use std::str::FromStr;
use std::sync::Arc;

use crate::timing::tags;
use crate::{
    BuildKey, BuildProduct, BuildRequirementError, Compiler, OptimizeError, OptimizeResult,
};

use destack_source::{ModuleId, ModuleVersion, PackageId, ProfileVersion};
use destack_workspace::{
    ArtifactKey, DebugMode, Module, ModuleMirData, OptimizeLevel as WorkspaceOptimizeLevel,
    OutputFormat, ProfileId, Target, TargetArch, TargetId,
};
use target_lexicon::Triple;

use super::{
    OptimizationLevel, Pipeline, PipelineContext, PipelineOptions, PipelineTarget, TypeContext,
    count_mir_size, default_pipeline,
};

impl Compiler {
    /// Build optimized MIR for one module and target.
    pub fn process_mir_optimized(
        &self,
        module: ModuleId,
        profile: ProfileId,
        target: TargetId,
    ) -> OptimizeResult<BuildProduct> {
        let module_stamp = self.module_stamp(module);
        let profile_stamp = self.profile_stamp(profile);
        self.ensure_module_profile_matches::<OptimizeError>(
            module_stamp.id,
            module_stamp.version,
            profile_stamp.id,
            profile_stamp.version,
        )?;
        let _timing = self.timing_scope(tags::OPTIMIZE_MODULE);

        // require MIR for this module and target
        self.require_mir(module_stamp.id, profile_stamp.id, &target)?;

        // optimize the module
        self.optimize_module(
            module_stamp.id,
            profile_stamp.id,
            module_stamp.version,
            profile_stamp.version,
            &target,
        )
    }

    /// Ensure optimized MIR exists for one module and target.
    pub fn require_mir_optimized(
        &self,
        module: ModuleId,
        profile: ProfileId,
        target: &TargetId,
    ) -> Result<(), BuildRequirementError> {
        let resolved_profile = self.program.profile_id_for_target(module, target);
        if resolved_profile != Some(profile) {
            self.error(OptimizeError::InvalidTarget {
                package: target.package_id,
                target: target.clone(),
                message: "target not found for profile resolution".to_string(),
            });
            return Ok(());
        }

        self.require_build_key(BuildKey::Artifact(ArtifactKey::MirOptimized {
            module,
            profile,
            target: target.clone(),
        }))
    }

    /// Optimize a module's MIR.
    fn optimize_module(
        &self,
        module: ModuleId,
        profile: ProfileId,
        module_version: ModuleVersion,
        profile_version: ProfileVersion,
        target: &TargetId,
    ) -> OptimizeResult<BuildProduct> {
        // skip stale tasks
        self.ensure_module_profile_matches::<OptimizeError>(
            module,
            module_version,
            profile,
            profile_version,
        )?;

        let resolved_profile = self
            .program
            .profile_id_for_target(module, target)
            .ok_or_else(|| OptimizeError::InvalidTarget {
                package: target.package_id,
                target: target.clone(),
                message: "target not found for profile resolution".to_string(),
            })?;
        if resolved_profile != profile {
            return Err(OptimizeError::InvalidTarget {
                package: target.package_id,
                target: target.clone(),
                message: format!("resolved to profile '{resolved_profile:?}', not '{profile:?}'"),
            });
        }

        // resolve pipeline for this target
        let target_config = self.target_for_module(module, target)?;
        let level = self.optimization_level_for_target_config(&target_config);
        let pipeline_target = if matches!(target_config.debug_mode, DebugMode::Vm) {
            PipelineTarget::Vm
        } else {
            PipelineTarget::Native
        };
        let pipeline = default_pipeline(level, pipeline_target);

        // read committed MIR artifact truth
        let mir = self
            .program
            .artifacts
            .mir(module, profile, target)
            .unwrap_or_else(|| unreachable!("missing MIR artifact for target '{target}'"));
        let mut tree = mir.tree.clone();
        let strings = mir.strings.clone();
        let profile_data = mir
            .profile
            .as_ref()
            .map(|profile: &destack_mir::ProfileTable| Arc::new(profile.clone()));

        // resolve pipeline options
        let module_ref = self.program.modules.get(module);
        let module_guard = module_ref.read();
        let options = self.pipeline_options_for_module(&module_guard, &target_config, level);

        // count mir size before optimization
        let before = count_mir_size(&tree);

        // run the pipeline
        let mut context = PipelineContext::new(
            &strings,
            options,
            module,
            target.clone(),
            profile_data.clone(),
        );
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
        let payload = ModuleMirData {
            id: module,
            version: module_version,
            target: target.clone(),
            tree,
            strings,
            profile: profile_data
                .map(|profile: Arc<destack_mir::ProfileTable>| profile.as_ref().clone()),
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

        Ok(BuildProduct::Mir(payload))
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
        module: &Module,
        target: &Target,
        level: OptimizationLevel,
    ) -> PipelineOptions {
        // resolve compiler options and derived restrictions for this target
        let compiler_options = self
            .program
            .with_dsconfig_options(module, |opts| opts.compiler.clone())
            .unwrap_or_default();
        let compiler_options =
            destack_workspace::Program::compiler_options_for_target(target, &compiler_options);

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
        let bits = match target.output {
            OutputFormat::Wasm => 32,
            OutputFormat::Native => (mem::size_of::<usize>() * 8) as u16,
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
    fn target_for_module(&self, module: ModuleId, target: &TargetId) -> OptimizeResult<Target> {
        // resolve module package
        let module_ref = self.program.modules.get(module);
        let module_guard = module_ref.read();
        let package_id = module_guard.package_id;

        // resolve target configuration
        self.target_for_package_only(package_id, target)
    }

    /// Resolve the target configuration for a package.
    fn target_for_package_only(
        &self,
        package: PackageId,
        target: &TargetId,
    ) -> OptimizeResult<Target> {
        // resolve target configuration
        let package_ref = self.program.packages.get(package);
        let package_guard = package_ref.read();
        let Some(target_config) = package_guard.targets.get(target).cloned() else {
            return Err(OptimizeError::InvalidTarget {
                package,
                target: target.clone(),
                message: "target not found".to_string(),
            });
        };

        Ok(target_config)
    }
}
