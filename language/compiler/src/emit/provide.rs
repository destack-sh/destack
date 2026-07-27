use std::sync::Arc;

use destack_artifact::{ArtifactDependencySet, ArtifactKey, ArtifactPayload, Asset};
use destack_repository::{ProfileId, ProviderContext};
use destack_source::{ModuleId, TargetId};

use crate::{Compiler, CompilerError, CompilerResult, EmitError};

use super::ObjectEmitter;
use super::bytecode::BytecodeEmitter;

impl Compiler {
    /// Collect inputs for one structured script.
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

        if !target_config.emit.is_script() {
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
        // the emit input depends on the resolved target pipeline
        let target_config =
            self.target_or_builtin(context, target)?
                .ok_or_else(|| EmitError::Internal {
                    anchor: module.into(),
                    module,
                    message: format!("target '{target}' not found"),
                })?;
        let mut dependencies = ArtifactDependencySet::default();

        if !target_config.emit.is_program() {
            return Err(EmitError::UnsupportedTarget {
                anchor: module.into(),
                module,
                target: target.to_string(),
            }
            .into());
        }

        // declare optimized code and its module dependency source
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

        if !target_config.emit.is_script() {
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
        if !target_config.emit.is_program() {
            return Err(EmitError::UnsupportedTarget {
                anchor: module.into(),
                module,
                target: target_name,
            }
            .into());
        }

        // preserve optimized MIR and its direct module dependencies
        let artifacts = self.artifact_reader(context);
        let mir = artifacts
            .mir_optimized(module, profile, target)
            .map_err(CompilerError::from)?;
        let resolved = artifacts
            .dir_resolved(module, profile)
            .map_err(CompilerError::from)?;
        let modules = resolved
            .target_modules()
            .filter(|target| *target != module)
            .collect::<Vec<_>>();
        let object = ObjectEmitter::new(module, &mir, modules)?;
        let (bytecode, frames) = BytecodeEmitter::new(module, &mir, &object).emit()?;
        let output = object.build(bytecode, frames);

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
        let content = self
            .repository
            .intern_content(file.content.payload().clone())?;
        let asset = Asset::new(file.ty, content, Some(module.uri.clone()), None);

        Ok(ArtifactPayload::Asset(Arc::new(asset)))
    }
}
