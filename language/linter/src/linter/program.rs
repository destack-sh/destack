use std::collections::VecDeque;
use std::sync::Arc;

use destack_artifact::{
    ArtifactDependencySet, ArtifactKey, ArtifactPayload, ComponentGraph, ComponentGraphProjection,
    ProgramLinted,
};
use destack_core::FxIndexSet;
use destack_repository::{
    ArtifactReader, Package, Profile, ProfileId, ProviderContext, ProviderError, Repository,
    Revision,
};
use destack_source::{ModuleId, TargetId};

use super::{DirProgram, LintSet, Linter, MirProgram};
use crate::{DiagnosticControls, DiagnosticIndex};

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
    /// The module graph.
    pub graph: Arc<ComponentGraph>,
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

        // load the checked module graph and target closure
        let graph = artifacts.component_graph(profile)?;
        let modules = Self::reachable_modules(&graph, &roots)?;

        Ok(Some(Self {
            package,
            profile: resolved_profile,
            target,
            roots: roots.into_boxed_slice(),
            modules,
            graph,
        }))
    }

    /// Return whether the target package owns one reachable module.
    pub fn owns(&self, module: ModuleId) -> bool {
        module.package_id == self.package.id && self.modules.binary_search(&module).is_ok()
    }

    /// Iterate module ids reachable from the target roots.
    pub fn module_ids(&self) -> impl Iterator<Item = ModuleId> + '_ {
        self.modules.iter().copied()
    }

    /// Iterate package-owned module ids.
    pub fn owned_module_ids(&self) -> impl Iterator<Item = ModuleId> + '_ {
        self.module_ids().filter(|module| self.owns(*module))
    }

    /// Return modules reachable from the roots.
    pub(super) fn reachable_modules(
        graph: &ComponentGraph,
        roots: &[ModuleId],
    ) -> Result<Box<[ModuleId]>, ProviderError> {
        let mut seen = FxIndexSet::default();
        let mut pending = VecDeque::from_iter(roots.iter().copied());

        // walk the module graph
        while let Some(module) = pending.pop_front() {
            if !graph.contains_module(module) {
                return Err(ProviderError::Internal {
                    message: format!("lint module {module:?} is missing from its component graph"),
                });
            }
            if !seen.insert(module) {
                continue;
            }

            pending.extend(graph.edges(module).iter().copied());
        }

        // stabilize module order
        let mut modules = seen.into_iter().collect::<Vec<_>>();
        modules.sort_unstable();

        Ok(modules.into_boxed_slice())
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
        let lints = self.resolve_lints(context, target.package_id())?;
        let mut dependencies = ArtifactDependencySet::default();
        self.observe_configuration(context, target.package_id(), &mut dependencies)?;
        if !lints.has_programs() {
            return Ok(dependencies);
        }

        let roots = self
            .repository
            .modules_for_target(revision, target)
            .map_err(|error| ProviderError::internal(error.to_string()))?;
        if roots.is_empty() {
            return Ok(dependencies);
        }

        // collect globals needed by DIR program lints
        let environment = if lints.has_dir_programs() {
            let Some(environment) =
                self.collect_global_environment(context, profile, &mut dependencies)?
            else {
                return Ok(dependencies);
            };

            Some(environment)
        } else {
            None
        };

        // collect the target module graph
        let mut graph_roots = roots.clone();
        if let Some(environment) = environment.as_deref() {
            graph_roots.extend(environment.globals.iter().copied());
            graph_roots.sort_unstable();
            graph_roots.dedup();
        }
        let graph_key = ArtifactKey::component_graph(profile);
        for root in &graph_roots {
            dependencies.project(graph_key, ComponentGraphProjection::ComponentOf(*root));
        }

        // resolve the projected components before declaring program inputs
        let artifacts = self.repository.artifact_reader(revision);
        let graph = match artifacts.component_graph(profile) {
            Ok(graph) => graph,
            Err(ProviderError::Blocked { .. }) => {
                dependencies.mark_partial();

                return Ok(dependencies);
            }
            Err(error) => return Err(error),
        };

        // seed the components reached by the program roots
        let mut components = FxIndexSet::default();
        let mut pending = VecDeque::new();
        for root in &graph_roots {
            let component = graph.component(*root).ok_or_else(|| {
                ProviderError::internal(format!(
                    "lint root {root:?} is missing from its checked component graph"
                ))
            })?;
            pending.push_back(component);
        }

        // project members and outgoing edges for every reachable component
        while let Some(component) = pending.pop_front() {
            if !components.insert(component) {
                continue;
            }

            dependencies.project(graph_key, ComponentGraphProjection::Members(component));
            dependencies.project(graph_key, ComponentGraphProjection::Dependencies(component));
            pending.extend(graph.dependencies(component).iter().copied());
        }

        let program_modules = LintProgram::reachable_modules(&graph, &roots)?;

        // require module linting and control validation
        for module in program_modules.iter().copied() {
            if module.package_id != target.package_id() {
                continue;
            }

            let repository_module = self.module(revision, module)?;
            if repository_module.is_code() {
                dependencies.require(ArtifactKey::module_linted(module, profile, target));
            }
        }

        // require checked DIR for DIR program lints
        if environment.is_some() {
            let modules = LintProgram::reachable_modules(&graph, &graph_roots)?;
            self.require_dir_modules(revision, &modules, profile, &mut dependencies)?;
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
        let lints = self.resolve_lints(context, target.package_id())?;
        if !lints.has_programs() {
            return Ok(ProgramLinted.into());
        }

        // load the target program
        let artifacts = self.repository.artifact_reader(revision);
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

            let checked = artifacts.dir_checked(module, profile)?;
            control_tables.push(checked.controls.clone());
        }
        let strings = self.repository.string_pool();
        let diagnostics = DiagnosticIndex::new(&self.check_warnings, lints.lints())?;
        let controls = DiagnosticControls::resolve(control_tables, &diagnostics, strings.as_ref());
        let controls = match controls {
            Ok(controls) => controls,
            Err(error) => return self.reject(context, error),
        };

        // execute both program representations
        self.lint_dir_program(context, &lints, &controls, program.clone())?;
        self.lint_mir_program(context, &lints, &controls, program)?;

        Ok(ProgramLinted.into())
    }

    /// Execute checked DIR program lints.
    fn lint_dir_program(
        &self,
        context: &dyn ProviderContext,
        lints: &LintSet,
        controls: &DiagnosticControls,
        program: Arc<LintProgram>,
    ) -> Result<(), ProviderError> {
        if !lints.has_dir_programs() {
            return Ok(());
        }

        // load checked DIR for the target closure and globals
        let revision = context.revision();
        let artifacts = self.repository.artifact_reader(revision);
        let profile = program.profile.id();
        let environment = artifacts.global_environment(profile)?;
        let mut roots = program.roots.to_vec();
        roots.extend(environment.globals.iter().copied());
        roots.sort_unstable();
        roots.dedup();
        let module_ids = LintProgram::reachable_modules(&program.graph, &roots)?;
        let dir = DirProgram::load(
            self.repository.as_ref(),
            revision,
            &artifacts,
            program,
            environment,
            &module_ids,
        )?;
        let strings = self.repository.string_pool();

        // execute enabled DIR program lints
        for (lint, severity, check) in lints.dir_programs() {
            if !controls.enables(lint, severity) {
                continue;
            }

            let output = check(&dir, lint)?;
            let (diagnostics, errors) =
                controls.apply(lint, severity, strings.as_ref(), output.into_diagnostics());
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
        controls: &DiagnosticControls,
        program: Arc<LintProgram>,
    ) -> Result<(), ProviderError> {
        if !lints.has_mir_programs() {
            return Ok(());
        }

        // load verified MIR and whole-program analysis
        let revision = context.revision();
        let artifacts = self.repository.artifact_reader(revision);
        let mir = MirProgram::load(self.repository.as_ref(), revision, &artifacts, program)?;
        let strings = self.repository.string_pool();

        // execute enabled MIR program lints
        for (lint, severity, check) in lints.mir_programs() {
            if !controls.enables(lint, severity) {
                continue;
            }

            let output = check(&mir, lint)?;
            let (diagnostics, errors) =
                controls.apply(lint, severity, strings.as_ref(), output.into_diagnostics());
            self.emit(context, diagnostics)?;
            self.emit(context, errors)?;
        }

        Ok(())
    }
}
