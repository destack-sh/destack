use std::sync::Arc;

use tspp_artifact::{
    ArtifactDependencySet, ArtifactKey, ArtifactPayload, Asset, Code, DirResolved, MirOptimized,
    Output,
};
use tspp_mir::ModuleCache;
use tspp_repository::{ProfileId, ProviderContext};
use tspp_source::{ModuleId, TargetId};

use crate::{Compiler, CompilerError, CompilerResult, EmitError};

use super::ObjectEmitter;
use super::bytecode::BytecodeEmitter;

#[cfg(all(feature = "native", not(target_arch = "wasm32")))]
use super::native::NativeEmitter;

impl Compiler {
    /// Collect inputs for one structured script.
    pub(crate) fn collect_script(
        &self,
        module: ModuleId,
        profile: ProfileId,
        target: TargetId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactDependencySet> {
        // read the target configuration
        let target_config =
            self.target_or_builtin(context, target)?
                .ok_or_else(|| EmitError::Internal {
                    anchor: module.into(),
                    module,
                    message: format!("target '{target}' not found"),
                })?;
        let mut dependencies = ArtifactDependencySet::default();

        // require a bundle target for script emission
        if target_config.output != Output::Bundle {
            return Err(EmitError::UnsupportedTarget {
                anchor: module.into(),
                module,
                target: target.to_string(),
            }
            .into());
        }

        // declare the DIR tables consumed by script emission
        dependencies.require(ArtifactKey::dir_parsed(module));
        dependencies.require(ArtifactKey::dir_bound(module, profile));
        dependencies.require(ArtifactKey::dir_imported(module, profile));
        dependencies.require(ArtifactKey::dir_expanded(module, profile));
        dependencies.require(ArtifactKey::dir_materialized(module, profile));
        self.observe_package_config(context, target.package_id(), &mut dependencies)?;

        Ok(dependencies)
    }

    /// Collect inputs for one Program object.
    pub(crate) fn collect_object(
        &self,
        module: ModuleId,
        profile: ProfileId,
        target: TargetId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactDependencySet> {
        // read the target configuration
        let target_config =
            self.target_or_builtin(context, target)?
                .ok_or_else(|| EmitError::Internal {
                    anchor: module.into(),
                    module,
                    message: format!("target '{target}' not found"),
                })?;
        let mut dependencies = ArtifactDependencySet::default();

        // require a program target for object emission
        if target_config.output != Output::Program {
            return Err(EmitError::UnsupportedTarget {
                anchor: module.into(),
                module,
                target: target.to_string(),
            }
            .into());
        }

        // require optimized MIR and resolved imports
        dependencies.require(ArtifactKey::mir_optimized(module, profile, target));
        dependencies.require(ArtifactKey::dir_resolved(module, profile));

        // track changes to the target configuration
        self.observe_package_config(context, target.package_id(), &mut dependencies)?;

        Ok(dependencies)
    }

    /// Collect inputs for one opaque asset.
    pub(crate) fn collect_asset(
        &self,
        module: ModuleId,
        _profile: ProfileId,
        target: TargetId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactDependencySet> {
        let mut dependencies = ArtifactDependencySet::default();
        dependencies.require(ArtifactKey::data(module));
        self.observe_package_config(context, target.package_id(), &mut dependencies)?;

        Ok(dependencies)
    }

    /// Build one structured script.
    pub(crate) fn provide_script(
        &self,
        module: ModuleId,
        profile: ProfileId,
        target: TargetId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactPayload> {
        // resolve target selection
        let target_config =
            self.target_or_builtin(context, target)?
                .ok_or_else(|| EmitError::Internal {
                    anchor: module.into(),
                    module,
                    message: format!("target '{target}' not found"),
                })?;
        let target_name = self.target_name(context.revision(), target)?;
        let resolved_profile = self.profile_id_for_target(context.revision(), &target)?;

        // require the profile selected by the target
        if resolved_profile != profile {
            return Err(EmitError::Internal {
                anchor: module.into(),
                module,
                message: format!(
                    "target '{target_name}' resolved to profile '{resolved_profile:?}', not '{profile:?}'"
                ),
            }
            .into());
        }

        // require a bundle target for script emission
        if target_config.output != Output::Bundle {
            return Err(EmitError::UnsupportedTarget {
                anchor: module.into(),
                module,
                target: target_name,
            }
            .into());
        }

        // emit script from materialized DIR
        let artifacts = self.artifact_reader(context);
        let output = self.emit_script(module, &target_config, profile, context, &artifacts)?;

        Ok(ArtifactPayload::Script(Arc::new(output)))
    }

    /// Build one Program object.
    pub(crate) fn provide_object(
        &self,
        module: ModuleId,
        profile: ProfileId,
        target: TargetId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactPayload> {
        // resolve target selection
        let target_config =
            self.target_or_builtin(context, target)?
                .ok_or_else(|| EmitError::Internal {
                    anchor: module.into(),
                    module,
                    message: format!("target '{target}' not found"),
                })?;
        let target_name = self.target_name(context.revision(), target)?;
        let resolved_profile = self.profile_id_for_target(context.revision(), &target)?;

        // require the profile selected by the target
        if resolved_profile != profile {
            return Err(EmitError::Internal {
                anchor: module.into(),
                module,
                message: format!(
                    "target '{target_name}' resolved to profile '{resolved_profile:?}', not '{profile:?}'"
                ),
            }
            .into());
        }

        // require a program target for object emission
        if target_config.output != Output::Program {
            return Err(EmitError::UnsupportedTarget {
                anchor: module.into(),
                module,
                target: target_name,
            }
            .into());
        }

        // read optimized MIR and resolved imports
        let artifacts = self.artifact_reader(context);
        let optimized = artifacts
            .read::<MirOptimized>((module, profile, target))
            .map_err(CompilerError::from)?;
        let resolved = artifacts
            .read::<DirResolved>((module, profile))
            .map_err(CompilerError::from)?;

        // collect resolved imports
        let modules = resolved
            .target_modules()
            .filter(|target| *target != module)
            .collect::<Vec<_>>();

        // share analyses across object and code emission
        let mut analyses = ModuleCache::with_target_layout(optimized.target);
        let mut object = ObjectEmitter::new(module, &optimized, modules, &mut analyses)?;

        // emit every representation selected by this Program target
        for code in target_config.code.iter().copied() {
            object = match code {
                Code::Bytecode => {
                    let bytecode =
                        BytecodeEmitter::new(module, &optimized, &object).emit(&mut analyses)?;

                    object.bytecode(bytecode)
                }
                Code::Native => {
                    #[cfg(all(feature = "native", not(target_arch = "wasm32")))]
                    {
                        let native =
                            NativeEmitter::new(module, &optimized, &object, &target_config)?
                                .emit()?;

                        object.native(native)
                    }
                    #[cfg(any(not(feature = "native"), target_arch = "wasm32"))]
                    {
                        return Err(EmitError::MissingEmitter {
                            anchor: module.into(),
                            module,
                            code,
                        }
                        .into());
                    }
                }
                Code::Wasm => {
                    return Err(EmitError::MissingEmitter {
                        anchor: module.into(),
                        module,
                        code,
                    }
                    .into());
                }
            };
        }

        // finalize the complete relocatable object
        let output = object.build();

        Ok(ArtifactPayload::Object(Arc::new(output)))
    }

    /// Build one opaque asset.
    pub(crate) fn provide_asset(
        &self,
        module: ModuleId,
        _profile: ProfileId,
        _target: TargetId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactPayload> {
        let module = self.module(context.revision(), module)?;
        let file = self.file(context, module.file_id)?;
        let asset = Asset::new(file.ty, file.blob(), Some(module.uri.clone()), None);

        Ok(ArtifactPayload::Asset(Arc::new(asset)))
    }
}
