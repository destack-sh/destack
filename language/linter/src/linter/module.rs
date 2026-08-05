use std::sync::Arc;

use destack_artifact::{
    ArtifactDependencySet, ArtifactKey, ArtifactPayload, ArtifactProjectionKey,
    DiagnosticControlIndex, ModuleLinted,
};
use destack_repository::{ProfileId, ProviderContext, ProviderError};
use destack_source::{ModuleId, TargetId};

use super::{Dir, LintSet, Linter, Mir};

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

        // require checking before resolving source controls
        dependencies.require(ArtifactKey::dir_declared(module, profile));
        dependencies.require(ArtifactKey::dir_checked(module, profile));
        let artifacts = self.artifact_reader(context);
        let checked = match artifacts.dir_checked(module, profile) {
            Ok(checked) => checked,
            Err(ProviderError::Blocked { .. }) => {
                dependencies.mark_partial();

                return Ok(dependencies);
            }
            Err(error) => return Err(error),
        };
        let control_tables = [checked.controls.clone()];
        let controls = DiagnosticControlIndex::new(control_tables.iter().map(Arc::as_ref))
            .map_err(|error| ProviderError::internal(error.to_string()))?;
        let lints = self.resolve_lints(context, target.package_id(), &controls)?;

        // require this module's checked DIR
        if lints.has_dir_modules() {
            let Some(environment) =
                self.collect_environment_bound(context, profile, &mut dependencies)?
            else {
                return Ok(dependencies);
            };

            // read the module graph, requiring the root edges first so a
            //  blocked read schedules the graph
            let graph_key = ArtifactKey::module_graph(profile);
            dependencies.require_projection(graph_key, ArtifactProjectionKey::ModuleEdges(module));
            let mut roots = environment.globals.clone();
            roots.push(module);
            roots.sort_unstable();
            roots.dedup();
            let graph = match artifacts.module_graph_reader(profile) {
                Ok(graph) => graph,
                Err(ProviderError::Blocked { .. }) => {
                    dependencies.mark_partial();

                    return Ok(dependencies);
                }
                Err(error) => return Err(error),
            };

            // project the reachable checked DIR modules
            let modules = graph.reachable(&roots)?;
            for module in modules.iter().copied() {
                dependencies
                    .require_projection(graph_key, ArtifactProjectionKey::ModuleEdges(module));
            }
            self.require_dir_modules(revision, &modules, profile, &mut dependencies)?;
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

        // resolve controls before deciding whether any lint executes
        let artifacts = self.artifact_reader(context);
        let checked = artifacts.dir_checked(module, profile)?;
        let control_tables = [checked.controls.clone()];
        let controls = DiagnosticControlIndex::new(control_tables.iter().map(Arc::as_ref))
            .map_err(|error| ProviderError::internal(error.to_string()))?;
        let lints = self.resolve_lints(context, target.package_id(), &controls)?;

        // run DIR and MIR module lints
        self.lint_dir_module(context, &lints, &controls, module, profile)?;
        self.lint_mir_module(context, &lints, &controls, module, profile, target)?;

        Ok(ModuleLinted.into())
    }

    /// Execute checked DIR module lints.
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

        // collect the module and global roots
        let revision = context.revision();
        let artifacts = self.artifact_reader(context);
        let environment = artifacts.environment_bound(profile)?;
        let graph = artifacts.module_graph_reader(profile)?;
        let mut roots = environment.globals.clone();
        roots.push(module);
        roots.sort_unstable();
        roots.dedup();

        // collect the reachable checked DIR modules
        let modules = graph.reachable(&roots)?;

        // load the reachable checked DIR modules
        let dir = Dir::load(
            self.repository.as_ref(),
            revision,
            &artifacts,
            profile,
            environment,
            &modules,
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
        let artifacts = self.artifact_reader(context);
        let strings = self.repository.string_pool().clone();
        let mut mir = Mir::load(&artifacts, profile, target, &[module], strings)?;
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
