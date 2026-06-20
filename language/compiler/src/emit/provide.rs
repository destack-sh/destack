use crate::{Compiler, CompilerResult, EmitError};
use destack_artifact::{ArtifactDependencySet, ArtifactKey, ArtifactPayload, Asset};
use destack_repository::ProviderContext;
use std::sync::Arc;

use destack_repository::ProfileId;
use destack_source::{ModuleId, TargetId};

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

        // declare the DIR tables used by JS emit
        if target_config.uses_js_emit_pipeline() {
            dependencies.require(ArtifactKey::dir_parsed(module));
            dependencies.require(ArtifactKey::dir_bound(module, profile));
            dependencies.require(ArtifactKey::dir_imported(module, profile));
            dependencies.require(ArtifactKey::dir_expanded(module, profile));
            dependencies.require(ArtifactKey::dir_checked(module, profile));
        }
        // reject unsupported script pipelines
        else {
            let input = self.script_input(module, profile, &target, &target_config)?;
            dependencies.require(input);
        }

        self.observe_package_config(context, target.package_id(), &mut dependencies)?;

        Ok(dependencies)
    }

    /// Collect inputs for one compiled-code object.
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

        // declare the MIR tables used by native emit
        if target_config.uses_native_emit_pipeline() {
            dependencies.require(ArtifactKey::mir_optimized(module, profile, target));
            dependencies.require(ArtifactKey::mir_lowered(module, profile, target));
        }
        // reject unsupported object pipelines
        else {
            let input = self.object_input(module, profile, &target, &target_config)?;
            dependencies.require(input);
        }

        self.observe_package_config(context, target.package_id(), &mut dependencies)?;

        Ok(dependencies)
    }

    /// Collect inputs for one opaque asset.
    pub(crate) fn collect_asset(
        &self,
        module: ModuleId,
        profile: ProfileId,
        target: TargetId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactDependencySet> {
        // the asset input depends on the source payload
        let target_config =
            self.target_or_builtin(context, target)?
                .ok_or_else(|| EmitError::Internal {
                    anchor: module.into(),
                    module,
                    message: format!("target '{target}' not found"),
                })?;
        let input = self.asset_input(module, profile, &target, &target_config)?;

        let mut dependencies = ArtifactDependencySet::default();
        dependencies.require(input);
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
        let resolved_profile = self.profile_id_for_target(context.revision(), module, &target)?;
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

        // emit script from the declared input
        let artifacts = self.artifact_reader(context.revision());
        let output = self.emit_target_script(
            module,
            profile,
            &target_config,
            &target_name,
            context,
            &artifacts,
        )?;

        Ok(ArtifactPayload::Script(Arc::new(output)))
    }

    /// Build one compiled-code object.
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
        let resolved_profile = self.profile_id_for_target(context.revision(), module, &target)?;
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

        // emit object from the declared input
        let artifacts = self.artifact_reader(context.revision());
        let output = self.emit_target_object(
            module,
            profile,
            &target,
            &target_config,
            &target_name,
            context,
            &artifacts,
        )?;

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
