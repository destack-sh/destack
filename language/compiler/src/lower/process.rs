use std::mem;
use std::str::FromStr;

use crate::timing::tags;
use crate::{Compiler, CompilerContext, LowerError, LowerResult, ModuleLowerer, RequirementError};

use destack_artifact::{ArtifactKey, EmitFormat, MirBase, TargetArch};
use destack_source::{ModuleId, TargetId};
use destack_workspace::Target;
use destack_workspace::workspace::ProfileId;
use target_lexicon::Triple;

impl Compiler {
    /// Build MIR for one module and target.
    pub fn process_mir(
        &self,
        module: ModuleId,
        profile: ProfileId,
        target: TargetId,
        context: &CompilerContext<'_>,
    ) -> LowerResult<()> {
        let revision = context.revision();
        let artifact_key = ArtifactKey::mir_base(module, profile, target);
        let artifact_stamp = context.artifact_stamp(&artifact_key);

        // reuse one persisted mir image when available
        if context
            .restore_cached_artifact(
                artifact_key,
                |compiler| {
                    compiler.load_mir_base_image(revision, module, artifact_stamp, profile, &target)
                },
                |store, version, payload| store.publish_mir_base(version, payload),
            )
            .is_some()
        {
            return Ok(());
        }

        let payload = self.lower_module(module, profile, target, context)?;
        if context.is_code_module(module) {
            self.stats.record_lower();
        }

        context.publish_artifact(artifact_key, payload.clone(), |store, version, payload| {
            store.publish_mir_base(version, payload)
        });
        context.store_artifact(&artifact_key, &payload, |compiler, artifact_stamp, mir| {
            compiler.store_mir_base_image(revision, module, profile, &target, artifact_stamp, mir)
        });

        Ok(())
    }

    /// Lower a module.
    fn lower_module(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
        target_id: TargetId,
        context: &CompilerContext<'_>,
    ) -> LowerResult<MirBase> {
        let resolved_profile = context
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

        self.require_dir_elaborated(context.revision(), module_id, profile)?;
        self.require_dir_patched(context.revision(), module_id, profile)?;
        self.require_intrinsic_environment(context.revision(), profile)
            .map_err(LowerError::from)?;

        // lowering depends on the selected library surface for well known layouts
        self.require_library_environment(context.revision(), profile)
            .map_err(LowerError::from)?;

        if !context.is_code_module(module_id) {
            return Err(LowerError::Internal {
                module: module_id,
                message: "attempted to lower MIR for non-code module".to_string(),
            });
        }

        // resolve target configuration
        let target = {
            self.repository
                .effective_target(context.revision(), target_id)
                .map_err(|error| LowerError::Internal {
                    module: module_id,
                    message: format!("failed to resolve target '{target_id}': {error}"),
                })?
                .ok_or_else(|| LowerError::Internal {
                    module: module_id,
                    message: format!("target '{target_id}' not found for lowering"),
                })?
        };

        // load the patched DIR artifact
        let dir = self
            .dir_patched(module_id, profile)
            .ok_or_else(|| LowerError::Internal {
                module: module_id,
                message: "missing patched DIR artifact for lowering".to_string(),
            })?;

        // lower the module
        let (mir_tree, mir_strings) = {
            let module = context.module(module_id);
            let pointer_bytes = self.pointer_bytes_for_target_config(module_id, &target)?;

            let mut lowerer = ModuleLowerer::new(
                self,
                context,
                module.as_ref(),
                profile,
                &dir.tree,
                &dir.roots,
                dir.anchor_node,
                &dir.symbols,
                &dir.types,
                &dir.captures,
                &target_id,
                pointer_bytes,
            )?;
            lowerer.lower_module()?;
            lowerer.finish()
        };

        let payload = MirBase {
            id: module_id,
            target: target_id,
            tree: mir_tree,
            strings: mir_strings,
            profile: None,
        };

        Ok(payload)
    }

    /// Ensure MIR exists for one module and target.
    pub fn require_mir(
        &self,
        revision: destack_workspace::Revision,
        module: ModuleId,
        profile: ProfileId,
        target: &TargetId,
    ) -> Result<(), RequirementError> {
        self.require_artifact(revision, ArtifactKey::mir_base(module, profile, *target))
    }

    /// Resolve the pointer size in bytes for a lowering target.
    /// #Architecture: where should we determine target pointer width? (in Lower feels a bit weird)
    pub(crate) fn pointer_bytes_for_target(
        &self,
        module_id: ModuleId,
        target_id: &TargetId,
        context: &CompilerContext<'_>,
    ) -> LowerResult<u8> {
        // resolve target configuration
        let target = {
            let module = context.module(module_id);
            let package = context.package(module.package_id);

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
        let pointer_bytes = match target.emit {
            EmitFormat::Wasm => 4,
            EmitFormat::Native => mem::size_of::<usize>() as u8,
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
