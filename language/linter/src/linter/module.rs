use std::sync::Arc;

use destack_artifact::{
    ArtifactDependencySet, ArtifactKey, ArtifactPayload, ArtifactProjectionKey,
    DiagnosticControlIndex, DirChecked, EnvironmentBound, ModuleLinted,
};
use destack_repository::{ProfileId, ProviderContext, ProviderError};
use destack_source::{ModuleId, TargetId};

use super::{Dir, LintProgram, LintScope, LintSet, Linter, Mir};

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
        let mut dependencies = ArtifactDependencySet::default();
        self.observe_configuration(context, target.package_id(), &mut dependencies)?;
        let repository_module = self.module(revision, module)?;
        if !repository_module.is_code() {
            return Ok(dependencies);
        }

        // resolve selected module implementations
        let lints = self.resolve_lints(context, target.package_id())?;
        if !lints.has_modules() {
            return Ok(dependencies);
        }

        // require controls for every selected module lint
        dependencies.require(ArtifactKey::dir_declared(module, profile));
        dependencies.require(ArtifactKey::dir_checked(module, profile));

        // require the DIR of this module and of every module its resolutions name
        if lints.has_modules() {
            dependencies.require(ArtifactKey::environment_bound(profile));
            for kind in lints.dir_indexes(LintScope::Module) {
                dependencies.require(ArtifactKey::module_index(module, profile, kind));
            }

            // read this module's edges out of the module graph
            let graph_key = ArtifactKey::module_graph(profile);
            dependencies
                .require_projection(graph_key, ArtifactProjectionKey::ModuleGraphEdges(module));
            let artifacts = self.artifact_reader(context);
            let reachable = match LintProgram::load_modules(profile, &[module], &artifacts) {
                Ok(modules) => modules,
                Err(ProviderError::Blocked { .. }) => {
                    dependencies.mark_partial();

                    return Ok(dependencies);
                }
                Err(error) => return Err(error),
            };

            // require the edges and the DIR of every reached module
            for reached in reachable.iter().copied() {
                dependencies.require_projection(
                    graph_key,
                    ArtifactProjectionKey::ModuleGraphEdges(reached),
                );
            }
            self.require_dir_modules(revision, &reachable, profile, &mut dependencies)?;
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
        let repository_module = self.module(revision, module)?;
        if !repository_module.is_code() {
            return Ok(ModuleLinted.into());
        }

        // skip controls when no module implementation is selected
        let mut lints = self.resolve_lints(context, target.package_id())?;
        if !lints.has_modules() {
            return Ok(ModuleLinted.into());
        }

        // activate selected lints from source controls
        let artifacts = self.artifact_reader(context);
        let checked = artifacts.read::<DirChecked>((module, profile))?;
        let control_tables = [checked.controls.clone()];
        let controls = DiagnosticControlIndex::new(control_tables.iter().map(Arc::as_ref))
            .map_err(|error| ProviderError::internal(error.to_string()))?;
        lints.retain_active(&controls);

        // run DIR and MIR module lints
        self.lint_dir_module(context, &lints, &controls, module, profile)?;
        self.lint_mir_module(context, &lints, &controls, module, profile, target)?;

        Ok(ModuleLinted.into())
    }

    /// Execute DIR module lints.
    fn lint_dir_module(
        &self,
        context: &dyn ProviderContext,
        lints: &LintSet,
        controls: &DiagnosticControlIndex<'_>,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<(), ProviderError> {
        if !lints.has_dir_modules() {
            return Ok(());
        }

        // load this module's DIR
        let revision = context.revision();
        let artifacts = self.artifact_reader(context);
        let environment = artifacts.read::<EnvironmentBound>(profile)?;
        let indexes = lints.dir_indexes(LintScope::Module).collect::<Vec<_>>();
        let dir = Dir::load(
            self.repository.as_ref(),
            revision,
            &artifacts,
            profile,
            environment,
            &[module],
            &[module],
            &indexes,
        )?;
        let module = dir.module(module)?;
        let strings = &dir.strings;

        // execute enabled DIR lints
        for (lint, severity, check) in lints.dir_modules() {
            let output = check(&module, lint)?;
            let (diagnostics, errors) = lint.apply_controls(
                controls,
                severity,
                strings.as_ref(),
                output.into_diagnostics(),
            );
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
        controls: &DiagnosticControlIndex<'_>,
        module: ModuleId,
        profile: ProfileId,
        target: TargetId,
    ) -> Result<(), ProviderError> {
        if !lints.has_mir_modules() {
            return Ok(());
        }

        // load this module's verified MIR and analyses
        let revision = context.revision();
        let artifacts = self.artifact_reader(context);
        let strings = self.repository.string_pool().clone();
        let mut mir = Mir::load(
            self.repository.as_ref(),
            revision,
            &artifacts,
            profile,
            target,
            &[module],
            strings,
        )?;
        let strings = mir.strings.clone();
        let module = mir.module_mut(module)?;

        // execute enabled MIR lints
        for (lint, severity, check) in lints.mir_modules() {
            let output = check(module, lint)?;
            let (diagnostics, errors) = lint.apply_controls(
                controls,
                severity,
                strings.as_ref(),
                output.into_diagnostics(),
            );
            self.emit(context, diagnostics)?;
            self.emit(context, errors)?;
        }

        Ok(())
    }
}
