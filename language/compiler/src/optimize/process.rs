use std::collections::HashMap;
use std::mem;
use std::path::PathBuf;
use std::str::FromStr;

use crate::timing::tags;
use crate::{
    Compiler, OptimizeError, OptimizeResult, TargetDiscoveryIssue, TaskDependencyError,
    TaskResultCollector,
};

use destack_compiler_macros::DefineTask;
use destack_source::{
    ModuleId, ModuleStamp, ModuleVersion, PackageId, PackageStamp, ProfileStamp, ProfileVersion,
};
use destack_workspace::{
    DebugMode, LtoMode, Module, OptimizeLevel as WorkspaceOptimizeLevel, OutputFormat, ProfileId,
    ProgramStamp, Target, TargetDiscovery, TargetId,
};
use target_lexicon::Triple;

use super::{
    ModuleWorkItem, OptimizationLevel, PackagePipeline, PackagePipelineContext, PackageWorkset,
    Pipeline, PipelineContext, PipelineOptions, PipelineTarget, ProgramPipeline,
    ProgramPipelineContext, ProgramWorkset, TypeContext, count_mir_size, default_package_pipeline,
    default_pipeline, default_program_pipeline,
};

/// Scope for optimization tasks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OptimizationScope {
    /// Optimize a single module.
    Module,
    /// Optimize all modules in a package.
    Package,
    /// Optimize all modules in the program.
    Program,
}

/// Task to optimize something.
#[derive(Debug, Clone, Hash, PartialEq, Eq, DefineTask)]
#[phase(Optimize)]
pub enum OptimizeTask {
    /// Optimize a module's MIR.
    #[task(code = 1, trace = "module={module} target={target}")]
    OptimizeModule {
        /// The module to optimize.
        module: ModuleStamp,
        /// The profile to optimize.
        profile: ProfileStamp,
        /// The target to optimize.
        target: TargetId,
    },
    /// Optimize all modules in a package for a target.
    #[task(code = 2, trace = "package={package} target={target}")]
    OptimizePackage {
        /// The package id to optimize.
        package: PackageStamp,
        /// The target id for this package.
        target: TargetId,
    },
    /// Optimize all modules in the program for a target name.
    #[task(code = 3, trace = "target={target}")]
    OptimizeProgram {
        /// The target name to optimize.
        target: String,
        /// Stamp of the current package versions.
        program_stamp: ProgramStamp,
    },
}

impl Compiler {
    /// Process an optimize task.
    pub fn process_optimize(&self, task: OptimizeTask) -> OptimizeResult<()> {
        // dispatch the optimize task
        match task {
            OptimizeTask::OptimizeModule {
                module,
                profile,
                target,
            } => {
                self.ensure_module_profile_matches::<OptimizeError>(
                    module.id,
                    module.version,
                    profile.id,
                    profile.version,
                )?;
                let _timing = self.timing_scope(tags::OPTIMIZE_MODULE);
                // require lowering for this module and target
                self.require_lower_module(module.id, profile.id, &target)?;

                // optimize the module
                self.optimize_module(
                    module.id,
                    profile.id,
                    module.version,
                    profile.version,
                    &target,
                )?;
            }
            OptimizeTask::OptimizePackage { package, target } => {
                self.ensure_package_version_matches::<OptimizeError>(package.id, package.version)?;
                let _timing = self.timing_scope(tags::OPTIMIZE_PACKAGE);
                // optimize the package for this target
                self.optimize_package(package.id, &target)?;
            }
            OptimizeTask::OptimizeProgram {
                target: target_name,
                program_stamp,
            } => {
                self.ensure_program_stamp_matches::<OptimizeError>(program_stamp)?;
                let _timing = self.timing_scope(tags::OPTIMIZE_PROGRAM);
                // optimize all packages that define the target
                self.optimize_program(&target_name)?;
            }
        }

        Ok(())
    }

    /// Ensure a module has been optimized.
    pub fn require_optimize(
        &self,
        module: ModuleId,
        profile: ProfileId,
        target: &TargetId,
    ) -> Result<(), TaskDependencyError> {
        // resolve optimization scope for this target
        let scope = self.optimization_scope_for_target(target);

        // validate profile mapping for module scope
        if matches!(scope, OptimizationScope::Module) {
            let resolved_profile = self.program.profile_id_for_target(module, target);
            if resolved_profile != Some(profile) {
                self.error(OptimizeError::InvalidTarget {
                    package: target.package_id,
                    target: target.clone(),
                    message: "target not found for profile resolution".to_string(),
                });
                return Ok(());
            }
        }

        // build the optimize task for this scope
        let task = match scope {
            OptimizationScope::Module => OptimizeTask::OptimizeModule {
                module: self.module_stamp(module),
                profile: self.profile_stamp(profile),
                target: target.clone(),
            },
            OptimizationScope::Package => OptimizeTask::OptimizePackage {
                package: self.package_stamp(target.package_id),
                target: target.clone(),
            },
            OptimizationScope::Program => OptimizeTask::OptimizeProgram {
                target: target.name.clone(),
                program_stamp: self.program_stamp(),
            },
        };

        // enqueue the task
        self.do_require_task_internal_only(task)
    }

    /// Optimize a module's MIR.
    fn optimize_module(
        &self,
        module: ModuleId,
        profile: ProfileId,
        module_version: ModuleVersion,
        profile_version: ProfileVersion,
        target: &TargetId,
    ) -> OptimizeResult<()> {
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
            return Ok(());
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

        // read module mir
        let module_ref = self.program.modules.get(module);
        let module_guard = module_ref.read();
        let mir = module_guard.mir(target);
        let mut tree = mir.tree.write();
        let strings = mir.strings.clone();
        let profile = mir.profile.clone();

        // resolve pipeline options
        let options = self.pipeline_options_for_module(&module_guard, &target_config, level);

        // count mir size before optimization
        let before = count_mir_size(&tree);

        // run the pipeline
        let mut context = PipelineContext::new(&strings, options, module, target.clone(), profile);
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

        // record metrics
        self.stats.record_optimize();
        self.stats.record_optimize_mir(
            before.functions,
            before.instructions,
            after.instructions,
            before.blocks,
            after.blocks,
        );

        Ok(())
    }

    /// Optimize all modules in a package for a target.
    fn optimize_package(&self, package: PackageId, target: &TargetId) -> OptimizeResult<()> {
        // resolve target configuration and package path
        let (target_config, package_path) = self.target_for_package(package, target)?;

        // resolve pipeline for this target
        let level = self.optimization_level_for_target_config(&target_config);
        let pipeline_target = if matches!(target_config.debug_mode, DebugMode::Vm) {
            PipelineTarget::Vm
        } else {
            PipelineTarget::Native
        };
        let pipeline = default_package_pipeline(level, pipeline_target);

        // discover and order target modules
        let mut modules =
            self.discover_target_modules(package, &package_path, &target_config, target)?;
        modules.sort();
        modules.dedup();

        // require lowering for code modules
        let mut collector = TaskResultCollector::new();
        for module in modules.iter().copied() {
            // skip non code modules
            if !self.is_code_module(module) {
                continue;
            }

            // require lowering for this module
            let profile = self
                .program
                .profile_id_for_target(module, target)
                .ok_or_else(|| OptimizeError::InvalidTarget {
                    package: target.package_id,
                    target: target.clone(),
                    message: "target not found for profile resolution".to_string(),
                })?;
            let result = self.require_lower_module(module, profile, target);
            collector.try_collect(result);
        }

        // yield on pending dependencies
        if let Some(dependency) = collector.try_into_yield_all() {
            return Err(OptimizeError::Yield { dependency });
        }

        // build the package workset
        let mut workset = PackageWorkset::new(package, target.clone(), level);
        let mut before_stats = HashMap::new();
        for module in modules {
            // skip non code modules
            if !self.is_code_module(module) {
                continue;
            }

            // gather module references and options
            let module_ref = self.program.modules.get(module);
            let module_guard = module_ref.read();
            let options = self.pipeline_options_for_module(&module_guard, &target_config, level);
            drop(module_guard);

            // track size before optimization
            let work_item = ModuleWorkItem::new(module, target.clone(), module_ref, options);
            let before = work_item.with_tree(count_mir_size);
            before_stats.insert(module, before);

            // add module to workset
            workset.add_module(work_item);
        }

        // run the package pipeline
        let mut context = PackagePipelineContext::new(package, target.clone());
        pipeline.run(&mut workset, &mut context);

        // collect accumulated diagnostics from verification passes
        for error in context.take_errors() {
            self.error(error);
        }
        for warning in context.take_warnings() {
            self.warning(warning);
        }

        // record metrics for each module
        for module in workset.modules() {
            let Some(before) = before_stats.get(&module.module_id()) else {
                continue;
            };

            let after = module.with_tree(count_mir_size);
            self.stats.record_optimize();
            self.stats.record_optimize_mir(
                before.functions,
                before.instructions,
                after.instructions,
                before.blocks,
                after.blocks,
            );
        }

        Ok(())
    }

    /// Optimize all modules in the program for a target name.
    fn optimize_program(&self, target_name: &str) -> OptimizeResult<()> {
        // collect package targets matching this name
        let mut targets = Vec::new();
        for package in self.program.packages.iter() {
            let package = package.read();
            let target_id = TargetId::new(package.id, target_name);

            // skip packages without this target
            let Some(target) = package.targets.get(&target_id).cloned() else {
                continue;
            };

            // resolve the level for this package
            let level = self.optimization_level_for_target_config(&target);
            targets.push((package.id, package.path.clone(), target_id, target, level));
        }

        // resolve pipeline target for this program run
        let mut pipeline_target = PipelineTarget::Native;
        for (_, _, _, target, _) in &targets {
            if matches!(target.debug_mode, DebugMode::Vm) {
                pipeline_target = PipelineTarget::Vm;
                break;
            }
        }

        // discover modules for each target
        let mut package_modules = Vec::new();
        let mut collector = TaskResultCollector::new();

        for (package_id, package_path, target_id, target, level) in targets {
            let mut modules =
                self.discover_target_modules(package_id, &package_path, &target, &target_id)?;
            modules.sort();
            modules.dedup();

            // require lowering for code modules
            for module in modules.iter().copied() {
                // skip non code modules
                if !self.is_code_module(module) {
                    continue;
                }

                // require lowering for this module
                let profile = self
                    .program
                    .profile_id_for_target(module, &target_id)
                    .ok_or_else(|| OptimizeError::InvalidTarget {
                        package: target_id.package_id,
                        target: target_id.clone(),
                        message: "target not found for profile resolution".to_string(),
                    })?;
                let result = self.require_lower_module(module, profile, &target_id);
                collector.try_collect(result);
            }

            package_modules.push((package_id, target_id, target, level, modules));
        }

        // yield on pending dependencies
        if let Some(dependency) = collector.try_into_yield_all() {
            return Err(OptimizeError::Yield { dependency });
        }

        // build the program workset
        let mut workset = ProgramWorkset::new();
        let mut before_stats = HashMap::new();

        for (package_id, target_id, target, level, modules) in package_modules {
            let mut package_workset = PackageWorkset::new(package_id, target_id.clone(), level);

            for module in modules {
                // skip non code modules
                if !self.is_code_module(module) {
                    continue;
                }

                // gather module references and options
                let module_ref = self.program.modules.get(module);
                let module_guard = module_ref.read();
                let options = self.pipeline_options_for_module(&module_guard, &target, level);
                drop(module_guard);

                // track size before optimization
                let work_item = ModuleWorkItem::new(module, target_id.clone(), module_ref, options);
                let before = work_item.with_tree(count_mir_size);
                before_stats.insert(module, before);

                // add module to workset
                package_workset.add_module(work_item);
            }

            workset.add_package(package_workset);
        }

        // build the program pipeline
        let pipeline = default_program_pipeline(pipeline_target);

        // run the program pipeline
        let mut context = ProgramPipelineContext::new(target_name.to_string());
        pipeline.run(&mut workset, &mut context);

        // collect accumulated diagnostics from verification passes
        for error in context.take_errors() {
            self.error(error);
        }
        for warning in context.take_warnings() {
            self.warning(warning);
        }

        // record metrics for each module
        for package in workset.packages() {
            for module in package.modules() {
                let Some(before) = before_stats.get(&module.module_id()) else {
                    continue;
                };

                let after = module.with_tree(count_mir_size);
                self.stats.record_optimize();
                self.stats.record_optimize_mir(
                    before.functions,
                    before.instructions,
                    after.instructions,
                    before.blocks,
                    after.blocks,
                );
            }
        }

        Ok(())
    }

    /// Resolve the optimization scope for a target.
    fn optimization_scope_for_target(&self, target: &TargetId) -> OptimizationScope {
        // resolve target configuration if available
        let package = self.program.packages.get(target.package_id);
        let package = package.read();
        let Some(target_config) = package.targets.get(target) else {
            return OptimizationScope::Module;
        };

        // map target configuration to a scope
        self.optimization_scope_for_target_config(target_config)
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

    /// Resolve the optimization scope for a target configuration.
    fn optimization_scope_for_target_config(&self, target: &Target) -> OptimizationScope {
        // honor disabled optimization
        if !target.optimize {
            return OptimizationScope::Module;
        }

        // resolve optimization level to handle auto lto
        let level = self.optimization_level_for_target_config(target);

        // select scope based on lto mode
        match target.lto_mode {
            LtoMode::None => OptimizationScope::Module,
            LtoMode::Thin => OptimizationScope::Package,
            LtoMode::Full => OptimizationScope::Program,
            LtoMode::Auto => {
                // enable thin lto only at o4
                if level == OptimizationLevel::O4 {
                    OptimizationScope::Package
                } else {
                    OptimizationScope::Module
                }
            }
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
        if let Some(triple) = target.target_triple.as_ref()
            && let Ok(triple) = Triple::from_str(triple)
            && let Ok(pointer_width) = triple.pointer_width()
        {
            let bits = pointer_width.bits() as u16;
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

    /// Resolve the target configuration and package path.
    fn target_for_package(
        &self,
        package: PackageId,
        target: &TargetId,
    ) -> OptimizeResult<(Target, Option<PathBuf>)> {
        // resolve package configuration
        let package_ref = self.program.packages.get(package);
        let package_guard = package_ref.read();
        let package_path = package_guard.path.clone();

        // resolve target configuration
        let Some(target_config) = package_guard.targets.get(target).cloned() else {
            return Err(OptimizeError::InvalidTarget {
                package,
                target: target.clone(),
                message: "target not found".to_string(),
            });
        };

        Ok((target_config, package_path))
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

    /// Discover modules for a target in a package.
    fn discover_target_modules(
        &self,
        package: PackageId,
        package_path: &Option<PathBuf>,
        target: &Target,
        target_id: &TargetId,
    ) -> OptimizeResult<Vec<ModuleId>> {
        // dispatch discovery by mode
        match target.discovery {
            TargetDiscovery::Entry => self
                .discover_entry_modules(package, package_path, target, target_id)
                .map_err(|issue| self.map_discovery_issue(issue)),
            TargetDiscovery::Include => self
                .discover_include_modules(package, package_path, target)
                .map_err(|issue| self.map_discovery_issue(issue)),
        }
    }

    /// Map a target discovery issue to an optimize error.
    fn map_discovery_issue(&self, issue: TargetDiscoveryIssue) -> OptimizeError {
        // map discovery issues to target errors
        match issue {
            TargetDiscoveryIssue::MissingPackagePath { package, target } => {
                OptimizeError::InvalidTarget {
                    package,
                    target,
                    message: "entry discovery requires package path".to_string(),
                }
            }
            TargetDiscoveryIssue::MissingEntry {
                package,
                target,
                path,
            } => OptimizeError::InvalidTarget {
                package,
                target,
                message: format!("entry point not found: {}", path.display()),
            },
        }
    }
}
