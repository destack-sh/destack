use destack_workspace::ProviderContext;
use std::mem;
use std::str::FromStr;

use crate::lower::LowerState;
use crate::{Compiler, CompilerError, CompilerResult, LowerError, LowerResult, ModuleLowerer};

use destack_artifact::{ArtifactPayload, EmitFormat, MirLowered, TargetArch};
use destack_source::{ModuleId, TargetId};
use destack_workspace::Target;
use destack_workspace::workspace::ProfileId;
use target_lexicon::Triple;

impl Compiler {
    /// Build MIR for one module and target.
    pub(crate) fn provide_mir(
        &self,
        module: ModuleId,
        profile: ProfileId,
        target: TargetId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactPayload> {
        let state = LowerState::new(module, profile, target, context);
        let target_config = self.require_lower_module_inputs(
            state.module,
            state.profile,
            state.target,
            state.context,
        )?;

        let payload = self.lower_module(
            state.module,
            state.profile,
            state.target,
            &target_config,
            state.context,
        )?;

        Ok(ArtifactPayload::MirLowered(payload))
    }

    /// Lower a module.
    fn lower_module(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
        target_id: TargetId,
        target: &Target,
        context: &dyn ProviderContext,
    ) -> CompilerResult<MirLowered> {
        if !self.is_code_module(context.revision(), module_id) {
            return Err(LowerError::Internal {
                anchor: (module_id).into(),
                module: module_id,
                message: "attempted to lower MIR for non-code module".to_string(),
            }
            .into());
        }

        // load DIR artifacts
        let artifacts = self.artifact_reader(context);
        let parsed = artifacts
            .dir_parsed(module_id)
            .map_err(CompilerError::from)?;
        let bound = artifacts
            .dir_bound(module_id, profile)
            .map_err(CompilerError::from)?;
        let expanded = artifacts
            .dir_expanded(module_id, profile)
            .map_err(CompilerError::from)?;
        let checked = artifacts
            .dir_checked(module_id, profile)
            .map_err(CompilerError::from)?;
        let materialized = artifacts
            .dir_materialized(module_id, profile)
            .map_err(CompilerError::from)?;
        let elaborated = artifacts
            .dir_elaborated(module_id, profile)
            .map_err(CompilerError::from)?;

        // lower the module
        let mir_tree = {
            let module = self.module(context.revision(), module_id);
            let pointer_bytes = self.pointer_bytes_for_target_config(module_id, target)?;
            let bindings = elaborated.binding_table(&bound, &expanded, &materialized);
            let types = elaborated.type_table(&bound, &expanded, &checked, &materialized);
            let resolutions = elaborated.resolution_table(&checked, &materialized);
            let captures = elaborated.capture_table(&checked, &materialized);

            let mut lowerer = ModuleLowerer::new(
                self,
                context,
                module.as_ref(),
                profile,
                &parsed.tree,
                &elaborated.roots,
                self.repository.string_pool().as_ref(),
                bound.module_node,
                &bindings,
                &types,
                &resolutions,
                &elaborated.guards,
                &captures,
                &target_id,
                pointer_bytes,
            )?;
            lowerer.lower_module()?;
            let (mir_tree, mir_strings) = lowerer.finish();
            self.repository.string_pool().ensure_all_from(&mir_strings);

            mir_tree
        };

        let payload = MirLowered { tree: mir_tree };

        Ok(payload)
    }

    /// Require the artifacts and target configuration needed to lower one module.
    fn require_lower_module_inputs(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
        target_id: TargetId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<Target> {
        let resolved_profile = self
            .target_profile_id(context.revision(), module_id, &target_id)
            .ok_or_else(|| LowerError::Internal {
                anchor: (module_id).into(),
                module: module_id,
                message: format!("target '{target_id}' not found for profile resolution"),
            })?;
        if resolved_profile != profile {
            return Err(LowerError::Internal {
                anchor: (module_id).into(),
                module: module_id,
                message: format!(
                    "target '{target_id}' resolved to profile '{resolved_profile:?}', not '{profile:?}'"
                ),
            }
            .into());
        }

        // lowering depends on the selected library surface for language item layouts
        self.artifact_reader(context)
            .global_environment(profile)
            .map_err(CompilerError::from)?;

        // resolve target configuration
        self.target_or_builtin(context, target_id)
            .ok_or_else(|| LowerError::Internal {
                anchor: (module_id).into(),
                module: module_id,
                message: format!("target '{target_id}' not found for lowering"),
            })
            .map_err(CompilerError::from)
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
                anchor: (module_id).into(),
                module: module_id,
                message: format!("invalid target triple '{triple}': {error}"),
            })?;
            let pointer_width = triple.pointer_width().map_err(|_| LowerError::Internal {
                anchor: (module_id).into(),
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
                anchor: (module_id).into(),
                module: module_id,
                message: format!("unsupported pointer size {pointer_bytes} bytes"),
            }
            .into()),
        }
    }
}
