use destack_artifact::{ArtifactDependencySet, ArtifactKey, ArtifactPayload, ModuleLinted};
use destack_repository::{ProfileId, ProviderContext, ProviderError};
use destack_source::{ModuleId, TargetId};

use super::{DirModule, LintSet, Linter, MirModule};
use crate::{DiagnosticControls, DiagnosticIndex};

impl Linter {
    /// Collect dependencies for one module lint artifact.
    pub(super) fn collect_module(
        &self,
        context: &dyn ProviderContext,
        module: ModuleId,
        profile: ProfileId,
        target: TargetId,
    ) -> Result<ArtifactDependencySet, ProviderError> {
        let revision = context.revision();
        let lints = self.resolve_lints(context, target.package_id())?;
        let mut dependencies = ArtifactDependencySet::default();
        self.observe_configuration(context, target.package_id(), &mut dependencies)?;
        let repository_module = self.module(revision, module)?;
        if !repository_module.is_code() {
            return Ok(dependencies);
        }

        // require checking for control validation
        dependencies.require(ArtifactKey::dir_checked(module, profile));

        // require this module's checked DIR
        if lints.has_dir_modules() {
            let Some(_) = self.collect_global_environment(context, profile, &mut dependencies)?
            else {
                return Ok(dependencies);
            };
            self.require_dir_modules(revision, &[module], profile, &mut dependencies)?;
        }

        // require this module's verified MIR
        if lints.has_mir_modules() {
            dependencies.require(ArtifactKey::mir_lowered(module, profile, target));
            dependencies.require(ArtifactKey::mir_verified(module, profile, target));
        }

        Ok(dependencies)
    }

    /// Provide one module lint artifact.
    pub(super) fn provide_module(
        &self,
        context: &dyn ProviderContext,
        module: ModuleId,
        profile: ProfileId,
        target: TargetId,
    ) -> Result<ArtifactPayload, ProviderError> {
        let revision = context.revision();
        let lints = self.resolve_lints(context, target.package_id())?;
        let repository_module = self.module(revision, module)?;
        if !repository_module.is_code() {
            return Ok(ModuleLinted.into());
        }

        // resolve controls before deciding whether any lint executes
        let artifacts = self.repository.artifact_reader(revision);
        let checked = artifacts.dir_checked(module, profile)?;
        let strings = self.repository.string_pool();
        let diagnostics = DiagnosticIndex::new(&self.check_warnings, lints.lints())?;
        let controls =
            DiagnosticControls::resolve([checked.controls.clone()], &diagnostics, strings.as_ref());
        let controls = match controls {
            Ok(controls) => controls,
            Err(error) => return self.reject(context, error),
        };

        // run DIR and MIR module lints
        self.lint_dir_module(context, &lints, &controls, module, profile, target)?;
        self.lint_mir_module(context, &lints, &controls, module, profile, target)?;

        Ok(ModuleLinted.into())
    }

    /// Execute checked DIR module lints.
    fn lint_dir_module(
        &self,
        context: &dyn ProviderContext,
        lints: &LintSet,
        controls: &DiagnosticControls,
        module: ModuleId,
        profile: ProfileId,
        target: TargetId,
    ) -> Result<(), ProviderError> {
        if !lints.has_dir_modules() {
            return Ok(());
        }

        // load this module's checked DIR
        let revision = context.revision();
        let artifacts = self.repository.artifact_reader(revision);
        let environment = artifacts.global_environment(profile)?;
        let repository_module = self.module(revision, module)?;
        let module = DirModule::load(
            self.repository.as_ref(),
            revision,
            profile,
            target,
            environment,
            repository_module,
            &artifacts,
        )?;
        let strings = self.repository.string_pool();

        // execute enabled DIR lints
        for (lint, severity, check) in lints.dir_modules() {
            if !controls.enables(lint, severity) {
                continue;
            }

            let output = check(&module, lint)?;
            let (diagnostics, errors) =
                controls.apply(lint, severity, strings.as_ref(), output.into_diagnostics());
            self.emit(context, diagnostics)?;
            self.emit(context, errors)?;
        }

        Ok(())
    }

    /// Execute verified MIR module lints.
    fn lint_mir_module(
        &self,
        context: &dyn ProviderContext,
        lints: &LintSet,
        controls: &DiagnosticControls,
        module: ModuleId,
        profile: ProfileId,
        target: TargetId,
    ) -> Result<(), ProviderError> {
        if !lints.has_mir_modules() {
            return Ok(());
        }

        // load this module's verified MIR and analyses
        let revision = context.revision();
        let artifacts = self.repository.artifact_reader(revision);
        let module = MirModule::load(&artifacts, profile, target, module)?;
        let strings = self.repository.string_pool();

        // execute enabled MIR lints
        for (lint, severity, check) in lints.mir_modules() {
            if !controls.enables(lint, severity) {
                continue;
            }

            let output = check(&module, lint)?;
            let (diagnostics, errors) =
                controls.apply(lint, severity, strings.as_ref(), output.into_diagnostics());
            self.emit(context, diagnostics)?;
            self.emit(context, errors)?;
        }

        Ok(())
    }
}
