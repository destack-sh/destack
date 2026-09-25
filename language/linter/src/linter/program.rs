use std::sync::Arc;

use tspp_artifact::{
    ArtifactDependencySet, ArtifactKey, ArtifactPayload, ArtifactProjectionKey,
    DiagnosticControlIndex, DirChecked, EnvironmentBound, ModuleGraph, ProgramLinted,
};
use tspp_core::FxIndexSet;
use tspp_repository::{
    ArtifactReader, Package, Profile, ProfileId, ProviderContext, ProviderError, Repository,
    Revision,
};
use tspp_source::{ModuleId, TargetId};

use super::{DirProgram, LintScope, LintSet, Linter, MirProgram};

/// One target program inspected by program lints.
#[derive(Debug)]
pub struct LintProgram {
    /// The target package.
    pub package: Arc<Package>,
    /// The resolved profile.
    pub profile: Arc<Profile>,
    /// The active target.
    pub target: TargetId,
    /// The target root modules.
    pub roots: Box<[ModuleId]>,
    /// The modules reachable from the target roots.
    pub(crate) modules: Box<[ModuleId]>,
}

impl LintProgram {
    /// Load one target program.
    pub(crate) fn load(
        repository: &Repository,
        revision: Revision,
        artifacts: &ArtifactReader<'_>,
        profile: ProfileId,
        target: TargetId,
    ) -> Result<Option<Self>, ProviderError> {
        let roots = repository
            .modules_for_target(revision, target)
            .map_err(|error| ProviderError::internal(error.to_string()))?;
        if roots.is_empty() {
            return Ok(None);
        }

        // load the target package and resolved profile
        let package = repository
            .package(revision, target.package_id())
            .map_err(|error| ProviderError::internal(error.to_string()))?
            .ok_or_else(|| {
                ProviderError::internal(format!("missing lint package {:?}", target.package_id()))
            })?;
        let resolved_profile = repository
            .profile_for_target(revision, target)
            .map_err(|error| ProviderError::internal(error.to_string()))?;
        if resolved_profile.id() != profile {
            return Err(ProviderError::internal(format!(
                "lint target {target:?} resolves profile {:?}, not {profile:?}",
                resolved_profile.id(),
            )));
        }

        // load the module graph and the target's import closure
        let modules = Self::load_modules(profile, &roots, artifacts)?;

        Ok(Some(Self {
            package,
            profile: resolved_profile,
            target,
            roots: roots.into_boxed_slice(),
            modules: modules.into_boxed_slice(),
        }))
    }

    /// Load the modules reachable from the roots and track their import edges.
    pub(super) fn load_modules(
        profile: ProfileId,
        roots: &[ModuleId],
        artifacts: &ArtifactReader<'_>,
    ) -> Result<Vec<ModuleId>, ProviderError> {
        let mut visited = FxIndexSet::default();
        let mut pending = roots.to_vec();

        // walk import edges through the graph of each module's package, tracking every edge list
        while let Some(module) = pending.pop() {
            if !visited.insert(module) {
                continue;
            }

            let edges =
                artifacts.project::<ModuleGraph, _, _>((module.package_id, profile), |graph| {
                    (
                        graph.edges(module),
                        [ArtifactProjectionKey::ModuleGraphEdges(module)],
                    )
                })?;
            let edges = edges.ok_or_else(|| {
                ProviderError::internal(format!("no module graph holds module '{module}'"))
            })?;
            pending.extend(edges.iter().copied());
        }

        // order the reached modules for a stable dependency set
        let mut modules = visited.into_iter().collect::<Vec<_>>();
        modules.sort_unstable();

        Ok(modules)
    }

    /// Return whether the target package owns one module.
    pub fn owns(&self, module: ModuleId) -> bool {
        module.package_id == self.package.id
    }

    /// Iterate module ids reachable from the target roots.
    pub fn module_ids(&self) -> impl Iterator<Item = ModuleId> + '_ {
        self.modules.iter().copied()
    }

    /// Iterate package-owned module ids.
    pub fn owned_module_ids(&self) -> impl Iterator<Item = ModuleId> + '_ {
        self.module_ids().filter(|module| self.owns(*module))
    }
}

impl Linter {
    /// Collect dependencies for one program lint artifact.
    pub(super) fn collect_program(
        &self,
        context: &dyn ProviderContext,
        profile: ProfileId,
        target: TargetId,
    ) -> Result<ArtifactDependencySet, ProviderError> {
        let revision = context.revision();
        let mut dependencies = ArtifactDependencySet::default();
        self.observe_configuration(context, target.package_id(), &mut dependencies)?;
        let lints = self.resolve_lints(context, target.package_id())?;
        if !lints.has_programs() {
            return Ok(dependencies);
        }

        // collect target roots for selected program implementations
        let roots = self
            .repository
            .modules_for_target(revision, target)
            .map_err(|error| ProviderError::internal(error.to_string()))?;
        if roots.is_empty() {
            return Ok(dependencies);
        }

        // read the root edges before walking the reachable graph
        for root in roots.iter().copied() {
            dependencies.require_projection(
                ArtifactKey::module_graph(root.package_id, profile),
                ArtifactProjectionKey::ModuleGraphEdges(root),
            );
        }
        let artifacts = self.artifact_reader(context);
        let program_modules = match LintProgram::load_modules(profile, &roots, &artifacts) {
            Ok(modules) => modules,
            Err(ProviderError::Blocked { .. }) => {
                dependencies.mark_partial();

                return Ok(dependencies);
            }
            Err(error) => return Err(error),
        };

        // project the program's import closure from the target roots
        for module in program_modules.iter().copied() {
            dependencies.require_projection(
                ArtifactKey::module_graph(module.package_id, profile),
                ArtifactProjectionKey::ModuleGraphEdges(module),
            );
        }
        let mut required = program_modules.iter().copied().collect::<FxIndexSet<_>>();

        // require controls from package-owned code modules
        for module in program_modules.iter().copied() {
            if module.package_id != target.package_id() {
                continue;
            }

            let repository_module = self.module(revision, module)?;
            if repository_module.is_code() {
                dependencies.require(ArtifactKey::dir_declared(module, profile));
                dependencies.require(ArtifactKey::dir_checked(module, profile));
                dependencies.require(ArtifactKey::dir_materialized(module, profile));
            }
        }

        // require DIR for DIR program lints and the MIR lints reading through it
        if lints.has_programs() {
            let Some(environment) =
                self.collect_environment_bound(context, profile, &mut dependencies)?
            else {
                return Ok(dependencies);
            };
            let mut graph_roots = roots.clone();
            graph_roots.extend(environment.globals.iter().copied());
            graph_roots.sort_unstable();
            graph_roots.dedup();

            // project modules introduced by compiler globals
            let modules = match LintProgram::load_modules(profile, &graph_roots, &artifacts) {
                Ok(modules) => modules,
                Err(ProviderError::Blocked { .. }) => {
                    dependencies.mark_partial();

                    return Ok(dependencies);
                }
                Err(error) => return Err(error),
            };
            for module in modules.iter().copied() {
                if required.insert(module) {
                    dependencies.require_projection(
                        ArtifactKey::module_graph(module.package_id, profile),
                        ArtifactProjectionKey::ModuleGraphEdges(module),
                    );
                }
            }

            self.require_dir_modules(revision, &modules, profile, &mut dependencies)?;
            let indexes = lints.dir_indexes(LintScope::Program).collect::<Vec<_>>();
            for module in program_modules.iter().copied() {
                if module.package_id != target.package_id()
                    || !self.module(revision, module)?.is_code()
                {
                    continue;
                }
                for kind in indexes.iter().copied() {
                    dependencies.require(ArtifactKey::module_index(module, profile, kind));
                }
            }
        }

        // require verified MIR and program analysis for MIR program lints
        if lints.has_mir_programs() {
            self.require_mir_modules(
                revision,
                &program_modules,
                profile,
                target,
                &mut dependencies,
            )?;
            dependencies.require(ArtifactKey::program_analysis(profile, target));
        }

        Ok(dependencies)
    }

    /// Provide one program lint artifact.
    pub(super) fn provide_program(
        &self,
        context: &dyn ProviderContext,
        profile: ProfileId,
        target: TargetId,
    ) -> Result<ArtifactPayload, ProviderError> {
        let revision = context.revision();

        // reject configuration before loading program artifacts
        let mut lints = self.resolve_lints(context, target.package_id())?;
        self.reject_invalid_lints(context, &lints)?;
        if !lints.has_programs() {
            return Ok(ProgramLinted.into());
        }

        // load the target program
        let artifacts = self.artifact_reader(context);
        let Some(program) = LintProgram::load(
            self.repository.as_ref(),
            revision,
            &artifacts,
            profile,
            target,
        )?
        else {
            return Ok(ProgramLinted.into());
        };
        let program = Arc::new(program);

        // resolve controls from package-owned code modules
        let mut control_tables = Vec::new();
        for module in program.owned_module_ids() {
            if !self.module(revision, module)?.is_code() {
                continue;
            }

            let checked = artifacts.read::<DirChecked>((module, profile))?;
            control_tables.push(checked.controls.clone());
        }
        let controls = DiagnosticControlIndex::new(control_tables.iter().map(Arc::as_ref))
            .map_err(|error| ProviderError::internal(error.to_string()))?;
        lints.retain_active(&controls);
        if !lints.has_programs() {
            return Ok(ProgramLinted.into());
        }

        // execute both program representations
        self.lint_dir_program(context, &lints, &controls, program.clone())?;
        self.lint_mir_program(context, &lints, &controls, program)?;

        Ok(ProgramLinted.into())
    }

    /// Execute DIR program lints.
    fn lint_dir_program(
        &self,
        context: &dyn ProviderContext,
        lints: &LintSet,
        controls: &DiagnosticControlIndex<'_>,
        program: Arc<LintProgram>,
    ) -> Result<(), ProviderError> {
        if !lints.has_dir_programs() {
            return Ok(());
        }

        // load DIR for the target modules and globals
        let revision = context.revision();
        let artifacts = self.artifact_reader(context);
        let profile = program.profile.id();
        let environment = artifacts.read::<EnvironmentBound>(profile)?;
        let mut roots = program.roots.to_vec();
        roots.extend(environment.globals.iter().copied());
        roots.sort_unstable();
        roots.dedup();
        let module_ids = LintProgram::load_modules(profile, &roots, &artifacts)?;
        let indexed_modules = program.owned_module_ids().collect::<Vec<_>>();
        let indexes = lints.dir_indexes(LintScope::Program).collect::<Vec<_>>();
        let program = DirProgram::load(
            self.repository.as_ref(),
            revision,
            &artifacts,
            program,
            environment,
            &module_ids,
            &indexed_modules,
            &indexes,
        )?;
        let strings = &program.dir.strings;

        // execute enabled DIR program lints
        for (lint, severity, check) in lints.dir_programs() {
            let output = check(&program, lint)?;
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

    /// Execute verified MIR program lints.
    fn lint_mir_program(
        &self,
        context: &dyn ProviderContext,
        lints: &LintSet,
        controls: &DiagnosticControlIndex<'_>,
        program: Arc<LintProgram>,
    ) -> Result<(), ProviderError> {
        if !lints.has_mir_programs() {
            return Ok(());
        }

        // load verified MIR and whole-program analysis
        let revision = context.revision();
        let artifacts = self.artifact_reader(context);
        let mut program =
            MirProgram::load(self.repository.as_ref(), revision, &artifacts, program)?;
        let strings = program.mir.strings.clone();

        // execute enabled MIR program lints
        for (lint, severity, check) in lints.mir_programs() {
            let output = check(&mut program, lint)?;
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
