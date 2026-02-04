use std::mem;
use std::str::FromStr;

use crate::timing::tags;
use crate::{Compiler, LowerError, LowerResult, ModuleLowerer, TaskDependencyError};

use destack_compiler_macros::DefineTask;
use destack_source::{
    CacheKind, ModuleId, ModuleStamp, ModuleVersion, ProfileStamp, ProfileVersion,
};
use destack_workspace::{ModuleMir, OutputFormat, ProfileId, Target, TargetArch, TargetId};
use target_lexicon::Triple;

/// Task to lower a DIR into MIR.
#[derive(Debug, Clone, Hash, PartialEq, Eq, DefineTask)]
#[phase(Lower)]
pub enum LowerTask {
    /// Lower a module into MIR.
    #[task(code = 1, trace = "module={module} target={target}")]
    LowerModule {
        /// The module stamp to lower.
        module: ModuleStamp,
        /// The profile stamp to lower.
        profile: ProfileStamp,
        /// Identify the target backend for lowering.
        target: TargetId,
    },
}

impl Compiler {
    /// Process a lower task.
    pub fn process_lower(&self, task: LowerTask) -> LowerResult<()> {
        match task {
            LowerTask::LowerModule {
                module,
                profile,
                target,
            } => {
                self.ensure_module_profile_matches::<LowerError>(
                    module.id,
                    module.version,
                    profile.id,
                    profile.version,
                )?;
                self.lower_module(
                    module.id,
                    profile.id,
                    module.version,
                    profile.version,
                    target,
                )?;
                if self.is_code_module(module.id) {
                    self.stats.record_lower();
                }
            }
        }
        Ok(())
    }

    /// Lower a module.
    fn lower_module(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
        module_version: ModuleVersion,
        profile_version: ProfileVersion,
        target_id: TargetId,
    ) -> LowerResult<()> {
        let resolved_profile = self
            .program
            .profile_id_for_target(module_id, &target_id)
            .ok_or_else(|| LowerError::Internal {
                module: module_id,
                message: format!("target '{target_id}' not found for profile resolution"),
            })?;
        if resolved_profile != profile {
            return Ok(());
        }
        let _timing = self.timing_scope(tags::LOWER_MODULE);

        // try to load MIR from cache
        let cache_handle = self.cache_handle_for_module(
            module_id,
            Some(profile),
            Some(&target_id),
            CacheKind::Mir,
        );
        if let Some(cache) = cache_handle.as_ref()
            && let Ok(Some(entry)) = cache.read_mir()
        {
            self.ensure_module_profile_matches::<LowerError>(
                module_id,
                module_version,
                profile,
                profile_version,
            )?;
            let module = self.program.modules.get(module_id);
            let mut module = module.write();
            self.ensure_module_profile_matches_guard::<LowerError>(
                &module,
                module_version,
                profile,
                profile_version,
            )?;
            let code = module.code_mut();
            code.mirs.retain(|mir| mir.target != target_id);
            code.mirs.push(ModuleMir::from_data(entry.payload));
            tracing::trace!(?module_id, ?target_id, "lower.module.cache");
            return Ok(());
        }

        self.require_elaborate_module(module_id, profile)?;
        self.require_execute_module_patch(module_id, profile)?;
        if !self.is_code_module(module_id) {
            return Ok(());
        }

        // resolve target configuration
        let target = {
            let module = self.program.modules.get(module_id);
            let module = module.read();
            let package = self.program.packages.get(module.package_id);
            let package = package.read();
            package
                .targets
                .get(&target_id)
                .cloned()
                .ok_or_else(|| LowerError::Internal {
                    module: module_id,
                    message: format!("target '{target_id}' not found for lowering"),
                })?
        };

        let module = self.program.modules.get(module_id);

        // initialize MIR for this target
        {
            self.ensure_module_profile_matches::<LowerError>(
                module_id,
                module_version,
                profile,
                profile_version,
            )?;
            let mut module = module.write();
            self.ensure_module_profile_matches_guard::<LowerError>(
                &module,
                module_version,
                profile,
                profile_version,
            )?;
            // replace existing MIR for this target, if any
            let code = module.code_mut();
            code.mirs.retain(|mir| mir.target != target_id);
            code.mirs
                .push(ModuleMir::new(module_id, module_version, target_id.clone()));
        }

        // lower the module
        let (mir_tree, mir_strings) = {
            let module = self.program.modules.get(module_id);
            let module = module.read();
            let dir = module.dir(profile);
            let dir_tree = dir.tree.read();
            let symbols = dir.symbols.read();
            let types = dir.types.read();
            let captures = dir.captures.read();
            let pointer_bytes = self.pointer_bytes_for_target_config(module_id, &target)?;

            let mut lowerer = ModuleLowerer::new(
                self,
                &module,
                profile,
                &dir_tree,
                &dir.roots,
                &symbols,
                &types,
                &captures,
                &target_id,
                pointer_bytes,
            );
            lowerer.lower_module()?;
            lowerer.finish()
        };

        // update the module with the new lowered MIR
        // (#Cleanup: should we mutate the ModuleMir in place..?)
        self.ensure_module_profile_matches::<LowerError>(
            module_id,
            module_version,
            profile,
            profile_version,
        )?;
        let module = self.program.modules.get(module_id);
        let mut module = module.write();
        self.ensure_module_profile_matches_guard::<LowerError>(
            &module,
            module_version,
            profile,
            profile_version,
        )?;
        let mir = module.mir_mut(&target_id);
        *mir.tree.write() = mir_tree;
        mir.strings = mir_strings;

        // write MIR to cache
        if let Some(cache) = cache_handle.as_ref() {
            self.ensure_module_profile_matches::<LowerError>(
                module_id,
                module_version,
                profile,
                profile_version,
            )?;
            let payload = module.mir(&target_id).to_data();
            if let Err(error) = cache.write_mir(payload) {
                tracing::debug!(?module_id, ?target_id, ?error, "lower.module.cache.write");
            }
        }

        Ok(())
    }

    /// Ensure a module has been lowered.
    pub fn require_lower_module(
        &self,
        module: ModuleId,
        profile: ProfileId,
        target: &TargetId,
    ) -> Result<(), TaskDependencyError> {
        let module = self.module_stamp(module);
        let profile = self.profile_stamp(profile);
        self.do_require_task_internal_only(LowerTask::LowerModule {
            module,
            profile,
            target: target.clone(),
        })
    }

    /// Resolve the pointer size in bytes for a lowering target.
    /// #Architecture: where should we determine target pointer width? (in Lower feels a bit weird)
    pub(crate) fn pointer_bytes_for_target(
        &self,
        module_id: ModuleId,
        target_id: &TargetId,
    ) -> LowerResult<u8> {
        // resolve target configuration
        let target = {
            let module = self.program.modules.get(module_id);
            let module = module.read();
            let package = self.program.packages.get(module.package_id);
            let package = package.read();

            package
                .targets
                .get(target_id)
                .cloned()
                .ok_or_else(|| LowerError::Internal {
                    module: module_id,
                    message: format!("target '{target_id}' not found for lowering"),
                })?
        };

        self.pointer_bytes_for_target_config(module_id, &target)
    }

    /// Resolve the pointer size in bytes for a target configuration.
    pub(crate) fn pointer_bytes_for_target_config(
        &self,
        module_id: ModuleId,
        target: &Target,
    ) -> LowerResult<u8> {
        // prefer explicit triple for pointer width
        if let Some(triple) = target.resolved_target_triple() {
            let triple = Triple::from_str(&triple).map_err(|error| LowerError::Internal {
                module: module_id,
                message: format!("invalid target triple '{triple}': {error}"),
            })?;
            let pointer_width = triple.pointer_width().map_err(|_| LowerError::Internal {
                module: module_id,
                message: format!("unsupported pointer width for '{triple}'"),
            })?;
            let pointer_bytes = pointer_width.bits() / 8;
            self.validate_pointer_bytes(module_id, pointer_bytes)?;
            return Ok(pointer_bytes);
        }

        // derive from explicit architecture when present
        if let Some(target_arch) = target.target_arch.as_ref() {
            let pointer_bytes = match target_arch {
                TargetArch::X86_64
                | TargetArch::Aarch64
                | TargetArch::Riscv64
                | TargetArch::PowerPc64
                | TargetArch::PowerPc64le
                | TargetArch::S390x
                | TargetArch::Mips64
                | TargetArch::Mips64el
                | TargetArch::LoongArch64
                | TargetArch::Wasm64 => 8,
                TargetArch::X86
                | TargetArch::Armv7
                | TargetArch::Armv6
                | TargetArch::Riscv32
                | TargetArch::Wasm32 => 4,
                TargetArch::Other(_) => mem::size_of::<usize>() as u8,
            };
            self.validate_pointer_bytes(module_id, pointer_bytes)?;
            return Ok(pointer_bytes);
        }

        // fall back to output defaults
        let pointer_bytes = match target.output {
            OutputFormat::Wasm => 4,
            OutputFormat::Native => mem::size_of::<usize>() as u8,
            _ => mem::size_of::<usize>() as u8,
        };

        self.validate_pointer_bytes(module_id, pointer_bytes)?;
        Ok(pointer_bytes)
    }

    /// Validate supported pointer byte sizes.
    pub(crate) fn validate_pointer_bytes(
        &self,
        module_id: ModuleId,
        pointer_bytes: u8,
    ) -> LowerResult<()> {
        // only 16, 32, and 64 bit pointer widths are supported
        match pointer_bytes {
            2 | 4 | 8 => Ok(()),
            _ => Err(LowerError::Internal {
                module: module_id,
                message: format!("unsupported pointer size {pointer_bytes} bytes"),
            }),
        }
    }
}
