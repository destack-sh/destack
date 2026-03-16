use std::mem;
use std::str::FromStr;

use crate::timing::tags;
use crate::{BuildKey, BuildRequirementError, Compiler, LowerError, LowerResult, ModuleLowerer};

use destack_source::{CacheKind, ModuleId, ModuleVersion, ProfileVersion};
use destack_workspace::{
    ArtifactKey, ModuleMir, OutputFormat, ProfileId, Target, TargetArch, TargetId,
};
use target_lexicon::Triple;

impl Compiler {
    /// Build MIR for one module and target.
    pub fn process_mir(
        &self,
        module: ModuleId,
        profile: ProfileId,
        target: TargetId,
    ) -> LowerResult<()> {
        let module_version = self.module_version(module);
        let profile_version = self.profile_version(profile);
        self.ensure_module_profile_matches::<LowerError>(
            module,
            module_version,
            profile,
            profile_version,
        )?;
        let payload = self.lower_module(
            module,
            profile,
            module_version,
            profile_version,
            target.clone(),
        )?;
        if self.is_code_module(module) {
            self.stats.record_lower();
        }

        self.program
            .artifacts
            .set_mir_base(module, profile, target, payload);

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
    ) -> LowerResult<ModuleMir> {
        let resolved_profile = self
            .program
            .profile_id_for_target(module_id, &target_id)
            .ok_or_else(|| LowerError::Internal {
                module: module_id,
                message: format!("target '{target_id}' not found for profile resolution"),
            })?;
        if resolved_profile != profile {
            return Err(LowerError::Internal {
                module: module_id,
                message: format!(
                    "target '{target_id}' resolved to profile '{resolved_profile:?}', not '{profile:?}'"
                ),
            });
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
            tracing::trace!(?module_id, ?target_id, "lower.module.cache");
            return Ok(entry.payload);
        }

        self.require_dir_elaborated(module_id, profile)?;
        self.require_dir_patched(module_id, profile)?;
        self.require_intrinsic_environment(profile)
            .map_err(LowerError::from)?;
        if !self.is_code_module(module_id) {
            return Err(LowerError::Internal {
                module: module_id,
                message: "attempted to lower MIR for non-code module".to_string(),
            });
        }

        // resolve target configuration
        let target = {
            let module = self.program.modules.get(module_id);
            let module = module.as_ref();
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

        // load the patched DIR artifact
        let dir = self
            .program
            .artifacts
            .dir_patched(module_id, profile)
            .ok_or_else(|| LowerError::Internal {
                module: module_id,
                message: "missing patched DIR artifact for lowering".to_string(),
            })?;

        // lower the module
        let (mir_tree, mir_strings) = {
            let module = self.program.modules.get(module_id);
            let module = module.as_ref();
            let pointer_bytes = self.pointer_bytes_for_target_config(module_id, &target)?;

            let mut lowerer = ModuleLowerer::new(
                self,
                &module,
                profile,
                &dir.tree,
                &dir.roots,
                dir.anchor_node,
                &dir.symbols,
                &dir.types,
                &dir.captures,
                &target_id,
                pointer_bytes,
            );
            lowerer.lower_module()?;
            lowerer.finish()
        };

        let payload = ModuleMir {
            id: module_id,
            version: module_version,
            target: target_id.clone(),
            tree: mir_tree,
            strings: mir_strings,
            profile: None,
        };

        // write MIR to cache
        if let Some(cache) = cache_handle.as_ref() {
            if let Err(error) = cache.write_mir(payload.clone()) {
                tracing::debug!(?module_id, ?target_id, ?error, "lower.module.cache.write");
            }
        }

        Ok(payload)
    }

    /// Ensure MIR exists for one module and target.
    pub fn require_mir(
        &self,
        module: ModuleId,
        profile: ProfileId,
        target: &TargetId,
    ) -> Result<(), BuildRequirementError> {
        self.require_build_key(BuildKey::Artifact(ArtifactKey::MirBase {
            module,
            profile,
            target: target.clone(),
        }))
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
            let module = module.as_ref();
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
