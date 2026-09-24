use std::sync::Arc;

use destack_artifact::{
    ArtifactDependencySet, ArtifactKey, ArtifactPayload, DirDeclared, EnvironmentBound,
    EnvironmentDeclared,
};
use destack_dir as dir;
use destack_repository::{ArtifactAttemptRecorder, ProfileId, ProviderContext, ProviderError};
use destack_source::ModuleId;

pub(crate) use super::state::Pass;
use crate::sema::{CheckModuleState, CheckState, ExternalModuleTable};
use crate::{Compiler, CompilerError, CompilerResult};

impl Compiler {
    /// Collect inputs for one DIR stage of one module.
    pub(crate) fn collect_dir_stage(
        &self,
        module: ModuleId,
        profile: ProfileId,
        pass: Pass,
    ) -> CompilerResult<ArtifactDependencySet> {
        // require every stage ahead of the pass's own
        let own = match pass {
            Pass::Declare => ArtifactKey::dir_declared(module, profile),
            Pass::Elaborate => ArtifactKey::dir_elaborated(module, profile),
            Pass::Check => ArtifactKey::dir_checked(module, profile),
            Pass::Materialize => ArtifactKey::dir_materialized(module, profile),
            Pass::Analyze => ArtifactKey::dir_analyzed(module, profile),
        };
        let mut dependencies = ArtifactDependencySet::default();
        for key in ArtifactKey::dir_stages(module, profile)
            .into_iter()
            .take_while(|key| *key != own)
        {
            dependencies.require(key);
        }
        dependencies.require_payload(ArtifactKey::environment_bound(profile));
        if pass != Pass::Declare {
            dependencies.require_payload(ArtifactKey::environment_declared(profile));
        }

        Ok(dependencies)
    }

    /// Provide one DIR stage of one module.
    pub(crate) fn provide_dir_stage(
        &self,
        module: ModuleId,
        profile: ProfileId,
        context: &dyn ProviderContext,
        pass: Pass,
    ) -> CompilerResult<ArtifactPayload> {
        // read the profile's environment
        let artifacts = self.artifact_reader(context);
        let global = artifacts
            .read::<EnvironmentBound>(profile)
            .map_err(CompilerError::from)?;
        let declared_environment = match pass {
            Pass::Declare => None,
            Pass::Elaborate | Pass::Check | Pass::Materialize | Pass::Analyze => Some(
                artifacts
                    .read::<EnvironmentDeclared>(profile)
                    .map_err(CompilerError::from)?,
            ),
        };
        let environment = self.environment(context.revision())?;

        // load the module's committed stages and run the pass over them
        let recorder = context.recorder();
        let (view, types) = ArtifactAttemptRecorder::breakdown_maybe(recorder, "load", || {
            let view = pass.read_stages(&artifacts, (module, profile))?;
            let types = view.types().clone();

            Ok::<_, CompilerError>((view, types))
        })?;
        let lists = dir::TypeListArena::following(view_latest(&types));
        let externals = ExternalModuleTable::default();
        let state = CheckModuleState::load(
            self,
            context.revision(),
            module,
            profile,
            view,
            &types,
            &lists,
        )?;
        let mut check = CheckState::new(
            self,
            context,
            &artifacts,
            profile,
            global,
            declared_environment,
            environment,
            state,
            &externals,
            pass,
            !matches!(pass, Pass::Materialize | Pass::Analyze) && context.records_events(),
        );
        ArtifactAttemptRecorder::breakdown_maybe(recorder, "run", || match pass {
            Pass::Declare => check.run_declare(),
            Pass::Elaborate => check.run_elaborate(),
            Pass::Check => check.run_check(),
            Pass::Materialize => check.run_materialize(),
            Pass::Analyze => check.run_analyze(),
        })?;

        // record solver counters and detailed events
        check.record_trace(context);

        // package the stage's DIR tables and report the pass's diagnostics
        Ok(match pass {
            Pass::Declare => {
                let (mut declared, diagnostics) = check.finish_declare(module)?;
                finish_lists(lists, &mut declared.types);
                context.emit_diagnostics(diagnostics);

                ArtifactPayload::DirDeclared(Arc::new(declared))
            }
            Pass::Elaborate => {
                let (mut elaborated, diagnostics) = check.finish_elaborate(module)?;
                finish_lists(lists, &mut elaborated.types);
                context.emit_diagnostics(diagnostics);

                ArtifactPayload::DirElaborated(Arc::new(elaborated))
            }
            Pass::Check => {
                let (mut checked, diagnostics) = check.finish_check(module)?;
                finish_lists(lists, &mut checked.types);
                context.emit_diagnostics(diagnostics);

                ArtifactPayload::DirChecked(Arc::new(checked))
            }
            Pass::Materialize => {
                let (mut materialized, diagnostics) = check.into_materialized()?;
                finish_lists(lists, &mut materialized.types);
                context.emit_diagnostics(diagnostics);

                ArtifactPayload::DirMaterialized(Arc::new(materialized))
            }
            Pass::Analyze => {
                let (mut analyzed, diagnostics) = check.into_analyzed()?;
                finish_lists(lists, &mut analyzed.types);
                context.emit_diagnostics(diagnostics);

                ArtifactPayload::DirAnalyzed(Arc::new(analyzed))
            }
        })
    }

    /// Collect inputs for the declared environment of one profile.
    pub(crate) fn collect_environment_declared(
        &self,
        profile: ProfileId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactDependencySet> {
        let mut dependencies = ArtifactDependencySet::default();
        dependencies.require_payload(ArtifactKey::environment_bound(profile));

        // implicit module declarations back every index
        let artifacts = self.artifact_reader(context);
        let environment = match artifacts.read::<EnvironmentBound>(profile) {
            Ok(environment) => environment,
            Err(ProviderError::Blocked { .. }) => {
                dependencies.mark_partial();

                return Ok(dependencies);
            }
            Err(error) => return Err(error.into()),
        };

        // require the declarations of every builtin and implicit module
        for module in self.environment_modules(context, &environment)? {
            dependencies.require_payload(ArtifactKey::dir_declared(module, profile));
        }

        Ok(dependencies)
    }

    /// Return the builtin package's code modules with the environment's implicit modules.
    fn environment_modules(
        &self,
        context: &dyn ProviderContext,
        environment: &EnvironmentBound,
    ) -> CompilerResult<Vec<ModuleId>> {
        let mut modules = environment.implicit_modules();
        for module in self.repository.builtin_module_ids(context.revision())? {
            let Some(loaded) = self.repository.module(context.revision(), module)? else {
                continue;
            };
            if loaded.is_code() {
                modules.push(module);
            }
        }
        modules.sort_unstable();
        modules.dedup();

        Ok(modules)
    }

    /// Build the declared environment for one profile.
    pub(crate) fn provide_environment_declared(
        &self,
        profile: ProfileId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactPayload> {
        // read the profile's bound environment
        let artifacts = self.artifact_reader(context);
        let bound = artifacts
            .read::<EnvironmentBound>(profile)
            .map_err(CompilerError::from)?;

        // union every builtin and implicit module's declared indexes
        let mut environment = EnvironmentDeclared::default();
        for module in self.environment_modules(context, &bound)? {
            let declared = artifacts
                .read::<DirDeclared>((module, profile))
                .map_err(CompilerError::from)?;
            for (symbol, definition) in declared.definitions.iter_definitions() {
                // index extension declarations by their resolved target
                let dir::Definition::Extension(extension) = definition else {
                    continue;
                };
                match extension.target.root() {
                    Some(root) => {
                        environment
                            .extensions_by_root
                            .entry(root)
                            .or_default()
                            .push(symbol);
                    }
                    None => environment.blanket_extensions.push(symbol),
                }
            }
        }

        Ok(ArtifactPayload::EnvironmentDeclared(Arc::new(environment)))
    }
}

/// Return the latest committed type segment of one table, the one a pass's lists continue.
fn view_latest<'t>(types: &'t dir::TypeTable<'static>) -> &'t dir::TypeSegment {
    types
        .segments()
        .last()
        .unwrap_or_else(|| unreachable!("a DIR view without type segments"))
}

/// Move the lists one pass interned into the type segment the pass finished.
fn finish_lists(lists: dir::TypeListArena, types: &mut Arc<dir::TypeSegment>) {
    let types =
        Arc::get_mut(types).unwrap_or_else(|| unreachable!("a finished type segment is unshared"));
    lists.finish_into(types);
}
