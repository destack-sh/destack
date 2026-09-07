use std::sync::Arc;

use destack_artifact::{
    ArtifactDependencySet, ArtifactKey, ArtifactPayload, Asset, Code, DirBound, DirChecked,
    DirDeclared, DirElaborated, DirExpanded, DirImported, DirMaterialized, DirParsed, DirResolved,
    MirLowered, MirOptimized, Output,
};
use destack_repository::{ProfileId, ProviderContext};
use destack_source::{ModuleId, TargetId};

use crate::{Compiler, CompilerError, CompilerResult, EmitError};

use super::ObjectEmitter;
use super::bytecode::BytecodeEmitter;
use super::js::ScriptEmitter;
#[cfg(all(feature = "native", not(target_arch = "wasm32")))]
use super::native::NativeEmitter;

impl Compiler {
    /// Collect inputs for one JavaScript module.
    pub(crate) fn collect_script(
        &self,
        module: ModuleId,
        profile: ProfileId,
        target: TargetId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactDependencySet> {
        // the emit input depends on the resolved target pipeline
        let target_config =
            self.target_or_builtin(context, target)?
                .ok_or_else(|| EmitError::Internal {
                    anchor: module.into(),
                    module,
                    message: format!("target '{target}' not found"),
                })?;
        let mut dependencies = ArtifactDependencySet::default();

        if target_config.output != Output::Bundle {
            return Err(EmitError::UnsupportedTarget {
                anchor: module.into(),
                module,
                target: target.to_string(),
            }
            .into());
        }

        // declare the DIR tables consumed by JavaScript emission
        dependencies.require(ArtifactKey::dir_parsed(module));
        dependencies.require(ArtifactKey::dir_bound(module, profile));
        dependencies.require(ArtifactKey::dir_resolved(module, profile));
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
        // the emit input depends on the resolved target pipeline
        let target_config =
            self.target_or_builtin(context, target)?
                .ok_or_else(|| EmitError::Internal {
                    anchor: module.into(),
                    module,
                    message: format!("target '{target}' not found"),
                })?;
        let mut dependencies = ArtifactDependencySet::default();

        if target_config.output != Output::Program {
            return Err(EmitError::UnsupportedTarget {
                anchor: module.into(),
                module,
                target: target.to_string(),
            }
            .into());
        }

        // declare optimized code and its module dependency source
        dependencies.require(ArtifactKey::mir_lowered(module, profile, target));
        dependencies.require(ArtifactKey::mir_optimized(module, profile, target));
        dependencies.require(ArtifactKey::dir_resolved(module, profile));

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

    /// Build one JavaScript module.
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

        if target_config.output != Output::Bundle {
            return Err(EmitError::UnsupportedTarget {
                anchor: module.into(),
                module,
                target: target_name,
            }
            .into());
        }

        // require a code module
        {
            let module = self.module(context.revision(), module)?;
            if !module.is_code() {
                return Err(CompilerError::Internal {
                    message: format!(
                        "asset modules are linked directly for module '{}'",
                        module.uri
                    ),
                });
            }
        }

        // read the complete materialized DIR
        let artifacts = self.artifact_reader(context);
        let parsed = artifacts
            .read::<DirParsed>(module)
            .map_err(CompilerError::from)?;
        let bound = artifacts
            .read::<DirBound>((module, profile))
            .map_err(CompilerError::from)?;
        let resolved = artifacts
            .read::<DirResolved>((module, profile))
            .map_err(CompilerError::from)?;
        let imported = artifacts
            .read::<DirImported>((module, profile))
            .map_err(CompilerError::from)?;
        let expanded = artifacts
            .read::<DirExpanded>((module, profile))
            .map_err(CompilerError::from)?;
        let declared = artifacts
            .read::<DirDeclared>((module, profile))
            .map_err(CompilerError::from)?;
        let elaborated = artifacts
            .read::<DirElaborated>((module, profile))
            .map_err(CompilerError::from)?;
        let checked = artifacts
            .read::<DirChecked>((module, profile))
            .map_err(CompilerError::from)?;
        let materialized = artifacts
            .read::<DirMaterialized>((module, profile))
            .map_err(CompilerError::from)?;

        // project the DIR tables consumed by emission
        let bindings = checked.binding_table(&bound, &expanded, &declared, &elaborated);
        let modules = expanded.module_table(&imported);
        let resolutions = checked.resolution_table(&declared, &elaborated);
        let decisions = materialized.decision_table(&declared, &elaborated, &checked);
        let types = materialized.type_table(&bound, &expanded, &declared, &elaborated, &checked);
        let view = expanded.view(&parsed);

        // emit one JavaScript module and its provenance
        let mut provenance = materialized.provenance.extend();
        let journal = provenance.record("emit-javascript");
        let mut output = ScriptEmitter::new(
            view,
            materialized.roots.as_ref(),
            self.repository.string_pool(),
            bindings,
            modules,
            &resolved.references,
            resolutions,
            decisions,
            types,
            &materialized.provenance,
            journal,
        )
        .emit()?;
        output.module.provenance = provenance.finish();

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
        if target_config.output != Output::Program {
            return Err(EmitError::UnsupportedTarget {
                anchor: module.into(),
                module,
                target: target_name,
            }
            .into());
        }

        // preserve optimized MIR and its direct module dependencies
        let artifacts = self.artifact_reader(context);
        let lowered = artifacts
            .read::<MirLowered>((module, profile, target))
            .map_err(CompilerError::from)?;
        let optimized = artifacts
            .read::<MirOptimized>((module, profile, target))
            .map_err(CompilerError::from)?;
        let resolved = artifacts
            .read::<DirResolved>((module, profile))
            .map_err(CompilerError::from)?;
        let modules = resolved
            .target_modules()
            .filter(|target| *target != module)
            .collect::<Vec<_>>();
        let mut provenance = optimized.provenance.extend();
        let mut object = ObjectEmitter::new(module, &lowered, &optimized, modules)?;

        // emit every representation selected by this Program target
        for code in target_config.code.iter().copied() {
            object = match code {
                Code::Bytecode => {
                    let bytecode =
                        BytecodeEmitter::new(module, &optimized, &object).emit(&mut provenance)?;

                    object.bytecode(bytecode)
                }
                Code::Native => {
                    #[cfg(all(feature = "native", not(target_arch = "wasm32")))]
                    {
                        let native = NativeEmitter::new(
                            module,
                            lowered.target,
                            &optimized,
                            &object,
                            &target_config,
                        )?
                        .emit(&mut provenance)?;

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
        let output = object.build(provenance.finish());

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
        let asset = Asset::new(file.ty, file.blob(), Some(module.uri.clone()));

        Ok(ArtifactPayload::Asset(Arc::new(asset)))
    }
}
